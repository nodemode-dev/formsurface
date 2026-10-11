"""The layer stack of the active texture set: paint, fill, filter and folder layers, masks, generators and smart materials.

Keyword arguments are those of the matching formsurface_* tool and are validated the same way."""

from . import tool


def layers() -> list:
    return tool("status").get("layers", [])


def add_paint(name: str = "Layer") -> int:
    return tool("add_layer", name=name)["id"]


def add_fill(name: str = "Fill", **settings) -> int:
    """projection, pattern, base_color, roughness, metallic, height, scale, rotation_degrees, planar_axis, sharpness and the fill_* channel switches."""
    return tool("add_fill_layer", name=name, **settings)["id"]


def set_fill(layer_id: int, **settings) -> dict:
    return tool("set_fill", layer_id=layer_id, **settings)


def add_filter(name: str = "Filter", **settings) -> int:
    return tool("add_filter_layer", name=name, **settings)["id"]


def set_filter(layer_id: int, **settings) -> dict:
    """Blur, sharpen, levels, invert, posterize, pixelate and color adjustments."""
    return tool("set_filter", layer_id=layer_id, **settings)


def add_folder(name: str = "Folder", layer_ids: list[int] | None = None) -> int:
    return tool("add_folder", name=name, layer_ids=layer_ids)["id"]


def set_folder(layer_id: int, folder_id: int) -> dict:
    return tool("set_layer_folder", layer_id=layer_id, folder_id=folder_id)


def update(layer_id: int, **changes) -> dict:
    """name, visible, opacity, blend (any of the 31 modes) and make_active."""
    return tool("update_layer", layer_id=layer_id, **changes)


def move(layer_id: int, offset: int) -> dict:
    return tool("move_layer", layer_id=layer_id, offset=offset)


def remove(layer_id: int) -> dict:
    return tool("remove_layer", layer_id=layer_id)


def merge_down(layer_id: int) -> dict:
    return tool("merge_down", layer_id=layer_id)


def flatten_folder(folder_id: int) -> dict:
    return tool("flatten_folder", folder_id=folder_id)


def flatten_stack(name: str | None = None) -> dict:
    return tool("flatten_stack", name=name)


def instance(layer_id: int, texture_set: str) -> dict:
    return tool("instance_layer", layer_id=layer_id, texture_set=texture_set)


def mask(layer_id: int, action: str = "keep", paint_target: str | None = None) -> dict:
    """action add_white, add_black, remove or keep; paint_target layer or mask."""
    return tool("layer_mask", layer_id=layer_id, action=action, paint_target=paint_target)


def mask_source(layer_id: int, source: str, **options) -> dict:
    """A generator name or id_color, with level, softness, invert, color, x, y and tolerance."""
    return tool("mask_source", layer_id=layer_id, source=source, **options)


def set_generator(layer_id: int, generator: str, **options) -> dict:
    return tool("set_generator", layer_id=layer_id, generator=generator, **options)


def apply_smart_material(name: str) -> dict:
    return tool("apply_smart_material", name=name)


def apply_smart_mask(layer_id: int, name: str) -> dict:
    return tool("apply_smart_mask", layer_id=layer_id, name=name)


def smart_materials() -> dict:
    return tool("list_smart_materials")


def paint_stroke(points: list[list[float]], pressure: list[float] | None = None) -> dict:
    """Viewport points from 0 to 1, painted with the current brush and paint material."""
    return tool("paint_stroke", points=points, pressure=pressure)


def set_brush(**settings) -> dict:
    return tool("set_brush", **settings)


def set_paint_material(**settings) -> dict:
    return tool("set_paint_material", **settings)


def set_base_material(**settings) -> dict:
    return tool("set_base_material", **settings)
