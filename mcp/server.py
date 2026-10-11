import base64
import os
from typing import Annotated, Literal

import httpx
from mcp.server.mcpserver import Image, MCPServer
from mcp.server.mcpserver.exceptions import ToolError
from pydantic import BaseModel, Field

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
        raise ToolError("The FormSurface service is not running. Start it with ./formsurface") from error
    if response.status_code >= 400:
        try:
            detail = response.json().get("detail", response.text)
        except ValueError:
            detail = response.text
        raise ToolError(f"FormSurface: {detail}")
    return response.json()["result"]


def drop_none(**values) -> dict:
    return {k: v for k, v in values.items() if v is not None}


Unit = Annotated[float, Field(ge=0.0, le=1.0)]


@server.tool()
def formsurface_status() -> dict:
    """Read the open document: mesh, texture size, texture sets and the active one, layers of the active set with ids, active layer, brush, paint material and camera."""
    return call("status")


@server.tool()
def formsurface_load_sample(kind: Literal["sphere", "torus", "capsule", "knot", "sets"]) -> dict:
    """Replace the mesh with a built-in sample. The first texture set keeps its layers. "sets" is a figure with two materials (body and trim) where the body spans UDIM tiles 1001 and 1002, giving three texture sets."""
    return call("load_sample", {"kind": kind})


@server.tool()
def formsurface_list_texture_sets() -> dict:
    """List texture sets: one per material, split per UDIM tile when a material's UVs leave 0 to 1. Each has its own layer stack, base material, bakes and exported maps."""
    return call("texture_sets")


@server.tool()
def formsurface_select_texture_set(name: str | None = None, index: int | None = None) -> dict:
    """Make a texture set active by name (for example body.1002 or trim) or index. Strokes, layers, fills, masks, smart materials and the base material then apply to it only; the viewport keeps showing every set."""
    return call("select_texture_set", drop_none(name=name, index=index))


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
    alignment: Literal["camera", "tangent_wrap", "tangent_planar", "uv"] | None = None,
    size_space: Literal["screen", "object"] | None = None,
    angle_degrees: Annotated[float | None, Field(ge=-180, le=180)] = None,
    follow_path: bool | None = None,
    tip: Literal["round", "square", "spray"] | None = None,
    spray_density: Annotated[float | None, Field(ge=0.02, le=1)] = None,
    lazy_px: Annotated[float | None, Field(ge=0, le=120)] = None,
    jitter_size: Unit | None = None,
    jitter_angle: Unit | None = None,
    jitter_position: Annotated[float | None, Field(ge=0, le=2)] = None,
    jitter_flow: Unit | None = None,
    symmetry: Literal["off", "mirror", "radial"] | None = None,
    symmetry_axis: Literal["x", "y", "z"] | None = None,
    radial_count: Annotated[int | None, Field(ge=2, le=32)] = None,
) -> dict:
    """Change brush settings. Size is in screen pixels (size_space screen) or thousandths of the mesh radius (object). alignment picks how the tip lies on the surface: camera projects it from the view, tangent_wrap follows curvature, tangent_planar lays it flat on the hit plane, uv paints in texture space. Only the values you pass change."""
    axis = {"x": 0, "y": 1, "z": 2}.get(symmetry_axis) if symmetry_axis else None
    return call("brush", drop_none(
        size_px=size_px, hardness=hardness, flow=flow, opacity=stroke_opacity, spacing=spacing, eraser=eraser, mirror_x=mirror_x, backface_culling=backface_culling,
        alignment=alignment, size_space=size_space, angle=angle_degrees, follow_path=follow_path, tip=tip, spray_density=spray_density, lazy_px=lazy_px,
        jitter_size=jitter_size, jitter_angle=jitter_angle, jitter_position=jitter_position, jitter_flow=jitter_flow,
        symmetry=symmetry, symmetry_axis=axis, radial_count=radial_count,
    ))


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


