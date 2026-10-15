# FormSurface

Paint directly on 3D models, layer by layer, the way Substance 3D Painter works. Part of the Form family.

![version](https://img.shields.io/badge/version-v0.50.1-f97316) ![engine](https://img.shields.io/badge/engine-Rust%20%2B%20wgpu-555) ![license](https://img.shields.io/badge/license-Apache--2.0-555)

## What works in v0.50.1

- **3D projection painting.** The brush hits the mesh in world space and writes into the UV textures, so strokes keep a constant on-screen size, cross UV seams and do not stretch on distorted islands. Occluded and back-facing surfaces are protected.
- **GPU engine in Rust.** wgpu running on WebGPU in the browser. Painting, compositing and the viewport never leave the GPU; nothing round-trips through the server while you paint.
- **Channels:** base color, roughness, metallic, height (shown as bump in the viewport, exported as a normal map), plus one extra channel per texture set: a named user channel, opacity (cut-out in the viewport), ambient occlusion (darkens shading), emissive (glows with the base color, exported as an emissive map), transmission (glass, refracted by the path tracer) or scattering (subsurface, random-walked by the path tracer), and up to two further named grayscale user channels per texture set that paint, fill, merge, flatten, undo, resample and page with the rest and export as their own maps (or pack into template maps as user2 and user3). Each channel can be switched on or off per stroke and per fill.
- **Polygon fill:** click to fill a triangle, a polygon (the original quad or n-gon for meshes from OBJ or ASCII PLY, otherwise the flat face around the triangle), a whole mesh part or a UV chunk with the paint material, in one undoable step.
- **Clone (C):** Ctrl+click a source, then paint to copy what is visible there, or only one chosen layer, at a fixed offset.
- **Path (L):** click points on the surface (drag a point to move it), then Enter paints a smooth or straight, open or closed curve through them: Paint stamps the current brush along it, Ribbon lays one even strip as wide as the brush (shaped by hardness, or the image tip repeated along the curve), Filled fills the inside; any mode can erase instead. The points stay on the model when the camera moves.
- **2D view (U):** show the active texture set's UV layout as base color, roughness, metallic or height, and paint, polygon-fill or draw paths straight into the texture, reaching faces the camera cannot see.
- **Text:** render text in a built-in or loaded font (TTF, OTF, WOFF) as a brush tip, a stencil or a fill layer image.
- **Flatten, merge and export:** collapse a texture set or a folder into one paint layer, merge a layer down into the one below, and save any layer (color, roughness, metallic, height with coverage in alpha) or layer mask as PNG.
- **Quick mask (T):** paint a temporary mask, shown in red where protected, that limits every stroke and fill until you turn it off; clear, allow all and invert.
- **Brush:** size in screen or object space, hardness, flow, stroke opacity, spacing, pen pressure on size or flow, backface culling, eraser.
- **Alignment and tips:** Painter's camera, tangent wrap, tangent planar and UV alignments; round, square and spray tips or any image as a brush alpha; angle, follow path, size, angle, position and flow jitter, lazy mouse.
- **Stencil and symmetry:** a screen-space stencil from any image (scale, rotation, offset); mirror or radial symmetry on X, Y or Z.
- **High-poly baking and smart materials:** tangent-space normal, ID, height, opacity, ambient occlusion and bent normal maps baked from a high-poly mesh through a distance cage or a custom cage mesh (AO and bent normal from rays cast over the high-poly detail), in a background worker so painting continues, optionally matching low and high parts by mesh name (_low/_high, _lo/_hi) so close parts never bake onto each other; smart materials (worn painted metal, rusty iron, dusty plastic, brushed gold, plus your own saved stacks) in an asset browser with thumbnails, search and a material or mask filter, where any asset can be shared as a .fsasset.json file and imported into another workspace and smart masks (eight presets, plus any layer's generator setup saved to the library and applied to fill layers or as a mask on any layer).
- **Baking, generators and filters:** GPU baking of ambient occlusion, curvature, thickness, position and world normal from the mesh, with AO and thickness either from depth maps or ray traced per texel through the mesh's BVH on the GPU; fill layers and masks driven by generators: cavity, edges, thickness, from top, light from a direction, position along a direction, 3D noise, dirt, metal edge wear and a mask builder mixing AO, curvature and noise, each with noise breakup; filter layers for blur, sharpen, pixelate, levels, invert, posterize, brightness, contrast, hue, saturation, gamma and roughness or metallic offsets.
- **Resources and color space:** imported fill images, brush alphas, stencils, image masks and HDR environments are listed in the Resources panel, each read as sRGB, Linear or Raw data and re-applied when that changes.
- **Fill layers and masks:** fill layers projected by UV (on the first or, for lightmap and trim layouts, the second UV channel), tri-planar, planar, spherical, cylindrical or warp mapping (a planar projection bent by dragging a 4 by 4 grid on the model), or Fill (match per UV tile), which puts one image of a UDIM sequence on each tile, with solid, checker, stripes, noise or image patterns; a white or black mask on any layer, painted with the full brush, or taken from an image, an ID map color or a generator that follows every bake, with a stack of mask effects on top (fill, generator, levels, invert, blur and compare mask, each with a blend mode and opacity), nested in groups that run their contents on their own mask and blend the result in, so a levels or invert inside a group touches only that group.
- **Layers and folders:** add, delete, reorder, hide, opacity, Painter's 31 layer blend modes (the three normal map modes combine height, since normals come from height here), undo and redo (24 strokes); folders that hide, fade and mask everything inside them (passthrough blending), nested up to 16 deep.
- **Viewport:** PBR shading with a choice of view transform (ACES, Standard, AgX, Filmic, or any display, view and look of an OpenColorIO 2 config: the built-in ACES CG and studio configs or an .ocio file in the workspace ocio folder, run by real OpenColorIO in the service and applied as a 3D LUT; fill images and environments can be read in any color space of that config; the working space is linear Rec.709 or ACEScg, and switching converts painted layers while base color maps still export as sRGB), exposure, a built-in studio sky or an HDR environment (Radiance .hdr) with rotation, background and background blur, brush cursor drawn on the surface, orbit, pan, zoom, frame. A progressive path tracer for the viewport and screenshots: every texture set's paint lit by the HDR or the studio sky and lights, with traced shadows, up to 12 bounces of light between parts, opacity cut-outs and emissive glow, HDR lighting importance-sampled so a sun converges quickly, an optional edge-aware denoiser guided by albedo, normals and depth, a thin-lens depth of field, glass and subsurface scattering for sets whose extra channel is transmission or scattering, the post effects on top, and shadows that pass through opacity cut-outs, refining until a chosen sample count and restarting on any change; Render to file traces the current view at any size up to 3840 by 3840 and saves a PNG. Post effects on the viewport and screenshots: bloom, glare, lens flare, depth of field, chromatic aberration, sharpen, vignette, film grain and color correction. Seam padding (dilation) on the composited maps.
- **Texture sets and UDIM:** one texture set per material, split per UDIM tile when UVs leave 0 to 1, each with its own layers, base material, undo, bakes, maps and resolution (Set size, 256 to 4096: resizing resamples the set's layers); the viewport shows them all. The UV tiles switch gives a material's tiles one shared layer stack, and a stroke paints across tile borders. Any layer can be instanced into another texture set, folders with their contents (layers later added to the source folder follow into its instances; strokes stay on the source, as in Painter); editing any copy updates them all. Idle texture sets can be paged out of GPU memory (their layer stacks and undo history move to the editor's memory while their finished look stays on screen) and come back when selected, exported or baked. A layer whose mask is black on a tile (a fill with a black mask not yet painted there) costs nothing on that tile: its fill and compositing are skipped until something writes its mask.
- **Mesh reload:** texture sets keep their layers across a reload, matched by material and UDIM tile; with Reproject layers on mesh reload, every carried set's paint and masks follow the surface instead of the old UVs, so a re-unwrapped or edited mesh keeps its painting.
- **Meshes:** glTF, GLB, OBJ, FBX, USD (USDA, USDC, USDZ), PLY and Collada with their material names, plus built-in samples (sphere, torus, capsule, torus knot, and a two-material figure on UDIM 1001 and 1002).
- **Export:** templates for separate PBR maps, glTF (packed occlusion, roughness, metallic), Unreal Engine (ORM, DirectX normal), Unity HDRP (mask map), Unity URP, Unity Standard specular, Godot 4, Blender, Arnold, Redshift, USD Preview Surface, Sketchfab and PBR specular glossiness (diffuse and specular converted from base color and metallic), plus your own templates built in the Export panel (any source per R, G, B and A) and saved to the workspace library; PNG (8, 16-bit), TIFF (8, 16, 32-bit float), OpenEXR (16-bit half, 32-bit float), Radiance HDR, JPEG and Photoshop PSD (8, 16, 32-bit), with base color written linear in float files; padding outside the UV islands by infinite dilation or a dilation width followed by transparent, a color or a diffusion fill; one set of files per texture set (material_suffix, with .udim for tiled materials), each at its own size or at a chosen export size from 512 to 8192 (as the GPU allows), for which the stack is recomputed one layer at a time (so memory depends on the size, not the layer count) with fills, generators and mask effects rendered natively and paint resampled, leaving the working textures untouched; on the painted UV layout or moved onto the mesh's second UV channel (glTF TEXCOORD_1, FBX and Collada second UV set) for lightmap and trim layouts, padded around the UV1 islands; the mesh as OBJ with its UDIM UVs, keeping the quads and n-gons of meshes from OBJ or ASCII PLY or triangulated, optionally tessellated up to three levels and displaced by the height channel; and a viewport screenshot.

## Claude Code inside the app

- **Terminal dock (Ctrl+J, Cmd+J on macOS).** A real terminal in the bottom dock (xterm.js on a PTY in the service), with tabs, restart and a resizable height that survives reloads; programs keep running while it is hidden.
- **Claude Code (Ctrl+Shift+J).** Starts your own Claude Code sign-in in a dock tab, already connected to the `formsurface` MCP server and told how to work on the model. Agent Link turns on by itself. The line under the dock sends a message to it.
- **Chat (Ctrl+Alt+J).** The same Claude Code without a terminal: Markdown replies, the tools it ran, memory and cost, Stop, New Chat, and hand-off between the Chat tab and a Claude Code tab in either direction.
- **Preferences (Ctrl+,).** The Claude Code program, the folder it starts in, a first message, and the interface language.

## Commands, menus and shortcuts

Every action is a command in one registry (`frontend/src/shell/commands.ts`), the same structure as formvis and formlit: the menu bar (File, Edit, Tools, Layer, Bake, View, Agent, Help), the title bar buttons, the command palette (Ctrl+K) and the keyboard all run the same commands. Edit > Keyboard Shortcuts rebinds any of them, and the bindings are kept per browser. New shortcuts include Add Paint Layer (Ctrl+Shift+N), Add Fill Layer (Ctrl+Shift+F), Add Fill Layer with Black Mask (Ctrl+Alt+Shift+F), Add Black Mask (Ctrl+Alt+M), Merge Down (Ctrl+E), Add Folder (Ctrl+G), Delete Layer (Ctrl+Backspace), Bake Mesh Maps (Ctrl+Shift+B), Pick Paint Color from Screen (I), Select Layer Under Cursor (Shift+S, or Alt+Shift+click in the viewport), Hide Part Under Cursor (H), Hide UV Island Under Cursor (Shift+H), Isolate Part Under Cursor (Alt+Shift+H), Show All Parts (Alt+H), Lasso Fill (Q) and Rectangle Fill (Shift+Q) that fill the visible surface inside a dragged shape with the paint material (per texel, in the 3D and 2D views), and Limit Strokes to Lasso (Alt+Q) or Rectangle (Alt+Shift+Q) that turn the shape into the quick mask so later strokes stay inside it. Under the base color, a strip of the last 12 colors applies one with a click, and its lock keeps a palette fixed. The Texture sets section counts hidden triangles, with Show all and Step back. Bake > Cage offset reads user channel 2 or 3 as a painted cage offset map: black keeps Cage front and rear, white pushes them out four times, only where a bake misses. Layer > Edit Layer in External App (Ctrl+Alt+E) opens the active layer as a PNG in Photoshop or any app set in Preferences and adds a fill layer above it that updates every time you save there. On Windows, `formsurface.bat help` lists the same commands as the shell script (build, serve, mcp, claude, doctor, install-desktop, freeze) and Alt+1 to Alt+5 for the Paint, Layers, Bake, Display and Export panels. Panel sections fold, and the fold state is remembered.

## Languages

The interface is in English by default and in 25 more languages: Spanish (Spain and Mexico), Brazilian Portuguese, French (France and Canada), German, Italian, Dutch, Danish, Finnish, Norwegian Bokmal, Swedish, Russian, Polish, Czech, Hungarian, Ukrainian, Turkish, Simplified and Traditional Chinese, Japanese, Korean, Indonesian, Thai and Vietnamese. Choose one in Preferences. English text is the key (`frontend/src/i18n/strings.json`), each language is a table in `frontend/src/i18n/locales` plus `more`, and the page translator swaps text before the first paint, so nothing flashes in English.

## Agents (MCP)

FormSurface ships an MCP server with 66 `formsurface_*` tools that drive the editor you have open: any menu command by id (list_commands, run_command), picking the layer under a point, hiding and isolating mesh parts, filling or limiting strokes to a screen polygon (select_region), image import with per-resource color space, post effects, custom bake cages, merge and flatten, layer export, environment lighting, smart masks, mesh loading, texture sets, the 2D view, paths, text resources, layer instances, layers, folders, export templates, fill layers, masks, baking (including from a high-poly mesh), generators, filters, smart materials, brush, paint and base material, camera, strokes, fill, undo, screenshot and map export. Claude Code picks it up from `.mcp.json`; other clients run `./formsurface mcp` (stdio). Press **Agent link** in the editor's top bar to let agents in; it is off by default. The `formsurface-painting` skill (`.claude/skills`, mirrored in `.agents/skills`) teaches agents the order of work.

`./formsurface batch script.json` runs the same tools from a JSON script (a list of `[tool, arguments]` pairs) against the linked editor, for repeatable jobs such as load mesh, bake, apply a smart material and export. `--pause` spaces the steps, `--keep-going` continues past a failed step, `--out` saves screenshots; the exit code is 1 when a step failed. It drives the open editor, or with `--headless` (and optionally `--size 1024`) it starts the service, the editor and a windowless Chrome, Chromium or Edge itself, runs, and stops them again, so it works from a terminal or CI with nothing open. Set `FORMSURFACE_BROWSER` to pick the browser and `FORMSURFACE_BROWSER_ARGS` to pass flags; on Linux the browser runs on a virtual display through `xvfb-run` when it is installed, because Chrome's own headless mode could not read textures back on a software GPU.

`./formsurface script job.py` (or `./formsurface script --headless job.py`) runs a Python script with the `formsurface_api` package importable. Its modules follow Painter's API: `project` (meshes, camera, undo), `textureset`, `layerstack` (paint, fill, filter and folder layers, masks, generators, smart materials, strokes), `baking`, `export`, `resource` (image import and color space) and `display` (environment, post effects, screenshots); `tool()` reaches any formsurface_* tool. Arguments are validated like the MCP tools and refusals raise `FormSurfaceError`. The same API also runs embedded in the editor: the Python panel runs scripts in the page through Pyodide (loaded on first use from the pyodide npm package, no network needed) with the same modules and argument checks, plus `event` (connect functions to mesh_loaded, stroke_end, layer_stack_changed, texture_set_changed, export_finished and bake_finished) and `ui` (plugin buttons and log lines). Run scripts runs every `.py` file in `workspace/scripts`, so plugins can add their buttons and event handlers in one click.

```python
from formsurface_api import project, layerstack, baking, export
project.load_mesh("crate.usdz")
baking.bake_mesh_maps()
layerstack.apply_smart_material("Rusty iron")
export.export_textures(template="godot")
```

## Parity

Measured against Substance 3D Painter 12.1.5 on 50 documented capabilities, v0.50.1 scores 47.5 of 50 (95 percent): the painting core, OpenColorIO 2 color management with an ACEScg working space, a path tracer with glass, subsurface scattering, depth of field, a denoiser and renders to file, texture sets each with their own channel list (an optional fifth channel), a smart asset browser with thumbnails and shareable asset files, a Python API embedded in the editor with events and plugin buttons, mask effect stacks, UV tiles with per-tile resolution, layer instancing across texture sets for every layer kind and folders, all seven of Painter's fill projections including warp and fill per UV tile, the path tool with paint, ribbon, filled and erase modes, true polygon fill on OBJ and PLY quads and n-gons, mesh export with n-gons, tessellation and displacement, headless batch runs, GPU ray-traced baking, mesh reload with reprojection, mesh import including USD, per-resource color space, viewport post effects, custom bake cages, engine and renderer export presets, brush engine, all of Painter's layer blend modes, clone from the stack or a layer, 2D view painting, merge down and folder flatten, quick mask, text resources, output templates, export formats and padding, HDR environment lighting, fill layers, masks, folders, baking from the mesh and from a high-poly mesh (normal, ID, height, opacity, AO, bent normal) with match by name, generators with a mask builder, smart masks, filter layers with levels, posterize and pixelate, smart materials, texture sets per material, UDIM tiles with one shared stack are real; painting channels beyond the four (one extra role per set plus two named user channels), user channels (up to three per set against Painter's eight), channel bit depth (16-bit float throughout, no per-channel choice), resolution (4096 to work in; exports above 4096 recompute the stack but 8192 has not yet run on a real GPU) and memory paging (whole texture sets rather than streamed tiles) are partial. The full page is in the planning sidecar.

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
| `./formsurface build` | Build the engine and the editor into `frontend/dist` |
| `./formsurface serve` | Serve the built editor from the service on 8766, one origin, no Vite |
| `./formsurface claude` | Claude Code in your own terminal, connected to the formsurface MCP server |
| `./formsurface doctor` | Check the toolchain, the builds and the running service |
| `./formsurface install-desktop` | PyInstaller and the Tauri toolchain for the desktop app |
| `./formsurface desktop dev` | Run the desktop app from source |
| `./build_app` | Build the desktop app start to finish (macOS .app and .dmg, Linux .deb); `--open`, `--install`, `--clean` |

## Desktop app

`./build_app` is the single build script at the root: it checks Python, Node and Rust (installing Rust with rustup when missing), installs the engine toolchain, the service and the editor, builds the WebGPU engine and the editor, freezes the service with PyInstaller into a sidecar (`setup/freeze_backend`, which runs a self-check first), and packs everything with Tauri 2. The app shows a splash, starts the sidecar on 127.0.0.1:8766, and opens the editor served by it, so the terminal dock, Claude Code and Chat work exactly as in the browser. On macOS the app is copied to Applications; on Linux, `--install` installs the .deb. Every step is logged to `build_app.log`. Linux needs `libwebkit2gtk-4.1-dev` and its companions (the script names them when they are missing). The sidecar is handed the app's process id and exits when the app is gone, so even a killed app never leaves it holding the port. WebKitGTK has no WebGPU yet, so on Linux the desktop app's engine runs on wgpu's WebGL 2 path (the status bar shows `Gl`), and so does any browser whose WebGPU has no adapter. On that path painting, layer pick, lasso and rectangle selections, hiding parts, fill layers, mask effects and groups, map export (including onto UV1) and the mesh bake work and match WebGPU; the path tracer and the ray-traced bake need storage buffers WebGL 2 does not have, and the viewport's depth of field reads no scene depth there, so use a WebGPU browser for those three.

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
| Ctrl+K | Command palette |
| Ctrl+J | Terminal dock |
| Ctrl+/ | Keyboard shortcuts (rebind any command) |

## Layout

```
native/fs-core     mesh, BVH picking, brush and layer model (pure Rust)
native/fs-engine   wgpu engine: projection paint, compositing, dilation, PBR viewport, export
frontend/          TypeScript + Vite editor; three.js is used only for its file loaders
frontend/src/shell command registry, menu bar, palette, dialogs, terminal dock and chat pane
frontend/src/i18n  the page translator and the 25 language tables
backend/           FastAPI service: saves projects and exported maps under workspace/
backend/app/agent  PTY terminals, Claude Code launch, chat over claude -p, SSE events, the command bus
src-tauri/         Tauri 2 desktop shell that starts the frozen service as a sidecar
setup/             install, engine, editor, freeze and desktop build scripts
build_app          the one root script that builds the desktop app
```

## Ports

| Service | Address |
| --- | --- |
| Editor | http://localhost:5176 |
| Service | http://127.0.0.1:8766 |

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
