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

## Next capabilities, in plan order

High-to-low baking with a cage (tangent-space normal, ID) by ray casting a high-poly BVH; blur and sharpen filters and folders; smart material and mask presets; engine in a Worker; tile paging for 4K and 8K; texture sets per material.
