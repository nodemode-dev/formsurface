---
name: engine
description: The Rust engine of FormSurface, fs-core and fs-engine on wgpu 30, compiled to WebAssembly. Load for painting, projection, stamps, layers, compositing, dilation, undo, viewport shading, export and wasm builds.
---

# Engine

## How a stroke works

1. `Engine::pick` builds a ray from the orbit camera basis and raycasts the BVH in fs-core. The same basis feeds `view_projection`, so the picked point and the drawn point agree.
2. `stroke_to` places stamps along the world-space segment every `spacing * diameter`. Radius converts screen pixels to world units at the hit distance, so size is constant on screen.
3. `Gpu::paint` renders the mesh with vertices placed at their UV. Each texel gets its world position and normal and paints when it is inside a stamp sphere, visible in the last view depth, facing the camera when culling is on, and facing the stamp normal. Coverage accumulates in `mask` with union blending.
4. `apply` writes the layer as snapshot over paint at stroke opacity. The snapshot taken in `begin_stroke` becomes the undo entry.
5. `composite` folds layers over the base material with blend modes into ping-pong targets, then `dilate` pads across UV island borders into `final_set`, which the viewport and exports read.

## Layer storage

Three Rgba16Float textures per layer: premultiplied color with coverage in alpha, premultiplied values (roughness, metallic, height) and per-channel coverage. Keep channels independent: a stroke that paints only roughness must not change metallic coverage.

## Rules for changes

- New GPU work goes in `gpu.rs`, new shaders in `shaders.rs`, JS-facing methods only in `lib.rs`. fs-core stays free of GPU types.
- Every queue write that differs per pass needs its own submit or its own buffer region; a second `write_buffer` before submit overwrites the first.
- Never use mapped-at-creation buffers for mesh data; some WebGPU implementations cap their size.
- Anything an agent can trigger must work when the tab is not animating: flush and composite before reading back.
- Build: `setup/build_engine`. In a container without the wasm target, build std from source with `RUSTC_BOOTSTRAP=1 FS_CARGO_FLAGS="-Zbuild-std=std,panic_abort"` after pointing the sysroot's `lib/rustlib/src/rust` at the Rust library sources.

## Verifying

Paint in the editor on the sphere and torus knot samples, then export maps and check the stroke in base color and roughness. In headless containers, Chromium needs `--enable-unsafe-webgpu --enable-features=Vulkan --use-vulkan=native` with lavapipe; the canvas does not present there, so verify through `snapshot` and `export_channel` readbacks and say so.

## Brush model (v0.3.0)

A stamp is 16 floats: center and world radius, normal and a mirrored flag, tangent and angle, UV position, UV radius and flow scale. Symmetric copies set the mirrored flag, which skips the visibility test and uses tangent-plane coordinates in camera and UV alignments (their screen or UV position would be the original's). Tips are evaluated in the stamp's local 2D frame after rotation; the stencil multiplies coverage in screen space. Jitter, lazy mouse and follow path live in `lib.rs` (`push_stamp`, `stroke_to`); the shader stays deterministic per stamp.

## Fill layers and masks (v0.4.0)

`upload_mesh` rasterizes world position (Rgba32Float) and normal into UV space once; `fill_layer_content` evaluates a fill layer per texel from those maps with the FILL shader and overwrites the layer's color, values and coverage, so fill layers are regenerated whenever their settings or the mesh change and are never painted directly. Each layer set carries a fourth texture, the mask (white by default), copied into undo snapshots; strokes on a mask go through `apply_mask`, and the composite scales opacity by it when `LayerInfo.mask` is on.

## Baking, generators and filters (v0.5.0)

`bake_maps` is rasterized, not ray traced: for each Fibonacci direction it renders an orthographic depth map of the mesh, then BAKE_ACCUM adds, per UV texel, visibility (AO, front-facing texels) or distance to the facing surface (thickness, back-facing texels); BAKE_FINAL divides and derives curvature from neighbouring geometry texels (scale 0.05 times the mesh radius). The result lives in `bake` (AO, curvature, thickness); position and world normal export straight from the geometry maps. Fill generators read `bake` per texel. Filter layers are evaluated inside the composite (mode.z) on the stack below them.

## High-to-low baking (v0.6.0)

`fs_core::bake::bake_high` runs on the CPU in wasm: it rasterizes the low mesh in UV space (position, normal, per-vertex tangent frame from `Mesh::tangents`), casts one ray per texel from the cage inward through the high-poly BVH, writes the high normal in the low tangent frame (B points along increasing V as stored, which is down in texture space) and an ID color per high-poly part, then dilates. The viewport and the normal export combine it with height; the OpenGL export flips the green channel. It blocks the tab for a few seconds at 2048 px; moving it to a Worker is part of the Scale milestone. Smart materials are frontend data (`frontend/src/ui/smart.ts`) applied through the same fill and filter calls; saved ones live in the service's workspace library.

## Texture sets and UDIM (v0.7.0)

`fs_core::sets::split` groups triangles by material and by the UDIM tile of their UV centroid (V is stored flipped, so the row is `floor(1 - v)`), duplicates vertices per set and moves UVs into 0 to 1. Sets are contiguous index ranges. `Gpu` holds the active set's state in its own fields (layers, undo, final maps, coverage, geometry, bakes, base) and parks the others in `parked`; `select_set` swaps them with `mem::swap`, never copies. `draw_mesh` draws the active range, `draw_all` the whole mesh (bake depth, so occlusion crosses sets), and the view draws every range with its own final maps. `Engine::each_set` runs bakes and fill refreshes on every set and composites each before moving on, because only the active set composites per frame. Never switch sets during a stroke.

## Folders (v0.8.0)

`LayerInfo.parent` names the containing folder (0 is the top level). `fs_core::tree_order` keeps the flat list so each folder's contents sit directly below it; every structural change ends in `Engine::normalize`, which also calls `Gpu::reorder`, because `composite` zips infos and GPU layers by index. Folders are passthrough: they are skipped as leaves, and each leaf takes the product of its ancestors' opacity, is skipped when any ancestor is hidden, and when ancestors carry masks gets one mask multiplied in `group_mask` by MASK_MUL (blend src Dst, dst Zero). A folder blend mode is not applied. Folders own a layer set only for their mask.

## Polygon fill, 16-bit export, bake worker (v0.9.0)

`fs_core::regions::select` picks triangles inside the active set's range: Triangle, Polygon (edge neighbours whose face normal is within 0.9995 of the seed's), Mesh (shared welded positions, so UV seams do not split it) and UvChunk (shared vertex indices, which three.js splits at UV seams). `Gpu::mask_triangles` draws them with the COVERAGE shader into the stroke mask, then the normal apply runs, so fills honour stroke opacity, eraser, masks and undo. `export(.., wide)` renders EXPORT into Rgba16Float and reads 8 bytes per texel; the editor turns halves into 16-bit values. `bake_high_detached` takes plain arrays so the module Worker in `frontend/src/engine/bake-worker.ts` can run it; `apply_high_bake(index, ..)` stores the maps in any set without switching to it.

## Next capabilities, in plan order

Quick mask; blur and sharpen filters; isolated folder blending; the whole engine in a Worker with OffscreenCanvas; a shared layer stack across UDIM tiles (Painter's UV tiles workflow); match-by-name and bent normal baking; tile paging for 4K and 8K; color management; EXR and 32-bit export.
