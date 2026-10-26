mod common;

use common::{cube, quad, seam_strip, unit_quad};
use fs_core::{Bvh, Mesh};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

#[test]
fn mesh_without_uvs_is_refused() {
    let (p, n, _, i) = quad(0.0, 1.0, 0.0, 1.0);
    assert!(Mesh::from_flat(&p, &n, &[], &i).is_err());
}

#[test]
fn mesh_with_out_of_range_index_is_refused() {
    let (p, n, uv, _) = quad(0.0, 1.0, 0.0, 1.0);
    assert!(Mesh::from_flat(&p, &n, &uv, &[0, 1, 9]).is_err());
    assert!(Mesh::from_flat(&p, &n, &uv, &[0, 1]).is_err());
    assert!(Mesh::from_flat(&[], &[], &[], &[]).is_err());
}

#[test]
fn mesh_bounds_of_the_cube() {
    let m = cube();
    assert!(m.center.iter().all(|c| close(*c, 0.0)));
    assert!(close(m.radius, 0.75f32.sqrt()));
    assert_eq!(m.triangle_count(), 12);
}

#[test]
fn missing_normals_are_computed_from_the_faces() {
    let (p, _, uv, i) = quad(0.0, 1.0, 0.0, 1.0);
    let m = Mesh::from_flat(&p, &[], &uv, &i).unwrap();
    assert!(m.normals.iter().all(|n| close(n[0], 0.0) && close(n[1], 0.0) && close(n[2].abs(), 1.0)));
}

#[test]
fn subset_keeps_vertices_and_cuts_triangles() {
    let m = cube().subset(6, 12);
    assert_eq!(m.triangle_count(), 4);
    assert_eq!(m.positions.len(), 24);
}

#[test]
fn tangent_follows_increasing_u() {
    let (t, b) = unit_quad().tangents();
    assert!(t.iter().all(|v| v[0] > 0.0 && close(v[1], 0.0)));
    assert!(b.iter().all(|v| v[1] > 0.0 && close(v[0], 0.0)));
}

#[test]
fn ray_down_the_quad_hits_with_position_and_uv() {
    let m = unit_quad();
    let bvh = Bvh::build(&m);
    let hit = bvh.raycast(&m, [0.25, 0.75, 2.0], [0.0, 0.0, -1.0]).unwrap();
    assert!(close(hit.t, 2.0));
    assert!(close(hit.position[0], 0.25) && close(hit.position[1], 0.75) && close(hit.position[2], 0.0));
    assert!(close(hit.uv[0], 0.25) && close(hit.uv[1], 0.75));
    assert!(close(hit.normal[2], 1.0));
}

#[test]
fn rays_that_miss_or_point_away_hit_nothing() {
    let m = unit_quad();
    let bvh = Bvh::build(&m);
    assert!(bvh.raycast(&m, [1.5, 0.5, 2.0], [0.0, 0.0, -1.0]).is_none());
    assert!(bvh.raycast(&m, [0.5, 0.5, 2.0], [0.0, 0.0, 1.0]).is_none());
}

#[test]
fn nearest_surface_wins() {
    let (mut p, mut n, mut uv, mut i) = quad(0.0, 1.0, 0.0, 1.0);
    let (mut p2, n2, uv2, i2) = quad(0.0, 1.0, 0.0, 1.0);
    for k in (2..p2.len()).step_by(3) {
        p2[k] = 1.0;
    }
    p.extend(p2);
    n.extend(n2);
    uv.extend(uv2);
    i.extend(i2.iter().map(|k| k + 4));
    let m = Mesh::from_flat(&p, &n, &uv, &i).unwrap();
    let bvh = Bvh::build(&m);
    let hit = bvh.raycast(&m, [0.5, 0.5, 3.0], [0.0, 0.0, -1.0]).unwrap();
    assert!(close(hit.position[2], 1.0));
    let skipped = bvh.raycast_filter(&m, [0.5, 0.5, 3.0], [0.0, 0.0, -1.0], &|t| t < 2).unwrap();
    assert!(close(skipped.position[2], 0.0));
}

#[test]
fn every_cube_face_is_hit_from_outside_with_its_own_normal() {
    let m = cube();
    let bvh = Bvh::build(&m);
    for axis in 0..3 {
        for sign in [-1.0f32, 1.0] {
            let mut origin = [0.05f32, 0.1, -0.07];
            origin[axis] = sign * 3.0;
            let mut dir = [0.0f32; 3];
            dir[axis] = -sign;
            let hit = bvh.raycast(&m, origin, dir).unwrap();
            assert!(close(hit.t, 2.5));
            assert!(close(hit.normal[axis], sign));
            assert!(hit.uv.iter().all(|c| (0.0..=1.0).contains(c)));
        }
    }
}

#[test]
fn many_rays_over_a_grid_agree_with_the_surface() {
    let n = 12usize;
    let (mut p, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new());
    for y in 0..=n {
        for x in 0..=n {
            let (fx, fy) = (x as f32 / n as f32, y as f32 / n as f32);
            p.extend_from_slice(&[fx, fy, 0.0]);
            uv.extend_from_slice(&[fx, fy]);
        }
    }
    for y in 0..n {
        for x in 0..n {
            let a = (y * (n + 1) + x) as u32;
            let b = a + 1;
            let c = a + n as u32 + 1;
            idx.extend_from_slice(&[a, b, c + 1, a, c + 1, c]);
        }
    }
    let m = Mesh::from_flat(&p, &[], &uv, &idx).unwrap();
    let bvh = Bvh::build(&m);
    for k in 0..400 {
        let x = 0.003 + (k % 20) as f32 * 0.0489;
        let y = 0.004 + (k / 20) as f32 * 0.0487;
        let hit = bvh.raycast(&m, [x, y, 1.0], [0.0, 0.0, -1.0]).unwrap();
        assert!(close(hit.uv[0], x) && close(hit.uv[1], y), "{x} {y} {:?}", hit.uv);
        assert!(close(hit.position[0], x) && close(hit.position[1], y));
    }
}

#[test]
fn hits_on_both_sides_of_a_uv_seam_land_in_their_own_islands() {
    let m = seam_strip();
    let bvh = Bvh::build(&m);
    let left = bvh.raycast(&m, [0.4999, 0.5, 1.0], [0.0, 0.0, -1.0]).unwrap();
    let right = bvh.raycast(&m, [0.5001, 0.5, 1.0], [0.0, 0.0, -1.0]).unwrap();
    assert!(close(left.position[0], 0.4999) && close(right.position[0], 0.5001));
    assert!(left.uv[0] > 0.39 && left.uv[0] <= 0.4 + 1e-4);
    assert!(right.uv[0] >= 0.6 - 1e-4 && right.uv[0] < 0.61);
    let rim = bvh.raycast(&m, [0.5, 0.5, 1.0], [0.0, 0.0, -1.0]).unwrap();
    assert!(close(rim.position[0], 0.5));
}

#[test]
fn uv_per_world_scales_with_the_island() {
    let m = seam_strip();
    let k = m.uv_per_world(0);
    assert!(close(k, 0.4f32.sqrt() / 0.5f32.sqrt()));
    assert!(close(m.uv_per_world(2), 0.4f32.sqrt() / 0.5f32.sqrt()));
}
