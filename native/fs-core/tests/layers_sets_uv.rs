mod common;

use common::{cube, quad};
use fs_core::document::tree_order;
use fs_core::regions::{select, Mode};
use fs_core::sets::{split, udim_of};
use fs_core::uvmap::uv1_lookup;
use fs_core::{ancestors, descendants, BlendMode, LayerInfo, LayerKind};

fn layer(id: u32, parent: u32, kind: LayerKind) -> LayerInfo {
    LayerInfo { id, name: format!("l{id}"), visible: true, opacity: 1.0, blend: BlendMode::Normal, kind, mask: false, parent, instance: false }
}

#[test]
fn blend_mode_codes_follow_the_shader_switch_order() {
    assert_eq!(BlendMode::ALL.len(), 31);
    for (i, mode) in BlendMode::ALL.iter().enumerate() {
        assert_eq!(mode.code() as usize, i);
    }
    assert_eq!(BlendMode::Normal.code(), 0);
    assert_eq!(BlendMode::Multiply.code(), 1);
    assert_eq!(BlendMode::Disable.code(), 8);
    assert_eq!(BlendMode::Replace.code(), 9);
    assert_eq!(BlendMode::SoftLight.code(), 18);
    assert_eq!(BlendMode::Tint.code(), 23);
    assert_eq!(BlendMode::NormalMapCombine.code(), 27);
    assert_eq!(BlendMode::InverseSubtract.code(), 30);
}

#[test]
fn blend_modes_round_trip_through_their_saved_names() {
    for mode in BlendMode::ALL {
        let text = serde_json::to_string(&mode).unwrap();
        assert_eq!(serde_json::from_str::<BlendMode>(&text).unwrap(), mode);
    }
    assert_eq!(serde_json::to_string(&BlendMode::ColorDodge).unwrap(), "\"color_dodge\"");
}

#[test]
fn folder_children_composite_before_their_folder() {
    let layers = vec![layer(1, 0, LayerKind::Paint), layer(2, 0, LayerKind::Folder), layer(3, 2, LayerKind::Paint), layer(4, 2, LayerKind::Paint)];
    let order: Vec<u32> = tree_order(&layers).iter().map(|l| l.id).collect();
    assert_eq!(order, vec![1, 3, 4, 2]);
}

#[test]
fn broken_parents_do_not_lose_or_loop_layers() {
    let layers = vec![layer(1, 99, LayerKind::Paint), layer(2, 2, LayerKind::Folder), layer(3, 1, LayerKind::Paint)];
    let order = tree_order(&layers);
    let mut ids: Vec<u32> = order.iter().map(|l| l.id).collect();
    ids.sort();
    assert_eq!(ids, vec![1, 2, 3]);
    assert!(order.iter().all(|l| l.parent == 0));
}

#[test]
fn ancestors_and_descendants_walk_nested_folders() {
    let layers = vec![layer(1, 0, LayerKind::Folder), layer(2, 1, LayerKind::Folder), layer(3, 2, LayerKind::Paint), layer(4, 0, LayerKind::Paint)];
    assert_eq!(ancestors(&layers, 3), vec![1, 0]);
    let mut d = descendants(&layers, 1);
    d.sort();
    assert_eq!(d, vec![2, 3]);
    assert!(descendants(&layers, 4).is_empty());
}

#[test]
fn udim_tiles_are_numbered_from_the_uv_cell() {
    assert_eq!(udim_of(0.5, 0.5), (0, 0));
    assert_eq!(udim_of(1.5, 0.5), (1, 0));
    assert_eq!(udim_of(0.5, -0.5), (0, 1));
}

#[test]
fn split_groups_triangles_by_material_and_tile_and_moves_uvs_home() {
    let (mut p, mut n, mut uv, mut i) = quad(0.0, 1.0, 0.0, 1.0);
    let (p2, n2, uv2, i2) = quad(1.0, 2.0, 1.0, 2.0);
    p.extend(p2);
    n.extend(n2);
    uv.extend(uv2);
    i.extend(i2.iter().map(|k| k + 4));
    let names = vec!["body".to_string()];
    let out = split(&p, &n, &uv, &i, &[0, 0, 0, 0], &names).unwrap();
    assert_eq!(out.sets.len(), 2);
    assert_eq!(out.sets[0].name(), "body.1001");
    assert_eq!(out.sets[1].name(), "body.1002");
    assert_eq!(out.sets[0].count + out.sets[1].count, 12);
    assert!(out.uvs.iter().all(|c| (0.0..=1.0).contains(c)));
    assert_eq!(out.order.len(), 4);
}

