import importlib.util
import os
import re
import shutil
import signal
import socket
import subprocess
import sys
import threading
import time

import pytest

from conftest import BACKEND, DATA_ENV, HEALTH_PATH, PORT_ENV, REPO, free_port, get_json

URL = re.compile(r"http://127\.0\.0\.1:(\d+)")
ENTRY = BACKEND / "formsurface_engine.py"
HEADLESS = REPO / "mcp" / "headless.py"


def start_entry(tmp_path, port):
    data = tmp_path / "data"
    data.mkdir()
    env = {k: v for k, v in os.environ.items() if not k.startswith("FORMSURFACE_")}
    env.update({DATA_ENV: str(data), PORT_ENV: str(port), "PYTHONUNBUFFERED": "1"})
    return subprocess.Popen([sys.executable, str(ENTRY)], cwd=BACKEND, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)


def first_announcement(proc, seconds=60):
    found = {}

    def read():
        for raw in proc.stdout:
            match = URL.search(raw.decode("utf-8", "replace"))
            if match:
                port = int(match.group(1))
                with socket.socket() as probe:
                    probe.settimeout(2)
                    found["refused"] = probe.connect_ex(("127.0.0.1", port)) != 0
                found["port"] = port
                return

    reader = threading.Thread(target=read, daemon=True)
    reader.start()
    reader.join(seconds)
    return found


def end(proc):
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
    proc.stdout.close()


def test_the_engine_entry_names_its_address_only_once_it_accepts_connections(tmp_path):
    port = free_port()
    proc = start_entry(tmp_path, port)
    try:
        found = first_announcement(proc)
        assert found.get("port") == port, "the entry never printed its address"
        assert found["refused"] is False, "the address was printed before the engine listened"
        assert get_json(f"http://127.0.0.1:{port}{HEALTH_PATH}")[1]["ok"] is True
    finally:
        end(proc)


def test_the_engine_entry_moves_to_a_free_port_when_its_own_is_taken_and_names_that_one(tmp_path):
    with socket.socket() as holder:
        holder.bind(("127.0.0.1", 0))
        holder.listen(1)
        taken = holder.getsockname()[1]
        proc = start_entry(tmp_path, taken)
        try:
            found = first_announcement(proc)
            assert found.get("port") not in (None, taken)
            assert found["refused"] is False
            assert get_json(f"http://127.0.0.1:{found['port']}{HEALTH_PATH}")[1]["ok"] is True
        finally:
            end(proc)


def closed_port():
    return f"http://127.0.0.1:{free_port()}"


@pytest.fixture
def headless(monkeypatch, tmp_path):
    if os.name != "posix":
        pytest.skip("the stand-in browser is a shell script")
    browser = tmp_path / "stand-in-browser"
    browser.write_text("#!/bin/sh\nsleep 120 &\nwait\n")
    browser.chmod(0o755)
    monkeypatch.setenv("FORMSURFACE_BROWSER", str(browser))
    monkeypatch.setenv("FORMSURFACE_HEADLESS_MODE", "native")
    monkeypatch.delenv("FORMSURFACE_BROWSER_ARGS", raising=False)
    spec = importlib.util.spec_from_file_location("formsurface_headless_under_test", HEADLESS)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    monkeypatch.setattr(module, "up", lambda url: True)
    monkeypatch.setattr(module, "SERVICE", closed_port())
    seen = []

    class Recording(module.Headless):
        def spawn(self, args, cwd, env=None):
            process = super().spawn(args, cwd, env)
            profile = next(a.split("=", 1)[1] for a in args if a.startswith("--user-data-dir="))
            seen.append((process, profile, os.getpgid(process.pid)))
            return process

    yield module, Recording, seen
    for _process, profile, group in seen:
        try:
            os.killpg(group, signal.SIGKILL)
        except ProcessLookupError:
            pass
        shutil.rmtree(profile, ignore_errors=True)


def group_is_gone(group):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        try:
            os.killpg(group, 0)
        except ProcessLookupError:
            return True
        time.sleep(0.1)
    return False


def assert_nothing_left(seen):
    assert len(seen) == 1, "the stand-in browser was started once"
    process, profile, group = seen[0]
    assert process.poll() is not None, "the browser the start opened is still running"
    assert group_is_gone(group), "a child of that browser is still running"
    assert not os.path.exists(profile), "the browser profile folder was left behind"


def test_a_headless_start_that_fails_stops_the_browser_it_opened_and_removes_its_profile(headless):
    module, Recording, seen = headless
    with pytest.raises(module.httpx.HTTPError):
        with Recording(quiet=True):
            pytest.fail("the start should not have succeeded: no service answers")
    assert_nothing_left(seen)


def test_a_headless_start_that_is_interrupted_stops_the_browser_it_opened(headless, monkeypatch):
    module, Recording, seen = headless

    def interrupted(*args, **kwargs):
        raise KeyboardInterrupt

    monkeypatch.setattr(module.httpx, "get", interrupted)
    with pytest.raises(KeyboardInterrupt):
        with Recording(quiet=True):
            pytest.fail("the start should not have succeeded: it was interrupted")
    assert_nothing_left(seen)


def test_a_headless_run_that_started_stops_the_browser_when_it_ends(headless, monkeypatch):
    module, Recording, seen = headless
    monkeypatch.setattr(module.httpx, "get", lambda *args, **kwargs: type("Reply", (), {"json": lambda self: {"editor_linked": True}})())
    with Recording(quiet=True):
        process, profile, _ = seen[0]
        assert process.poll() is None and os.path.isdir(profile)
    assert_nothing_left(seen)
