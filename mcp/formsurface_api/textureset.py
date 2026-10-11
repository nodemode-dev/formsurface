"""Texture sets (one per material, split per UDIM tile) and the UV tiles workflow."""

from . import tool


def list_sets() -> list:
    return tool("list_texture_sets").get("texture_sets", [])


def select(name: str | None = None, index: int | None = None) -> dict:
    """Make a texture set active by name (body, trim.1002) or index."""
    return tool("select_texture_set", name=name, index=index)


def uv_tiles(on: bool | None = None) -> dict:
    """Share one layer stack across a material's UDIM tiles (on), or read the current state."""
    return tool("uv_tiles", on=on)
