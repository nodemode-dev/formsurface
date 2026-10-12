"""Project level calls: the open document, meshes, undo and the camera."""

from . import tool


def status() -> dict:
    """Mesh, texture size, texture sets, layers of the active set, brush, paint material and camera."""
    return tool("status")


def load_sample(kind: str = "sphere") -> dict:
    """Load a built-in mesh: sphere, torus, capsule, knot or sets."""
    return tool("load_sample", kind=kind)


def load_mesh(file: str, reproject: bool | None = None) -> dict:
    """Load a mesh from the workspace meshes folder (glb, gltf, obj, fbx, usd, usda, usdc, usdz, ply or dae); reproject carries the first texture set's painting over by surface position."""
    return tool("load_mesh", file=file, reproject=reproject)


def export_mesh(name: str | None = None, ngons: bool | None = None, subdivide: int | None = None, displacement: float | None = None) -> dict:
    """Write the mesh as OBJ with its UDIM UVs into the workspace exports folder, optionally keeping n-gons or tessellated and displaced by height."""
    return tool("export_mesh", name=name, ngons=ngons, subdivide=subdivide, displacement=displacement)


def undo(steps: int = 1) -> dict:
    return tool("undo", steps=steps)


def redo(steps: int = 1) -> dict:
    return tool("redo", steps=steps)


def set_camera(yaw_degrees: float = 35.0, pitch_degrees: float = 15.0, zoom: float = 1.0) -> dict:
    return tool("set_camera", yaw_degrees=yaw_degrees, pitch_degrees=pitch_degrees, zoom=zoom)
