import json
import os
import re
import shutil
import struct
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import warnings
from pathlib import Path

import pytest

from test_window import APP_ID, CLEAR_ENV_PREFIX, DIST, ENGINE_CMD, ENGINE_DIR, ENGINE_ENV, HEALTH_PATH, free_port

VIEWPORT = {"width": 1440, "height": 900}
START = "/?size=1024"
BOOT_S = 180
STEP_S = 90
MESH_NAME = "model (1).glb"
BUSY = "; wait for it to finish$"
UNDO_AT_ONCE = """(limit) => new Promise((done, fail) => {
  const start = performance.now();
  const look = () => {
    const button = document.querySelector('.fs-toast-undo button');
    if (button) { const text = button.parentElement.querySelector('.fs-toast-text').textContent; button.click(); done(text); return; }
    if (performance.now() - start > limit) { fail(new Error('no Undo toast appeared')); return; }
    setTimeout(look, 10);
  };
  look();
})"""


@pytest.fixture(scope="module")
def service():
    assert (DIST / "index.html").is_file(), f"build the window first: {DIST} has no index.html (npm run build)"
    port = free_port()
    sandbox = Path(tempfile.mkdtemp(prefix=f"{APP_ID}-home-"))
    data = sandbox / "data"
    data.mkdir()
    (sandbox / "home").mkdir()
    values = {"port": str(port), "data": str(data), "dist": str(DIST)}
    env = {k: v for k, v in os.environ.items() if not k.startswith(CLEAR_ENV_PREFIX)}
    env.update({k: v.format(**values) for k, v in ENGINE_ENV.items()})
    env["HOME"] = str(sandbox / "home")
    proc = subprocess.Popen([a.format(**values) for a in ENGINE_CMD], cwd=ENGINE_DIR, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    base = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline and proc.poll() is None:
        try:
            with urllib.request.urlopen(base + HEALTH_PATH, timeout=2) as r:
                if r.status == 200:
                    break
        except OSError:
            time.sleep(0.25)
    else:
        proc.kill()
        out = proc.communicate(timeout=5)[0].decode("utf-8", "replace")
        raise RuntimeError(f"engine did not answer {HEALTH_PATH}:\n{out[-4000:]}")
    yield {"base": base, "data": data}
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
    shutil.rmtree(sandbox, ignore_errors=True)


@pytest.fixture(scope="module")
def chromium():
    from playwright.sync_api import sync_playwright

    with sync_playwright() as p:
        browser = p.chromium.launch()
        yield browser
        browser.close()


@pytest.fixture
def context(chromium):
    ctx = chromium.new_context(viewport=VIEWPORT)
    ctx.set_default_timeout(STEP_S * 1000)
    yield ctx
    ctx.close()


def call(base, method, path, body=None):
    data = None if body is None else json.dumps(body).encode()
    request = urllib.request.Request(base + path, data=data, method=method, headers={"Content-Type": "application/json"} if data else {})
    try:
        with urllib.request.urlopen(request, timeout=STEP_S + 30) as r:
            return r.status, json.loads(r.read() or b"null")
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read() or b"null")


def open_window(context, base, query=START):
    page = context.new_page()
    page.goto(base + query, wait_until="load", timeout=BOOT_S * 1000)
    page.wait_for_function("() => document.body.classList.contains('app-ready') && document.getElementById('notice').hidden", timeout=BOOT_S * 1000)
    return page


def home_open(page):
    return page.evaluate("() => !document.querySelector('section.fs-home').hidden && document.body.classList.contains('nm-home')")


def wait_home(page, shown):
    page.wait_for_function(f"() => !document.querySelector('section.fs-home').hidden === {str(shown).lower()} && document.body.classList.contains('nm-home') === {str(shown).lower()}", timeout=STEP_S * 1000)


def wait_status(page, pattern):
    page.wait_for_function("(p) => new RegExp(p).test(document.getElementById('status').textContent)", arg=pattern, timeout=STEP_S * 1000)
    return page.text_content("#status")


