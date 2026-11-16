import json
import os
import re
import shutil
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = Path(os.environ.get("FORMSURFACE_DATA", ROOT / "workspace"))
PROJECTS = DATA / "projects"
TRASH = PROJECTS / ".trash"
SETTINGS = PROJECTS / ".home.json"

SAFE = re.compile(r"^[A-Za-z0-9 _.-]{1,80}\Z")
ENTRY = re.compile(r"^([A-Za-z0-9 _.-]{1,80})-(\d{1,12})\Z")
RESERVED = {"trash"}
PROJECT_FILE = "project.fsproj"
INFO_FILE = "info.json"
THUMB_FILE = "thumb.webp"
MESH_SUFFIXES = {".glb", ".gltf", ".obj", ".fbx", ".usd", ".usda", ".usdc", ".usdz", ".ply", ".dae"}
THUMB_LIMIT = 2_000_000
MESH_LIMIT = 1 << 30
THUMB_TYPES = ((b"RIFF", "image/webp"), (b"\xff\xd8\xff", "image/jpeg"), (b"\x89PNG\r\n\x1a\n", "image/png"))
OUTSIDE = "The project library does not follow a link that leads out of it"

TEMPLATE_CATEGORIES = ["Basics", "UV layout", "Texture sets", "Baking"]
TEMPLATES = [
    {"id": "sphere", "name": "Sphere", "category": "Basics", "tags": ["sphere", "ball", "primitive"], "description": "A smooth sphere with one texture set. The quickest way to try brushes, materials and masks.", "sample": "sphere"},
    {"id": "capsule", "name": "Capsule", "category": "Basics", "tags": ["capsule", "pill", "primitive"], "description": "A capsule with one texture set, for strokes that run along a long rounded form.", "sample": "capsule"},
    {"id": "torus", "name": "Torus", "category": "Basics", "tags": ["torus", "ring", "donut", "primitive"], "description": "A torus with one texture set, for painting around a hole and across a wrapping seam.", "sample": "torus"},
    {"id": "knot", "name": "Torus Knot", "category": "UV layout", "tags": ["knot", "seams", "uv"], "description": "A torus knot with many UV seams, for checking that strokes cross seams cleanly.", "sample": "knot"},
    {"id": "uv2", "name": "Sphere with a second UV channel", "category": "UV layout", "tags": ["uv1", "second uv", "lightmap"], "description": "A sphere that carries a second UV channel, for exporting maps onto UV1.", "sample": "uv2"},
    {"id": "sets", "name": "Figure with Three Texture Sets", "category": "Texture sets", "tags": ["udim", "materials", "figure"], "description": "A figure with two materials and a body spread over UDIM tiles 1001 and 1002.", "sample": "sets"},
    {"id": "pair", "name": "Sphere Pair with High-Poly", "category": "Baking", "tags": ["bake", "high-poly", "normal map"], "description": "Two low-poly spheres with their high-poly pair loaded, ready to bake maps matched by name.", "sample": "pair"},
]
DEFAULT_TEMPLATE = "sphere"


class LibraryError(Exception):
    def __init__(self, status: int, detail: str) -> None:
        super().__init__(detail)
        self.status = status
        self.detail = detail


def check_id(name: str) -> str:
    if not isinstance(name, str) or not SAFE.match(name) or name.startswith(".") or name.strip() != name:
        raise LibraryError(400, "Names may use letters, digits, spaces, dot, dash and underscore")
    return name


def inside(path: Path) -> bool:
    try:
        return path.resolve() == PROJECTS.resolve() / path.relative_to(PROJECTS)
    except (OSError, ValueError):
        return False


def project_dir(name: str, need_file: bool = False) -> Path:
    path = PROJECTS / check_id(name)
    if name.lower() in RESERVED or path.is_symlink() or not path.is_dir() or not inside(path):
        raise LibraryError(404, "No such project")
    if need_file and not ((path / PROJECT_FILE).is_file() and inside(path / PROJECT_FILE)):
        raise LibraryError(404, "No such project")
    return path


