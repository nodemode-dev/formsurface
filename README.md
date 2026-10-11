# FormSurface

Paint directly on 3D models, layer by layer, the way Substance 3D Painter works. Part of the Form family.

![version](https://img.shields.io/badge/version-v0.3.0-e0793a) ![engine](https://img.shields.io/badge/engine-Rust%20%2B%20wgpu-555) ![license](https://img.shields.io/badge/license-Apache--2.0-555)

## What works in v0.3.0

- **3D projection painting.** The brush hits the mesh in world space and writes into the UV textures, so strokes keep a constant on-screen size, cross UV seams and do not stretch on distorted islands. Occluded and back-facing surfaces are protected.
- **GPU engine in Rust.** wgpu running on WebGPU in the browser. Painting, compositing and the viewport never leave the GPU; nothing round-trips through the server while you paint.
- **Channels:** base color, roughness, metallic, height (shown as bump in the viewport, exported as a normal map). Each channel can be switched on or off per stroke.
- **Brush:** size in screen or object space, hardness, flow, stroke opacity, spacing, pen pressure on size or flow, backface culling, eraser.
- **Alignment and tips:** Painter's camera, tangent wrap, tangent planar and UV alignments; round, square and spray tips or any image as a brush alpha; angle, follow path, size, angle, position and flow jitter, lazy mouse.
- **Stencil and symmetry:** a screen-space stencil from any image (scale, rotation, offset); mirror or radial symmetry on X, Y or Z.
- **Fill layers and masks:** fill layers projected by UV, tri-planar, planar, spherical or cylindrical mapping, with solid, checker, stripes, noise or image patterns; a white or black mask on any layer, painted with the full brush.
- **Layers:** add, delete, reorder, hide, opacity, 8 blend modes, undo and redo (24 strokes).
- **Viewport:** PBR shading with tone mapping, exposure, brush cursor drawn on the surface, orbit, pan, zoom, frame. Seam padding (dilation) on the composited maps.
- **Meshes:** glTF, GLB, OBJ, FBX, plus built-in samples (sphere, torus, capsule, torus knot).
- **Export:** base color, roughness, metallic, height and normal (OpenGL) PNG maps, and a viewport screenshot.

## Agents (MCP)

FormSurface ships an MCP server with 19 `formsurface_*` tools that drive the editor you have open: layers, fill layers, masks, brush, paint and base material, camera, strokes, fill, undo, screenshot and map export. Claude Code picks it up from `.mcp.json`; other clients run `./formsurface mcp` (stdio). Press **Agent link** in the editor's top bar to let agents in; it is off by default. The `formsurface-painting` skill (`.claude/skills`, mirrored in `.agents/skills`) teaches agents the order of work.

## Parity

Measured against Substance 3D Painter 12.1.5 on 50 documented capabilities, v0.3.0 scores 14.5 of 50 (29 percent): the painting core and brush engine are real, texturing features (fill projections, masks, generators, smart materials, baking, texture sets, UDIMs, color management) are not built yet. The full page is in the planning sidecar.

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

## Controls

| Input | Action |
| --- | --- |
| Left drag | Paint |
| Right drag or Alt + left drag | Orbit |
| Middle drag or Alt + right drag | Pan |
| Wheel | Zoom |
| `[` `]` | Brush size |
| `B` / `E` / `M` | Paint / eraser / mirror |
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