def with_service(page, act, done, no_service):
    for _ in range(6):
        page.evaluate("() => { document.getElementById('status').textContent = ''; }")
        act()
        text = wait_status(page, f"{done}|{no_service}|{BUSY}|rror|ailed|400")
        if re.search(BUSY, text):
            page.wait_for_timeout(2000)
            continue
        if not re.search(no_service, text):
            return text
        warnings.warn(f"the editor's 800 ms service check gave up under load: {text}")
    return text


def save(page):
    return with_service(page, lambda: page.keyboard.press("Control+S"), "^Saved ", "^Start the FormSurface service")


def menu_items(page, label):
    page.click(f".cv-menu-btn:text-is('{label}')")
    panel = page.locator(".cv-menu:not([hidden])").first
    panel.wait_for(timeout=STEP_S * 1000)
    return panel.locator(":scope > .cv-menu-item")


def choose(page, menu, item):
    menu_items(page, menu).filter(has=page.locator(f".cv-menu-label:text-is('{item}')")).first.click(no_wait_after=True)


def visible_page(page):
    return page.evaluate("() => [...document.querySelectorAll('section.fs-home [data-page]')].filter((p) => !p.hidden).map((p) => p.dataset.page)")


def library(service):
    return call(service["base"], "GET", "/api/projects?detail=1")[1]["projects"]


def trash(service):
    return call(service["base"], "GET", "/api/projects/trash")[1]["projects"]


def wait_for(check, what, seconds=STEP_S):
    deadline = time.monotonic() + seconds
    while True:
        value = check()
        if value:
            return value
        assert time.monotonic() < deadline, what
        time.sleep(0.25)


def card_menu(page, project, key):
    page.locator(f"section.fs-home [data-page]:not([hidden]) .fs-card[data-project='{project}']").first.click(button="right")
    page.click(f".fs-home-menu:not([hidden]) .fs-menu-item[data-k='{key}']", no_wait_after=True)


def rect(page, selector):
    return page.evaluate("(s) => { const r = [...document.querySelectorAll(s)].find((n) => n.offsetParent !== null).getBoundingClientRect(); return { left: r.left, top: r.top, right: r.right, bottom: r.bottom }; }", selector)


def ink_top(page, selector):
    return page.evaluate("""(s) => {
      const h = document.querySelector(s);
      const cs = getComputedStyle(h);
      const ctx = document.createElement('canvas').getContext('2d');
      ctx.font = `${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
      const m = ctx.measureText(h.textContent);
      const lead = (parseFloat(cs.lineHeight) - m.fontBoundingBoxAscent - m.fontBoundingBoxDescent) / 2;
      return h.getBoundingClientRect().top + lead + m.fontBoundingBoxAscent - m.actualBoundingBoxAscent;
    }""", selector)


def overlap(a, b):
    return a["left"] < b["right"] and b["left"] < a["right"] and a["top"] < b["bottom"] and b["top"] < a["bottom"]


