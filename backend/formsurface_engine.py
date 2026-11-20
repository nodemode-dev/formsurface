import json
import os
import runpy
import sys

if sys.stdin is None:
    sys.stdin = open(os.devnull, encoding="utf-8")
if sys.stdout is None:
    sys.stdout = open(os.devnull, "w", encoding="utf-8")
if sys.stderr is None:
    sys.stderr = open(os.devnull, "w", encoding="utf-8")


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


def exit_with_parent() -> None:
    """When the desktop app passes its process id, end this engine as soon as that process is gone."""
    raw = os.environ.get("FORMSURFACE_PARENT_PID", "")
    if not raw.isdigit():
        return
    parent = int(raw)

    def alive() -> bool:
        if os.name == "nt":
            import ctypes

            handle = ctypes.windll.kernel32.OpenProcess(0x1000, False, parent)
            if not handle:
                return False
            code = ctypes.c_ulong()
            ctypes.windll.kernel32.GetExitCodeProcess(handle, ctypes.byref(code))
            ctypes.windll.kernel32.CloseHandle(handle)
            return code.value == 259
        try:
            os.kill(parent, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return True
        return True

    def watch() -> None:
        import time

        while alive():
            time.sleep(1.5)
        unpacked = getattr(sys, "_MEIPASS", "")
        if os.path.basename(unpacked).startswith("_MEI"):
            import shutil

            shutil.rmtree(unpacked, ignore_errors=True)
        os._exit(0)

    import threading

    threading.Thread(target=watch, name="parent-watch", daemon=True).start()


UNPACK_MARK = "formsurface-engine.pid"


def sweep_stale_unpacks() -> None:
    """Removes the temporary folders that earlier launches of this engine left behind when they were forced closed."""
    unpacked = getattr(sys, "_MEIPASS", "")
    if os.name != "posix" or not os.path.basename(unpacked).startswith("_MEI"):
        return
    import re
    import shutil
    import threading

    def alive(pid: int) -> bool:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        except PermissionError:
            return False
        return True

    def owner(folder: str, name: str) -> int | None:
        named = re.fullmatch(r"_MEI([0-9a-f]{8})[A-Za-z0-9]{6}", name)
        if named:
            return int(named.group(1), 16)
        try:
            with open(os.path.join(folder, UNPACK_MARK), encoding="ascii") as mark:
                raw = mark.read().strip()
        except (OSError, ValueError):
            return None
        return int(raw) if raw.isdigit() else None

    def run() -> None:
        root, own = os.path.dirname(unpacked), os.path.basename(unpacked)
        try:
            with open(os.path.join(unpacked, UNPACK_MARK), "w", encoding="ascii") as mark:
                mark.write(str(os.getpid()))
            names = os.listdir(root)
        except OSError:
            return
        for name in names:
            folder = os.path.join(root, name)
            if not name.startswith("_MEI") or name == own or os.path.islink(folder):
                continue
            pid = owner(folder, name)
            if pid is None or alive(pid):
                continue
            shutil.rmtree(folder, ignore_errors=True)

    threading.Thread(target=run, name="unpack-sweep", daemon=True).start()


def stop_on_request(app) -> None:
    import signal
    import threading

    def shutdown() -> dict:
        threading.Timer(0.2, signal.raise_signal, args=(signal.SIGINT,)).start()
        last = threading.Timer(3.0, os._exit, args=(0,))
        last.daemon = True
        last.start()
        return {"closing": True}

    app.post("/api/shutdown")(shutdown)


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

    from app import window_key
    from app.main import app as fastapi_app

    if window_key.stdout_mode():
        print(window_key.LINE + window_key.KEY, flush=True)
    exit_with_parent()
    sweep_stale_unpacks()
    stop_on_request(fastapi_app)
    port = usable_port(int(os.environ.get("FORMSURFACE_PORT", "8766") or 8766)) or free_port()
    os.environ["FORMSURFACE_PORT"] = str(port)
    uvicorn.run(fastapi_app, host="127.0.0.1", port=port, log_level="info", timeout_graceful_shutdown=1)
    return 0


if __name__ == "__main__":
    import multiprocessing

    multiprocessing.freeze_support()
    sys.exit(main())
