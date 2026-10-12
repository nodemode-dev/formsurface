"""Viewport lighting, post effects, the 2D view and screenshots."""

from pathlib import Path

from . import tool


def set_environment(**options) -> dict:
    """file (.hdr in the workspace environments folder) or studio, rotation_degrees, blur, background, exposure."""
    return tool("set_environment", **options)


def path_trace(**options) -> dict:
    """on, bounces (1 to 12), max_samples (64, 256, 1024, 4096), samples to render before returning."""
    return tool("path_trace", **options)


def set_post_effects(**effects) -> dict:
    return tool("set_post_effects", **effects)


def view_2d(**options) -> dict:
    return tool("view_2d", **options)


def screenshot(path: str | None = None) -> bytes:
    """PNG bytes of the viewport, also written to path when given."""
    data = tool("screenshot")
    if path:
        Path(path).write_bytes(data)
    return data
