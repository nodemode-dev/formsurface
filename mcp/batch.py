import argparse
import asyncio
import base64
import json
import sys
import time
from pathlib import Path

import httpx

sys.path.insert(0, str(Path(__file__).resolve().parent))
import server as formsurface  # noqa: E402

USAGE = """Run a FormSurface batch script against the editor that is open with Agent link on.

A script is JSON: either a list of [tool, arguments] pairs or {"steps": [{"tool": ..., "args": {...}}]}.
Tools are the formsurface_* MCP tools, with the same arguments and checks.
Example: [["formsurface_load_mesh", {"file": "crate.glb"}], ["formsurface_bake_maps", {}],
          ["formsurface_apply_smart_material", {"name": "Rusty iron"}], ["formsurface_export_maps", {"template": "unreal"}]]"""


def steps_of(script: object) -> list[tuple[str, dict]]:
    raw = script.get("steps", []) if isinstance(script, dict) else script
    steps = []
    for item in raw if isinstance(raw, list) else []:
        if isinstance(item, list) and len(item) == 2 and isinstance(item[0], str) and isinstance(item[1], dict):
            steps.append((item[0], item[1]))
        elif isinstance(item, dict) and isinstance(item.get("tool"), str):
            steps.append((item["tool"], item.get("args") or {}))
        else:
            raise SystemExit(f"Step {len(steps) + 1} is neither [tool, arguments] nor {{\"tool\", \"args\"}}")
    if not steps:
        raise SystemExit("The script has no steps")
    return steps


def wait_for_editor(seconds: float) -> None:
    deadline = time.monotonic() + seconds
    while True:
        try:
            health = httpx.get(f"{formsurface.SERVICE}/api/health", timeout=2, trust_env=False).json()
        except httpx.HTTPError:
            raise SystemExit("The FormSurface service is not running. Start it with ./formsurface")
        if health.get("editor_linked"):
            return
        if time.monotonic() >= deadline:
            raise SystemExit("No editor is linked. Open http://localhost:5176 and press Agent link, or pass --wait to give yourself time.")
        time.sleep(1)


async def run(steps: list[tuple[str, dict]], out: Path, keep_going: bool, pause: float) -> int:
    known = {tool.name for tool in await formsurface.server.list_tools()}
    failures = 0
    for number, (tool, args) in enumerate(steps, 1):
        if tool not in known:
            print(f"{number:>3} {tool}: unknown tool", file=sys.stderr)
            return 2
        if number > 1 and pause > 0:
            await asyncio.sleep(pause)
        started = time.monotonic()
        parts = []
        try:
            result = await formsurface.server.call_tool(tool, args)
            error = getattr(result, "isError", None) or getattr(result, "is_error", False)
        except Exception as exc:
            result, error = None, True
            parts.append(str(exc))
        for content in getattr(result, "content", []):
            if getattr(content, "type", "") == "image":
                out.mkdir(parents=True, exist_ok=True)
                path = out / f"step{number:02d}_{tool}.png"
                path.write_bytes(base64.b64decode(content.data))
                parts.append(f"image saved to {path}")
            elif hasattr(content, "text"):
                parts.append(content.text.replace("\n", " ")[:400])
        print(f"{number:>3} {tool} ({time.monotonic() - started:.1f} s) {'FAILED ' if error else ''}{' '.join(parts)}")
        if error:
            failures += 1
            if not keep_going:
                return 1
    return 1 if failures else 0


def main() -> None:
    parser = argparse.ArgumentParser(prog="./formsurface batch", description=USAGE, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("script", help="JSON batch script")
    parser.add_argument("--wait", type=float, default=0, help="seconds to wait for an editor to link")
    parser.add_argument("--out", default="workspace/batch", help="folder for screenshots")
    parser.add_argument("--keep-going", action="store_true", help="continue after a failed step")
    parser.add_argument("--pause", type=float, default=0, help="seconds to wait between steps")
    parser.add_argument("--headless", action="store_true", help="start the service, the editor and a windowless Chrome, Chromium or Edge as needed, run, then stop them")
    parser.add_argument("--size", type=int, choices=[1024, 2048, 4096], help="texture size for a headless run (the editor's default otherwise)")
    options = parser.parse_args()
    steps = steps_of(json.loads(Path(options.script).read_text()))
    if options.headless:
        from headless import Headless
        with Headless(size=options.size):
            code = asyncio.run(run(steps, Path(options.out), options.keep_going, options.pause))
        sys.exit(code)
    wait_for_editor(options.wait)
    sys.exit(asyncio.run(run(steps, Path(options.out), options.keep_going, options.pause)))


if __name__ == "__main__":
    main()
