import json
import os
import re
from pathlib import Path

from fastapi import FastAPI, File, Form, HTTPException, Request, UploadFile
from fastapi.responses import JSONResponse

VERSION = "0.1.0"
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


@app.get("/api/health")
def health():
    return {"ok": True, "version": VERSION, "data": str(DATA)}


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
