import runpy
import sys
from pathlib import Path

USAGE = "usage: ./formsurface script [--headless [--size 1024|2048|4096]] FILE.py [arguments]\nRuns the script with formsurface_api importable against the editor that is open with Agent link on.\n--headless starts the service, the editor and a windowless Chrome, Chromium or Edge as needed, and stops them afterwards."


def main() -> None:
    args = sys.argv[1:]
    headless = bool(args) and args[0] == "--headless"
    if headless:
        args = args[1:]
    size = None
    if len(args) > 1 and args[0] == "--size":
        size = int(args[1]) if args[1] in {"1024", "2048", "4096"} else None
        if size is None:
            raise SystemExit("--size is 1024, 2048 or 4096")
        args = args[2:]
    if not args or args[0] in {"-h", "--help"}:
        print(USAGE)
        sys.exit(0 if args else 2)
    script = Path(args[0]).resolve()
    if not script.is_file():
        raise SystemExit(f"No script at {script}")
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    sys.argv = [str(script), *args[1:]]
    if headless:
        from headless import Headless
        with Headless(size=size):
            runpy.run_path(str(script), run_name="__main__")
        return
    runpy.run_path(str(script), run_name="__main__")


if __name__ == "__main__":
    main()