Projection = Literal["uv", "triplanar", "planar", "spherical", "cylindrical"]
PatternKind = Literal["solid", "checker", "stripes", "noise"]


def fill_settings(**values) -> dict:
    axis = values.pop("planar_axis", None)
    if axis is not None:
        values["axis"] = {"x": 0, "y": 1, "z": 2}[axis]
    return drop_none(**values)


@server.tool()
def formsurface_add_fill_layer(
    name: str = "Fill",
    projection: Projection = "uv",
    pattern: PatternKind = "solid",
    base_color: Annotated[str | None, Field(pattern=r"^#[0-9a-fA-F]{6}$")] = None,
    roughness: Unit | None = None,
    metallic: Unit | None = None,
    height: Unit | None = None,
    scale: Annotated[float | None, Field(gt=0, le=64)] = None,
    rotation_degrees: Annotated[float | None, Field(ge=-180, le=180)] = None,
    planar_axis: Literal["x", "y", "z"] | None = None,
    sharpness: Annotated[float | None, Field(ge=1, le=32)] = None,
    fill_color: bool | None = None,
    fill_roughness: bool | None = None,
    fill_metallic: bool | None = None,
    fill_height: bool | None = None,
) -> dict:
    """Add a fill layer above the active layer: channel values projected over the whole mesh, optionally through a pattern (the pattern decides where the fill shows). triplanar suits meshes without clean UVs; planar projects along one axis. Add a black mask with formsurface_layer_mask and paint white into it to reveal the fill only where painted."""
    settings = fill_settings(projection=projection, pattern=pattern, base_color=base_color, roughness=roughness, metallic=metallic, height=height, scale=scale,
                             rotation=rotation_degrees, planar_axis=planar_axis, sharpness=sharpness, use_color=fill_color, use_roughness=fill_roughness,
                             use_metallic=fill_metallic, use_height=fill_height)
    return call("add_fill_layer", {"name": name, "settings": settings})


@server.tool()
def formsurface_set_fill(
    layer_id: int,
    projection: Projection | None = None,
    pattern: PatternKind | None = None,
    base_color: Annotated[str | None, Field(pattern=r"^#[0-9a-fA-F]{6}$")] = None,
    roughness: Unit | None = None,
    metallic: Unit | None = None,
    height: Unit | None = None,
    scale: Annotated[float | None, Field(gt=0, le=64)] = None,
    rotation_degrees: Annotated[float | None, Field(ge=-180, le=180)] = None,
    offset_u: Annotated[float | None, Field(ge=-1, le=1)] = None,
    offset_v: Annotated[float | None, Field(ge=-1, le=1)] = None,
    planar_axis: Literal["x", "y", "z"] | None = None,
    sharpness: Annotated[float | None, Field(ge=1, le=32)] = None,
    amount: Unit | None = None,
) -> dict:
    """Change a fill layer's projection, pattern or values. Only the values you pass change."""
    settings = fill_settings(projection=projection, pattern=pattern, base_color=base_color, roughness=roughness, metallic=metallic, height=height, scale=scale,
                             rotation=rotation_degrees, offset_x=offset_u, offset_y=offset_v, planar_axis=planar_axis, sharpness=sharpness, opacity=amount)
    return call("set_fill", {"id": layer_id, "settings": settings})


@server.tool()
def formsurface_bake_maps(directions: Annotated[int, Field(ge=8, le=256)] = 64) -> dict:
    """Bake ambient occlusion, curvature, thickness, position and world normal from the current mesh on the GPU. Needed before fill generators cavity, edges and thin; more directions give smoother maps and take longer. Exported with formsurface_export_maps once baked."""
    return call("bake", {"directions": directions}, timeout=300)


