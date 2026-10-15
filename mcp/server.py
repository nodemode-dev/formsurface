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
def formsurface_load_sample(kind: Literal["sphere", "torus", "capsule", "knot", "sets", "uv2"]) -> dict:
    """Replace the mesh with a built-in sample. The first texture set keeps its layers. "sets" is a figure with two materials (body and trim) where the body spans UDIM tiles 1001 and 1002, giving three texture sets. "uv2" is a sphere with a second UV channel laid out differently from the first, for exporting maps onto UV1."""
    return call("load_sample", {"kind": kind})


@server.tool()
def formsurface_instance_layer(layer_id: int, texture_set: str) -> dict:
    """Put a live instance of a layer of the active texture set into another texture set (by name such as trim or body.1002, or index), as Painter's Instantiate across texture sets. Any layer kind can be instanced, folders with everything inside them. Changing settings, mask generator, name, opacity, blend or visibility of any copy then updates every copy, and a layer added to (or moved into) a source folder is replicated into each instance folder. As in Painter, paint actions (strokes, polygon fill, paths) stay on the source layer's texture set: an instanced paint layer carries its parameters but not its strokes, and painting an instance copy is refused. An instance copy cannot itself be instanced; instance its source."""
    return call("instance_layer", {"id": layer_id, "texture_set": texture_set})


@server.tool()
def formsurface_load_mesh(file: str, reproject: bool | None = None) -> dict:
    """Replace the mesh with a glb, gltf, obj, fbx, usd, usda, usdc, usdz, ply or dae from the workspace meshes folder (the mesh update step of a batch). Texture sets keep their layers when their material and UDIM tile match a set of the old mesh (the first set always continues from the first), in UV space by default. With reproject true (it stays on until set false), every carried set's paint, fill masks and painted masks follow the surface instead of the old UVs, so an edited or re-unwrapped version of the same model keeps its painting; texels with no old surface within 5 percent of the mesh radius come out empty, and undo history is cleared. Unmatched sets start fresh and bakes are reset either way."""
    return call("load_mesh", drop_none(file=file, reproject=reproject), timeout=180)


@server.tool()
def formsurface_list_texture_sets() -> dict:
    """List texture sets: one per material, split per UDIM tile when a material's UVs leave 0 to 1. Each has its own layer stack, base material, bakes and exported maps."""
    return call("texture_sets")