def tiny_glb():
    positions = struct.pack("<12f", -1, -1, 0, 1, -1, 0, 1, 1, 0, -1, 1, 0)
    normals = struct.pack("<12f", *([0, 0, 1] * 4))
    uvs = struct.pack("<8f", 0, 1, 1, 1, 1, 0, 0, 0)
    indices = struct.pack("<6H", 0, 1, 2, 0, 2, 3)
    blob = positions + normals + uvs + indices
    views, offset = [], 0
    for part, target in ((positions, 34962), (normals, 34962), (uvs, 34962), (indices, 34963)):
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(part), "target": target})
        offset += len(part)
    gltf = {
        "asset": {"version": "2.0"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": "quad"}],
        "materials": [{"name": "paint"}],
        "meshes": [{"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}, "indices": 3, "material": 0}]}],
        "buffers": [{"byteLength": len(blob)}],
        "bufferViews": views,
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "count": 4, "type": "VEC3", "min": [-1, -1, 0], "max": [1, 1, 0]},
            {"bufferView": 1, "componentType": 5126, "count": 4, "type": "VEC3"},
            {"bufferView": 2, "componentType": 5126, "count": 4, "type": "VEC2"},
            {"bufferView": 3, "componentType": 5123, "count": 6, "type": "SCALAR"},
        ],
    }
    text = json.dumps(gltf).encode()
    text += b" " * (-len(text) % 4)
    blob += b"\x00" * (-len(blob) % 4)
    chunks = struct.pack("<I4s", len(text), b"JSON") + text + struct.pack("<I4s", len(blob), b"BIN\x00") + blob
    return struct.pack("<4sII", b"glTF", 2, 12 + len(chunks)) + chunks


def test_home_shows_at_first_launch_its_pages_switch_and_esc_does_not_leave_it_without_a_project(service, context):
    failed, errors = [], []
    page = context.new_page()
    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.on("requestfailed", lambda r: failed.append(f"{r.url} {r.failure}"))
    page.on("response", lambda r: failed.append(f"{r.url} {r.status}") if r.status >= 400 else None)
    page.goto(service["base"] + START, wait_until="load", timeout=BOOT_S * 1000)
    page.wait_for_function("() => document.body.classList.contains('app-ready') && document.getElementById('notice').hidden", timeout=BOOT_S * 1000)
    assert home_open(page), "Home covers the window at the first launch"
    assert visible_page(page) == ["start"]
    for name in ("recent", "templates", "trash", "start"):
        page.click(f".fs-home-nav .fs-home-nav-item[data-v='{name}']")
        assert visible_page(page) == [name]
        assert page.get_attribute(f".fs-home-nav .fs-home-nav-item[data-v='{name}']", "class").split().count("is-active") == 1
        assert page.locator(".fs-home-nav .fs-home-nav-item.is-active").count() == 1
    page.keyboard.press("Escape")
    page.wait_for_timeout(500)
    assert home_open(page), "Esc left Home although no project is open"
    assert not failed, f"failed requests: {failed}"
    assert not errors, f"console errors: {errors}"


def test_a_project_goes_round_the_library_from_the_window(service, context):
    base = service["base"]
    page = open_window(context, base)
    page.click(".fs-tpl-grid .fs-card[data-template='sphere']", no_wait_after=True)
    page.fill(".cv-dialog .fs-tpl-title", "Round trip")
    page.click(".cv-dialog-foot .btn.primary", no_wait_after=True)
    wait_home(page, False)
    wait_for(lambda: [p for p in library(service) if p["title"] == "Round trip" and p["saved"]], "a project made from a template is saved at once", BOOT_S)

    saved = save(page)
    assert saved.startswith("Saved "), saved
    saved = wait_for(lambda: [p for p in library(service) if p["title"] == "Round trip" and p["thumb"]], "the saved project lists with a preview")
    pid = saved[0]["id"]
    assert saved[0]["saved"] and saved[0]["layers"] >= 1

    page.keyboard.press("Control+Alt+h")
    wait_home(page, True)
    page.click(".fs-home-nav .fs-home-nav-item[data-v='recent']")
    card = page.locator(f".fs-recent-full .fs-card[data-project='{pid}']")
    card.wait_for(timeout=STEP_S * 1000)
    assert card.locator(".fs-card-name").text_content() == "Round trip"
    page.wait_for_function(f"() => {{ const i = document.querySelector(\".fs-recent-full .fs-card[data-project='{pid}'] img\"); return i && i.complete && i.naturalWidth > 0 && i.src.includes('/thumb'); }}", timeout=STEP_S * 1000)
    page.keyboard.press("Escape")
    wait_home(page, False)

    first = menu_items(page, "File").first
    assert first.locator(".cv-menu-label").text_content() == "Home"
    assert "Alt" in (first.locator(".cv-menu-keys").text_content() or "")
    first.click(no_wait_after=True)
    wait_home(page, True)
    page.keyboard.press("Escape")
    wait_home(page, False)
    page.close()

    page = open_window(context, base)
    assert home_open(page), "Home shows at launch with the default preferences"
    page.click(f".fs-recent-grid .fs-card[data-project='{pid}']", no_wait_after=True)
    wait_home(page, False)
    page.keyboard.press("Control+Alt+h")
    wait_home(page, True)
    assert page.is_visible(".fs-home-open")
    assert page.text_content(".fs-home-open .fs-home-open-name") == "Round trip"

    page.click(".fs-home-nav .fs-home-nav-item[data-v='recent']")
    card_menu(page, pid, "del")
    assert page.evaluate(UNDO_AT_ONCE, STEP_S * 1000) == 'Moved "Round trip" to the Trash.'
    assert home_open(page)
    wait_for(lambda: [p for p in library(service) if p["id"] == pid] and not trash(service), "Undo put the project back")
    page.locator(f".fs-recent-full .fs-card[data-project='{pid}']").wait_for(timeout=STEP_S * 1000)

    card_menu(page, pid, "dup")
    copy = wait_for(lambda: [p for p in library(service) if p["id"] != pid], "Duplicate made a second project")[0]
    assert copy["title"] == "Round trip copy"
    page.locator(f".fs-recent-full .fs-card[data-project='{copy['id']}']").wait_for(timeout=STEP_S * 1000)
    for project in (pid, copy["id"]):
        card_menu(page, project, "del")
        page.locator(f".fs-recent-full .fs-card[data-project='{project}']").wait_for(state="detached", timeout=STEP_S * 1000)
    wait_for(lambda: len(trash(service)) == 2 and not library(service), "both projects are in the Trash")

    page.click(".fs-home-nav .fs-home-nav-item[data-v='trash']")
    page.locator(".fs-trash-list .fs-trash-row").nth(1).wait_for(timeout=STEP_S * 1000)
    original = next(d["entry"] for d in trash(service) if d["id"] == pid)
    page.click(f".fs-trash-row[data-entry='{original}'] [data-a='restore']")
    wait_for(lambda: [p["id"] for p in library(service)] == [pid], "Restore put the project back under its id")
    page.locator(f".fs-trash-row[data-entry='{original}']").wait_for(state="detached", timeout=STEP_S * 1000)
    with urllib.request.urlopen(f"{base}/api/projects/{urllib.parse.quote(pid)}/file", timeout=STEP_S) as r:
        assert r.status == 200 and r.read(4) == b"FSPJ"

    page.click(".fs-home-nav .fs-home-nav-item[data-v='recent']")
    card_menu(page, pid, "del")
    wait_for(lambda: len(trash(service)) == 2, "the project went to the Trash again")
    page.click(".fs-home-nav .fs-home-nav-item[data-v='trash']")
    left = next(d["entry"] for d in trash(service) if d["id"] == copy["id"])
    page.locator(f".fs-trash-row[data-entry='{left}']").wait_for(timeout=STEP_S * 1000)
    page.click(f".fs-trash-row[data-entry='{left}'] [data-a='purge']")
    page.locator(".fs-home-confirm:not([hidden])").wait_for(timeout=STEP_S * 1000)
    bar = rect(page, ".fs-home-confirm")
    ink = ink_top(page, "[data-page='trash'] h1.fs-home-title")
    assert bar["bottom"] <= ink, f"the confirm bar (bottom {bar['bottom']}) covers the page title's letters (top {ink})"
    assert not overlap(bar, rect(page, ".fs-btn-empty-trash")), "the confirm bar covers Empty Trash"
    page.click(".fs-home-confirm [data-a='confirm-yes']")
    wait_for(lambda: [d["id"] for d in trash(service)] == [pid], "Delete permanently removed that entry and kept the other")
    assert [p["id"] for p in library(service)] == []
    page.locator(f".fs-trash-row[data-entry='{left}']").wait_for(state="detached", timeout=STEP_S * 1000)

    page.click(".fs-btn-empty-trash")
    page.locator(".fs-home-confirm:not([hidden])").wait_for(timeout=STEP_S * 1000)
    page.click(".fs-home-confirm [data-a='confirm-yes']")
    wait_for(lambda: not trash(service), "Empty Trash removed the rest")
    page.locator(".fs-trash-none").wait_for(state="visible", timeout=STEP_S * 1000)
    assert page.locator(".fs-trash-list .fs-trash-row").count() == 0
    assert page.is_disabled(".fs-btn-empty-trash")
    assert not library(service)
    assert [p.name for p in (service["data"] / "projects").iterdir() if not p.name.startswith(".")] == []
    page.close()


def test_export_and_save_succeed_for_a_mesh_named_with_brackets_and_a_space(service, context):
    base, data = service["base"], service["data"]
    glb = tiny_glb()
    page = open_window(context, base)
    page.set_input_files("#mesh-file", files=[{"name": MESH_NAME, "mimeType": "model/gltf-binary", "buffer": glb}])
    assert wait_status(page, "^Loaded ") == f"Loaded {MESH_NAME}"
    wait_home(page, False)

    exported = with_service(page, lambda: page.keyboard.press("Shift+Control+E"), "^Exported \\d+ maps to ", "^Downloaded ")
    assert re.match(r"^Exported \d+ maps to ", exported), exported
    folder = data / "exports" / "model _1_"
    maps = sorted(p.name for p in folder.glob("*.png"))
    assert len(maps) == int(exported.split()[1]) and all(name.startswith("model _1__") for name in maps), maps

    written = with_service(page, lambda: choose(page, "File", "Export Mesh (OBJ)"), "^Mesh written to (?!your downloads)", "^Mesh written to your downloads")
    assert written.startswith("Mesh written to "), written
    assert (folder / "model _1_.obj").read_bytes().startswith(b"# FormSurface")

    saved = save(page)
    assert saved.startswith("Saved "), saved
    project = wait_for(lambda: [p for p in library(service) if p["mesh"] == MESH_NAME and p["meshStored"]], "the project lists with its stored mesh")[0]
    with urllib.request.urlopen(f"{base}/api/projects/{urllib.parse.quote(project['id'])}/mesh", timeout=STEP_S) as r:
        assert r.read() == glb
        assert r.headers["X-Mesh-Name"] == MESH_NAME
    assert (data / "projects" / project["id"] / "project.fsproj").read_bytes()[:4] == b"FSPJ"
    page.close()


def test_a_blocked_engine_download_is_reported_instead_of_starting_for_ever(service, context):
    context.route("**/*.wasm", lambda route: route.abort())
    page = context.new_page()
    page.goto(service["base"] + START, wait_until="load", timeout=BOOT_S * 1000)
    page.wait_for_function("() => /could not start/.test(document.getElementById('notice').textContent)", timeout=BOOT_S * 1000)
    assert page.is_visible("#notice")
    assert "Starting the GPU engine" not in page.text_content("#notice")
    assert page.text_content("#status") == "GPU engine unavailable"
    page.close()


def test_agent_commands_refuse_a_layer_id_that_does_not_exist(service, context):
    base = service["base"]
    page = open_window(context, base, START + "&agent=1")
    wait_for(lambda: call(base, "GET", HEALTH_PATH)[1]["editor_linked"], "the window turned Agent link on")
    status, before = call(base, "POST", "/api/agent", {"command": "status"})
    assert status == 200 and before["result"]["layers"], before
    missing = 1 + max(layer["id"] for layer in before["result"]["layers"])
    for command, args in [
        ("mask", {"id": missing, "action": "add_white"}),
        ("update_layer", {"id": missing, "name": "renamed", "opacity": 0.5}),
        ("remove_layer", {"id": missing}),
        ("move_layer", {"id": missing, "offset": 1}),
        ("load_image", {"file": "none.png", "target": "mask", "layer_id": missing}),
        ("fill_tile_images", {"files": ["none.png"], "layer_id": missing}),
    ]:
        code, body = call(base, "POST", "/api/agent", {"command": command, "args": args, "timeout": STEP_S})
        assert code == 422, (command, code, body)
        assert body["detail"] == f"No layer with id {missing}", (command, body)
    _, after = call(base, "POST", "/api/agent", {"command": "status"})
    assert after["result"]["layers"] == before["result"]["layers"]
    assert after["result"]["active_layer"] == before["result"]["active_layer"]
    page.close()
