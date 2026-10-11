use crate::{add, cross, normalize, scale, sub, Vec3};

pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
    pub center: Vec3,
    pub radius: f32,
}

impl Mesh {
    pub fn from_flat(positions: &[f32], normals: &[f32], uvs: &[f32], indices: &[u32]) -> Result<Mesh, String> {
        if positions.len() % 3 != 0 || positions.is_empty() {
            return Err("Mesh positions must be a non-empty multiple of 3".into());
        }
        let count = positions.len() / 3;
        if uvs.len() != count * 2 {
            return Err("This mesh has no UV coordinates. FormSurface paints into UV textures, so the mesh needs a UV layout.".into());
        }
        let indices: Vec<u32> = if indices.is_empty() { (0..count as u32).collect() } else { indices.to_vec() };
        if indices.len() % 3 != 0 || indices.iter().any(|&i| i as usize >= count) {
            return Err("Mesh indices are out of range".into());
        }
        let positions: Vec<Vec3> = positions.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();
        let uvs: Vec<[f32; 2]> = uvs.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
        let normals = if normals.len() == count * 3 {
            normals.chunks_exact(3).map(|n| normalize([n[0], n[1], n[2]])).collect()
        } else {
            smooth_normals(&positions, &indices)
        };
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in &positions {
            for a in 0..3 {
                lo[a] = lo[a].min(p[a]);
                hi[a] = hi[a].max(p[a]);
            }
        }
        let center = scale(add(lo, hi), 0.5);
        let radius = positions.iter().map(|p| crate::length(sub(*p, center))).fold(0.0, f32::max).max(1e-6);
        Ok(Mesh { positions, normals, uvs, indices, center, radius })
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn interleaved(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.positions.len() * 8);
        for i in 0..self.positions.len() {
            out.extend_from_slice(&self.positions[i]);
            out.extend_from_slice(&self.normals[i]);
            out.extend_from_slice(&self.uvs[i]);
        }
        out
    }
}

fn smooth_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for t in indices.chunks_exact(3) {
        let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let n = cross(sub(positions[b], positions[a]), sub(positions[c], positions[a]));
        for &i in &[a, b, c] {
            normals[i] = add(normals[i], n);
        }
    }
    normals.into_iter().map(normalize).collect()
}
