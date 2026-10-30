import asyncio
import json
import os
import re
import uuid
from array import array
from pathlib import Path

from fastapi import FastAPI, File, Form, HTTPException, Request, Response, UploadFile, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, JSONResponse

from app import environments, ocio
from app.agent import config as agent_config
from app.agent import router as agent_router
from app.local_guard import LocalGuard

VERSION = "0.59.10"
ROOT = Path(__file__).resolve().parents[2]
DATA = Path(os.environ.get("FORMSURFACE_DATA", ROOT / "workspace"))
SAFE = re.compile(r"^[A-Za-z0-9 _.-]{1,80}$")
LOCAL_HOSTS = {"127.0.0.1", "::1", "localhost", "testclient"}

app = FastAPI(title="FormSurface service", version=VERSION, lifespan=agent_router.lifespan)
app.include_router(agent_router.router)


@app.middleware("http")
async def local_only(request: Request, call_next):
    if request.client and request.client.host not in LOCAL_HOSTS:
        return JSONResponse({"detail": "FormSurface only answers this computer"}, status_code=403)
    return await call_next(request)


app.add_middleware(LocalGuard, origins=agent_config.window_origins())


def folder(kind: str, name: str) -> Path:
    if not SAFE.match(name) or name.startswith("."):
        raise HTTPException(400, "Names may use letters, digits, spaces, dot, dash and underscore")
    path = DATA / kind / name
    path.mkdir(parents=True, exist_ok=True)
    return path


MAP_SIGNATURES = {
    ".png": (b"\x89PNG\r\n\x1a\n",),
    ".tif": (b"II*\x00", b"MM\x00*"),
    ".exr": (b"\x76\x2f\x31\x01",),
    ".hdr": (b"#?RADIANCE",),
    ".jpg": (b"\xff\xd8\xff",),
    ".psd": (b"8BPS",),
}


async def write_maps(path: Path, maps: list[UploadFile]) -> list[str]:
    written = []
    for upload in maps:
        name = Path(upload.filename or "").name
        signatures = MAP_SIGNATURES.get(Path(name).suffix.lower())
        if signatures is None or not SAFE.match(name):
            raise HTTPException(400, f"Unexpected map file {name!r}")
        data = await upload.read()
        if not data.startswith(signatures):
            raise HTTPException(400, f"{name} does not match its {Path(name).suffix} format")
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


@app.post("/api/exports/{name}/mesh")
async def export_mesh(name: str, mesh: UploadFile = File(...)):
    path = folder("exports", name)
    file = Path(mesh.filename or "").name
    if not file.endswith(".obj") or not SAFE.match(file):
        raise HTTPException(400, f"Unexpected mesh file {file!r}")
    data = await mesh.read()
    if not data.startswith(b"# FormSurface"):
        raise HTTPException(400, f"{file} is not an OBJ written by FormSurface")
    (path / file).write_bytes(data)
    return {"folder": str(path), "file": file}


@app.post("/api/projects/{name}")
async def save_project(name: str, document: str = Form(...), maps: list[UploadFile] = File(...)):
    path = folder("projects", name)
    try:
        parsed = json.loads(document)
    except json.JSONDecodeError as error:
        raise HTTPException(400, f"Project document is not JSON: {error}") from error
    (path / "project.json").write_text(json.dumps({"formsurface": VERSION, **parsed}, indent=2))
    return {"folder": str(path), "files": await write_maps(path, maps)}


@app.get("/api/library")
def list_library():
    root = DATA / "library"
    if not root.exists():
        return {"materials": []}
    materials = []
    for path in sorted(root.glob("*.json")):
        try:
            materials.append(json.loads(path.read_text()))
        except json.JSONDecodeError:
            continue
    return {"materials": materials}


