"""Mesh map baking from the paint mesh and from a high-poly mesh."""

from . import tool


def bake_mesh_maps(directions: int = 64, method: str | None = None, rays: int | None = None) -> dict:
    """Ambient occlusion, curvature, thickness, position and world normal for every texture set; method traced ray traces AO and thickness on the GPU."""
    return tool("bake_maps", directions=directions, method=method, rays=rays)


def bake_high_poly(mesh_file: str | None = None, **options) -> dict:
    """Normal, ID, height, opacity, AO and bent normal from a high-poly mesh: sample, cage_front, cage_rear, match_by_name, ao_rays, cage_file, distance_cage."""
    return tool("bake_high_poly", mesh_file=mesh_file, **options)
