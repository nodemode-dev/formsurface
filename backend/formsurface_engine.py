import json
import logging
import os
import runpy
import sys

if sys.stdin is None:
    sys.stdin = open(os.devnull, encoding="utf-8")
if sys.stdout is None:
    sys.stdout = open(os.devnull, "w", encoding="utf-8")
if sys.stderr is None:
    sys.stderr = open(os.devnull, "w", encoding="utf-8")

LOG_NAME = "formsurface-engine.log"
LOG_BYTES = 1_048_576
LOG_KEEP = 3


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


def release(data) -> None:
    from app import startup

    lock = data / startup.LOCK_NAME
    try:
        if int(json.loads(lock.read_text(encoding="utf-8")).get("pid", 0)) == os.getpid():
            lock.unlink()
    except (OSError, ValueError, AttributeError):
        return


def start_logging(data):
    from logging.handlers import RotatingFileHandler

    folder = data / "Logs"
    folder.mkdir(parents=True, exist_ok=True)
    handler = RotatingFileHandler(folder / LOG_NAME, maxBytes=LOG_BYTES, backupCount=LOG_KEEP, encoding="utf-8")
    handler.setFormatter(logging.Formatter("%(asctime)s %(levelname)s %(name)s: %(message)s"))
    log = logging.getLogger("formsurface")
    log.setLevel(logging.INFO)
    log.addHandler(handler)
    log.info("engine started, pid %s, data folder %s", os.getpid(), data)
    return folder


def exit_with_parent(data) -> None:
    from app import startup

    shell = startup.shell_pid()
    if shell <= 1:
        return

    def leave() -> None:
        logging.getLogger("formsurface").info("the app that started this engine is gone (process %s), stopping", shell)
        release(data)
        unpacked = getattr(sys, "_MEIPASS", "")
        if os.path.basename(unpacked).startswith("_MEI"):
            import shutil

            shutil.rmtree(unpacked, ignore_errors=True)
        os._exit(0)

    startup.watch(shell, leave)


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
    from pathlib import Path

    from app import startup

    try:
        data = Path(os.environ["FORMSURFACE_DATA"]).expanduser()
        data.mkdir(parents=True, exist_ok=True)
        logs = start_logging(data)
    except OSError as err:
        print(f"formsurface cannot use its data folder: {err}", file=sys.stderr)
        return 5
    startup.keep_errors(logs)
    log = logging.getLogger("formsurface")
    if startup.alone():
        beside = startup.app_beside(Path(sys.executable))
        if beside is not None and startup.open_app(beside):
            log.info("started without its app, so it opened %s and stopped", beside)
            return 0
        log.error("started without its app, and no app program was found near %s", sys.executable)
        print("formsurface-engine is a part of FormSurface and serves nothing on its own. Open FormSurface instead.", file=sys.stderr)
        return 4
    try:
        _, replaced = startup.claim(data, startup.shell_pid())
    except startup.Busy as err:
        log.error("%s", err)
        print(err, file=sys.stderr)
        return 2
    if replaced:
        log.warning("an engine left running without its app was stopped (process %s)", replaced)
    exit_with_parent(data)
    import uvicorn

    from app import window_key
    from app.main import app as fastapi_app

    if window_key.stdout_mode():
        print(window_key.LINE + window_key.KEY, flush=True)
    sweep_stale_unpacks()
    stop_on_request(fastapi_app)
    port = usable_port(int(os.environ.get("FORMSURFACE_PORT", "8766") or 8766)) or free_port()
    os.environ["FORMSURFACE_PORT"] = str(port)
    server = uvicorn.Server(uvicorn.Config(fastapi_app, host="127.0.0.1", port=port, log_level="info", timeout_graceful_shutdown=1))
    try:
        server.run()
    finally:
        release(data)
    return 0 if server.started else 3


if __name__ == "__main__":
    import multiprocessing

    multiprocessing.freeze_support()
    sys.exit(main())