#[test]
fn split_keeps_one_untiled_set_per_plain_material() {
    let out = split(&cube_positions(), &[], &cube_uvs(), &cube_indices(), &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1], &["a".into(), "b".into()]).unwrap();
    let names: Vec<String> = out.sets.iter().map(|s| s.name()).collect();
    assert_eq!(names, vec!["a", "b"]);
    assert_eq!(out.sets[0].count, 30);
    assert_eq!(out.sets[1].count, 6);
}

#[test]
fn split_refuses_meshes_without_uvs_or_with_bad_indices() {
    let (p, _, uv, i) = quad(0.0, 1.0, 0.0, 1.0);
    assert!(split(&p, &[], &[], &i, &[], &[]).is_err());
    assert!(split(&p, &[], &uv, &[0, 1, 7], &[], &[]).is_err());
}

fn cube_positions() -> Vec<f32> {
    cube().positions.iter().flatten().copied().collect()
}

fn cube_uvs() -> Vec<f32> {
    cube().uvs.iter().flatten().copied().collect()
}

fn cube_indices() -> Vec<u32> {
    cube().indices
}

fn texel(out: &[f32], size: usize, x: usize, y: usize) -> &[f32] {
    &out[(y * size + x) * 4..(y * size + x) * 4 + 4]
}

fn island(x0: f32, x1: f32) -> (Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<u32>) {
    let uv0 = vec![[x0, 0.0], [x1, 0.0], [x1, 1.0], [x0, 1.0]];
    let uv1 = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    (uv0, uv1, vec![0, 1, 2, 0, 2, 3])
}

#[test]
fn uv1_lookup_interpolates_inside_the_island_and_leaves_the_outside_empty() {
    let (uv0, uv1, idx) = island(0.0, 0.5);
    let out = uv1_lookup(&uv0, &uv1, &idx, 16, 0);
    let inside = texel(&out, 16, 3, 8);
    assert_eq!(inside[3], 1.0);
    assert!((inside[0] - (3.5 / 8.0)).abs() < 1e-4);
    assert!((inside[1] - (8.5 / 16.0)).abs() < 1e-4);
    assert_eq!(texel(&out, 16, 12, 8), &[0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn growing_closes_the_gap_between_two_uv_islands() {
    let (mut uv0, mut uv1, mut idx) = island(0.0, 0.4375);
    let (b0, b1, bi) = island(0.5625, 1.0);
    uv0.extend(b0);
    uv1.extend(b1);
    idx.extend(bi.iter().map(|k| k + 4));
    let bare = uv1_lookup(&uv0, &uv1, &idx, 16, 0);
    assert_eq!(texel(&bare, 16, 7, 8)[3], 0.0);
    assert_eq!(texel(&bare, 16, 8, 8)[3], 0.0);
    let grown = uv1_lookup(&uv0, &uv1, &idx, 16, 1);
    for x in [7usize, 8] {
        assert!(texel(&grown, 16, x, 8)[3] > 0.0, "texel {x} still empty");
    }
}

#[test]
fn polygon_selection_takes_a_cube_face_and_mesh_selection_the_whole_cube() {
    let m = cube();
    let face = select(&m, 0, 0, 12, Mode::Polygon);
    let mut sorted = face.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1]);
    assert_eq!(select(&m, 0, 0, 12, Mode::Mesh).len(), 12);
    assert_eq!(select(&m, 3, 0, 12, Mode::Triangle), vec![3]);
}

#[test]
fn selection_outside_the_range_is_only_the_seed() {
    let m = cube();
    assert_eq!(select(&m, 20, 0, 12, Mode::Mesh), vec![20]);
}
