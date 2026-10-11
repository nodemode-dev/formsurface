"""Imported images and environments, with their color space."""

from . import tool


def import_image(file: str, target: str = "fill", layer_id: int | None = None, color_space: str | None = None) -> dict:
    """A png, jpg or webp from the workspace images folder as fill, alpha, stencil or mask."""
    return tool("load_image", file=file, target=target, layer_id=layer_id, color_space=color_space)


def list_resources() -> list:
    return tool("set_color_space").get("resources", [])


def set_color_space(resource: str | int, color_space: str) -> dict:
    """srgb, linear or raw; resource is a name or the index from list_resources."""
    return tool("set_color_space", resource=resource, color_space=color_space)


def text(text: str, target: str = "alpha", font: str | None = None, bold: bool | None = None) -> dict:
    return tool("text_resource", text=text, target=target, font=font, bold=bold)
