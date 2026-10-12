import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import httpx

ROOT = Path(__file__).resolve().parents[1]
SERVICE = os.environ.get("FORMSURFACE_SERVICE", "http://127.0.0.1:8766")
EDITOR = "http://localhost:5176"

BROWSERS = [
    "google-chrome", "google-chrome-stable", "chromium", "chromium-browser", "microsoft-edge", "microsoft-edge-stable", "msedge", "chrome",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
]


def up(url: str) -> bool:
    try:
        return httpx.get(url, timeout=1, trust_env=False).status_code < 500
    except httpx.HTTPError:
        return False


def find_browser() -> str:
    chosen = os.environ.get("FORMSURFACE_BROWSER")
    for candidate in [chosen] if chosen else BROWSERS:
        path = shutil.which(candidate) or (candidate if Path(candidate).is_file() else None)
        if path:
            return path
    raise SystemExit("No Chrome, Chromium or Edge found for a headless run. Install one or set FORMSURFACE_BROWSER to its executable.")


class Headless:
    """Starts whatever is missing (service, editor dev server, a headless browser with Agent link on) and stops what it started."""

    def __init__(self, quiet: bool = False, size: int | None = None) -> None:
        self.started: list[subprocess.Popen] = []
        self.profile: str | None = None
        self.quiet = quiet
        self.size = size

    def spawn(self, args: list[str], cwd: Path, env: dict | None = None) -> subprocess.Popen:
        out = subprocess.DEVNULL
        group = {"start_new_session": True} if os.name == "posix" else {"creationflags": getattr(subprocess, "CREATE_NEW_PROCESS_GROUP", 0)}
        process = subprocess.Popen(args, cwd=cwd, env=env or os.environ.copy(), stdout=out, stderr=out, stdin=subprocess.DEVNULL, **group)
        self.started.append(process)
        return process

    def wait(self, check, seconds: float, what: str) -> None:
        deadline = time.monotonic() + seconds
        while not check():
            if time.monotonic() > deadline:
                self.stop()
                raise SystemExit(f"{what} did not start within {seconds:.0f} s")
            time.sleep(0.5)

    def say(self, text: str) -> None:
        if not self.quiet:
            print(f"headless: {text}", file=sys.stderr, flush=True)

    def __enter__(self) -> "Headless":
        if not up(f"{SERVICE}/api/health"):
            self.say("starting the service")
            self.spawn([sys.executable, "-m", "uvicorn", "app.main:app", "--host", "127.0.0.1", "--port", "8766"], ROOT / "backend")
            self.wait(lambda: up(f"{SERVICE}/api/health"), 30, "The FormSurface service")
        if not up(EDITOR):
            npx = shutil.which("npx") or shutil.which("npx.cmd")
            if not npx:
                raise SystemExit("npx is not on PATH; run ./formsurface install first")
            self.say("starting the editor")
            self.spawn([npx, "vite", "--port", "5176", "--strictPort"], ROOT / "frontend")
            self.wait(lambda: up(EDITOR), 90, "The editor dev server")
        browser = find_browser()
        self.profile = tempfile.mkdtemp(prefix="formsurface-headless-")
        extra = os.environ.get("FORMSURFACE_BROWSER_ARGS", "").split()
        if sys.platform.startswith("linux") and hasattr(os, "geteuid") and os.geteuid() == 0 and "--no-sandbox" not in extra:
            extra.append("--no-sandbox")
        xvfb = shutil.which("xvfb-run") if sys.platform.startswith("linux") and os.environ.get("FORMSURFACE_HEADLESS_MODE") != "native" else None
        prefix = [xvfb, "-a"] if xvfb else []
        mode = [] if xvfb else ["--headless=new"]
        self.say(f"opening the editor in {Path(browser).name} without a window ({'virtual X display' if xvfb else 'native headless'})")
        self.spawn([*prefix, browser, *mode, "--enable-unsafe-webgpu", "--enable-features=Vulkan,WebGPUService", "--ignore-gpu-blocklist",
                    "--no-first-run", "--no-default-browser-check", f"--user-data-dir={self.profile}", "--window-size=1280,800", *extra,
                    f"{EDITOR}/?agent=1" + (f"&size={self.size}" if self.size else "")], ROOT)
        self.wait(lambda: httpx.get(f"{SERVICE}/api/health", timeout=2, trust_env=False).json().get("editor_linked", False), 120, "A headless editor with Agent link")
        self.say("editor linked")
        return self

    def stop(self) -> None:
        for process in reversed(self.started):
            if process.poll() is not None:
                continue
            try:
                if os.name == "posix":
                    os.killpg(process.pid, signal.SIGTERM)
                else:
                    process.terminate()
                process.wait(timeout=10)
            except (ProcessLookupError, subprocess.TimeoutExpired):
                if os.name == "posix":
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                else:
                    process.kill()
        self.started.clear()
        if self.profile:
            shutil.rmtree(self.profile, ignore_errors=True)
            self.profile = None

    def __exit__(self, *_exc) -> None:
        self.stop()
