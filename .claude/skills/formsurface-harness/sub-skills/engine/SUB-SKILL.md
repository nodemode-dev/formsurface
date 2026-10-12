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

## Quick mask and spatial filters (v0.10.0)

`quick` is one more per-set texture (swapped with the set, cleared black on mesh upload). `quick_mode` is editor state: 1 makes `begin_stroke` copy `quick` into `scratch.mask` and route the stroke through APPLY_MASK into `quick` with value 1 (eraser 0), and `end_stroke` returns the snapshot to the pool instead of the undo stack; 2 makes `paint` multiply the stroke mask by `quick` into `group_mask` before applying, so the accumulated stroke mask itself is never changed. The view samples `quick` for the active set (white for parked sets) and tints by `1 - quick` when the camera's params.z is set. Filter blur and sharpen live in COMPOSITE's filter branch; they read `uv_cover` (binding 7) to weight taps, and their radius rides in `opacity.z`, sharpen in `opacity.w`.

## UV tiles and mask sources (v0.11.0)

With `uv_tiles` on, tiled sets of one material are siblings. A public engine call that changes the stack starts with `relay`: if not already relaying and siblings exist, it sets `relaying`, runs the call on the active set, then switches to each sibling, runs it again and composites it, then switches back. Calls that create layers use `relay_new` so every tile gets the same id. Strokes are not relayed; `stroke_begin`, `flush` and `stroke_end` walk `each_sibling` themselves because pending stamps and stroke state on the Engine are shared. Each set therefore owns its stroke mask and `stroke` snapshot (both in SetState). Any new public method that changes layers must add its relay line, and any new undoable operation must push an entry on every sibling (see `polygon_fill`) or linked undo drifts. Mask generators live in `mask_gens` per set, render through the FILL shader into `scratch` and copy its coverage R (roughness coverage, forced on) into the layer mask; `render_fills` re-renders them after bakes and mesh loads.

## Mesh export, flatten, clone (v0.12.0)

`mesh_obj` restores each set's UDIM offset and flips V back to the importer's convention, so an exported OBJ re-imports into the same texture sets. `flatten_stack` composites, writes `final_set` into a new top layer at full coverage through PAIR_TO_SET and removes every other layer; it is relayed like other stack changes and is not undoable. Clone is `StrokeMode::Clone`: `push_stamp` fixes `gpu.clone_offset` in texels from the first stamp after a source is set, and APPLY reads `final_set` at that offset (color.w and values.w carry it, mode.z enables it). It reads the live composite, so a stroke that overlaps its own source repeats what it just painted.

## Match by name and instancing (v0.13.0)

`set_match_names` takes the low-poly part index per triangle and both name lists; `match_key` strips _low/_high/_lo/_hi (case-insensitive) and the keys go to `bake_high_matched`, where `Bvh::raycast_filter` only accepts high-poly triangles whose key matches the texel's low part (unmatched low parts hit only unmatched high parts). Sets map back to whole-mesh triangles through `SplitMesh.order`, so keys must be re-sliced per set (`texture_set_match_keys`). Instancing keeps `instance_groups`: lists of (set index, layer id). `propagate(id, work)` runs `work` on every other member after switching sets, and is skipped while `instancing` or `relaying` so it never recurses. Only public stack calls (`set_fill`, `set_filter`, `mask_generator`, `update_layer`) propagate; a new layer property that should follow instances needs its own propagate line. `set_mesh` clears the groups because set indices change.

## 2D view and paths (v0.14.0)

`flat: Option<[cx, cy, px_per_uv]>` turns the viewport into the active set's UV layout: `flat_matrix` maps (u, v, 0, 1) to clip space, the VIEW shader switches on `params.w` (written in `encode_view` from `gpu.flat`) and places vertices at their UV, shading base color only, and `encode_view` skips parked sets. Strokes in 2D push UV stamps (`push_uv_stamp`): `normal.w` 2 marks them, the PAINT shader then skips visibility, facing and the stencil and measures the tip in UV units (`uv.z`); they do not relay to sibling tiles. Image alphas and planar image fills sample with V flipped because their local Y points up. `surface_point` and `screen_point` let the editor keep path control points in world space; the editor paces path samples over frames because a burst of stamps can stall a software GPU.

## Environment and generators (v0.16.0)

`Gpu::set_environment` box-shrinks the equirect HDR to at most 2048 wide, then 128 by 64 and 32 by 16 with wrap-around blur, as Rgba16Float. `ENV_COMMON` (Env struct, `env_uv`, `env_sample(dir, level 0 to 2)`) is prepended to VIEW and BACKGROUND, which declare their own bindings. VIEW turns the two direct lights off when `env.a.x` is set and uses the small level for diffuse and the level by roughness for reflections. BACKGROUND uses the shared FULLSCREEN vertex stage (non-mesh pipelines get it prepended, so never declare `vs` there) and writes `frag_depth` 0.99999 so it only fills pixels the mesh left at the cleared depth; it is drawn after the mesh. `Engine::push_env_params` fills `gpu.env_params` (on, rotation, blur, background, camera basis, tan of half fov, exposure) before every render and snapshot. The fill uniform is 36 floats: `builder` (AO, curvature, noise weights, noise scale) and `direction` (unit xyz, noise breakup); generator kinds 5 to 10 are light, position, noise, mask builder, dirt and metal edge wear, using `fbm3` on mesh-local position.

