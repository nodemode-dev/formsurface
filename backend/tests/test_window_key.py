import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

import pytest
from starlette.websockets import WebSocketDisconnect

from conftest import BACKEND, ENGINE_PORT, free_port, window_headers

READS = {"GET", "HEAD", "OPTIONS"}
OPEN = {("POST", "/api/agent"), ("POST", "/api/window/key")}
LOOPBACK = {"host": f"127.0.0.1:{ENGINE_PORT}"}
SENTENCE = "Only the formsurface window can do this"
REFUSED = {"detail": SENTENCE}
WRONG = {"x-formsurface-window-key": "x" * 43}
TAB = {"x-formsurface-tab": "t" * 43}
SOCKET_WAIT = 20


def receive_within(ws, seconds=SOCKET_WAIT):
    import anyio

    async def take():
        with anyio.fail_after(seconds):
            return await ws._send_rx.receive()

    message = ws.portal.call(take)
    ws._raise_on_close(message)
    return message


def write_operations(app):
    found = [(m.upper(), path) for path, item in app.openapi()["paths"].items() for m in item if m.upper() not in READS]
    found += [("POST", r.path) for r in app.routes if getattr(r, "path", "") == "/api/shutdown"]
    return found


def test_the_open_list_is_short_and_named():
    from app import window_key

    assert set(window_key.OPEN) == {"/api/agent", "/api/window/key"}


def test_every_write_route_needs_the_window_key(app, bare):
    ops = write_operations(app)
    assert len(ops) > 20
    answered = []
    for method, path in ops:
        if (method, path) in OPEN:
            continue
        url = re.sub(r"\{[^}]+\}", "x", path)
        for headers in ({}, WRONG, TAB):
            r = bare.request(method, url, json={}, headers=headers)
            if r.status_code != 403 or r.json() != REFUSED:
                answered.append(f"{method} {path} {r.status_code}")
    assert not answered, "write routes that answer without the window key: " + ", ".join(answered)


def test_a_command_with_the_key_reaches_the_route(client):
    r = client.post("/api/commands/prefs.get", json={})
    assert r.status_code == 200 and r.json()["ok"] is True


def test_an_agent_tab_with_its_header_and_the_key_is_still_refused(client):
    r = client.post("/api/commands/prefs.get", json={}, headers=TAB)
    assert r.status_code == 403


def test_the_mcp_relay_stays_open_to_outside_agents(bare):
    r = bare.post("/api/agent", json={"command": "get_state", "args": {}, "timeout": 1})
    assert r.status_code == 409


def test_the_event_stream_needs_the_key_or_a_ticket(bare, client):
    for headers in ({}, WRONG):
        assert bare.get("/api/events", headers=headers).status_code == 403
    assert bare.get("/api/events?ticket=nothing").status_code == 403
    assert bare.post("/api/window/ticket").status_code == 403


def test_a_ticket_opens_the_stream_once(client):
    from app import window_key

    ticket = client.post("/api/window/ticket").json()["ticket"]
    assert window_key.redeem(ticket) is True
    assert window_key.redeem(ticket) is False


def test_a_chat_picture_needs_the_key_or_a_ticket_spent_once(bare, client):
    from app.agent import config

    folder = config.data_dir() / "Chat"
    folder.mkdir(parents=True, exist_ok=True)
    picture = folder / "seen.png"
    picture.write_bytes(b"\x89PNG\r\n\x1a\nfs")
    url = "/api/chat/image?path=" + urllib.parse.quote(str(picture))
    for headers in ({}, WRONG):
        r = bare.get(url, headers=headers)
        assert r.status_code == 403 and r.json() == REFUSED
    assert bare.get(url, headers=TAB).status_code == 403
    assert bare.get(url + "&ticket=nothing").status_code == 403
    assert client.get(url).content == picture.read_bytes()
    ticket = client.post("/api/window/ticket").json()["ticket"]
    assert bare.get(url + "&ticket=" + ticket).content == picture.read_bytes()
    assert bare.get(url + "&ticket=" + ticket).status_code == 403


def test_bootstrap_answers_once_then_refuses(bare, monkeypatch):
    from app import window_key

    monkeypatch.setitem(window_key.STATE, "claimed", False)
    monkeypatch.setitem(window_key.STATE, "secondClaim", False)
    first = bare.post("/api/window/key")
    assert first.status_code == 200 and first.json()["key"] == window_key.KEY
    assert bare.post("/api/window/key").status_code == 409
    assert bare.get("/api/window/key").json()["secondClaim"] is True


