import asyncio
import json
import os
import re
import uuid
from pathlib import Path

from fastapi import FastAPI, File, Form, HTTPException, Request, UploadFile, WebSocket, WebSocketDisconnect
from fastapi.responses import JSONResponse

VERSION = "0.2.0"
ROOT = Path(__file__).resolve().parents[2]
DATA = Path(os.environ.get("FORMSURFACE_DATA", ROOT / "workspace"))
SAFE = re.compile(r"^[A-Za-z0-9 _.-]{1,80}$")
LOCAL_HOSTS = {"127.0.0.1", "::1", "localhost", "testclient"}

app = FastAPI(title="FormSurface service", version=VERSION)


@app.middleware("http")
async def local_only(request: Request, call_next):
    if request.client and request.client.host not in LOCAL_HOSTS:
        return JSONResponse({"detail": "FormSurface only answers this computer"}, status_code=403)
    return await call_next(request)


def folder(kind: str, name: str) -> Path:
    if not SAFE.match(name) or name.startswith("."):
        raise HTTPException(400, "Names may use letters, digits, spaces, dot, dash and underscore")
    path = DATA / kind / name
    path.mkdir(parents=True, exist_ok=True)
    return path


async def write_maps(path: Path, maps: list[UploadFile]) -> list[str]:
    written = []
    for upload in maps:
        name = Path(upload.filename or "").name
        if not name.endswith(".png") or not SAFE.match(name):
            raise HTTPException(400, f"Unexpected map file {name!r}")
        data = await upload.read()
        if not data.startswith(b"\x89PNG\r\n\x1a\n"):
            raise HTTPException(400, f"{name} is not a PNG")
        (path / name).write_bytes(data)
        written.append(name)
    return written


class Relay:
    def __init__(self) -> None:
        self.socket: WebSocket | None = None
        self.pending: dict[str, asyncio.Future] = {}

    async def send(self, command: str, args: dict, timeout: float) -> dict:
        if self.socket is None:
            raise HTTPException(409, "No FormSurface editor is linked. Open http://localhost:5176 and turn on Agent link in the top bar.")
        key = uuid.uuid4().hex
        future = asyncio.get_running_loop().create_future()
        self.pending[key] = future
        try:
            await self.socket.send_json({"id": key, "command": command, "args": args})
            reply = await asyncio.wait_for(future, timeout)
        except asyncio.TimeoutError as error:
            raise HTTPException(504, f"The editor did not answer {command} within {timeout:.0f} s") from error
        finally:
            self.pending.pop(key, None)
        if not reply.get("ok"):
            raise HTTPException(422, reply.get("error", "The editor refused the command"))
        return reply.get("result", {})


relay = Relay()


@app.websocket("/api/live")
async def live(socket: WebSocket):
    host = socket.client.host if socket.client else ""
    if host not in LOCAL_HOSTS:
        await socket.close(code=1008)
        return
    await socket.accept()
    if relay.socket is not None:
        try:
            await relay.socket.close(code=1000)
        except RuntimeError:
            pass
    relay.socket = socket
    try:
        while True:
            message = await socket.receive_json()
            future = relay.pending.get(message.get("id", ""))
            if future and not future.done():
                future.set_result(message)
    except WebSocketDisconnect:
        pass
    finally:
        if relay.socket is socket:
            relay.socket = None


@app.post("/api/agent")
async def agent(request: Request):
    body = await request.json()
    command = body.get("command")
    if not isinstance(command, str) or not command:
        raise HTTPException(400, "Body needs a command name")
    args = body.get("args") or {}
    if not isinstance(args, dict):
        raise HTTPException(400, "args must be an object")
    return {"command": command, "result": await relay.send(command, args, float(body.get("timeout", 60)))}


@app.get("/api/health")
def health():
    return {"ok": True, "version": VERSION, "data": str(DATA), "editor_linked": relay.socket is not None}


@app.post("/api/exports/{name}")
async def export_maps(name: str, maps: list[UploadFile] = File(...)):
    path = folder("exports", name)
    return {"folder": str(path), "files": await write_maps(path, maps)}


@app.post("/api/projects/{name}")
async def save_project(name: str, document: str = Form(...), maps: list[UploadFile] = File(...)):
    path = folder("projects", name)
    try:
        parsed = json.loads(document)
    except json.JSONDecodeError as error:
        raise HTTPException(400, f"Project document is not JSON: {error}") from error
    (path / "project.json").write_text(json.dumps({"formsurface": VERSION, **parsed}, indent=2))
    return {"folder": str(path), "files": await write_maps(path, maps)}


@app.get("/api/projects")
def list_projects():
    root = DATA / "projects"
    if not root.exists():
        return {"projects": []}
    return {"projects": sorted(p.name for p in root.iterdir() if (p / "project.json").exists())}


@app.get("/api/projects/{name}")
def read_project(name: str):
    path = DATA / "projects" / name
    if not SAFE.match(name) or not (path / "project.json").exists():
        raise HTTPException(404, "No such project")
    return {"folder": str(path), "document": json.loads((path / "project.json").read_text()), "maps": sorted(p.name for p in path.glob("*.png"))}
