import json
import os
import runpy
import sys


def resource_dir() -> str:
    return getattr(sys, "_MEIPASS", os.path.dirname(os.path.abspath(__file__)))


def selfcheck() -> int:
    info = {"python": sys.version.split()[0]}
    missing = []
    for name in ("fastapi", "uvicorn", "httpx", "mcp", "PyOpenColorIO", "app.main", "app.agent.router", "app.agent.terminal"):
        try:
            __import__(name)
        except Exception as exc:
            missing.append(f"{name}: {exc}")
    info["frontend_bundled"] = os.path.isfile(os.path.join(resource_dir(), "frontend_dist", "index.html"))
    info["mcp_bundled"] = os.path.isfile(os.path.join(resource_dir(), "mcp", "server.py"))
    info["missing"] = missing
    print(json.dumps(info))
    return 1 if missing or not info["frontend_bundled"] or not info["mcp_bundled"] else 0


def usable_port(port: int) -> int:
    import socket

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.settimeout(0.5)
        return 0 if probe.connect_ex(("127.0.0.1", port)) == 0 else port


def free_port() -> int:
    import socket

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def run_mcp() -> int:
    folder = os.path.join(resource_dir(), "mcp")
    sys.path.insert(0, folder)
    sys.argv = [os.path.join(folder, "server.py")]
    runpy.run_path(sys.argv[0], run_name="__main__")
    return 0


def main() -> int:
    if "--selfcheck" in sys.argv:
        return selfcheck()
    if "--mcp" in sys.argv:
        return run_mcp()
    bundled = os.path.join(resource_dir(), "frontend_dist")
    if os.path.isdir(bundled):
        os.environ.setdefault("FORMSURFACE_STATIC", bundled)
    os.environ.setdefault("FORMSURFACE_DATA", os.path.join(os.path.expanduser("~"), "FormSurface"))
    import uvicorn

    from app.main import app as fastapi_app

    port = usable_port(int(os.environ.get("FORMSURFACE_PORT", "8766") or 8766)) or free_port()
    os.environ["FORMSURFACE_PORT"] = str(port)
    print(f"FormSurface engine on http://127.0.0.1:{port}", flush=True)
    uvicorn.run(fastapi_app, host="127.0.0.1", port=port, log_level="info")
    return 0


if __name__ == "__main__":
    import multiprocessing

    multiprocessing.freeze_support()
    sys.exit(main())