def test_bootstrap_does_not_exist_when_the_shell_reads_the_key(bare, monkeypatch):
    from app import window_key

    monkeypatch.setenv(window_key.STDOUT_ENV, "1")
    monkeypatch.setitem(window_key.STATE, "claimed", False)
    assert bare.post("/api/window/key").status_code == 404


def test_live_client_without_the_key_does_not_replace_the_window(client):
    from app.main import relay

    with client.websocket_connect("/api/live", headers=LOOPBACK) as window:
        window.send_json({"type": "key", "key": window_headers()["x-formsurface-window-key"]})
        deadline = time.monotonic() + 5
        while relay.socket is None and time.monotonic() < deadline:
            time.sleep(0.02)
        held = relay.socket
        assert held is not None
        with pytest.raises(WebSocketDisconnect) as refused:
            with client.websocket_connect("/api/live", headers=LOOPBACK) as other:
                other.send_json({"type": "key", "key": "y" * 43})
                receive_within(other)
        assert refused.value.code == 4403
        assert relay.socket is held


def test_terminal_socket_without_the_key_cannot_drive_the_shell(client):
    term = client.post("/api/commands/terminal.open", json={"cols": 80, "rows": 24}).json()["result"]
    sid = term["id"]
    try:
        with pytest.raises(WebSocketDisconnect) as refused:
            with client.websocket_connect(f"/api/terminal/{sid}", headers=LOOPBACK) as ws:
                ws.send_json({"type": "key", "key": "y" * 43})
                ws.send_json({"type": "input", "data": "true\r"})
                receive_within(ws)
        assert refused.value.code == 4403
        with client.websocket_connect(f"/api/terminal/{sid}", headers=LOOPBACK) as ws:
            ws.send_json({"type": "key", "key": window_headers()["x-formsurface-window-key"]})
            ws.send_json({"type": "resize", "cols": 90, "rows": 30})
            assert isinstance(receive_within(ws)["text"], str)
    finally:
        client.post("/api/commands/terminal.close", json={"id": sid})


def start_engine(tmp_path, stdout_key):
    port = free_port()
    env = {k: v for k, v in os.environ.items() if not k.startswith("FORMSURFACE_")}
    env.update(FORMSURFACE_DATA=str(tmp_path), FORMSURFACE_PORT=str(port), PYTHONUNBUFFERED="1")
    if stdout_key:
        env["FORMSURFACE_WINDOW_KEY_STDOUT"] = "1"
    proc = subprocess.Popen([sys.executable, "formsurface_engine.py"], cwd=BACKEND, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    lines = []
    deadline = time.monotonic() + 60
    while not any("http://127.0.0.1:" in line for line in lines) and time.monotonic() < deadline:
        line = proc.stdout.readline()
        if not line:
            break
        lines.append(line)
    return proc, port, lines


def stop_engine(proc):
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()


def post(port, path, headers):
    request = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=b"{}", method="POST", headers={"content-type": "application/json", **headers})
    try:
        with urllib.request.urlopen(request, timeout=5) as r:
            return r.status, json.loads(r.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read().decode("utf-8"))


def test_the_engine_tells_each_kind_of_caller_apart(tmp_path):
    proc, port, lines = start_engine(tmp_path, True)
    try:
        keyed = [line for line in lines if line.startswith("formsurface-window-key: ")]
        assert len(keyed) == 1, "".join(lines)
        key = keyed[0].split(": ", 1)[1].strip()
        assert len(key) >= 32
        assert lines.index(keyed[0]) < len(lines) - 1
        window = post(port, "/api/commands/prefs.get", {"x-formsurface-window-key": key})
        assert window[0] == 200 and window[1]["ok"] is True
        assert post(port, "/api/commands/prefs.get", {}) == (403, REFUSED)
        assert post(port, "/api/commands/prefs.get", TAB) == (403, REFUSED)
        assert post(port, "/api/commands/prefs.get", {"x-formsurface-window-key": "z" * 43}) == (403, REFUSED)
        assert post(port, "/api/shutdown", {}) == (403, REFUSED)
        assert post(port, "/api/window/key", {})[0] == 404
        assert proc.poll() is None
    finally:
        stop_engine(proc)


def test_engine_started_from_source_prints_no_key(tmp_path):
    proc, port, lines = start_engine(tmp_path, False)
    try:
        assert any("http://127.0.0.1:" in line for line in lines), "".join(lines)
        assert not any("window-key" in line for line in lines)
        status, body = post(port, "/api/window/key", {})
        assert status == 200 and len(body["key"]) >= 32
        assert post(port, "/api/window/key", {})[0] == 409
    finally:
        stop_engine(proc)
