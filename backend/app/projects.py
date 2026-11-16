from fastapi import APIRouter, HTTPException, Request
from fastapi.responses import FileResponse

from app import project_library as library

router = APIRouter()


def guarded(call, *args):
    try:
        return call(*args)
    except library.LibraryError as error:
        raise HTTPException(error.status, error.detail) from error
    except OSError as error:
        raise HTTPException(500, f"The project library could not be changed: {error.strerror or error}") from error


async def body_object(request: Request) -> dict:
    try:
        body = await request.json()
    except ValueError as error:
        raise HTTPException(400, "Send a JSON object") from error
    if not isinstance(body, dict):
        raise HTTPException(400, "Send a JSON object")
    return body


@router.get("/api/project-library")
def library_state():
    return {"counts": guarded(library.counts), "settings": guarded(library.read_settings)}


@router.post("/api/project-library/settings")
async def save_settings(request: Request):
    return guarded(library.write_settings, await body_object(request))


@router.get("/api/project-templates")
def list_templates():
    return {"categories": library.TEMPLATE_CATEGORIES, "templates": [{**t, "source": "builtin"} for t in library.TEMPLATES], "default": library.DEFAULT_TEMPLATE}


@router.post("/api/projects")
async def create_project(request: Request):
    body = await body_object(request)
    title = body.get("title") if isinstance(body.get("title"), str) else None
    template = body.get("template") if isinstance(body.get("template"), str) else None
    mesh = body.get("mesh") if isinstance(body.get("mesh"), str) else None
    return guarded(library.create, title, template, mesh)


@router.get("/api/projects/trash")
def list_trash():
    return {"projects": guarded(library.list_trash)}


@router.delete("/api/projects/trash")
def empty_trash():
    return {"ok": True, "removed": guarded(library.empty_trash)}


@router.post("/api/projects/trash/{entry}/restore")
def restore_project(entry: str):
    return guarded(library.restore, entry)


@router.delete("/api/projects/trash/{entry}")
def purge_project(entry: str):
    guarded(library.purge, entry)
    return {"ok": True}


@router.delete("/api/projects/{name}")
def trash_project(name: str):
    return guarded(library.move_to_trash, name)


@router.post("/api/projects/{name}/duplicate")
def duplicate_project(name: str):
    return guarded(library.duplicate, name)


@router.put("/api/projects/{name}/info")
async def put_info(name: str, request: Request):
    return guarded(library.put_info, name, await body_object(request))


@router.post("/api/projects/{name}/opened")
def mark_opened(name: str):
    return guarded(library.mark_opened, name)


@router.put("/api/projects/{name}/thumb")
async def put_thumb(name: str, request: Request):
    declared = request.headers.get("content-length", "")
    if declared.isdigit() and int(declared) > library.THUMB_LIMIT:
        raise HTTPException(400, "A preview is an image of at most 2 MB")
    data = bytearray()
    async for chunk in request.stream():
        data.extend(chunk)
        if len(data) > library.THUMB_LIMIT:
            raise HTTPException(400, "A preview is an image of at most 2 MB")
    guarded(library.put_thumb, name, bytes(data))
    return {"ok": True}


@router.get("/api/projects/{name}/thumb")
def get_thumb(name: str):
    file, kind = guarded(library.thumb, name)
    return FileResponse(file, media_type=kind, headers={"Cache-Control": "no-cache"})


@router.put("/api/projects/{name}/mesh")
async def put_mesh(name: str, request: Request, file: str = ""):
    path, target, original = guarded(library.mesh_target, name, file)
    declared = request.headers.get("content-length", "")
    if declared.isdigit() and int(declared) > library.MESH_LIMIT:
        raise HTTPException(400, "The mesh is larger than 1 GB")
    partial = target.with_name(f"{target.name}.part")
    if not library.inside(partial):
        raise HTTPException(400, library.OUTSIDE)
    size = 0
    try:
        with partial.open("wb") as handle:
            async for chunk in request.stream():
                size += len(chunk)
                if size > library.MESH_LIMIT:
                    raise HTTPException(400, "The mesh is larger than 1 GB")
                handle.write(chunk)
        if size == 0:
            raise HTTPException(400, "The mesh file is empty")
        return guarded(library.finish_mesh, path, target, partial, original)
    finally:
        partial.unlink(missing_ok=True)


@router.get("/api/projects/{name}/mesh")
def get_mesh(name: str):
    file, original = guarded(library.mesh, name)
    return FileResponse(file, media_type="application/octet-stream", headers={"X-Mesh-Name": original.encode("ascii", "replace").decode("ascii")})


def detailed() -> list[dict]:
    return guarded(library.list_projects)