@server.tool()
def formsurface_bake_high_poly(
    mesh_file: str | None = None,
    use_sample: bool = False,
    cage_front: Annotated[float, Field(ge=0, le=0.5)] = 0.05,
    cage_rear: Annotated[float, Field(ge=0, le=0.5)] = 0.05,
) -> dict:
    """Bake a tangent-space normal map and an ID map (one color per high-poly part) from a high-poly mesh onto the painted mesh. mesh_file is a glb, gltf, obj or fbx the person put in the workspace meshes folder; use_sample loads a low sphere with a bumpy two-part high-poly sphere for a demonstration. Cage distances are fractions of the mesh radius in front of and behind the low surface; raise them if parts of the high-poly are missed."""
    return call("bake_high", drop_none(file=mesh_file, sample=use_sample or None, front=cage_front, rear=cage_rear), timeout=300)


@server.tool()
def formsurface_list_smart_materials() -> dict:
    """List smart materials (built in and saved) with their layers, and the smart mask presets."""
    return call("smart_list")


@server.tool()
def formsurface_apply_smart_material(name: str) -> dict:
    """Add a smart material above the active layer in one step, as a folder named after it holding its layers (hide or fade the whole material through the folder). Its generators need baked mesh maps (formsurface_bake_maps) to place wear and dirt."""
    return call("smart_apply", {"name": name})


@server.tool()
def formsurface_save_smart_material(name: str) -> dict:
    """Save the current fill and filter layers as a smart material in the workspace library."""
    return call("smart_save", {"name": name})


@server.tool()
def formsurface_apply_smart_mask(layer_id: int, name: Literal["Edge wear", "Cavity dirt", "Dust from top", "Thin parts", "Protect edges"]) -> dict:
    """Apply a smart mask preset (a generator setup) to a fill layer."""
    return call("smart_mask", {"id": layer_id, "name": name})


@server.tool()
def formsurface_set_generator(
    layer_id: int,
    generator: Literal["none", "cavity", "edges", "thin", "top"],
    level: Unit = 0.5,
    softness: Annotated[float, Field(ge=0.001, le=0.5)] = 0.1,
    invert: bool = False,
) -> dict:
    """Drive where a fill layer shows from baked maps: cavity puts it in occluded crevices (dirt), edges on convex curvature (wear), thin on thin parts, top fades it from the top of the mesh down. level moves the threshold, softness widens the transition. Bake first for cavity, edges and thin."""
    return call("set_fill", {"id": layer_id, "settings": {"generator": generator, "generator_level": level, "generator_softness": softness, "generator_invert": invert}})


@server.tool()
def formsurface_add_filter_layer(
    name: str = "Filter",
    brightness: Annotated[float, Field(ge=-0.5, le=0.5)] = 0.0,
    contrast: Annotated[float, Field(ge=-1, le=1)] = 0.0,
    hue_degrees: Annotated[float, Field(ge=-180, le=180)] = 0.0,
    saturation: Annotated[float, Field(ge=0, le=2)] = 1.0,
    gamma: Annotated[float, Field(ge=0.2, le=3)] = 1.0,
    roughness_offset: Annotated[float, Field(ge=-1, le=1)] = 0.0,
    metallic_offset: Annotated[float, Field(ge=-1, le=1)] = 0.0,
) -> dict:
    """Add a filter layer that adjusts every layer below it: brightness, contrast, hue, saturation and gamma of base color, and offsets for roughness and metallic. Limit it with a mask like any layer."""
    return call("add_filter_layer", {"name": name, "settings": {"brightness": brightness, "contrast": contrast, "hue": hue_degrees, "saturation": saturation, "gamma": gamma, "roughness_offset": roughness_offset, "metallic_offset": metallic_offset}})


@server.tool()
def formsurface_layer_mask(
    layer_id: int,
    action: Literal["add_white", "add_black", "remove", "keep"] = "keep",
    paint_target: Literal["layer", "mask"] | None = None,
) -> dict:
    """Give a layer a white mask (shows everything) or a black mask (hides everything), remove it, and choose whether strokes paint the layer or its mask. On a mask, the paint material's base color brightness is the mask value: white reveals, black hides. Fill layers are always painted through their mask."""
    return call("mask", drop_none(id=layer_id, action=action, paint_target=paint_target))


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
    """Move a layer up (positive) or down (negative) among the layers in the same folder; a folder moves with its contents."""
    return call("move_layer", {"id": layer_id, "offset": offset})


