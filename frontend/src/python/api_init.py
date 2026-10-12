"""FormSurface Python API, embedded in the editor: the same modules and validated tools as ./formsurface script, run inside the page.

Modules follow Substance 3D Painter's API layout: project, textureset, layerstack, baking, export, resource and display, plus event and ui, which only exist embedded.
Every call raises FormSurfaceError when the editor refuses it. tool() reaches any formsurface_* tool by name.
"""

import server as _server

__all__ = ["FormSurfaceError", "tool", "tools", "project", "textureset", "layerstack", "baking", "export", "resource", "display", "event", "ui"]


class FormSurfaceError(RuntimeError):
    pass


def tools() -> list[str]:
    """Names of every tool the API can call."""
    return _server.server.names()


def tool(tool_name: str, /, **args):
    """Call a formsurface_* tool (the prefix is optional). Returns its result as a dict, or PNG bytes for screenshots."""
    full = tool_name if tool_name.startswith("formsurface_") else f"formsurface_{tool_name}"
    clean = {k: v for k, v in args.items() if v is not None}
    try:
        return _server.server.call(full, clean)
    except Exception as error:
        raise FormSurfaceError(str(error)) from error


from . import baking, display, event, export, layerstack, project, resource, textureset, ui  # noqa: E402
