# FormSurface

Paint directly on 3D models, layer by layer, the way Substance 3D Painter works. Part of the Form family.

![version](https://img.shields.io/badge/version-v0.22.0-e0793a) ![engine](https://img.shields.io/badge/engine-Rust%20%2B%20wgpu-555) ![license](https://img.shields.io/badge/license-Apache--2.0-555)

## What works in v0.22.0

- **3D projection painting.** The brush hits the mesh in world space and writes into the UV textures, so strokes keep a constant on-screen size, cross UV seams and do not stretch on distorted islands. Occluded and back-facing surfaces are protected.
- **GPU engine in Rust.** wgpu running on WebGPU in the browser. Painting, compositing and the viewport never leave the GPU; nothing round-trips through the server while you paint.
- **Channels:** base color, roughness, metallic, height (shown as bump in the viewport, exported as a normal map). Each channel can be switched on or off per stroke.
- **Polygon fill:** click to fill a triangle, a polygon (the original quad or n-gon for meshes from OBJ or ASCII PLY, otherwise the flat face around the triangle), a whole mesh part or a UV chunk with the paint material, in one undoable step.
- **Clone (C):** Ctrl+click a source, then paint to copy what is visible there, or only one chosen layer, at a fixed offset.
- **Path (L):** click points on the surface, then Enter paints a smooth or straight, open or closed curve through them with the current brush, or erases along it. The points stay on the model when the camera moves.
- **2D view (U):** show the active texture set's UV layout as base color, roughness, metallic or height, and paint, polygon-fill or draw paths straight into the texture, reaching faces the camera cannot see.
- **Text:** render text in a built-in or loaded font (TTF, OTF, WOFF) as a brush tip, a stencil or a fill layer image.
- **Flatten, merge and export:** collapse a texture set or a folder into one paint layer, merge a layer down into the one below, and save any layer (color, roughness, metallic, height with coverage in alpha) or layer mask as PNG.
- **Quick mask (T):** paint a temporary mask, shown in red where protected, that limits every stroke and fill until you turn it off; clear, allow all and invert.
- **Brush:** size in screen or object space, hardness, flow, stroke opacity, spacing, pen pressure on size or flow, backface culling, eraser.
- **Alignment and tips:** Painter's camera, tangent wrap, tangent planar and UV alignments; round, square and spray tips or any image as a brush alpha; angle, follow path, size, angle, position and flow jitter, lazy mouse.
- **Stencil and symmetry:** a screen-space stencil from any image (scale, rotation, offset); mirror or radial symmetry on X, Y or Z.
- **High-poly baking and smart materials:** tangent-space normal, ID, height, opacity, ambient occlusion and bent normal maps baked from a high-poly mesh through a distance cage or a custom cage mesh (AO and bent normal from rays cast over the high-poly detail), in a background worker so painting continues, optionally matching low and high parts by mesh name (_low/_high, _lo/_hi) so close parts never bake onto each other; smart materials (worn painted metal, rusty iron, dusty plastic, brushed gold, plus your own saved stacks) and smart masks (eight presets, plus any layer's generator setup saved to the library and applied to fill layers or as a mask on any layer).
- **Baking, generators and filters:** GPU baking of ambient occlusion, curvature, thickness, position and world normal from the mesh, with AO and thickness either from depth maps or ray traced per texel through the mesh's BVH on the GPU; fill layers and masks driven by generators: cavity, edges, thickness, from top, light from a direction, position along a direction, 3D noise, dirt, metal edge wear and a mask builder mixing AO, curvature and noise, each with noise breakup; filter layers for blur, sharpen, pixelate, levels, invert, posterize, brightness, contrast, hue, saturation, gamma and roughness or metallic offsets.
- **Resources and color space:** imported fill images, brush alphas, stencils, image masks and HDR environments are listed in the Resources panel, each read as sRGB, Linear or Raw data and re-applied when that changes.
- **Fill layers and masks:** fill layers projected by UV, tri-planar, planar, spherical or cylindrical mapping, with solid, checker, stripes, noise or image patterns; a white or black mask on any layer, painted with the full brush, or taken from an image, an ID map color or a generator that follows every bake.
- **Layers and folders:** add, delete, reorder, hide, opacity, Painter's 31 layer blend modes (the three normal map modes combine height, since normals come from height here), undo and redo (24 strokes); folders that hide, fade and mask everything inside them (passthrough blending), nested up to 16 deep.
- **Viewport:** PBR shading with a choice of view transform (ACES, Standard, AgX, Filmic), exposure, a built-in studio sky or an HDR environment (Radiance .hdr) with rotation, background and background blur, brush cursor drawn on the surface, orbit, pan, zoom, frame. Post effects on the viewport and screenshots: bloom, glare, lens flare, depth of field, chromatic aberration, sharpen, vignette, film grain and color correction. Seam padding (dilation) on the composited maps.
- **Texture sets and UDIM:** one texture set per material, split per UDIM tile when UVs leave 0 to 1, each with its own layers, base material, undo, bakes and maps; the viewport shows them all. The UV tiles switch gives a material's tiles one shared layer stack, and a stroke paints across tile borders. Fill and filter layers can be instanced into another texture set; editing any copy updates them all.
- **Mesh reload:** texture sets keep their layers across a reload, matched by material and UDIM tile; with Reproject layers on mesh reload, every carried set's paint and masks follow the surface instead of the old UVs, so a re-unwrapped or edited mesh keeps its painting.
- **Meshes:** glTF, GLB, OBJ, FBX, USD (USDA, USDC, USDZ), PLY and Collada with their material names, plus built-in samples (sphere, torus, capsule, torus knot, and a two-material figure on UDIM 1001 and 1002).
- **Export:** templates for separate PBR maps, glTF (packed occlusion, roughness, metallic), Unreal Engine (ORM, DirectX normal), Unity HDRP (mask map), Unity URP, Unity Standard specular, Godot 4, Blender, Arnold, Redshift, USD Preview Surface, Sketchfab and PBR specular glossiness (diffuse and specular converted from base color and metallic), plus your own templates built in the Export panel (any source per R, G, B and A) and saved to the workspace library; PNG (8, 16-bit), TIFF (8, 16, 32-bit float), OpenEXR (16-bit half, 32-bit float), Radiance HDR, JPEG and Photoshop PSD (8, 16, 32-bit), with base color written linear in float files; padding outside the UV islands by infinite dilation or a dilation width followed by transparent, a color or a diffusion fill; one set of files per texture set (material_suffix, with .udim for tiled materials); the mesh as OBJ with its UDIM UVs, keeping the quads and n-gons of meshes from OBJ or ASCII PLY or triangulated, optionally tessellated up to three levels and displaced by the height channel; and a viewport screenshot.

## Agents (MCP)

FormSurface ships an MCP server with 56 `formsurface_*` tools that drive the editor you have open: image import with per-resource color space, post effects, custom bake cages, merge and flatten, layer export, environment lighting, smart masks, mesh loading, texture sets, the 2D view, paths, text resources, layer instances, layers, folders, export templates, fill layers, masks, baking (including from a high-poly mesh), generators, filters, smart materials, brush, paint and base material, camera, strokes, fill, undo, screenshot and map export. Claude Code picks it up from `.mcp.json`; other clients run `./formsurface mcp` (stdio). Press **Agent link** in the editor's top bar to let agents in; it is off by default. The `formsurface-painting` skill (`.claude/skills`, mirrored in `.agents/skills`) teaches agents the order of work.

`./formsurface batch script.json` runs the same tools from a JSON script (a list of `[tool, arguments]` pairs) against the linked editor, for repeatable jobs such as load mesh, bake, apply a smart material and export. `--pause` spaces the steps, `--keep-going` continues past a failed step, `--out` saves screenshots; the exit code is 1 when a step failed. It drives the open editor, or with `--headless` (and optionally `--size 1024`) it starts the service, the editor and a windowless Chrome, Chromium or Edge itself, runs, and stops them again, so it works from a terminal or CI with nothing open. Set `FORMSURFACE_BROWSER` to pick the browser and `FORMSURFACE_BROWSER_ARGS` to pass flags; on Linux the browser runs on a virtual display through `xvfb-run` when it is installed, because Chrome's own headless mode could not read textures back on a software GPU.

`./formsurface script job.py` (or `./formsurface script --headless job.py`) runs a Python script with the `formsurface_api` package importable. Its modules follow Painter's API: `project` (meshes, camera, undo), `textureset`, `layerstack` (paint, fill, filter and folder layers, masks, generators, smart materials, strokes), `baking`, `export`, `resource` (image import and color space) and `display` (environment, post effects, screenshots); `tool()` reaches any formsurface_* tool. Arguments are validated like the MCP tools and refusals raise `FormSurfaceError`. The API runs in its own process against the linked editor rather than inside it.

```python
from formsurface_api import project, layerstack, baking, export
project.load_mesh("crate.usdz")
baking.bake_mesh_maps()
layerstack.apply_smart_material("Rusty iron")
export.export_textures(template="godot")
```

## Parity

Measured against Substance 3D Painter 12.1.5 on 50 documented capabilities, v0.23.0 scores 41 of 50 (82 percent): the painting core, true polygon fill on OBJ and PLY quads and n-gons, mesh export with n-gons, tessellation and displacement, headless batch runs, GPU ray-traced baking, mesh reload with reprojection, mesh import including USD, per-resource color space, viewport post effects, custom bake cages, engine and renderer export presets, brush engine, all of Painter's layer blend modes, clone from the stack or a layer, 2D view painting, merge down and folder flatten, quick mask, text resources, output templates, export formats and padding, HDR environment lighting, fill layers, masks, folders, baking from the mesh and from a high-poly mesh (normal, ID, height, opacity, AO, bent normal) with match by name, generators with a mask builder, smart masks, filter layers with levels, posterize and pixelate, smart materials, texture sets per material, UDIM tiles with one shared stack are real; the path tool (paint and erase, no ribbon or filled modes), layer instancing (fill and filter layers), the Python API (it runs beside the editor, not embedded in it) and view transforms without OCIO are partial; per-tile resolution, per-channel formats, stacked mask effects, OCIO configs and a full asset library are not built yet. The full page is in the planning sidecar.

## Quick start

Requirements: Rust (rustup), Node 22.12+, Python 3.11+, and a browser with WebGPU (Chrome, Edge, Safari 26, Firefox on Windows or macOS).

```bash
./formsurface install     # wasm target, wasm-bindgen 0.2.129, backend venv, npm packages, engine build
./formsurface             # backend on 127.0.0.1:8766, editor on http://localhost:5176
```

Windows: `formsurface_oc_install.bat`, then `formsurface.bat`.

| Command | Does |
| --- | --- |
| `./formsurface engine` | Rebuild the Rust engine into `frontend/src/engine/pkg` |
| `./formsurface backend` | Backend only |
| `./formsurface frontend` | Editor only |
| `./formsurface batch script.json` | Run a JSON script of MCP tool calls against the linked editor |

## Controls

| Input | Action |
| --- | --- |
| Left drag | Paint |
| Right drag or Alt + left drag | Orbit |
| Middle drag or Alt + right drag | Pan |
| Wheel | Zoom |
| `[` `]` | Brush size |
| `B` / `E` / `P` / `C` / `L` / `M` | Paint / eraser / polygon fill / clone / path / mirror |
| `U` | Toggle the 2D UV view |
| Enter / Backspace / Esc (path) | Stroke the path / remove the last point / clear |
| Ctrl + click (clone) | Set the clone source |
| `T` | Quick mask: edit, then use |
| `F` | Frame mesh |
| Ctrl+Z, Ctrl+Shift+Z | Undo, redo |

## Layout

```
native/fs-core     mesh, BVH picking, brush and layer model (pure Rust)
native/fs-engine   wgpu engine: projection paint, compositing, dilation, PBR viewport, export
frontend/          TypeScript + Vite editor; three.js is used only for its file loaders
backend/           FastAPI service: saves projects and exported maps under workspace/
setup/             install and engine build scripts
```

## Ports

| Service | Address |
| --- | --- |
| Editor | http://localhost:5176 |
| Service | http://127.0.0.1:8766 |

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
