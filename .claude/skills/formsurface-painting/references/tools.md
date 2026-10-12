# FormSurface MCP tools (server 0.38.0)

When this table and the server disagree, the server wins and this file is stale.

| Tool | Parameters | Returns |
| --- | --- | --- |
| `formsurface_status` | none | mesh, texture size, texture sets, active texture set, layers of the active set, active layer, brush, paint material, camera, viewport |
| `formsurface_load_sample` | kind: sphere, torus, capsule, knot, sets (figure with body on UDIM 1001 and 1002 plus trim) | status |
| `formsurface_list_texture_sets` | none | texture sets (index, name, material, udim, tiled, triangles, layers) and the active index |
| `formsurface_select_texture_set` | name (for example body.1002, trim) or index, size (256 to 4096: that set's own resolution; layers resampled, undo cleared, bake again), extra_channel (off, user, opacity, ambient_occlusion, emissive, transmission, scattering), extra_name (user channel name), user_channels (up to two more channel names for slots 2 and 3; an empty list removes them, and turning a slot off hides its paint until it is turned on again) | the set, its size, its extra channel, its user channels, its layers and active layer |
| `formsurface_set_camera` | yaw_degrees, pitch_degrees, zoom (1 frames the mesh) | camera |
| `formsurface_set_brush` | size_px 1 to 400, hardness, flow, stroke_opacity, spacing, eraser, mirror_x, backface_culling, alignment (camera, tangent_wrap, tangent_planar, uv), size_space (screen, object), angle_degrees, follow_path, tip (round, square, spray), spray_density, lazy_px, jitter_size, jitter_angle, jitter_position, jitter_flow, symmetry (off, mirror, radial), symmetry_axis (x, y, z), radial_count | brush |
| `formsurface_set_paint_material` | base_color hex, roughness, metallic, height, paint_color, paint_roughness, paint_metallic, paint_height, extra, paint_extra (the set's extra channel), extra2, paint_extra2, extra3, paint_extra3 (user channels 2 and 3) | paint material |
| `formsurface_set_base_material` | base_color hex, roughness, metallic | empty |
| `formsurface_add_layer` | name | new id and layers |
| `formsurface_add_fill_layer` | name, projection (uv, triplanar, planar, spherical, cylindrical), pattern (solid, checker, stripes, noise), base_color, roughness, metallic, height, scale, rotation_degrees, planar_axis, sharpness, fill_color, fill_roughness, fill_metallic, fill_height | id and settings |
| `formsurface_set_fill` | layer_id, projection (uv, triplanar, planar, spherical, cylindrical, warp, tile_match), pattern, base_color, roughness, metallic, height, scale, rotation_degrees, offset_u, offset_v, planar_axis, sharpness, amount, warp_points (16 [x, y] grid points, rows top first, rest at i/3, j/3), reset_warp, tile_images (workspace images named with their UDIM, one per tile; UV tiles on), extra, use_extra (the set's extra channel), extra2, use_extra2, extra3, use_extra3 (user channels 2 and 3) | settings, plus tiles and skipped files for tile_images |
| `formsurface_bake_maps` | directions 8 to 256, method (depth, traced: AO and thickness ray traced on the GPU, slower), rays 4 to 128 | baked flag, method, seconds for traced, map names |
| `formsurface_bake_high_poly` | mesh_file (in workspace/meshes), use_sample, sample (sphere, pair), cage_front, cage_rear (fractions of mesh radius), match_by_name, ao_rays (0 to 64, default 16; 0 skips AO and bent normal), cage_file (custom cage in workspace/meshes: the low mesh with vertices moved), distance_cage (drop the custom cage) | hit count, map names (normal, ID, height, opacity, AO, bent normal), matched part count |
| `formsurface_view_2d` | on, channel (base_color, roughness, metallic, height, extra, user2, user3), center_u, center_v, zoom (frame part of the layout) | view (2d or 3d); in 2D, paint_stroke, paint_path and polygon_fill points land on the UV layout |
| `formsurface_paint_path` | points (2 to 64 [x, y] in 0 to 1, kept on the surface), closed, smooth, erase, mode (paint stamps the brush; ribbon lays one strip as wide as the brush, cross-section from hardness or the image tip's alpha repeated along the curve; filled fills the inside, 3 or more points) | control points, missed points, samples, mode; one undo step; ribbon and filled reach only the surface visible from the current view |
| `formsurface_text_resource` | text, target (alpha, stencil, fill), font (sans-serif, serif, monospace or a loaded font), bold | image size, brush |
| `formsurface_load_mesh` | file (glb, gltf, obj, fbx, usd, usda, usdc, usdz, ply, dae in workspace/meshes), reproject (carry every matched set's painting over by surface position; stays on until false) | status; bakes reset, sets keep their layers when material and tile match (reprojected when asked, undo cleared) |
| `formsurface_instance_layer` | layer_id (any layer or folder on the active set, not itself an instance copy), texture_set (name or index) | new layer id in the target set; settings edits to any copy update all copies, layers added to a source folder follow; strokes stay on the source and painting a copy is refused |
| `formsurface_list_smart_materials` | none | materials with layer names and whether they have a thumbnail, smart mask names, saved smart mask names |
| `formsurface_apply_smart_material` | name | new layer ids, baked flag |
| `formsurface_save_smart_material` | name | saved file |
| `formsurface_apply_smart_mask` | layer_id, name (Edge wear, Cavity dirt, Dust from top, Thin parts, Protect edges, Grunge dirt, Metal edge wear, Sun bleached or a saved name) | applied_as (fill generator or layer mask) and settings |
| `formsurface_save_smart_mask` | layer_id, name | file and the saved generator settings |
| `formsurface_set_generator` | layer_id, generator (none, cavity, edges, thin, top, light, position, noise, mask_builder, dirt, metal_edge_wear), level, softness, invert, noise, noise_scale, direction [x, y, z], builder_ao, builder_curvature, builder_noise | fill settings |
| `formsurface_set_environment` | file (.hdr in workspace/environments), studio, rotation_degrees, blur, background, exposure, view_transform (aces, standard, agx, filmic) | environment, rotation, blur, background, exposure, view transform |
| `formsurface_path_trace` | on, bounces (1 to 12), max_samples (64, 256, 1024, 4096), samples (render that many before returning), denoise, aperture (0 to 1), focus (-1 to 1 mesh radii), scatter_radius (0.005 to 0.5 mesh radii) | on, denoise, aperture, focus, rendering, bounces, samples, max_samples, texture_side |
| `formsurface_path_render` | width, height (16 to 3840), samples (1 to 4096), name | folder, file, samples, width, height |
| `formsurface_add_filter_layer` | name, brightness, contrast, hue_degrees, saturation, gamma, roughness_offset, metallic_offset, blur, sharpen | id and settings |
| `formsurface_layer_mask` | layer_id, action (add_white, add_black, remove, keep), paint_target (layer, mask), effects (list replacing the mask's effect stack: kind fill, generator, levels, invert, blur, compare; blend, opacity, visible and the kind's values) | layers, whether strokes paint the mask, and the effect stack |
| `formsurface_update_layer` | layer_id, name, visible, opacity, blend (normal, multiply, screen, overlay, add, subtract, inverse_subtract, darken, lighten, disable, replace, divide, inverse_divide, difference, exclusion, signed_addition, linear_burn, color_burn, color_dodge, soft_light, hard_light, vivid_light, linear_light, pin_light, tint, saturation, color, value, normal_map_combine, normal_map_detail, normal_map_inverse_detail; the normal map modes combine height), make_active | layers and active layer |
| `formsurface_move_layer` | layer_id, offset (positive is up) | layers |
| `formsurface_remove_layer` | layer_id | layers |
| `formsurface_fill_layer` | none | filled layer id |
| `formsurface_paint_stroke` | points: list of [x, y] in 0 to 1, pressure | points, points_on_surface, layer |
| `formsurface_undo` | steps 1 to 24 | undone count |
| `formsurface_redo` | steps 1 to 24 | redone count |
| `formsurface_screenshot` | none | PNG image of the viewport (post effects included) |
| `formsurface_load_image` | file (png, jpg or webp in workspace/images), target (fill, alpha, stencil, mask), layer_id, color_space (srgb, linear, raw) | the resource with its index, and layers |
| `formsurface_set_color_space` | resource (name or index), color_space (srgb, linear, raw, or ocio: and a color space of the active OCIO config); no arguments lists resources | the resource, re-applied in its new color space |
| `formsurface_texture_memory` | page_out_idle, auto (page out idle sets after every switch) | freed_bytes, auto, sets with resident_bytes and paged_bytes |
| `formsurface_color_management` | config (built-in OCIO config name or .ocio file in workspace/ocio, empty for none), display, view, look, working_space (linear_rec709, acescg); no arguments lists configs | config, display, view, look, viewport_space, lut_size, displays, looks, working_space |
| `formsurface_set_post_effects` | reset, bloom, bloom_threshold, glare, lens_flare, dof_aperture (px, 0 off), dof_focus (-1 front to 1 back of the model), aberration, sharpen, vignette, vignette_softness, grain, gain, contrast, saturation, temperature, tint | current post settings; viewport and screenshots only, exports never change |
| `formsurface_add_folder` | name, layer_ids to move inside | folder id, moved ids, layers |
| `formsurface_set_layer_folder` | layer_id, folder_id (0 for top level) | layers |
| `formsurface_list_export_templates` | none | templates with their maps and packed sources |
| `formsurface_clone_source` | x, y in 0 to 1 on the active set | clone tool on; the next stroke fixes the offset |
| `formsurface_clone_layer` | layer_id (0 for everything visible) | what the clone tool copies |
| `formsurface_flatten_stack` | name | new layer id, layers (destructive, not undoable) |
| `formsurface_merge_down` | layer_id (paint or fill, with a paint or fill layer below in the same folder) | new layer id, layers (destructive, not undoable) |
| `formsurface_flatten_folder` | folder_id (paint and fill children only) | new layer id, layers (destructive, not undoable) |
| `formsurface_export_layer` | layer_id, name | folder with layer_ID_basecolor, roughness, metallic and height PNGs, coverage in alpha |
| `formsurface_export_layer_mask` | layer_id, name | folder |
| `formsurface_export_mesh` | name, ngons (keep OBJ or PLY quads and n-gons, default on), subdivide (0 to 3 levels), displacement (0 to 0.2 of the radius, by the height channel) | folder (OBJ with UDIM UVs, one group per texture set) |
| `formsurface_uv_tiles` | on (omit to read) | uv_tiles and texture sets with linked flags |
| `formsurface_mask_source` | layer_id, source (cavity, edges, thin, top, id_color), color hex or x, y point for id_color, tolerance, level, softness, invert | layers, picked color |
| `formsurface_quick_mask` | mode (off, edit, on), action (clear, allow_all, invert) | mode |
| `formsurface_set_filter` | layer_id, brightness, contrast, hue_degrees, saturation, gamma, roughness_offset, metallic_offset, blur, sharpen, levels_in_low, levels_in_high, levels_out_low, levels_out_high, invert, posterize (0 to 32), pixelate (0 to 64) (only given values change) | settings |
| `formsurface_polygon_fill` | x, y in 0 to 1, mode (triangle, polygon, mesh, uv_chunk) | filled triangle count, mode, layer |
| `formsurface_export_maps` | name, template (pbr, gltf, unreal, unity_hdrp, unity_urp, unity_standard_specular, godot, blender, arnold, redshift, usd, sketchfab, spec_gloss or a saved name), custom_maps (suffix plus rgb (basecolor, normal_gl, normal_dx, diffuse, specular) or r, g, b, a sources, which include extra, user2 and user3), format (png, tif, exr, hdr, jpg, psd), bit_depth (8, 16, 32 as the format allows), padding (infinite, transparent, color, diffusion), padding_width, padding_color | folder, template, format, bit depth, padding and file names; every texture set is exported, as name_material_suffix (plus .udim for tiled materials) when there is more than one |
| `formsurface_save_export_template` | name, maps (same shape as custom_maps), label | file; the template joins the editor's list and export_maps |

The same tools run from a JSON script with `./formsurface batch script.json` (a list of `[tool, arguments]` pairs; `--pause`, `--keep-going`, `--out`). Suggest it when the person wants the same job repeated on many meshes.

Errors arrive as text after "Error executing tool name:" and name the fix: no editor linked (turn on Agent link), service not running (run ./formsurface), or a timeout.
