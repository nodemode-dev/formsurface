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

## Next capabilities, in plan order

Alignment modes (camera, tangent wrap, tangent planar, UV), size space choices, angle, jitters, lazy mouse, alphas and stencils, radial symmetry, engine in a Worker, tile paging for 4K and 8K, texture sets per material.
