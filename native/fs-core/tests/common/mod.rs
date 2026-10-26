#![allow(dead_code)]
use fs_core::Mesh;

pub fn quad(x0: f32, x1: f32, u0: f32, u1: f32) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<u32>) {
    let positions = vec![x0, 0.0, 0.0, x1, 0.0, 0.0, x1, 1.0, 0.0, x0, 1.0, 0.0];
    let normals = vec![0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
    let uvs = vec![u0, 0.0, u1, 0.0, u1, 1.0, u0, 1.0];
    (positions, normals, uvs, vec![0, 1, 2, 0, 2, 3])
}

pub fn unit_quad() -> Mesh {
    let (p, n, uv, i) = quad(0.0, 1.0, 0.0, 1.0);
    Mesh::from_flat(&p, &n, &uv, &i).unwrap()
}

pub fn seam_strip() -> Mesh {
    let (mut p, mut n, mut uv, mut i) = quad(0.0, 0.5, 0.0, 0.4);
    let (p2, n2, uv2, i2) = quad(0.5, 1.0, 0.6, 1.0);
    p.extend(p2);
    n.extend(n2);
    uv.extend(uv2);
    i.extend(i2.iter().map(|k| k + 4));
    Mesh::from_flat(&p, &n, &uv, &i).unwrap()
}

pub fn cube() -> Mesh {
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
    ];
    let (mut p, mut n, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (f, (normal, a, b)) in faces.iter().enumerate() {
        for (su, sv) in [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            for k in 0..3 {
                p.push(normal[k] * 0.5 + a[k] * su * 0.5 + b[k] * sv * 0.5);
                n.push(normal[k]);
            }
            uv.push(su * 0.5 + 0.5);
            uv.push(sv * 0.5 + 0.5);
        }
        let o = f as u32 * 4;
        idx.extend_from_slice(&[o, o + 1, o + 2, o, o + 2, o + 3]);
    }
    Mesh::from_flat(&p, &n, &uv, &idx).unwrap()
}