@app.post("/api/library/{name}")
async def save_library(name: str, request: Request):
    if not SAFE.match(name) or name.startswith("."):
        raise HTTPException(400, "Names may use letters, digits, spaces, dot, dash and underscore")
    body = await request.json()
    layers = body.get("layers") if isinstance(body, dict) else None
    if not isinstance(layers, list) or not layers or len(layers) > 64:
        raise HTTPException(400, "An intelligent material needs 1 to 64 layers")
    root = DATA / "library"
    root.mkdir(parents=True, exist_ok=True)
    path = root / f"{name}.json"
    entry = {"name": name, "layers": layers}
    thumbnail = clean_thumbnail(body.get("thumbnail"))
    if thumbnail:
        entry["thumbnail"] = thumbnail
    path.write_text(json.dumps(entry, indent=2))
    return {"file": str(path)}


THUMBNAIL = re.compile(r"^data:image/(png|webp|jpeg);base64,[A-Za-z0-9+/=]+$")


def clean_thumbnail(value):
    if value is None:
        return None
    if not isinstance(value, str) or len(value) > 200_000 or not THUMBNAIL.match(value):
        raise HTTPException(400, "A thumbnail is a base64 PNG, WebP or JPEG data URL under 200 KB")
    return value


TEMPLATE_SOURCES = {"basecolor", "normal_gl", "normal_dx", "roughness", "smoothness", "metallic", "height", "ao", "curvature", "thickness", "ao_high", "height_high", "opacity", "extra", "user2", "user3", "diffuse", "specular", "one", "zero"}
RGB_SOURCES = {"basecolor", "normal_gl", "normal_dx", "diffuse", "specular"}
SUFFIX = re.compile(r"^[A-Za-z0-9_-]{1,40}$")


@app.get("/api/templates")
def list_templates():
    root = DATA / "templates"
    templates = []
    for path in sorted(root.glob("*.json")) if root.exists() else []:
        try:
            templates.append(json.loads(path.read_text()))
        except json.JSONDecodeError:
            continue
    return {"templates": templates}


@app.post("/api/templates/{name}")
async def save_template(name: str, request: Request):
    if not SAFE.match(name) or name.startswith("."):
        raise HTTPException(400, "Names may use letters, digits, spaces, dot, dash and underscore")
    body = await request.json()
    maps = body.get("maps") if isinstance(body, dict) else None
    if not isinstance(maps, list) or not 1 <= len(maps) <= 16:
        raise HTTPException(400, "A template needs 1 to 16 maps")
    clean = []
    for item in maps:
        if not isinstance(item, dict) or not isinstance(item.get("suffix"), str) or not SUFFIX.match(item["suffix"]):
            raise HTTPException(400, "Each map needs a suffix of letters, digits, dash or underscore")
        entry = {"suffix": item["suffix"]}
        for key in ("rgb", "r", "g", "b", "a"):
            value = item.get(key)
            if value is None:
                continue
            color_only = value in {"diffuse", "specular"}
            if value not in TEMPLATE_SOURCES or (key == "rgb" and value not in RGB_SOURCES) or (key != "rgb" and color_only):
                raise HTTPException(400, f"Unknown source {value!r} for {key} in {item['suffix']}")
            entry[key] = value
        if "rgb" not in entry and "r" not in entry:
            raise HTTPException(400, f"Map {item['suffix']} needs rgb or r")
        clean.append(entry)
    root = DATA / "templates"
    root.mkdir(parents=True, exist_ok=True)
    path = root / f"{name}.json"
    path.write_text(json.dumps({"name": name, "label": str(body.get("label") or name)[:80], "maps": clean}, indent=2))
    return {"file": str(path)}


MASK_KEYS = {"generator", "generator_level", "generator_softness", "generator_invert", "generator_noise", "noise_scale", "generator_direction", "builder_ao", "builder_curvature", "builder_noise"}
GENERATORS = {"cavity", "edges", "thin", "top", "light", "position", "noise", "mask_builder", "dirt", "metal_edge_wear"}


