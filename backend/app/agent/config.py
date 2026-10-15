import json
import os
import sys
import threading
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parents[3]
FROZEN = bool(getattr(sys, "frozen", False))
_lock = threading.Lock()


def port() -> int:
    return int(os.environ.get("FORMSURFACE_PORT", "8766"))


def service_url() -> str:
    return f"http://127.0.0.1:{port()}"


def data_dir() -> Path:
    raw = os.environ.get("FORMSURFACE_DATA")
    if raw:
        path = Path(raw).expanduser()
    elif FROZEN:
        path = Path.home() / "FormSurface"
    else:
        path = REPO_DIR / "workspace"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _prefs_path() -> Path:
    return data_dir() / "prefs.json"


def load_prefs() -> dict:
    try:
        value = json.loads(_prefs_path().read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return {}
    return value if isinstance(value, dict) else {}


def save_prefs(values: dict) -> dict:
    with _lock:
        merged = load_prefs() | {k: v for k, v in values.items() if isinstance(v, (str, int, float, bool))}
        _prefs_path().write_text(json.dumps(merged, indent=1), encoding="utf-8")
    return merged
