"""Texture, layer and mask export through output templates."""

from . import tool


def templates() -> dict:
    return tool("list_export_templates")


def export_textures(name: str | None = None, template: str | None = None, **options) -> dict:
    """custom_maps, format, bit_depth, padding, padding_width and padding_color; returns the folder and files."""
    return tool("export_maps", name=name, template=template, **options)


def save_template(name: str, maps: list[dict], label: str | None = None) -> dict:
    return tool("save_export_template", name=name, maps=maps, label=label)


def export_layer(layer_id: int, name: str | None = None) -> dict:
    return tool("export_layer", layer_id=layer_id, name=name)


def export_mask(layer_id: int, name: str | None = None) -> dict:
    return tool("export_layer_mask", layer_id=layer_id, name=name)