@app.get("/api/smart-masks")
def list_smart_masks():
    root = DATA / "smart_masks"
    masks = []
    for path in sorted(root.glob("*.json")) if root.exists() else []:
        try:
            masks.append(json.loads(path.read_text()))
        except json.JSONDecodeError:
            continue
    return {"masks": masks}


@app.post("/api/smart-masks/{name}")
async def save_smart_mask(name: str, request: Request):
    if not SAFE.match(name) or name.startswith("."):
        raise HTTPException(400, "Names may use letters, digits, spaces, dot, dash and underscore")
    body = await request.json()
    settings = body.get("settings") if isinstance(body, dict) else None
    if not isinstance(settings, dict) or settings.get("generator") not in GENERATORS:
        raise HTTPException(400, f"An intelligent mask needs a generator: {', '.join(sorted(GENERATORS))}")
    clean = {}
    for key, value in settings.items():
        if key not in MASK_KEYS:
            continue
        if key == "generator_direction":
            if not isinstance(value, list) or len(value) != 3 or not all(isinstance(v, (int, float)) for v in value):
                raise HTTPException(400, "generator_direction is three numbers")
        elif key == "generator":
            pass
        elif key == "generator_invert":
            value = bool(value)
        elif not isinstance(value, (int, float)):
            raise HTTPException(400, f"{key} must be a number")
        clean[key] = value
    root = DATA / "smart_masks"
    root.mkdir(parents=True, exist_ok=True)
    path = root / f"{name}.json"
    entry = {"name": name, "settings": clean}
    thumbnail = clean_thumbnail(body.get("thumbnail"))
    if thumbnail:
        entry["thumbnail"] = thumbnail
    path.write_text(json.dumps(entry, indent=2))
    return {"file": str(path)}


@app.get("/api/environments/{name}")
def read_environment(name: str):
    path = DATA / "environments" / name
    environments.ensure(name, DATA / "environments")
    if not SAFE.match(name) or name.startswith(".") or path.suffix.lower() != ".hdr" or not path.is_file():
        raise HTTPException(404, "Put the environment in the workspace environments folder as a Radiance .hdr file")
    return FileResponse(path)


@app.get("/api/images/{name}")
def read_image(name: str):
    path = DATA / "images" / name
    if not SAFE.match(name) or name.startswith(".") or path.suffix.lower() not in {".png", ".jpg", ".jpeg", ".webp"} or not path.is_file():
        raise HTTPException(404, "Put the image in the workspace images folder (png, jpg or webp)")
    return FileResponse(path)


@app.get("/api/scripts")
def list_scripts():
    folder = DATA / "scripts"
    names = sorted(p.name for p in folder.glob("*.py") if p.is_file() and SAFE.match(p.name) and not p.name.startswith(".")) if folder.is_dir() else []
    return {"scripts": names}


@app.get("/api/scripts/{name}")
def read_script(name: str):
    path = DATA / "scripts" / name
    if not SAFE.match(name) or name.startswith(".") or path.suffix.lower() != ".py" or not path.is_file():
        raise HTTPException(404, "Put the script in the workspace scripts folder (.py)")
    return FileResponse(path, media_type="text/plain")


MESH_SUFFIXES = {".glb", ".gltf", ".obj", ".fbx", ".usd", ".usda", ".usdc", ".usdz", ".ply", ".dae"}


@app.get("/api/meshes/{name}")
def read_mesh(name: str):
    path = DATA / "meshes" / name
    if not SAFE.match(name) or name.startswith(".") or path.suffix.lower() not in MESH_SUFFIXES or not path.is_file():
        raise HTTPException(404, "Put the mesh in the workspace meshes folder (glb, gltf, obj, fbx, usd, usda, usdc, usdz, ply or dae)")
    return FileResponse(path)


@app.get("/api/projects")
def list_projects():
    root = DATA / "projects"
    if not root.exists():
        return {"projects": []}
    return {"projects": sorted(p.name for p in root.iterdir() if (p / "project.json").exists() or (p / "project.fsproj").exists())}