## Merge, layer export, clone from a layer (v0.17.0)

`Gpu::merge_layers(top, bottom, target, ...)` renders MERGE into a pooled set, then copies it into the target, so the target may also be the bottom (folder flatten accumulates into the new layer this way). Per channel: coverage is top plus bottom times one minus top, after each layer's opacity and mask; color uses the top blend mode only where the bottom has coverage, so a non-normal top over empty bottom becomes normal paint. Engine `merge_down` and `flatten_folder` relay to linked tiles with `relay_new`; folders with nested folders or filters are refused. `read_rgba` takes keep_alpha: viewport snapshots force 255, layer export keeps coverage. APPLY binding 7 is the clone coverage; with `paint.mode.w` set it unpremultiplies a layer's color and values and scales the stroke by that layer's coverage; when the source is the active layer it reads the stroke snapshot. `uv_triangle` lets polygon fill work from a UV point in 2D; `flat_channel` rides in the camera's cursor_normal.w.

## Blend modes, filters and high-poly maps (v0.18.0)

`BlendMode::ALL` order is the shader code (`code()` is the index), so a new mode goes at the end of the enum and of ALL, with a matching case in `blend_any` inside `shaders::BLEND`, which COMPOSITE and MERGE both prepend. `blend_any(m, a, b, values)` runs on color (values false) and on roughness, metallic, height (values true); tint, saturation, color and value pass values through, and the normal map modes keep color and combine height (z). The layer uniform is 24 floats: `levels` (in low, in high, out low, out high) and `style` (posterize steps, pixelate block in texels, invert). Pixelate reads the stack below at block centres before blur; levels, invert and posterize act on base color in sRGB before brightness. `bake_high_matched(..., ao_rays)` adds height ((front minus t plus rear) over reach), opacity, and with rays above 0 AO and bent normal from cosine rays at each high-poly hit; every map is dilated. `bake_high_detached` returns them after hits and texels; the worker transfers them as `BakeResult`.

## Post effects and custom cages (v0.19.0)

When `post_params[19]` is set (any effect away from its default, computed in `push_env_params`), `encode_view` draws the scene into `scene` (viewport size, surface format) instead of the target, BLOOM writes a quarter-size thresholded blur into `bloom`, and POST composes into the target, reading `depth` for depth of field (linear depth from the same near and far the frame used, carried in `post.e`). With every effect off the old direct path runs. Both textures are recreated in `resize`. Effects act on the tone-mapped image, so bloom thresholds are display values. A custom cage is stored as 9 floats per split triangle (corners in split order via `tri_order`), sliced per set like match keys (`texture_set_cage`); `cage_ray` casts from the cage point to the low point and falls back to the distance cage where they coincide; reach becomes that distance plus rear.

## Color space (v0.20.0)

Every color in the stack is linear and every export encodes with gamma 2.2, so anything decoded on the way in must use gamma 2.2 too. Fill images are `Rgba8Unorm` and FILL's `pattern` decodes them with `pow(rgb, 2.2)`; an `Rgba8UnormSrgb` texture would use the exact sRGB curve and shift the darks (8/255 came back as 0.065). Per-resource color space lives in the editor: it converts the bytes before upload (linear-marked colors are encoded, srgb-marked data is decoded) and re-uploads on change.

## Reprojection and view transforms (v0.21.0)

`set_mesh` with `reproject` on takes the old mesh and BVH before they are replaced, rasterizes the new first set (`Mesh::subset`) and, per texel, casts along the normal from both sides within 5 percent of the radius onto the old first set's triangles (`bake::reproject_map`), giving an Rgba32Float map of old UV plus a found flag. `Gpu::reproject_layers` resamples every layer's color, values, coverage and mask through that map (REPROJECT, four targets, textureLoad on the map) into a pooled set and copies back; texels not found get no coverage and a white mask. It clears undo and redo because old snapshots are in the old UV layout. The view transform is `env.look.x` (the Env uniform is 80 bytes now): `view_transform` in ENV_COMMON serves VIEW and BACKGROUND; 0 is the old ACES fit, so the default picture is unchanged.

## Ray-traced bake and carried sets (v0.22.0)

`Bvh::gpu_buffers` writes nodes as three vec4 per node (lo plus leaf start, hi plus count, miss link plus node total) and triangles in BVH order. TRACE reads nodes as an array of unsigned integer vec4 and bitcasts the coordinates: integers stored as float bits are denormals that some GPUs flush to zero. Traversal is stackless (descend to i+1 on a hit, jump to the miss link otherwise) with a guard of twice the node count. A full trace in one go hung Chrome's GPU process on lavapipe (its watchdog), so `bake_traced_begin` plus `bake_traced_step(rows)` trace a few rows per call and the editor calls one step per animation frame; keep that pacing. `upload_mesh` takes a carry map: each new set gets the old `SetState` chosen by the engine (material and tile name; set 0 from set 0), with range updated and bake flags cleared, and the engine moves the matching `Parked` data across.

## Next capabilities, in plan order

Per-tile resolution; isolated folder blending; the whole engine in a Worker with OffscreenCanvas (also what a headless batch needs); tile paging for 4K and 8K; color management; stacked mask effects; warp and stylization filters.
