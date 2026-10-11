# FormSurface

Paint directly on 3D models, layer by layer, the way Substance 3D Painter works. Part of the Form family.

![version](https://img.shields.io/badge/version-v0.12.0-e0793a) ![engine](https://img.shields.io/badge/engine-Rust%20%2B%20wgpu-555) ![license](https://img.shields.io/badge/license-Apache--2.0-555)

## What works in v0.12.0

- **3D projection painting.** The brush hits the mesh in world space and writes into the UV textures, so strokes keep a constant on-screen size, cross UV seams and do not stretch on distorted islands. Occluded and back-facing surfaces are protected.
- **GPU engine in Rust.** wgpu running on WebGPU in the browser. Painting, compositing and the viewport never leave the GPU; nothing round-trips through the server while you paint.
- **Channels:** base color, roughness, metallic, height (shown as bump in the viewport, exported as a normal map). Each channel can be switched on or off per stroke.
- **Polygon fill:** click to fill a triangle, a flat polygon, a whole mesh part or a UV chunk with the paint material, in one undoable step.
- **Clone (C):** Ctrl+click a source, then paint to copy what is visible there at a fixed offset.
- **Flatten and mask export:** collapse a texture set into one paint layer; save any layer mask as PNG.
- **Quick mask (T):** paint a temporary mask, shown in red where protected, that limits every stroke and fill until you turn it off; clear, allow all and invert.
- **Brush:** size in screen or object space, hardness, flow, stroke opacity, spacing, pen pressure on size or flow, backface culling, eraser.
- **Alignment and tips:** Painter's camera, tangent wrap, tangent planar and UV alignments; round, square and spray tips or any image as a brush alpha; angle, follow path, size, angle, position and flow jitter, lazy mouse.
- **Stencil and symmetry:** a screen-space stencil from any image (scale, rotation, offset); mirror or radial symmetry on X, Y or Z.
- **High-poly baking and smart materials:** tangent-space normal and ID maps baked from a high-poly mesh through a cage, in a background worker so painting continues, optionally matching low and high parts by mesh name (_low/_high, _lo/_hi) so close parts never bake onto each other; smart materials (worn painted metal, rusty iron, dusty plastic, brushed gold, plus your own saved stacks) and smart mask presets.
- **Baking, generators and filters:** GPU baking of ambient occlusion, curvature, thickness, position and world normal from the mesh; fill layers driven by them (cavity dirt, edge wear, thin areas, from top); filter layers for blur, sharpen, brightness, contrast, hue, saturation, gamma and roughness or metallic offsets.
- **Fill layers and masks:** fill layers projected by UV, tri-planar, planar, spherical or cylindrical mapping, with solid, checker, stripes, noise or image patterns; a white or black mask on any layer, painted with the full brush, or taken from an image, an ID map color or a generator that follows every bake.
- **Layers and folders:** add, delete, reorder, hide, opacity, 8 blend modes, undo and redo (24 strokes); folders that hide, fade and mask everything inside them (passthrough blending), nested up to 16 deep.
- **Viewport:** PBR shading with tone mapping, exposure, brush cursor drawn on the surface, orbit, pan, zoom, frame. Seam padding (dilation) on the composited maps.
- **Texture sets and UDIM:** one texture set per material, split per UDIM tile when UVs leave 0 to 1, each with its own layers, base material, undo, bakes and maps; the viewport shows them all. The UV tiles switch gives a material's tiles one shared layer stack, and a stroke paints across tile borders. Fill and filter layers can be instanced into another texture set; editing any copy updates them all.
- **Meshes:** glTF, GLB, OBJ, FBX with their material names, plus built-in samples (sphere, torus, capsule, torus knot, and a two-material figure on UDIM 1001 and 1002).
- **Export:** templates for separate PBR maps, glTF (packed occlusion, roughness, metallic), Unreal Engine (ORM, DirectX normal), Unity HDRP (mask map) and Unity URP, or custom packed maps from agents; 8 or 16-bit PNG; one set of PNGs per texture set (material_suffix, with .udim for tiled materials); the mesh as OBJ with its UDIM UVs; and a viewport screenshot.

## Agents (MCP)

FormSurface ships an MCP server with 43 `formsurface_*` tools that drive the editor you have open: mesh loading, texture sets, layer instances, layers, folders, export templates, fill layers, masks, baking (including from a high-poly mesh), generators, filters, smart materials, brush, paint and base material, camera, strokes, fill, undo, screenshot and map export. Claude Code picks it up from `.mcp.json`; other clients run `./formsurface mcp` (stdio). Press **Agent link** in the editor's top bar to let agents in; it is off by default. The `formsurface-painting` skill (`.claude/skills`, mirrored in `.agents/skills`) teaches agents the order of work.

`./formsurface batch script.json` runs the same tools from a JSON script (a list of `[tool, arguments]` pairs) against the linked editor, for repeatable jobs such as load mesh, bake, apply a smart material and export. `--pause` spaces the steps, `--keep-going` continues past a failed step, `--out` saves screenshots; the exit code is 1 when a step failed. It drives the open editor; it is not headless.

## Parity

Measured against Substance 3D Painter 12.1.5 on 50 documented capabilities, v0.13.0 scores 26 of 50 (52 percent): the painting core, brush engine, polygon fill, clone, quick mask, fill layers, masks, folders, baking from the mesh and from a high-poly mesh with match by name, generators, filter layers, smart materials, texture sets per material, UDIM tiles with one shared stack, engine export presets and OBJ mesh export are real; layer instancing (fill and filter layers) and the batch runner are partial; per-tile resolution, stacked mask effects, color management, EXR and 32-bit export, headless automation and a full asset library are not built yet. The full page is in the planning sidecar.

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
| `B` / `E` / `P` / `C` / `M` | Paint / eraser / polygon fill / clone / mirror |
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