@server.tool()
def formsurface_select_texture_set(
    name: str | None = None,
    index: int | None = None,
    size: Literal[256, 512, 1024, 2048, 4096] | None = None,
    extra_channel: Literal["off", "user", "opacity", "ambient_occlusion", "emissive", "transmission", "scattering"] | None = None,
    extra_name: Annotated[str, Field(pattern=r"^[A-Za-z0-9_]{1,24}$")] | None = None,
    user_channels: Annotated[list[Annotated[str, Field(pattern=r"^[A-Za-z0-9_]{1,24}$")]], Field(max_length=2)] | None = None,
) -> dict:
    """Make a texture set active by name (for example body.1002 or trim) or index. Strokes, layers, fills, masks, smart materials and the base material then apply to it only; the viewport keeps showing every set. size sets that set's (or UV tile's) own resolution, as in Painter's texture set settings: its layers and quick mask are resampled, fills and generators re-rendered, undo cleared, bakes must be run again, and its maps export at that size. extra_channel gives the set a fifth channel (Painter's per-set channel list): user (named by extra_name, exported under that name), opacity (shown as cut-out in the viewport), ambient_occlusion (darkens the shading) emissive (glows with the base color, exported as an emissive map), transmission (glass of index 1.5 tinted by the base color, refracted in the path tracer, dithered see-through in the raster view) or scattering (subsurface: the path tracer random-walks light inside the mesh over the scatter radius, tinted by the base color; the raster view only softens it); off removes it. It is painted with formsurface_set_paint_material paint_extra and filled with formsurface_set_fill extra and use_extra. formsurface_list_texture_sets reports each set's size, extra channel and user channels. user_channels adds up to two more named grayscale channels to the set beside the extra channel (slots 2 and 3, in list order), each painted with formsurface_set_paint_material extra2/paint_extra2 or extra3/paint_extra3, filled with formsurface_set_fill extra2/use_extra2 or extra3/use_extra3, kept through flatten, merge, undo, resize and paging, and exported as its own map under its name; an empty list turns them off, which hides their paint until a slot is turned on again."""
    channels = None if user_channels is None else [{"slot": k + 2, "name": n} for k, n in enumerate(user_channels)]
    return call("select_texture_set", drop_none(name=name, index=index, size=size, extra_channel=extra_channel, extra_name=extra_name, user_channels=channels), timeout=120)


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
    across_sets: bool | None = None,
) -> dict:
    """Change brush settings. Size is in screen pixels (size_space screen) or thousandths of the mesh radius (object). alignment picks how the tip lies on the surface: camera projects it from the view, tangent_wrap follows curvature, tangent_planar lays it flat on the hit plane, uv paints in texture space. across_sets true lets a stroke that crosses onto another texture set keep painting there, into that set's layer of the same name and kind (sets without one are skipped; one undo takes the stroke back in every set). Only the values you pass change."""
    axis = {"x": 0, "y": 1, "z": 2}.get(symmetry_axis) if symmetry_axis else None
    return call("brush", drop_none(
        size_px=size_px, hardness=hardness, flow=flow, opacity=stroke_opacity, spacing=spacing, eraser=eraser, mirror_x=mirror_x, backface_culling=backface_culling,
        alignment=alignment, size_space=size_space, angle=angle_degrees, follow_path=follow_path, tip=tip, spray_density=spray_density, lazy_px=lazy_px,
        jitter_size=jitter_size, jitter_angle=jitter_angle, jitter_position=jitter_position, jitter_flow=jitter_flow,
        symmetry=symmetry, symmetry_axis=axis, radial_count=radial_count, across_sets=across_sets,
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
    extra: Unit | None = None,
    paint_extra: bool | None = None,
    extra2: Unit | None = None,
    paint_extra2: bool | None = None,
    extra3: Unit | None = None,
    paint_extra3: bool | None = None,
) -> dict:
    """Set what strokes and fills apply: channel values and which channels are painted. Height 0.5 is flat, above raises, below carves. extra and paint_extra drive the texture set's extra channel (set its role with formsurface_select_texture_set extra_channel): opacity 1 is solid, ambient occlusion 1 is unoccluded, emissive is the glow strength of the base color, a user channel is any value. extra2/paint_extra2 and extra3/paint_extra3 drive user channels 2 and 3 (add them with formsurface_select_texture_set user_channels); on a set without them they paint nothing."""
    return call("material", drop_none(base_color=base_color, roughness=roughness, metallic=metallic, height=height, use_color=paint_color, use_roughness=paint_roughness, use_metallic=paint_metallic, use_height=paint_height, extra=extra, use_extra=paint_extra,
                                      extra2=extra2, use_extra2=paint_extra2, extra3=extra3, use_extra3=paint_extra3))


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


Projection = Literal["uv", "triplanar", "planar", "spherical", "cylindrical", "warp", "tile_match"]
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
    warp_points: Annotated[list[tuple[float, float]], Field(min_length=16, max_length=16)] | None = None,
    extra: Unit | None = None,
    use_extra: bool | None = None,
    extra2: Unit | None = None,
    use_extra2: bool | None = None,
    extra3: Unit | None = None,
    use_extra3: bool | None = None,
    reset_warp: bool = False,
    tile_images: Annotated[list[str], Field(min_length=1, max_length=99)] | None = None,
    uv_set: Literal[0, 1] | None = None,
) -> dict:
    """Change a fill layer's projection, pattern or values. Only the values you pass change. projection warp is a planar projection along planar_axis whose 4 by 4 grid bends the pattern: warp_points gives the 16 grid points row by row (top row first, left to right) as [x, y] in the projection square, where 0 to 1 spans the mesh's bounds across the axis (x right, y down) and the rest grid is i/3, j/3; a pattern point stays attached to its grid cell, so moving a point drags that part of the pattern with it. reset_warp restores the rest grid. tile_images (Painter's Fill (match per UV tile)) takes image files from the workspace images folder whose names carry a UDIM number (wood.1001.png, wood_1002.png) and puts each on its own UV tile of the material, filling the tile once with no scale or offset; it needs UV tiles on so the material's tiles share the layer, and sets projection tile_match with the image pattern. uv_set 1 lays a uv projection out on the mesh's second UV channel (lightmap or trim UVs) instead of the first; meshes without one keep UV0."""
    tiles = None
    if tile_images is not None:
        tiles = call("fill_tile_images", {"files": tile_images, "layer_id": layer_id}, timeout=180)
    warp = [0.0] * 32
    if reset_warp:
        warp = [c for k in range(16) for c in ((k % 4) / 3, (k // 4) / 3)]
    elif warp_points is not None:
        if any(not (-2 <= c <= 3) for p in warp_points for c in p):
            raise ValueError("warp_points coordinates must be between -2 and 3")
        warp = [float(c) for p in warp_points for c in p]
    settings = fill_settings(projection=projection, pattern=pattern, base_color=base_color, roughness=roughness, metallic=metallic, height=height, scale=scale,
                             rotation=rotation_degrees, offset_x=offset_u, offset_y=offset_v, planar_axis=planar_axis, sharpness=sharpness, opacity=amount, extra=extra, use_extra=use_extra,
                             extra2=extra2, use_extra2=use_extra2, extra3=extra3, use_extra3=use_extra3, uv_set=uv_set,
                             warp=warp if reset_warp or warp_points is not None else None)
    result = call("set_fill", {"id": layer_id, "settings": settings})
    if tiles is not None:
        result = {**result, "tiles": tiles.get("tiles"), "skipped": tiles.get("skipped")}
    return result


@server.tool()
def formsurface_bake_maps(
    directions: Annotated[int, Field(ge=8, le=256)] = 64,
    method: Literal["depth", "traced"] | None = None,
    rays: Annotated[int, Field(ge=4, le=128)] | None = None,
) -> dict:
    """Bake ambient occlusion, curvature, thickness, position and world normal from the current mesh on the GPU. Needed before fill generators cavity, edges and thin; more directions give smoother maps and take longer. method depth rasterizes depth maps from the given directions; method traced then ray traces ambient occlusion and thickness per texel through the mesh's BVH on the GPU with rays per texel (more is smoother and slower), which sees concave detail the depth maps miss. Exported with formsurface_export_maps once baked."""
    return call("bake", drop_none(directions=directions, method=method, rays=rays), timeout=600)


@server.tool()
def formsurface_bake_high_poly(
    mesh_file: str | None = None,
    use_sample: bool = False,
    cage_front: Annotated[float, Field(ge=0, le=0.5)] = 0.05,
    cage_rear: Annotated[float, Field(ge=0, le=0.5)] = 0.05,
    match_by_name: bool | None = None,
    sample: Literal["sphere", "pair"] | None = None,
    ao_rays: Annotated[int, Field(ge=0, le=64)] | None = None,
    cage_file: str | None = None,
    distance_cage: bool = False,
) -> dict:
    """Bake from a high-poly mesh onto the painted mesh: a tangent-space normal map, an ID map (one color per high-poly part), height (high surface above or below the low one, mid gray is level, cage range maps to 0 to 1), opacity (white where the high mesh was found) and, with ao_rays above 0 (default 16, more is smoother and slower), ambient occlusion and a bent normal map from the high-poly detail. The extra maps export as ambient_occlusion_high, height_high, opacity and bent_normal, and ao_high, height_high and opacity are template sources. mesh_file is a glb, gltf, obj, fbx, usd, usda, usdc, usdz, ply or dae the person put in the workspace meshes folder; use_sample (or sample sphere) loads a low sphere with a bumpy two-part high-poly sphere, and sample pair loads two touching low spheres named left_low and right_low with high-poly left_high and right_high. Cage distances are fractions of the mesh radius in front of and behind the low surface; raise them if parts of the high-poly are missed. match_by_name pairs low and high parts whose mesh names match once _low and _high (or _lo and _hi) are removed, so close parts never bake onto each other; unmatched parts bake only against unmatched high-poly parts. cage_file is a custom cage in the workspace meshes folder: the low-poly mesh with its vertices moved (same triangles in the same order); rays then run from the cage to the low surface and cage_rear still reaches behind it. It stays set for later bakes until distance_cage is true or another mesh is loaded."""
    chosen = sample or ("sphere" if use_sample else None)
    return call("bake_high", drop_none(file=mesh_file, sample=chosen, front=cage_front, rear=cage_rear, match_by_name=match_by_name, ao_rays=ao_rays, cage_file=cage_file, distance_cage=distance_cage or None), timeout=600)


@server.tool()
def formsurface_load_image(
    file: str,
    target: Literal["fill", "alpha", "stencil", "mask"] = "fill",
    layer_id: int | None = None,
    color_space: Literal["srgb", "linear", "raw"] | None = None,
) -> dict:
    """Import a png, jpg or webp from the workspace images folder as a fill layer image, brush alpha, stencil or layer mask (mask over the UV layout). layer_id picks the fill layer or masked layer (otherwise the active one). color_space says how the file's values are read: srgb (gamma encoded color), linear (linear values, for example a linear albedo or a data map saved as linear) or raw (numbers used exactly, the default for alphas, stencils and masks); fill images default to srgb. Change it later with formsurface_set_color_space."""
    return call("load_image", drop_none(file=file, target=target, layer_id=layer_id, color_space=color_space), timeout=120)


@server.tool()
def formsurface_set_color_space(resource: str | int | None = None, color_space: Annotated[str, Field(pattern=r"^(srgb|linear|raw|ocio:.{1,120})$")] | None = None) -> dict:
    """Per-resource color space override, like Painter's: list imported images and HDR environments with their index and color space (no arguments), or set one by name or index (the index is needed when one file was imported for two targets). Images are re-read and re-applied at once: a fill image marked linear is encoded to sRGB before use; an alpha, stencil or mask marked srgb is decoded to linear values; raw uses the stored numbers. An environment marked srgb has its values decoded from gamma before lighting (Radiance files are linear by default). With an OCIO config active (formsurface_color_management), a fill image or environment can be marked ocio: followed by any color space of that config (for example ocio:ACEScg or ocio:ACES2065-1); the service converts it with OpenColorIO into the viewport's linear Rec.709."""
    if resource is None or color_space is None:
        return call("resources")
    return call("resource_color_space", {"resource": resource, "color_space": color_space})


@server.tool()
def formsurface_set_post_effects(
    reset: bool = False,
    bloom: Annotated[float | None, Field(ge=0, le=2)] = None,
    bloom_threshold: Annotated[float | None, Field(ge=0, le=0.99)] = None,
    glare: Annotated[float | None, Field(ge=0, le=2)] = None,
    lens_flare: Annotated[float | None, Field(ge=0, le=2)] = None,
    dof_aperture: Annotated[float | None, Field(ge=0, le=24)] = None,
    dof_focus: Annotated[float | None, Field(ge=-1, le=1)] = None,
    aberration: Unit | None = None,
    sharpen: Annotated[float | None, Field(ge=0, le=2)] = None,
    vignette: Unit | None = None,
    vignette_softness: Unit | None = None,
    grain: Unit | None = None,
    gain: Annotated[float | None, Field(ge=0, le=2)] = None,
    contrast: Annotated[float | None, Field(ge=-1, le=1)] = None,
    saturation: Annotated[float | None, Field(ge=0, le=2)] = None,
    temperature: Annotated[float | None, Field(ge=-1, le=1)] = None,
    tint: Annotated[float | None, Field(ge=-1, le=1)] = None,
) -> dict:
    """Viewport post effects, applied after tone mapping to the viewport and screenshots only (exported maps never change): bloom above a brightness threshold, glare streaks and lens flare ghosts from the same bright areas, depth of field (dof_aperture is the largest blur in pixels, 0 off; dof_focus -1 is the model's front, 0 its centre, 1 its back), chromatic aberration, sharpen, vignette with softness, film grain, and color correction (gain is brightness, contrast, saturation, temperature warm or cool, tint green or magenta). reset puts every effect back to off first; only the values given change."""
    values = drop_none(bloom=bloom, bloom_threshold=bloom_threshold, glare=glare, lens_flare=lens_flare, dof_aperture=dof_aperture, dof_focus=dof_focus, aberration=aberration, sharpen=sharpen, vignette=vignette, vignette_softness=vignette_softness, grain=grain, gain=gain, contrast=contrast, saturation=saturation, temperature=temperature, tint=tint)
    return call("set_post_effects", {"reset": reset, **values})


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
def formsurface_apply_smart_mask(layer_id: int, name: Annotated[str, Field(min_length=1, max_length=80)]) -> dict:
    """Apply a smart mask (a generator setup) by name: the built-in Edge wear, Cavity dirt, Dust from top, Thin parts, Protect edges, Grunge dirt, Metal edge wear and Sun bleached, or one saved with formsurface_save_smart_mask (formsurface_list_smart_materials lists them). A fill layer takes it as its generator; any other layer gets it as a generator mask that follows every bake."""
    return call("smart_mask", {"id": layer_id, "name": name})


@server.tool()
def formsurface_save_smart_mask(layer_id: int, name: Annotated[str, Field(pattern=r"^[A-Za-z0-9 _.-]{1,80}$")]) -> dict:
    """Save the generator setup of a layer (a fill layer's generator, or a generator mask from formsurface_mask_source) as a named smart mask in the workspace library, so it can be applied to other layers and models. Built-in names cannot be reused."""
    return call("save_smart_mask", {"id": layer_id, "name": name})


GeneratorName = Literal["none", "cavity", "edges", "thin", "top", "light", "position", "noise", "mask_builder", "dirt", "metal_edge_wear"]


@server.tool()
def formsurface_set_generator(
    layer_id: int,
    generator: GeneratorName,
    level: Unit = 0.5,
    softness: Annotated[float, Field(ge=0.001, le=0.5)] = 0.1,
    invert: bool = False,
    noise: Unit | None = None,
    noise_scale: Annotated[float, Field(ge=0.1, le=64)] | None = None,
    direction: Annotated[list[float], Field(min_length=3, max_length=3)] | None = None,
    builder_ao: Unit | None = None,
    builder_curvature: Unit | None = None,
    builder_noise: Unit | None = None,
) -> dict:
    """Drive where a fill layer shows. From baked maps: cavity (occluded crevices, dirt), edges (convex curvature, wear), thin (thickness); dirt and metal_edge_wear are cavity and edges broken up with grunge noise. From the mesh alone: top (fades from the top down), position (gradient along direction), light (faces turned toward direction, x y z in mesh space), noise (3D noise at noise_scale). mask_builder mixes cavity, curvature and noise by the builder weights. noise breaks up any generator's edge. level moves the threshold, softness widens the transition. Bake first for the baked-map generators."""
    settings = {"generator": generator, "generator_level": level, "generator_softness": softness, "generator_invert": invert}
    settings.update(drop_none(generator_noise=noise, noise_scale=noise_scale, generator_direction=direction, builder_ao=builder_ao, builder_curvature=builder_curvature, builder_noise=builder_noise))
    return call("set_fill", {"id": layer_id, "settings": settings})


@server.tool()
def formsurface_set_environment(
    file: Annotated[str, Field(pattern=r"^[A-Za-z0-9 _.-]{1,80}\.hdr$")] | None = None,
    studio: bool = False,
    rotation_degrees: Annotated[float, Field(ge=-180, le=180)] | None = None,
    blur: Unit | None = None,
    background: bool | None = None,
    exposure: Annotated[float, Field(ge=0.2, le=3)] | None = None,
    view_transform: Literal["aces", "standard", "agx", "filmic"] | None = None,
) -> dict:
    """Light the viewport with an HDR environment: file is a Radiance .hdr (equirectangular) in the workspace environments folder; studio true returns to the built-in sky and two lights. rotation_degrees turns the environment around the vertical axis, blur softens the background (lighting is unchanged), background shows or hides it behind the model, exposure scales everything. view_transform is the display transform from scene light to screen: aces (filmic fit, the default), standard (sRGB, highlights clip), agx or filmic (Hable). Only the viewport changes, never the exported maps."""
    return call("environment", drop_none(file=file, studio=studio or None, rotation_degrees=rotation_degrees, blur=blur, background=background, exposure=exposure, view_transform=view_transform), timeout=120)


@server.tool()
def formsurface_path_trace(
    on: bool | None = None,
    bounces: Annotated[int, Field(ge=1, le=12)] | None = None,
    max_samples: Literal[64, 256, 1024, 4096] | None = None,
    samples: Annotated[int, Field(ge=1, le=4096)] | None = None,
    denoise: bool | None = None,
    aperture: Unit | None = None,
    focus: Annotated[float, Field(ge=-1, le=1)] | None = None,
    scatter_radius: Annotated[float, Field(ge=0.005, le=0.5)] | None = None,
) -> dict:
    """Progressive path tracing of the viewport: the painted texture sets (base color, roughness, metallic, height and normal, the extra channel as opacity cut-out or emissive) lit by the HDR environment, or by the studio sky with its two lights and traced shadows, with light bouncing up to bounces times. It keeps refining every frame until max_samples and restarts when the camera, lighting or paint changes; strokes show the raster view while painting. samples renders that many before returning, so a screenshot right after shows a converged image. HDR lighting is importance-sampled, so small bright sources such as a sun converge quickly. denoise smooths the traced image with an edge-aware filter guided by albedo, normals and depth, useful below a few hundred samples. aperture (0 to 1) opens a thin lens for depth of field and focus (-1 to 1, in mesh radii) moves the focal plane in front of or behind the orbit target. scatter_radius is the subsurface mean free path as a share of the mesh radius for sets whose extra channel is scattering. Viewport and screenshots only, never the exported maps."""
    return call("path_trace", drop_none(on=on, bounces=bounces, max_samples=max_samples, samples=samples, denoise=denoise, aperture=aperture, focus=focus, scatter_radius=scatter_radius), timeout=600)


@server.tool()
def formsurface_texture_memory(page_out_idle: bool = False, auto: bool | None = None) -> dict:
    """GPU memory of each texture set's layer stack and undo history: resident_bytes on the GPU and paged_bytes held in the editor's memory instead. page_out_idle moves every texture set except the active one out of GPU memory now; auto true does that after every texture set switch. A paged set keeps its finished look in the viewport and path tracer and is paged back in when selected, exported or baked."""
    return call("texture_memory", drop_none(page_out_idle=page_out_idle or None, auto=auto), timeout=300)


@server.tool()
def formsurface_color_management(
    config: Annotated[str, Field(pattern=r"^[A-Za-z0-9 _.-]{0,80}$")] | None = None,
    display: str | None = None,
    view: str | None = None,
    look: str | None = None,
    working_space: Literal["linear_rec709", "acescg"] | None = None,
) -> dict:
    """OpenColorIO 2 color management for the viewport, screenshots and path-traced renders. With no arguments it lists the built-in configs (ACES CG and studio configs) and the .ocio files in the workspace ocio folder. config picks one (an empty string returns to the built-in view transforms); display, view and look pick from that config (defaults are the config's default display and view, no look). The service runs real OpenColorIO from the working space and the editor applies the result as a 65-point 3D LUT over a log2 shaper. working_space sets the space painting, compositing and shading happen in: linear_rec709 (default) or acescg, as in Painter's ACES mode; switching converts every painted layer, clears undo, and makes paint and fill colors, fill images and environments convert into it, so wide-gamut ACEScg colors survive. Base color maps still export as sRGB-encoded Rec.709."""
    return call("color_management", drop_none(config=config, display=display, view=view, look=look, working_space=working_space), timeout=120)


@server.tool()
def formsurface_path_render(
    width: Annotated[int, Field(ge=16, le=3840)] = 1920,
    height: Annotated[int, Field(ge=16, le=3840)] = 1080,
    samples: Annotated[int, Field(ge=1, le=4096)] = 256,
    name: Annotated[str, Field(pattern=r"^[A-Za-z0-9_-]{1,64}$")] | None = None,
) -> dict:
    """Path trace the current camera view at width by height with that many samples and save it as a PNG in the workspace exports folder (named render_WxH_Nspp.png), using the path tracer's bounces, denoise and lens settings. Turn path tracing on first with formsurface_path_trace. The viewport shows the raster view while it renders."""
    return call("path_render", drop_none(width=width, height=height, samples=samples, name=name), timeout=3600)


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
    blur: Annotated[float, Field(ge=0, le=32)] = 0.0,
    sharpen: Annotated[float, Field(ge=0, le=4)] = 0.0,
) -> dict:
    """Add a filter layer that adjusts every layer below it: blur (radius in texels at 1024 px, scaled with texture size) and sharpen of every channel including height, then brightness, contrast, hue, saturation and gamma of base color, and offsets for roughness and metallic. Blur stays inside UV islands. Limit it with a mask like any layer."""
    return call("add_filter_layer", {"name": name, "settings": {"brightness": brightness, "contrast": contrast, "hue": hue_degrees, "saturation": saturation, "gamma": gamma, "roughness_offset": roughness_offset, "metallic_offset": metallic_offset, "blur": blur, "sharpen": sharpen}})


@server.tool()
def formsurface_layer_mask(
    layer_id: int,
    action: Literal["add_white", "add_black", "remove", "keep"] = "keep",
    paint_target: Literal["layer", "mask"] | None = None,
    effects: Annotated[list[dict], Field(max_length=16)] | None = None,
) -> dict:
    """Give a layer a white mask (shows everything) or a black mask (hides everything), remove it, and choose whether strokes paint the layer or its mask. On a mask, the paint material's base color brightness is the mask value: white reveals, black hides. Fill layers are always painted through their mask.

    effects replaces the mask's effect stack (Painter's mask effects), applied in order over the mask and its painted strokes; [] clears it. Each effect is an object with kind fill (value 0 to 1), generator (fill: {generator: cavity, edges, thin, top, light, position, noise, dirt, metal_edge_wear or mask_builder, generator_level, generator_softness, generator_invert}), levels (in_low, in_high, out_low, out_high), invert, blur (radius 0 to 32 px), compare (layer: another layer id whose mask is combined) or group (count: how many of the following effects belong to it, nested groups and their contents included; they run on their own black mask, so a levels or invert inside touches only the group's contents, and the group's result blends onto the effects above it); fill, generator, compare and group take blend (normal, multiply, add, subtract, lighten, darken, screen); every effect takes opacity and visible. The result reports the stack."""
    return call("mask", drop_none(id=layer_id, action=action, paint_target=paint_target, effects=effects))


@server.tool()
def formsurface_update_layer(
    layer_id: int,
    name: str | None = None,
    visible: bool | None = None,
    opacity: Unit | None = None,
    blend: Literal["normal", "multiply", "screen", "overlay", "add", "subtract", "darken", "lighten", "disable", "replace", "divide", "inverse_divide", "difference", "exclusion", "signed_addition", "linear_burn", "color_burn", "color_dodge", "soft_light", "hard_light", "vivid_light", "linear_light", "pin_light", "tint", "saturation", "color", "value", "normal_map_combine", "normal_map_detail", "normal_map_inverse_detail", "inverse_subtract"] | None = None,
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
def formsurface_set_filter(
    layer_id: int,
    brightness: Annotated[float | None, Field(ge=-0.5, le=0.5)] = None,
    contrast: Annotated[float | None, Field(ge=-1, le=1)] = None,
    hue_degrees: Annotated[float | None, Field(ge=-180, le=180)] = None,
    saturation: Annotated[float | None, Field(ge=0, le=2)] = None,
    gamma: Annotated[float | None, Field(ge=0.2, le=3)] = None,
    roughness_offset: Annotated[float | None, Field(ge=-1, le=1)] = None,
    metallic_offset: Annotated[float | None, Field(ge=-1, le=1)] = None,
    blur: Annotated[float | None, Field(ge=0, le=32)] = None,
    sharpen: Annotated[float | None, Field(ge=0, le=4)] = None,
    levels_in_low: Unit | None = None,
    levels_in_high: Unit | None = None,
    levels_out_low: Unit | None = None,
    levels_out_high: Unit | None = None,
    invert: bool | None = None,
    posterize: Annotated[int | None, Field(ge=0, le=32)] = None,
    pixelate: Annotated[int | None, Field(ge=0, le=64)] = None,
) -> dict:
    """Change an existing filter layer's settings; only the values given change. Besides blur, sharpen, brightness, contrast, hue, saturation, gamma and the roughness or metallic offsets: levels (input black and white points remapped to output black and white) on base color, invert of base color, posterize (number of color steps, 0 off) and pixelate (block size in texels at 1024 px, 0 off, applied to every channel)."""
    return call("set_filter", {"id": layer_id, "settings": drop_none(brightness=brightness, contrast=contrast, hue=hue_degrees, saturation=saturation, gamma=gamma, roughness_offset=roughness_offset, metallic_offset=metallic_offset, blur=blur, sharpen=sharpen, levels_in_low=levels_in_low, levels_in_high=levels_in_high, levels_out_low=levels_out_low, levels_out_high=levels_out_high, invert=invert, posterize=posterize, pixelate=pixelate)})


@server.tool()
def formsurface_export_mesh(
    name: str | None = None,
    ngons: bool | None = None,
    subdivide: Annotated[int, Field(ge=0, le=3)] | None = None,
    displacement: Annotated[float, Field(ge=0, le=0.2)] | None = None,
) -> dict:
    """Write the loaded mesh as an OBJ into the workspace exports folder: positions, normals, the original UDIM UVs and one group per texture set (usemtl with the material name). ngons true writes the original quads and n-gons when the mesh came from an OBJ or ASCII PLY (other formats arrive triangulated), false triangulates. subdivide tessellates every triangle 1 to 3 levels (4, 16 or 64 triangles each) and displacement pushes the tessellated surface along its normal by the height channel, as a fraction of the mesh radius (mid gray stays put); tessellated output is triangulated."""
    return call("export_mesh", drop_none(name=name, ngons=ngons, subdivide=subdivide, displacement=displacement), timeout=600)


@server.tool()
def formsurface_flatten_stack(name: str = "Flattened") -> dict:
    """Replace every layer of the active texture set (all its tiles with UV tiles on) with one paint layer holding exactly what the viewport shows. Fill, filter, folder and generator settings are lost and this cannot be undone; ask the person first."""
    return call("flatten", {"name": name})


@server.tool()
def formsurface_export_layer_mask(layer_id: int, name: str | None = None) -> dict:
    """Save a layer's mask as a grayscale PNG in the workspace exports folder."""
    return call("export_mask", drop_none(id=layer_id, name=name), timeout=120)


@server.tool()
def formsurface_clone_source(x: Annotated[float, Field(ge=0, le=1)], y: Annotated[float, Field(ge=0, le=1)]) -> dict:
    """Set the clone source at a viewport point (0,0 top left) on the active texture set and switch the brush to clone. The next formsurface_paint_stroke fixes the offset between source and stroke; strokes then copy what is visible at that offset (every switched-on channel) into the active paint layer. The offset is in UV space, so keep source and target on the same UV island. formsurface_set_brush with eraser false returns to painting."""
    return call("clone_source", {"x": x, "y": y})


@server.tool()
def formsurface_merge_down(layer_id: int) -> dict:
    """Merge a paint or fill layer into the paint or fill layer directly below it in the same folder: one paint layer replaces both, with their opacity, masks and the top layer's blend mode baked in. Fill settings become pixels. Not undoable; ask the person first."""
    return call("merge_down", {"id": layer_id})


@server.tool()
def formsurface_flatten_folder(folder_id: int) -> dict:
    """Flatten a folder of paint and fill layers into one paint layer that keeps the folder's opacity, visibility and mask. Nested folders and filter layers must be flattened or moved out first. Not undoable; ask the person first."""
    return call("flatten_folder", {"id": folder_id})


@server.tool()
def formsurface_export_layer(layer_id: int, name: str | None = None) -> dict:
    """Write one paint or fill layer to the workspace exports folder as four PNGs (base color, roughness, metallic, height), each with the layer's coverage in alpha, without the layers below or the base material."""
    return call("export_layer", drop_none(id=layer_id, name=name), timeout=120)


@server.tool()
def formsurface_clone_layer(layer_id: int = 0) -> dict:
    """Choose what the clone tool copies: layer_id of a paint or fill layer copies only that layer (where it has no paint nothing is copied); 0 copies everything visible."""
    return call("clone_layer", {"id": layer_id})


@server.tool()
def formsurface_view_2d(
    channel: Literal["base_color", "roughness", "metallic", "height", "extra", "user2", "user3"] | None = None,
    on: bool | None = None,
    center_u: Annotated[float, Field(ge=-1, le=2)] | None = None,
    center_v: Annotated[float, Field(ge=-1, le=2)] | None = None,
    zoom: Annotated[float, Field(ge=0.25, le=32)] | None = None,
) -> dict:
    """Switch the viewport to the 2D view (the active texture set's UV layout) or back to 3D; channel picks what it shows (base_color, roughness, metallic, height, the extra channel or user channels 2 and 3). In the 2D view, formsurface_paint_stroke, formsurface_paint_path and formsurface_polygon_fill points land on the layout (u right, v down, the 0 to 1 square framed at zoom 1) and paint straight into the texture, reaching faces the 3D camera cannot see; brush size is in screen pixels of that view. center_u, center_v and zoom frame part of the layout. ID picking needs the 3D view; symmetry and the stencil do not apply in 2D, and strokes stay on the active UDIM tile."""
    return call("view_2d", drop_none(on=on, center_u=center_u, center_v=center_v, zoom=zoom, channel=channel))


@server.tool()
def formsurface_paint_path(
    points: Annotated[list[tuple[float, float]], Field(min_length=2, max_length=64)],
    closed: bool = False,
    smooth: bool = True,
    erase: bool = False,
    mode: Literal["paint", "ribbon", "filled"] = "paint",
) -> dict:
    """Painter's path tool: place control points on the surface at normalized viewport points (0,0 top left) and paint along a smooth curve through them (smooth false joins them with straight segments; closed loops back to the first point), with the current brush and paint material. mode paint stamps the brush along the curve; ribbon lays one continuous strip as wide as the brush size, its cross-section the brush hardness or, with an image tip, the brush alpha repeated along the curve; filled fills the inside of the curve (always closed, at least three points). Ribbon and filled are applied from the current view to the visible surface, so the camera should face the area. The points are kept on the surface, so the curve follows the model rather than the screen; in the 2D view they are UV positions and the curve is drawn on the layout. erase removes paint instead. One undo step."""
    return call("paint_path", {"points": [list(p) for p in points], "closed": closed, "smooth": smooth, "erase": erase, "mode": mode}, timeout=120)


@server.tool()
def formsurface_text_resource(
    text: Annotated[str, Field(min_length=1, max_length=120)],
    target: Literal["alpha", "stencil", "fill"] = "alpha",
    font: str = "sans-serif",
    bold: bool = True,
) -> dict:
    """Render text with a font and use it as a resource: alpha makes it the brush tip (each stamp prints the text; set_brush angle or follow_path turns it), stencil lays it over the screen so strokes paint the letters, fill puts it in the active fill layer's image pattern in that layer's color (project it with formsurface_set_fill, for example planar). font is sans-serif, serif, monospace, or a font file the person loaded in the Text section."""
    return call("text_resource", {"text": text, "target": target, "font": font, "bold": bold})


@server.tool()
def formsurface_uv_tiles(on: bool | None = None) -> dict:
    """Painter's UV tiles workflow. When on, the UDIM tiles of a material share one layer stack: adding, changing or removing layers, fills, filters and masks on one tile applies to all of them, and a stroke paints across tile borders in one go. Turning it on copies the active tile's stack to the material's other tiles and clears their previous paint. Without on, reports the current state."""
    return call("uv_tiles", drop_none(on=on))


@server.tool()
def formsurface_mask_source(
    layer_id: int,
    source: Literal["cavity", "edges", "thin", "top", "light", "position", "noise", "mask_builder", "dirt", "metal_edge_wear", "id_color"],
    color: Annotated[str | None, Field(pattern=r"^#[0-9a-fA-F]{6}$")] = None,
    x: Annotated[float | None, Field(ge=0, le=1)] = None,
    y: Annotated[float | None, Field(ge=0, le=1)] = None,
    tolerance: Annotated[float | None, Field(ge=0, le=1)] = None,
    level: Unit | None = None,
    softness: Unit | None = None,
    invert: bool | None = None,
) -> dict:
    """Give a layer a mask from a source instead of painting it. cavity, edges, thin and top generate it from the baked mesh maps (level, softness, invert) and regenerate after every bake. id_color selects one color of the baked ID map: pass color, or a viewport point x, y on the part (0,0 top left) to pick it there. Strokes into the mask still work afterwards."""
    return call("mask_source", drop_none(id=layer_id, source=source, color=color, x=x, y=y, tolerance=tolerance, level=level, softness=softness, invert=invert))


@server.tool()
def formsurface_quick_mask(mode: Literal["off", "edit", "on"] | None = None, action: Literal["clear", "allow_all", "invert"] | None = None) -> dict:
    """Painter's quick mask for the active texture set, a temporary mask that is not a layer. edit: strokes and fills mark where paint may land (the eraser protects), shown in red where protected; on: every stroke, fill and polygon fill only lands where it was marked; off: no limit. clear protects everything, allow_all allows everything, invert swaps them. Not undoable."""
    return call("quick_mask", drop_none(mode=mode, action=action))


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
def formsurface_list_commands() -> dict:
    """Every editor menu command with its id, label, shortcut keys, whether it can run now and, for toggles, whether it is on. The same registry drives the menu bar, the palette and the keyboard."""
    return call("list_commands")


@server.tool()
def formsurface_run_command(id: Annotated[str, Field(min_length=3, max_length=80)]) -> dict:
    """Run one editor menu command by id from formsurface_list_commands, exactly as the person would from the menu, for example layer.addFillBlack, layer.mergeDown, view.2d or tool.mirror. Commands that open a window for the person (file dialogs, preferences, the terminal) are refused."""
    return call("run_command", {"id": id}, timeout=120)


@server.tool()
def formsurface_pick_layer(
    x: Annotated[float, Field(ge=0, le=1)],
    y: Annotated[float, Field(ge=0, le=1)],
) -> dict:
    """Select the top visible paint or fill layer that paints the surface at a normalized viewport point (0,0 top left), as Alt+Shift+click does in the editor. Returns the selected layer or null when no layer covers that point."""
    return call("pick_layer", {"x": x, "y": y})


@server.tool()
def formsurface_hide_part(
    x: Annotated[float, Field(ge=0, le=1)],
    y: Annotated[float, Field(ge=0, le=1)],
    mode: Literal["triangle", "polygon", "mesh", "uv_island"] = "mesh",
    isolate: bool = False,
) -> dict:
    """Hide the part of the model under a normalized viewport point (or, with isolate, hide everything else) so strokes and fills reach the surfaces behind it, such as the inside of a mouth or a cupboard. Hidden parts stay in exports. Returns how many triangles are hidden."""
    return call("hide_part", {"x": x, "y": y, "mode": mode, "isolate": isolate})


@server.tool()
def formsurface_select_region(
    points: Annotated[list[tuple[float, float]], Field(min_length=3, max_length=512)],
    limit: bool = False,
) -> dict:
    """Select the visible surface inside a screen polygon (normalized viewport corners, 0,0 top left; four corners make a rectangle). With limit false the area is filled with the paint material on the active layer as one undo step; with limit true it becomes the quick mask and quick mask turns on, so later strokes and fills stay inside it until quick mask is turned off."""
    return call("select_region", {"points": [list(p) for p in points], "limit": limit}, timeout=120)


@server.tool()
def formsurface_show_all_parts() -> dict:
    """Show every part hidden with formsurface_hide_part."""
    return call("show_all_parts")


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


Source = Literal["basecolor", "normal_gl", "normal_dx", "roughness", "smoothness", "metallic", "height", "ao", "curvature", "thickness", "ao_high", "height_high", "opacity", "extra", "user2", "user3", "one", "zero"]


class MapSpec(BaseModel):
    suffix: Annotated[str, Field(pattern=r"^[A-Za-z0-9_-]{1,40}$")]
    rgb: Literal["basecolor", "normal_gl", "normal_dx", "diffuse", "specular"] | None = None
    r: Source | None = None
    g: Source | None = None
    b: Source | None = None
    a: Source | None = None


@server.tool()
def formsurface_list_export_templates() -> dict:
    """List export templates (pbr, gltf, unreal, unity_hdrp, unity_urp, plus saved ones marked saved) with the maps each writes and how channels are packed."""
    return call("export_templates")


@server.tool()
def formsurface_export_maps(
    name: str | None = None,
    template: Annotated[str, Field(pattern=r"^[A-Za-z0-9 _.-]{1,80}$")] | None = None,
    custom_maps: Annotated[list[MapSpec] | None, Field(max_length=16)] = None,
    format: Literal["png", "tif", "exr", "hdr", "jpg", "psd"] | None = None,
    bit_depth: Literal[8, 16, 32] | None = None,
    padding: Literal["infinite", "transparent", "color", "diffusion"] | None = None,
    padding_width: Annotated[int, Field(ge=0, le=256)] | None = None,
    padding_color: Annotated[str, Field(pattern=r"^#[0-9a-fA-F]{6}$")] | None = None,
    size: Literal[0, 128, 256, 512, 1024, 2048, 4096, 8192] | None = None,
    uv_channel: Literal[0, 1] | None = None,
) -> dict:
    """Export maps for every texture set into the FormSurface workspace and return the folder. template is pbr (base color, roughness, metallic, height and normal plus baked maps), gltf, unreal, unity_hdrp, unity_urp, unity_standard_specular, godot, blender, arnold, redshift, usd, sketchfab, spec_gloss, or a template saved with formsurface_save_export_template (see formsurface_list_export_templates). custom_maps defines files for this export only: each has a suffix and either rgb (basecolor, normal_gl, normal_dx, or diffuse and specular converted from base color and metallic for specular glossiness workflows) or r, g, b, a sources; smoothness is 1 minus roughness (glossiness) and ao is white until baked; extra, user2 and user3 pack the set's extra channel and user channels 2 and 3 (black where a set lacks them). With several sets, files are named name_material_suffix, with .udim for tiled materials. format and bit_depth: png 8 or 16, tif 8, 16 or 32 (float), exr 16 (half) or 32 (float), hdr 32, jpg 8, psd 8, 16 or 32; float files store base color linear, everything else as in PNG. padding fills texels outside the UV islands: infinite dilation (default), or padding_width texels of dilation followed by transparent (alpha 0; not jpg or hdr), color (padding_color) or diffusion (a smooth blur fill). size exports every texture set at that size instead of its own (0, the default, keeps each set's size), up to 8192 where the GPU allows, as Painter's export size does: the stack is recomputed at that size, with fills, generators, patterns and mask effects rendered natively and paint layers, image masks and baked maps resampled; the working textures are untouched, and the stack is composited one layer at a time, so the export's GPU memory depends on the size rather than the layer count (about 3 GB at 4096 and 12 GB at 8192 in half float). uv_channel 1 writes the maps in the mesh's second UV layout (glTF TEXCOORD_1, for lightmaps or trim sheets) instead of the painted UV0 layout, padding around the UV1 islands; it fails when the mesh has no second UV channel, and tangent-space normal maps keep UV0's tangent frame. Omitted values use the editor's Export choices."""
    maps = [m.model_dump(exclude_none=True) for m in custom_maps] if custom_maps else None
    return call("export", drop_none(name=name, template=template, maps=maps, format=format, bit_depth=bit_depth, padding=padding, padding_width=padding_width, padding_color=padding_color, size=size, uv_channel=uv_channel), timeout=900)


@server.tool()
def formsurface_save_export_template(
    name: Annotated[str, Field(pattern=r"^[A-Za-z0-9 _.-]{1,80}$")],
    maps: Annotated[list[MapSpec], Field(min_length=1, max_length=16)],
    label: str | None = None,
) -> dict:
    """Save an output template to the workspace library (workspace/templates) so it appears in the editor's Template list and in formsurface_export_maps template for later exports. maps use the same shape as custom_maps. Built-in template names cannot be reused."""
    return call("save_export_template", drop_none(name=name, label=label, maps=[m.model_dump(exclude_none=True) for m in maps]))


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
