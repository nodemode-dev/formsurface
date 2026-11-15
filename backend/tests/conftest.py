import importlib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

import pytest

BACKEND = Path(__file__).resolve().parents[1]
REPO = BACKEND.parent

APP_ID = "formsurface"
APP_MODULE = "app.main:app"
ENV_PREFIX = "FORMSURFACE"
DATA_ENV = "FORMSURFACE_DATA"
PORT_ENV = "FORMSURFACE_PORT"
ENGINE_PORT = 8766
HEALTH_PATH = "/api/health"
VERSION_PATH = "/api/health"
VERSION_KEY = "version"
WINDOW_ORIGINS = [
    "http://localhost:5176",
    "http://127.0.0.1:5176",
    "http://localhost:5177",
    "http://127.0.0.1:5177",
    "tauri://localhost",
    "http://tauri.localhost",
]
WS_PATH = "/api/live"
MCP_SERVER = REPO / "mcp" / "server.py"
VERSION_SITES = [
    ("VERSION", r"^v?(\S+)\s*$"),
    ("README.md", r"badge/version-v([0-9.]+)-"),
    ("formsurface", r"^formsurface (\S+) ·"),
    ("backend/app/main.py", r'^VERSION = "([^"]+)"'),
    ("frontend/package.json", r'^  "version": "([^"]+)"'),
    ("frontend/src/version.ts", r"^export const VERSION = '([^']+)'"),
    ("native/Cargo.toml", r'^version = "([^"]+)"'),
    ("src-tauri/Cargo.toml", r'^version = "([^"]+)"'),
    ("src-tauri/tauri.conf.json", r'^  "version": "([^"]+)"'),
    (".claude/skills/formsurface-harness/SKILL.md", r'^  version: "([^"]+)"'),
    (".claude/skills/formsurface-painting/SKILL.md", r'^  version: "([^"]+)"'),
    (".agents/skills/formsurface-painting/SKILL.md", r'^  version: "([^"]+)"'),
    (".claude/skills/formsurface-painting/references/tools.md", r"^# FormSurface MCP tools \(server ([0-9.]+)\)"),
    (".agents/skills/formsurface-painting/references/tools.md", r"^# FormSurface MCP tools \(server ([0-9.]+)\)"),
    ("frontend/package-lock.json", r'^  "version": "([^"]+)"'),
]
STRIP_EXEMPT = []

_SANDBOX = Path(tempfile.mkdtemp(prefix=f"{APP_ID}-tests-"))
for _name in ("data", "home"):
    (_SANDBOX / _name).mkdir()
os.environ[DATA_ENV] = str(_SANDBOX / "data")
os.environ[PORT_ENV] = str(ENGINE_PORT)
os.environ["HOME"] = str(_SANDBOX / "home")
os.environ["USERPROFILE"] = str(_SANDBOX / "home")
for _key in [k for k in os.environ if k.startswith(f"{ENV_PREFIX}_") and k not in (DATA_ENV, PORT_ENV)]:
    os.environ.pop(_key)
sys.path.insert(0, str(BACKEND))


def pytest_sessionfinish(session, exitstatus):
    shutil.rmtree(_SANDBOX, ignore_errors=True)


def own_origin():
    return f"http://127.0.0.1:{ENGINE_PORT}"


@pytest.fixture(scope="session")
def app():
    module, _, attr = APP_MODULE.partition(":")
    return getattr(importlib.import_module(module), attr)


@pytest.fixture(scope="session")
def client(app):
    from fastapi.testclient import TestClient

    with TestClient(app, base_url=own_origin()) as c:
        yield c


@pytest.fixture(scope="session")
def data_dir():
    return _SANDBOX / "data"


@pytest.fixture(scope="session")
def repo_versions():
    found = {}
    for rel, pattern in VERSION_SITES:
        text = (REPO / rel.split("#")[0]).read_text(encoding="utf-8")
        match = re.search(pattern, text, re.MULTILINE)
        found[rel] = match.group(1) if match else None
    return found


@pytest.fixture(scope="session")
def repo_version(repo_versions):
    return next(iter(repo_versions.values()))


def free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def get_json(url, timeout=2.0):
    with urllib.request.urlopen(url, timeout=timeout) as r:
        return r.status, json.loads(r.read().decode("utf-8") or "null")


@pytest.fixture(scope="session")
def engine():
    port = free_port()
    data = _SANDBOX / "engine-data"
    data.mkdir()
    env = dict(os.environ, **{DATA_ENV: str(data), PORT_ENV: str(port)})
    proc = subprocess.Popen(
        [sys.executable, "-m", "uvicorn", APP_MODULE, "--host", "127.0.0.1", "--port", str(port)],
        cwd=BACKEND, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    base = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 60
    ready = False
    while time.monotonic() < deadline and proc.poll() is None:
        try:
            status, _ = get_json(base + HEALTH_PATH)
            ready = status == 200
        except OSError:
            ready = False
        if ready:
            break
        time.sleep(0.25)
    if not ready:
        proc.kill()
        out = proc.communicate(timeout=5)[0].decode("utf-8", "replace")
        raise RuntimeError(f"engine did not answer {HEALTH_PATH} within 60 s:\n{out[-4000:]}")
    yield {"base": base, "port": port, "data": data, "env": env, "proc": proc}
    proc.terminate()
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        proc.kill()
