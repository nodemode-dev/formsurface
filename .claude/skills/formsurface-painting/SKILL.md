---
name: formsurface-painting
description: Paint and texture 3D models in FormSurface through its MCP server (22 formsurface_* tools that drive the editor open in the person's browser). Use whenever an agent should texture, paint, recolor, add wear or trim, layer materials, set roughness or metallic, take a viewport screenshot or export PBR maps from FormSurface, or when formsurface tools fail with no editor linked. Trigger on paint this model, texture the mesh in formsurface, add a gold rim, make it look worn, export texture maps, formsurface_paint_stroke, formsurface_status, Agent link, or ./formsurface mcp, even when MCP is not named. Not for changing FormSurface source code, which is the formsurface-harness skill.
license: Apache-2.0
metadata:
  version: "0.4.0"
  author: nodemode-ai
---

# Painting with FormSurface over MCP

The tools give an agent hands; this skill gives the order of work. FormSurface paints like Substance 3D Painter: strokes project onto the visible surface from the current camera and land in UV textures, on the active layer, with the current brush and paint material.

## Before anything

1. Call `formsurface_status`. If it fails with "No FormSurface editor is linked", ask the person to open http://localhost:5176 and press Agent link in the top bar; if the service is not running, ask them to run `./formsurface`. Do not retry blindly.
2. Read `layers`, `active_layer`, `brush`, `paint_material`, `camera` and `viewport` from the status. Layer ids are what later calls take.

## The loop that works

1. Set up the surface: `formsurface_set_base_material` for the material under everything.
2. One idea per layer: `formsurface_add_layer` with a descriptive name, so the person can hide or tune it later.
3. Choose what paints: `formsurface_set_paint_material` sets values and which channels a stroke writes. Switch off channels you do not mean to change (for a color-only tint, turn off roughness and metallic).
4. For broad coverage add a fill layer with `formsurface_add_fill_layer` (a base material, or a pattern through tri-planar projection when UVs are messy), then tune it with `formsurface_set_fill`. To put a material only in some places, give that fill layer a black mask with `formsurface_layer_mask`, set the paint material's base color to white and paint the mask; black paint hides it again. `formsurface_fill_layer` floods the active paint layer instead.
5. For wear and grime that follow the shape, run `formsurface_bake_maps` once per mesh, then give fill layers a generator with `formsurface_set_generator`: cavity for dirt in crevices, edges for worn edges, thin for thin parts, top for dust or fading from above. Adjust level and softness, then screenshot. For overall color or roughness shifts, add a filter layer with `formsurface_add_filter_layer` above the layers it should change.
5. For marks, place the camera with `formsurface_set_camera`, call `formsurface_screenshot`, find where the surface sits in the image, then `formsurface_paint_stroke` with normalized viewport points (0,0 top left, 1,1 bottom right) along that region. The result reports `points_on_surface`; zero means the stroke missed, so re-aim instead of repeating.
6. Check with another screenshot from a second angle. Strokes paint only what the camera sees; turn the camera to reach the other side.
7. Undo is per stroke or fill (`formsurface_undo`), up to 24 steps.
8. Finish with `formsurface_export_maps`, which returns the folder of base color, roughness, metallic, height and normal PNGs.

## Judgment

- Brush size is in screen pixels, so it shrinks on the model as the camera zooms out. Zoom in (`zoom` below 1) for detail.
- Height 0.5 is flat; paint height only on its own layer so it can be removed.
- Mirror X paints the opposite side through the mesh center; use it for symmetric models only. Radial symmetry repeats a mark around an axis (bolts on a wheel, panels on a can).
- Alignment: tangent_wrap (default) hugs curved surfaces; tangent_planar gives crisp decal-like marks on flat areas; camera paints exactly the screen footprint, stretching on grazing faces; uv follows the texture layout and distorts where the UVs do.
- For organic wear use the spray tip with size and position jitter; for seams and trims use follow_path with a square tip.
- Tell the person what you changed by layer name. Never delete their layers unless asked.

## Reference

`references/tools.md` lists every tool with its parameters. `EVALS.json` holds the requests this skill is checked against.
