import runpy
import sys
from pathlib import Path

USAGE = "usage: ./formsurface script FILE.py [arguments]\nRuns the script with formsurface_api importable; it drives the editor that is open with Agent link on."


def main() -> None:
    if len(sys.argv) < 2 or sys.argv[1] in {"-h", "--help"}:
        print(USAGE)
        sys.exit(0 if len(sys.argv) > 1 else 2)
    script = Path(sys.argv[1]).resolve()
    if not script.is_file():
        raise SystemExit(f"No script at {script}")
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    sys.argv = [str(script), *sys.argv[2:]]
    runpy.run_path(str(script), run_name="__main__")


if __name__ == "__main__":
    main()
