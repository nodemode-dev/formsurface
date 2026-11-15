use fs_core::bake::{bake_high, reproject_map};
use fs_core::{Bvh, Mesh};

const SIZE: usize = 32;

fn high() -> (Mesh, Bvh) {
    let mesh = Mesh::geometry_only(&[-1.0, -1.0, 0.1, 3.0, -1.0, 0.1, -1.0, 3.0, 0.1], &[], &[0, 1, 2]).unwrap();
    let bvh = Bvh::build(&mesh);
    (mesh, bvh)
}

fn low(extra_uv: Option<[f32; 2]>) -> Mesh {
    let mut p = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0];
    let mut uv = vec![0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0];
    let mut i = vec![0, 1, 2, 0, 2, 3];
    if let Some(bad) = extra_uv {
        p.extend([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
        uv.extend([0.0, 0.0, 1.0, 0.0, bad[0], bad[1]]);
        i.extend([4, 5, 6]);
    }
    Mesh::from_flat(&p, &[], &uv, &i).unwrap()
}

#[test]
fn bake_covers_the_whole_map_of_a_clean_quad() {
    let (high, bvh) = high();
    let out = bake_high(&low(None), &high, &bvh, &[0], SIZE, 0.5, 0.5);
    assert_eq!(out.texels, SIZE * SIZE);
    assert_eq!(out.hits, SIZE * SIZE);
}

#[test]
fn a_triangle_with_a_nan_uv_does_not_wipe_the_texels_of_its_neighbours() {
    let (high, bvh) = high();
    let clean = bake_high(&low(None), &high, &bvh, &[0], SIZE, 0.5, 0.5);
    for bad in [[f32::NAN, 1.0], [0.0, f32::NAN], [f32::NAN, f32::NAN]] {
        let out = bake_high(&low(Some(bad)), &high, &bvh, &[0], SIZE, 0.5, 0.5);
        assert_eq!(out.hits, clean.hits, "uv {bad:?}");
        assert_eq!(out.texels, clean.texels, "uv {bad:?}");
        assert_eq!(out.normal, clean.normal, "uv {bad:?}");
    }
}

#[test]
fn a_mesh_whose_only_triangle_has_a_nan_uv_bakes_nothing() {
    let (high, bvh) = high();
    let only = Mesh::from_flat(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0], &[], &[0.0, 0.0, 1.0, 0.0, f32::NAN, 1.0], &[0, 1, 2]).unwrap();
    let out = bake_high(&only, &high, &bvh, &[0], SIZE, 0.5, 0.5);
    assert_eq!(out.texels, 0);
    assert_eq!(out.hits, 0);
}

#[test]
fn reprojection_ignores_a_triangle_with_a_nan_uv() {
    let old = low(None);
    let bvh = Bvh::build(&old);
    let clean = reproject_map(&low(None), &old, &bvh, (0, 2), SIZE, 0.1);
    let dirty = reproject_map(&low(Some([f32::NAN, 1.0])), &old, &bvh, (0, 2), SIZE, 0.1);
    assert_eq!(clean.found, SIZE * SIZE);
    assert_eq!(dirty.found, clean.found);
    assert_eq!(dirty.texels, clean.texels);
}
