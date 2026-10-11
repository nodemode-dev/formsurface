import base64
import os
from typing import Annotated, Literal

import httpx
from mcp.server.mcpserver import Image, MCPServer
from pydantic import Field

SERVICE = os.environ.get("FORMSURFACE_SERVICE", "http://127.0.0.1:8766").rstrip("/")
if httpx.URL(SERVICE).host not in ("127.0.0.1", "localhost", "::1"):
    raise SystemExit("FormSurface MCP only talks to the local FormSurface service")

server = MCPServer(
    "formsurface",
    instructions=(
        "Drive the FormSurface 3D texture painter that is open in the person's browser. "
        "Call formsurface_status first. Painting happens on the active layer with the current brush and material; "
        "strokes are given in normalized viewport coordinates (0,0 top left, 1,1 bottom right) for the current camera, "
        "so set the camera, take a screenshot to see where the surface is, then paint. Every stroke and fill can be undone."
    ),
)


def call(command: str, args: dict | None = None, timeout: float = 60) -> dict:
    try:
        response = httpx.post(f"{SERVICE}/api/agent", json={"command": command, "args": args or {}, "timeout": timeout}, timeout=timeout + 5, trust_env=False)
    except httpx.ConnectError as error:
        raise RuntimeError("The FormSurface service is not running. Start it with ./formsurface") from error
    if response.status_code >= 400:
        try:
            detail = response.json().get("detail", response.text)
        except ValueError:
            detail = response.text
        raise RuntimeError(f"FormSurface: {detail}")
    return response.json()["result"]


def drop_none(**values) -> dict:
    return {k: v for k, v in values.items() if v is not None}


Unit = Annotated[float, Field(ge=0.0, le=1.0)]


@server.tool()
def formsurface_status() -> dict:
    """Read the open document: mesh, texture size, layers with ids, active layer, brush, paint material and camera."""
    return call("status")


@server.tool()
def formsurface_load_sample(kind: Literal["sphere", "torus", "capsule", "knot"]) -> dict:
    """Replace the mesh with a built-in sample. Layers are kept."""
    return call("load_sample", {"kind": kind})


@server.tool()
def formsurface_set_camera(yaw_degrees: float = 35.0, pitch_degrees: float = 15.0, zoom: Annotated[float, Field(gt=0.1, le=10)] = 1.0) -> dict:
    """Orbit the camera around the mesh. yaw turns around the vertical axis, pitch tilts (positive looks from above), zoom 1 frames the whole mesh, smaller is closer."""
    return call("camera", {"yaw": yaw_degrees, "pitch": pitch_degrees, "zoom": zoom})


@server.tool()
def formsurface_set_brush(
    size_px: Annotated[float | None, Field(ge=1, le=400)] = None,
    hardness: Unit | None = None,
    flow: Unit | None = None,
    stroke_opacity: Unit | None = None,
    spacing: Annotated[float | None, Field(ge=0.02, le=1)] = None,
    eraser: bool | None = None,
    mirror_x: bool | None = None,
    backface_culling: bool | None = None,
) -> dict:
    """Change brush settings. Size is in screen pixels of the viewport. Only the values you pass change."""
    return call("brush", drop_none(size_px=size_px, hardness=hardness, flow=flow, opacity=stroke_opacity, spacing=spacing, eraser=eraser, mirror_x=mirror_x, backface_culling=backface_culling))


@server.tool()
def formsurface_set_paint_material(
    base_color: Annotated[str | None, Field(pattern=r"^#[0-9a-fA-F]{6}$")] = None,
    roughness: Unit | None = None,
    metallic: Unit | None = None,
    height: Unit | None = None,
    paint_color: bool | None = None,
    paint_roughness: bool | None = None,
    paint_metallic: bool | None = None,
    paint_height: bool | None = None,
) -> dict:
    """Set what strokes and fills apply: channel values and which channels are painted. Height 0.5 is flat, above raises, below carves."""
    return call("material", drop_none(base_color=base_color, roughness=roughness, metallic=metallic, height=height, use_color=paint_color, use_roughness=paint_roughness, use_metallic=paint_metallic, use_height=paint_height))


@server.tool()
def formsurface_set_base_material(
    base_color: Annotated[str | None, Field(pattern=r"^#[0-9a-fA-F]{6}$")] = None,
    roughness: Unit | None = None,
    metallic: Unit | None = None,
) -> dict:
    """Set the material under all layers."""
    return call("base", drop_none(base_color=base_color, roughness=roughness, metallic=metallic))


@server.tool()
def formsurface_add_layer(name: str = "Agent layer") -> dict:
    """Add a paint layer above the active one and make it active."""
    return call("add_layer", {"name": name})


@server.tool()
def formsurface_update_layer(
    layer_id: int,
    name: str | None = None,
    visible: bool | None = None,
    opacity: Unit | None = None,
    blend: Literal["normal", "multiply", "screen", "overlay", "add", "subtract", "darken", "lighten"] | None = None,
    make_active: bool = False,
) -> dict:
    """Rename, show or hide, set opacity or blend mode of a layer, or make it the active layer."""
    return call("update_layer", drop_none(id=layer_id, name=name, visible=visible, opacity=opacity, blend=blend, active=make_active))


@server.tool()
def formsurface_move_layer(layer_id: int, offset: int) -> dict:
    """Move a layer up (positive) or down (negative) in the stack."""
    return call("move_layer", {"id": layer_id, "offset": offset})


@server.tool()
def formsurface_remove_layer(layer_id: int) -> dict:
    """Delete a layer. The last remaining layer cannot be deleted."""
    return call("remove_layer", {"id": layer_id})


@server.tool()
def formsurface_fill_layer() -> dict:
    """Fill the whole active layer with the current paint material (only the channels that are switched on)."""
    return call("fill")


@server.tool()
def formsurface_paint_stroke(
    points: Annotated[list[tuple[float, float]], Field(min_length=1, max_length=2000)],
    pressure: Unit = 1.0,
) -> dict:
    """Paint one stroke on the active layer through the current camera. points are normalized viewport coordinates; consecutive points are joined, and parts of the stroke that miss the mesh do not paint."""
    return call("stroke", {"points": [list(p) for p in points], "pressure": pressure}, timeout=120)


@server.tool()
def formsurface_undo(steps: Annotated[int, Field(ge=1, le=24)] = 1) -> dict:
    """Undo strokes and fills."""
    return call("undo", {"steps": steps})


@server.tool()
def formsurface_redo(steps: Annotated[int, Field(ge=1, le=24)] = 1) -> dict:
    """Redo strokes and fills."""
    return call("redo", {"steps": steps})


@server.tool()
def formsurface_screenshot() -> Image:
    """Render the viewport (without the brush cursor) and return it as a PNG image."""
    result = call("screenshot", timeout=120)
    return Image(data=base64.b64decode(result["png_base64"]), format="png")


@server.tool()
def formsurface_export_maps(name: str | None = None) -> dict:
    """Export base color, roughness, metallic, height and normal PNG maps into the FormSurface workspace and return the folder."""
    return call("export", drop_none(name=name), timeout=180)


if __name__ == "__main__":
    server.run("stdio")
