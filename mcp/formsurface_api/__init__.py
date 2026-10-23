"""FormSurface Python API: drives the editor that is open with Agent link on, through the same validated tools as the MCP server.

Modules follow the usual texture-painter scripting layout: project, textureset, layerstack, baking, export, resource and display.
Every call raises FormSurfaceError when the editor refuses it. tool() reaches any formsurface_* tool by name.
"""

import asyncio
import base64
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import server as _server  # noqa: E402

__all__ = ["FormSurfaceError", "tool", "tools", "project", "textureset", "layerstack", "baking", "export", "resource", "display"]


class FormSurfaceError(RuntimeError):
    pass


def tools() -> list[str]:
    """Names of every tool the API can call."""
    return sorted(t.name for t in asyncio.run(_server.server.list_tools()))


def tool(tool_name: str, /, **args):
    """Call a formsurface_* tool (the prefix is optional). Returns its result as a dict, or PNG bytes for screenshots."""
    full = tool_name if tool_name.startswith("formsurface_") else f"formsurface_{tool_name}"
    clean = {k: v for k, v in args.items() if v is not None}
    try:
        result = asyncio.run(_server.server.call_tool(full, clean))
    except Exception as error:
        raise FormSurfaceError(str(error).replace(f"Error executing tool {full}: ", "")) from error
    texts = []
    for content in getattr(result, "content", []) or []:
        if getattr(content, "type", "") == "image":
            return base64.b64decode(content.data)
        if hasattr(content, "text"):
            texts.append(content.text)
    if getattr(result, "isError", False):
        raise FormSurfaceError(" ".join(texts) or f"{full} failed")
    structured = getattr(result, "structuredContent", None)
    if isinstance(structured, dict):
        return structured.get("result", structured) if set(structured) == {"result"} else structured
    joined = "".join(texts)
    try:
        return json.loads(joined)
    except ValueError:
        return joined


from . import baking, display, export, layerstack, project, resource, textureset  # noqa: E402