def trash_dir(entry: str) -> tuple[Path, str, int]:
    match = ENTRY.match(entry) if isinstance(entry, str) else None
    if not match or entry.startswith("."):
        raise LibraryError(404, "That project is no longer in the Trash.")
    path = TRASH / entry
    if path.is_symlink() or not path.is_dir() or not inside(path):
        raise LibraryError(404, "That project is no longer in the Trash.")
    return path, match.group(1), int(match.group(2))


def write_bytes_atomic(target: Path, data: bytes) -> None:
    partial = target.with_name(f"{target.name}.part")
    if not inside(partial):
        raise LibraryError(400, OUTSIDE)
    partial.write_bytes(data)
    partial.replace(target)


def write_json_atomic(target: Path, value) -> None:
    write_bytes_atomic(target, json.dumps(value, indent=2).encode("utf-8"))


def read_json(path: Path) -> dict:
    if not inside(path):
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return value if isinstance(value, dict) else {}


def iso(seconds: float) -> str:
    return datetime.fromtimestamp(seconds, timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def stored_mesh(path: Path) -> Path | None:
    name = read_json(path / INFO_FILE).get("meshFile")
    if not isinstance(name, str) or not re.fullmatch(r"mesh\.[a-z0-9]{2,5}", name):
        return None
    file = path / name
    return file if file.is_file() and not file.is_symlink() else None


def summary(path: Path) -> dict:
    info = read_json(path / INFO_FILE)
    file = path / PROJECT_FILE
    saved = file.is_file()
    modified = file.stat().st_mtime if saved else path.stat().st_mtime
    opened = info.get("opened") if isinstance(info.get("opened"), (int, float)) else 0
    title = info.get("title") if isinstance(info.get("title"), str) and info.get("title").strip() else path.name
    return {
        "id": path.name,
        "title": title,
        "mesh": info.get("mesh") if isinstance(info.get("mesh"), str) else "",
        "sets": info.get("sets") if isinstance(info.get("sets"), int) else None,
        "layers": info.get("layers") if isinstance(info.get("layers"), int) else None,
        "template": info.get("template") if isinstance(info.get("template"), str) else "",
        "bytes": file.stat().st_size if saved else 0,
        "modified": iso(modified),
        "recent": iso(max(modified, opened)),
        "saved": saved,
        "thumb": (path / THUMB_FILE).is_file(),
        "meshStored": stored_mesh(path) is not None,
    }


def list_projects() -> list[dict]:
    if not PROJECTS.is_dir():
        return []
    found = []
    for path in PROJECTS.iterdir():
        if path.name.startswith(".") or path.name.lower() in RESERVED or path.is_symlink() or not SAFE.match(path.name):
            continue
        if (path / PROJECT_FILE).is_file():
            info = read_json(path / INFO_FILE)
            opened = info.get("opened") if isinstance(info.get("opened"), (int, float)) else 0
            found.append((max((path / PROJECT_FILE).stat().st_mtime, opened), path.name, summary(path)))
    found.sort(key=lambda item: (item[0], item[1]), reverse=True)
    return [item[2] for item in found]


def sweep_unsaved() -> None:
    if not PROJECTS.is_dir():
        return
    cutoff = time.time() - 86400
    for path in PROJECTS.iterdir():
        if path.name.startswith(".") or path.is_symlink() or not path.is_dir():
            continue
        if [p.name for p in path.iterdir()] == [INFO_FILE] and "created" in read_json(path / INFO_FILE) and path.stat().st_mtime < cutoff:
            shutil.rmtree(path, ignore_errors=True)


def unique_id(base: str) -> str:
    stem = re.sub(r"[^A-Za-z0-9 _.-]", "_", base)[:60].strip(" .") or "untitled"
    taken = {p.name.lower() for p in PROJECTS.iterdir()} if PROJECTS.is_dir() else set()
    taken |= RESERVED
    if stem.lower() not in taken:
        return stem
    for n in range(2, 10000):
        candidate = f"{stem} {n}"
        if candidate.lower() not in taken:
            return candidate
    raise LibraryError(409, "Too many projects share that name")


def template(tid: str) -> dict | None:
    return next((t for t in TEMPLATES if t["id"] == tid), None)


def create(title: str | None, template_id: str | None, mesh: str | None) -> dict:
    chosen = None
    if template_id:
        chosen = template(template_id)
        if chosen is None:
            raise LibraryError(404, "No such template")
    name = (title or "").strip()[:120] or (chosen["name"] if chosen else "Untitled project")
    PROJECTS.mkdir(parents=True, exist_ok=True)
    sweep_unsaved()
    path = PROJECTS / unique_id(name)
    path.mkdir()
    info = {"title": name, "created": time.time()}
    if chosen:
        info["template"] = chosen["id"]
        info["mesh"] = chosen["sample"]
    elif isinstance(mesh, str) and mesh:
        info["mesh"] = mesh[:200]
    write_json_atomic(path / INFO_FILE, info)
    return {**summary(path), "sample": chosen["sample"] if chosen else ""}


def put_info(name: str, patch: dict) -> dict:
    path = project_dir(name)
    info = read_json(path / INFO_FILE)
    for key, kind, limit in (("title", str, 120), ("mesh", str, 200), ("template", str, 80)):
        value = patch.get(key)
        if isinstance(value, kind) and value.strip():
            info[key] = value.strip()[:limit]
    for key in ("sets", "layers"):
        value = patch.get(key)
        if isinstance(value, int) and not isinstance(value, bool) and 0 <= value < 100000:
            info[key] = value
    write_json_atomic(path / INFO_FILE, info)
    return summary(path)


def mark_opened(name: str) -> dict:
    path = project_dir(name, need_file=True)
    info = read_json(path / INFO_FILE)
    info["opened"] = time.time()
    write_json_atomic(path / INFO_FILE, info)
    return summary(path)


def duplicate(name: str) -> dict:
    source = project_dir(name, need_file=True)
    info = read_json(source / INFO_FILE)
    title = f"{info.get('title') if isinstance(info.get('title'), str) and info.get('title').strip() else source.name} copy"[:120]
    target = PROJECTS / unique_id(f"{source.name} copy")
    staging = PROJECTS / f".copy-{target.name}"
    if staging.exists():
        shutil.rmtree(staging)
    shutil.copytree(source, staging, symlinks=True, ignore=shutil.ignore_patterns("*.part"))
    info["title"] = title
    info.pop("opened", None)
    info["created"] = time.time()
    write_json_atomic(staging / INFO_FILE, info)
    staging.replace(target)
    os.utime(target / PROJECT_FILE)
    return summary(target)


def move_to_trash(name: str) -> dict:
    source = project_dir(name)
    if not inside(TRASH):
        raise LibraryError(400, OUTSIDE)
    TRASH.mkdir(parents=True, exist_ok=True)
    stamp = int(time.time())
    while (TRASH / f"{source.name}-{stamp}").exists():
        stamp += 1
    entry = f"{source.name}-{stamp}"
    source.replace(TRASH / entry)
    settings = read_settings()
    changed = {key: [x for x in settings[key] if x != name] for key in ("pinned", "recentHidden")}
    if changed != settings:
        write_json_atomic(SETTINGS, changed)
    return {"ok": True, "entry": entry}


def list_trash() -> list[dict]:
    if not TRASH.is_dir() or not inside(TRASH):
        return []
    found = []
    for path in TRASH.iterdir():
        match = ENTRY.match(path.name)
        if not match or path.is_symlink() or not path.is_dir():
            continue
        info = read_json(path / INFO_FILE)
        title = info.get("title") if isinstance(info.get("title"), str) and info.get("title").strip() else match.group(1)
        found.append({
            "entry": path.name,
            "id": match.group(1),
            "title": title,
            "layers": info.get("layers") if isinstance(info.get("layers"), int) else None,
            "deletedAt": int(match.group(2)),
            "thumb": (path / THUMB_FILE).is_file(),
        })
    found.sort(key=lambda item: (item["deletedAt"], item["entry"]), reverse=True)
    return found


def restore(entry: str) -> dict:
    source, name, _ = trash_dir(entry)
    target = PROJECTS / name
    if target.exists() or name.lower() in RESERVED:
        target = PROJECTS / unique_id(name)
    source.replace(target)
    return summary(target)


def purge(entry: str) -> None:
    path, _, _ = trash_dir(entry)
    shutil.rmtree(path)


def empty_trash() -> int:
    removed = 0
    for item in list_trash():
        try:
            purge(item["entry"])
            removed += 1
        except (LibraryError, OSError):
            continue
    return removed


def put_thumb(name: str, data: bytes) -> None:
    path = project_dir(name)
    if not data or len(data) > THUMB_LIMIT:
        raise LibraryError(400, "A preview is an image of at most 2 MB")
    if not any(data.startswith(magic) for magic, _ in THUMB_TYPES):
        raise LibraryError(400, "A preview is a WebP, JPEG or PNG image")
    write_bytes_atomic(path / THUMB_FILE, data)


def thumb(name: str) -> tuple[Path, str]:
    file = project_dir(name) / THUMB_FILE
    if not file.is_file() or file.is_symlink():
        raise LibraryError(404, "No preview")
    with file.open("rb") as handle:
        head = handle.read(12)
    kind = next((media for magic, media in THUMB_TYPES if head.startswith(magic)), "image/webp")
    return file, kind


def mesh_target(name: str, filename: str) -> tuple[Path, Path, str]:
    path = project_dir(name)
    original = Path(filename or "").name
    suffix = Path(original).suffix.lower()
    if suffix not in MESH_SUFFIXES or not original or len(original) > 200:
        raise LibraryError(400, "A mesh is a glb, gltf, obj, fbx, usd, usda, usdc, usdz, ply or dae file")
    return path, path / f"mesh{suffix}", original


def finish_mesh(path: Path, target: Path, partial: Path, original: str) -> dict:
    partial.replace(target)
    for other in path.glob("mesh.*"):
        if other != target and other.suffix.lower() in MESH_SUFFIXES:
            other.unlink(missing_ok=True)
    info = read_json(path / INFO_FILE)
    info["meshFile"] = target.name
    info["mesh"] = original
    write_json_atomic(path / INFO_FILE, info)
    return {"ok": True, "bytes": target.stat().st_size}


def mesh(name: str) -> tuple[Path, str]:
    path = project_dir(name)
    file = stored_mesh(path)
    if file is None:
        raise LibraryError(404, "This project has no stored mesh")
    original = read_json(path / INFO_FILE).get("mesh")
    return file, original if isinstance(original, str) and original else file.name


def clean_ids(value) -> list[str]:
    if not isinstance(value, list):
        return []
    seen: list[str] = []
    for item in value:
        if isinstance(item, str) and SAFE.match(item) and not item.startswith(".") and item not in seen:
            seen.append(item)
    return seen[:500]


def read_settings() -> dict:
    raw = read_json(SETTINGS)
    return {"pinned": clean_ids(raw.get("pinned")), "recentHidden": clean_ids(raw.get("recentHidden"))}


def write_settings(patch: dict) -> dict:
    settings = read_settings()
    for key in ("pinned", "recentHidden"):
        if key in patch:
            settings[key] = clean_ids(patch[key])
    PROJECTS.mkdir(parents=True, exist_ok=True)
    write_json_atomic(SETTINGS, settings)
    return settings


def counts() -> dict:
    return {"projects": len(list_projects()), "templates": len(TEMPLATES), "trash": len(list_trash())}