@server.tool()
def formsurface_remove_layer(layer_id: int) -> dict:
    """Delete a layer, or a folder with everything inside it. The texture set must keep at least one layer."""
    return call("remove_layer", {"id": layer_id})


@server.tool()
def formsurface_fill_layer() -> dict:
    """Fill the whole active layer with the current paint material (only the channels that are switched on)."""
    return call("fill")


@server.tool()
def formsurface_polygon_fill(
    x: Annotated[float, Field(ge=0, le=1)],
    y: Annotated[float, Field(ge=0, le=1)],
    mode: Literal["triangle", "polygon", "mesh", "uv_chunk"] = "mesh",
) -> dict:
    """Fill the part of the active texture set under a viewport point (0,0 top left, 1,1 bottom right) with the paint material at stroke opacity, like Painter's polygon fill: triangle, polygon (the flat face around it), mesh (the connected part) or uv_chunk (the UV island). Works through layer masks the same way strokes do and is undoable. Returns the filled triangle count."""
    return call("polygon_fill", {"x": x, "y": y, "mode": mode})


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


Source = Literal["basecolor", "normal_gl", "normal_dx", "roughness", "smoothness", "metallic", "height", "ao", "curvature", "thickness", "one", "zero"]


class MapSpec(BaseModel):
    suffix: Annotated[str, Field(pattern=r"^[A-Za-z0-9_-]{1,40}$")]
    rgb: Literal["basecolor", "normal_gl", "normal_dx"] | None = None
    r: Source | None = None
    g: Source | None = None
    b: Source | None = None
    a: Source | None = None


@server.tool()
def formsurface_list_export_templates() -> dict:
    """List export templates (pbr, gltf, unreal, unity_hdrp, unity_urp) with the maps each writes and how channels are packed."""
    return call("export_templates")


@server.tool()
def formsurface_export_maps(
    name: str | None = None,
    template: Literal["pbr", "gltf", "unreal", "unity_hdrp", "unity_urp"] | None = None,
    custom_maps: Annotated[list[MapSpec] | None, Field(max_length=16)] = None,
    bit_depth: Literal[8, 16] | None = None,
) -> dict:
    """Export maps for every texture set into the FormSurface workspace and return the folder. template pbr writes base color, roughness, metallic, height and normal (OpenGL) plus baked maps; gltf, unreal, unity_hdrp and unity_urp pack channels for those engines (unreal flips the normal to DirectX). custom_maps defines your own files: each has a suffix and either rgb (basecolor, normal_gl, normal_dx) or r, g, b, a sources; smoothness is 1 minus roughness and ao is white until baked. With several sets, files are named name_material_suffix, with .udim appended for tiled materials. Without template, the editor's Export template choice is used. bit_depth 16 writes 16-bit PNGs (smooth height and normal gradients); without it the editor's choice is used."""
    maps = [m.model_dump(exclude_none=True) for m in custom_maps] if custom_maps else None
    return call("export", drop_none(name=name, template=template, maps=maps, bit_depth=bit_depth), timeout=300)


@server.tool()
def formsurface_add_folder(name: str = "Folder", layer_ids: list[int] | None = None) -> dict:
    """Add a layer folder above the active layer and move the given layers into it. A folder's visibility, opacity and mask apply to everything inside (children blend straight onto the layers below, like Painter's passthrough). While a folder is the active layer, new layers are created inside it."""
    return call("add_folder", drop_none(name=name, layer_ids=layer_ids))


@server.tool()
def formsurface_set_layer_folder(layer_id: int, folder_id: int = 0) -> dict:
    """Move a layer or folder into a folder (it becomes the top item there), or to the top level with folder_id 0."""
    return call("set_parent", {"id": layer_id, "folder_id": folder_id})


if __name__ == "__main__":
    server.run("stdio")
