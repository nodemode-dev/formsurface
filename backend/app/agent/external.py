import os
import re
import subprocess
import sys
from pathlib import Path

from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import FileResponse

from app.agent import config

router = APIRouter()
NAME = re.compile(r"^[A-Za-z0-9_.-]{1,80}$")
PNG = b"\x89PNG\r\n\x1a\n"


def _path(name: str) -> Path:
    if not NAME.match(name) or name.startswith("."):
        raise HTTPException(400, "Names may use letters, digits, dot, dash and underscore")
    folder = config.data_dir() / "external"
    folder.mkdir(parents=True, exist_ok=True)
    return folder / f"{name}.png"


def _open(path: Path) -> str:
    app = str(config.load_prefs().get("externalEditor") or "").strip()
    try:
        if sys.platform == "darwin":
            cmd = ["open", "-a", app, str(path)] if app else ["open", str(path)]
        elif os.name == "nt":
            if not app:
                os.startfile(str(path))
                return "the default app"
            cmd = [app, str(path)]
        else:
            cmd = [app, str(path)] if app else ["xdg-open", str(path)]
        subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    except (OSError, ValueError) as exc:
        raise HTTPException(500, f"Could not open {path.name} in {app or 'the default app'}: {exc}") from None
    return app or "the default app"


@router.post("/api/external/{name}")
async def send(name: str, request: Request, open_app: bool = True) -> dict:
    path = _path(name)
    data = await request.body()
    if not data.startswith(PNG):
        raise HTTPException(400, "Send a PNG image")
    path.write_bytes(data)
    opened = _open(path) if open_app else ""
    return {"path": str(path), "mtime": path.stat().st_mtime, "opened": opened}


@router.get("/api/external/{name}")
def stamp(name: str) -> dict:
    path = _path(name)
    if not path.is_file():
        raise HTTPException(404, f"{path.name} is gone")
    return {"path": str(path), "mtime": path.stat().st_mtime}


@router.get("/api/external/{name}/image")
def image(name: str) -> FileResponse:
    path = _path(name)
    if not path.is_file():
        raise HTTPException(404, f"{path.name} is gone")
    return FileResponse(path, media_type="image/png", headers={"Cache-Control": "no-store"})
