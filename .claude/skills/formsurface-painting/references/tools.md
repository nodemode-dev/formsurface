# FormSurface MCP tools (server 0.8.0)

When this table and the server disagree, the server wins and this file is stale.

| Tool | Parameters | Returns |
| --- | --- | --- |
| `formsurface_status` | none | mesh, texture size, texture sets, active texture set, layers of the active set, active layer, brush, paint material, camera, viewport |
| `formsurface_load_sample` | kind: sphere, torus, capsule, knot, sets (figure with body on UDIM 1001 and 1002 plus trim) | status |
| `formsurface_list_texture_sets` | none | texture sets (index, name, material, udim, tiled, triangles, layers) and the active index |
| `formsurface_select_texture_set` | name (for example body.1002, trim) or index | the set, its layers and active layer |
| `formsurface_set_camera` | yaw_degrees, pitch_degrees, zoom (1 frames the mesh) | camera |
| `formsurface_set_brush` | size_px 1 to 400, hardness, flow, stroke_opacity, spacing, eraser, mirror_x, backface_culling, alignment (camera, tangent_wrap, tangent_planar, uv), size_space (screen, object), angle_degrees, follow_path, tip (round, square, spray), spray_density, lazy_px, jitter_size, jitter_angle, jitter_position, jitter_flow, symmetry (off, mirror, radial), symmetry_axis (x, y, z), radial_count | brush |
| `formsurface_set_paint_material` | base_color hex, roughness, metallic, height, paint_color, paint_roughness, paint_metallic, paint_height | paint material |
| `formsurface_set_base_material` | base_color hex, roughness, metallic | empty |
| `formsurface_add_layer` | name | new id and layers |
| `formsurface_add_fill_layer` | name, projection (uv, triplanar, planar, spherical, cylindrical), pattern (solid, checker, stripes, noise), base_color, roughness, metallic, height, scale, rotation_degrees, planar_axis, sharpness, fill_color, fill_roughness, fill_metallic, fill_height | id and settings |
| `formsurface_set_fill` | layer_id, projection, pattern, base_color, roughness, metallic, height, scale, rotation_degrees, offset_u, offset_v, planar_axis, sharpness, amount | settings |
| `formsurface_bake_maps` | directions 8 to 256 | baked flag and map names |
| `formsurface_bake_high_poly` | mesh_file (in workspace/meshes), use_sample, cage_front, cage_rear (fractions of mesh radius) | hit count and map names |
| `formsurface_list_smart_materials` | none | materials with layer names, smart mask names |
| `formsurface_apply_smart_material` | name | new layer ids, baked flag |
| `formsurface_save_smart_material` | name | saved file |
| `formsurface_apply_smart_mask` | layer_id, name (Edge wear, Cavity dirt, Dust from top, Thin parts, Protect edges) | fill settings |
| `formsurface_set_generator` | layer_id, generator (none, cavity, edges, thin, top), level, softness, invert | fill settings |
| `formsurface_add_filter_layer` | name, brightness, contrast, hue_degrees, saturation, gamma, roughness_offset, metallic_offset | id and settings |
| `formsurface_layer_mask` | layer_id, action (add_white, add_black, remove, keep), paint_target (layer, mask) | layers and whether strokes paint the mask |
| `formsurface_update_layer` | layer_id, name, visible, opacity, blend (normal, multiply, screen, overlay, add, subtract, darken, lighten), make_active | layers and active layer |
| `formsurface_move_layer` | layer_id, offset (positive is up) | layers |
| `formsurface_remove_layer` | layer_id | layers |
| `formsurface_fill_layer` | none | filled layer id |
| `formsurface_paint_stroke` | points: list of [x, y] in 0 to 1, pressure | points, points_on_surface, layer |
| `formsurface_undo` | steps 1 to 24 | undone count |
| `formsurface_redo` | steps 1 to 24 | redone count |
| `formsurface_screenshot` | none | PNG image of the viewport |
| `formsurface_add_folder` | name, layer_ids to move inside | folder id, moved ids, layers |
| `formsurface_set_layer_folder` | layer_id, folder_id (0 for top level) | layers |
| `formsurface_list_export_templates` | none | templates with their maps and packed sources |
| `formsurface_export_maps` | name, template (pbr, gltf, unreal, unity_hdrp, unity_urp), custom_maps (suffix plus rgb or r, g, b, a sources) | folder, template and map names; every texture set is exported, as name_material_suffix (plus .udim for tiled materials) when there is more than one |

Errors arrive as text naming the fix: no editor linked (turn on Agent link), service not running (run ./formsurface), or a timeout.