@app.post("/api/projects/{name}/file")
async def save_project_file(name: str, request: Request):
    """The editor's full project file (stack and layer pixels, see projectBlob in the editor), stored as sent."""
    path = folder("projects", name)
    data = await request.body()
    if data[:4] != b"FSPJ":
        raise HTTPException(400, "Not a FormSurface project file")
    target = path / "project.fsproj"
    partial = path / "project.fsproj.part"
    partial.write_bytes(data)
    partial.replace(target)
    return {"folder": str(path), "file": str(target), "bytes": len(data)}


@app.get("/api/projects/{name}/file")
def read_project_file(name: str):
    path = DATA / "projects" / name / "project.fsproj"
    if not SAFE.match(name) or not path.exists():
        raise HTTPException(404, "No such project")
    return FileResponse(path, media_type="application/octet-stream")


@app.get("/api/projects/{name}")
def read_project(name: str):
    path = DATA / "projects" / name
    if not SAFE.match(name) or not (path / "project.json").exists():
        raise HTTPException(404, "No such project")
    return {"folder": str(path), "document": json.loads((path / "project.json").read_text()), "maps": sorted(p.name for p in path.glob("*.png"))}


def ocio_config(name: str):
    if not SAFE.match(name) or name.startswith("."):
        raise HTTPException(400, "Config names may use letters, digits, spaces, dot, dash and underscore")
    try:
        return ocio.load(name, DATA / "ocio")
    except RuntimeError as error:
        raise HTTPException(501, str(error))
    except FileNotFoundError as error:
        raise HTTPException(404, str(error))
    except Exception as error:
        raise HTTPException(422, f"OpenColorIO could not read {name}: {error}")


@app.get("/api/ocio")
def list_ocio():
    folder = DATA / "ocio"
    files = sorted(p.name for p in folder.glob("*.ocio") if p.is_file() and SAFE.match(p.name)) if folder.is_dir() else []
    return {"available": ocio.available(), "version": ocio.version(), "builtin": ocio.builtin_names(), "files": files}


@app.get("/api/ocio/{name}")
def describe_ocio(name: str):
    return ocio.describe(ocio_config(name))


@app.post("/api/ocio/{name}/convert")
async def ocio_convert(name: str, request: Request, src: str, dst: str = ""):
    config = ocio_config(name)
    body = await request.body()
    if not body or len(body) % 12:
        raise HTTPException(400, "Send RGB float32 triples")
    rgb = array("f")
    rgb.frombytes(body)
    try:
        out = ocio.convert(config, src, list(rgb), dst)
    except Exception as error:
        raise HTTPException(422, f"OpenColorIO could not convert from {src}: {error}")
    return Response(array("f", out).tobytes(), media_type="application/octet-stream", headers={"X-Space": dst or ocio.viewport_space(config)})


@app.get("/api/ocio/{name}/lut")
def ocio_lut(name: str, display: str, view: str, look: str = "", source: str = ""):
    config = ocio_config(name)
    try:
        data, meta = ocio.bake(config, display, view, look, source)
    except Exception as error:
        raise HTTPException(422, f"OpenColorIO could not build {display} / {view}: {error}")
    return Response(array("f", data).tobytes(), media_type="application/octet-stream", headers={"X-Lut": json.dumps(meta)})


STATIC = Path(os.environ.get("FORMSURFACE_STATIC", ROOT / "frontend" / "dist"))


@app.get("/{path:path}", include_in_schema=False)
def frontend_files(path: str):
    if path.startswith("api/") or not (STATIC / "index.html").is_file():
        raise HTTPException(404, "Not found")
    target = (STATIC / path).resolve()
    if path and target.is_file() and STATIC.resolve() in target.parents:
        return FileResponse(target)
    return FileResponse(STATIC / "index.html")
