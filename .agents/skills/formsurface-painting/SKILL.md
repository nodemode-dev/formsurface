---
name: formsurface-painting
description: Paint and texture 3D models in FormSurface through its MCP server (33 formsurface_* tools that drive the editor open in the person's browser). Use whenever an agent should texture, paint, recolor, add wear or trim, layer materials, set roughness or metallic, take a viewport screenshot or export PBR maps from FormSurface, or when formsurface tools fail with no editor linked. Trigger on paint this model, texture the mesh in formsurface, add a gold rim, make it look worn, export texture maps, formsurface_paint_stroke, formsurface_status, Agent link, or ./formsurface mcp, even when MCP is not named. Not for changing FormSurface source code, which is the formsurface-harness skill.
license: Apache-2.0
metadata:
  version: "0.9.0"
  author: nodemode-ai
---

# Painting with FormSurface over MCP

The tools give an agent hands; this skill gives the order of work. FormSurface paints like Substance 3D Painter: strokes project onto the visible surface from the current camera and land in UV textures, on the active layer, with the current brush and paint material.

## Before anything

1. Call `formsurface_status`. If it fails with "No FormSurface editor is linked", ask the person to open http://localhost:5176 and press Agent link in the top bar; if the service is not running, ask them to run `./formsurface`. Do not retry blindly.
2. Read `texture_sets`, `active_texture_set`, `layers`, `active_layer`, `brush`, `paint_material`, `camera` and `viewport` from the status. Layer ids are what later calls take.
3. A mesh with several materials, or UVs outside 0 to 1 (UDIM tiles), has one texture set per material and tile, each with its own layers, base material and maps, like Painter's texture sets. Call `formsurface_select_texture_set` before working on a part; strokes, layers, fills and smart materials only reach the active set, so a stroke over another set paints nothing there. Layer ids belong to their set.

## The loop that works

1. Set up the surface: `formsurface_set_base_material` for the material under everything.
2. One idea per layer: `formsurface_add_layer` with a descriptive name, so the person can hide or tune it later.
3. Choose what paints: `formsurface_set_paint_material` sets values and which channels a stroke writes. Switch off channels you do not mean to change (for a color-only tint, turn off roughness and metallic).
4. For broad coverage add a fill layer with `formsurface_add_fill_layer` (a base material, or a pattern through tri-planar projection when UVs are messy), then tune it with `formsurface_set_fill`. To put a material only in some places, give that fill layer a black mask with `formsurface_layer_mask`, set the paint material's base color to white and paint the mask; black paint hides it again. `formsurface_fill_layer` floods the active paint layer instead.
5. For wear and grime that follow the shape, run `formsurface_bake_maps` once per mesh, then give fill layers a generator with `formsurface_set_generator`: cavity for dirt in crevices, edges for worn edges, thin for thin parts, top for dust or fading from above. Adjust level and softness, then screenshot. For overall color or roughness shifts, add a filter layer with `formsurface_add_filter_layer` above the layers it should change.
6. Fastest start: `formsurface_list_smart_materials`, bake, then `formsurface_apply_smart_material` (worn painted metal, rusty iron, dusty plastic, brushed gold, or anything the person saved). Retune one layer with `formsurface_apply_smart_mask` or `formsurface_set_generator`, and save a good result with `formsurface_save_smart_material`.
7. When the person has a high-poly sculpt, have them put it in the workspace meshes folder and run `formsurface_bake_high_poly` with its file name before painting; raise the cage distances if the result reports texels that missed.
8. For marks, place the camera with `formsurface_set_camera`, call `formsurface_screenshot`, find where the surface sits in the image, then `formsurface_paint_stroke` with normalized viewport points (0,0 top left, 1,1 bottom right) along that region. The result reports `points_on_surface`; zero means the stroke missed, so re-aim instead of repeating.
For a whole part rather than a mark (a bolt, a panel, a UV island, one flat face), use `formsurface_polygon_fill` at a point on it instead of strokes: mesh fills the connected part, uv_chunk the UV island, polygon the flat face, triangle a single triangle. It uses the paint material at stroke opacity and is one undo step.
9. Check with another screenshot from a second angle. Strokes paint only what the camera sees; turn the camera to reach the other side.
10. Undo is per stroke or fill (`formsurface_undo`), up to 24 steps, and belongs to the active texture set.
11. Finish with `formsurface_export_maps`, which returns the folder of base color, roughness, metallic, height and normal PNGs for every texture set (bakes add their maps). When the person names a target engine, pass its template (gltf, unreal, unity_hdrp, unity_urp); for any other layout, pass custom_maps with packed sources. Bake first so packed AO is not plain white. Pass bit_depth 16 when height or normal maps will drive displacement or close-up detail.

Group related layers with `formsurface_add_folder` (for example all the wear of one material) so the person can hide or fade them together; smart materials already arrive in a folder of their own name. While a folder is the active layer, new layers go inside it.

## Judgment

- Brush size is in screen pixels, so it shrinks on the model as the camera zooms out. Zoom in (`zoom` below 1) for detail.
- Height 0.5 is flat; paint height only on its own layer so it can be removed.
- Mirror X paints the opposite side through the mesh center; use it for symmetric models only. Radial symmetry repeats a mark around an axis (bolts on a wheel, panels on a can).
- Alignment: tangent_wrap (default) hugs curved surfaces; tangent_planar gives crisp decal-like marks on flat areas; camera paints exactly the screen footprint, stretching on grazing faces; uv follows the texture layout and distorts where the UVs do.
- For organic wear use the spray tip with size and position jitter; for seams and trims use follow_path with a square tip.
- Tell the person what you changed by layer name. Never delete their layers unless asked.

## Reference

`references/tools.md` lists every tool with its parameters. `EVALS.json` holds the requests this skill is checked against.
