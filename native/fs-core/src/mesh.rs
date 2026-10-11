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

    pub fn geometry_only(positions: &[f32], normals: &[f32], indices: &[u32]) -> Result<Mesh, String> {
        let count = positions.len() / 3;
        Mesh::from_flat(positions, normals, &vec![0.0; count * 2], indices)
    }

    pub fn tangents(&self) -> (Vec<Vec3>, Vec<Vec3>) {
        let mut t = vec![[0.0f32; 3]; self.positions.len()];
        let mut b = vec![[0.0f32; 3]; self.positions.len()];
        for tri in self.indices.chunks_exact(3) {
            let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
            let e1 = sub(self.positions[i1], self.positions[i0]);
            let e2 = sub(self.positions[i2], self.positions[i0]);
            let (du1, dv1) = (self.uvs[i1][0] - self.uvs[i0][0], self.uvs[i1][1] - self.uvs[i0][1]);
            let (du2, dv2) = (self.uvs[i2][0] - self.uvs[i0][0], self.uvs[i2][1] - self.uvs[i0][1]);
            let det = du1 * dv2 - du2 * dv1;
            if det.abs() < 1e-12 {
                continue;
            }
            let r = 1.0 / det;
            let tt = scale(sub(scale(e1, dv2), scale(e2, dv1)), r);
            let bb = scale(sub(scale(e2, du1), scale(e1, du2)), r);
            for &i in &[i0, i1, i2] {
                t[i] = add(t[i], tt);
                b[i] = add(b[i], bb);
            }
        }
        (t, b)
    }

    pub fn uv_per_world(&self, triangle: u32) -> f32 {
        let i = &self.indices[triangle as usize * 3..triangle as usize * 3 + 3];
        let (a, b, c) = (i[0] as usize, i[1] as usize, i[2] as usize);
        let world = crate::length(cross(sub(self.positions[b], self.positions[a]), sub(self.positions[c], self.positions[a])));
        let (ua, ub, uc) = (self.uvs[a], self.uvs[b], self.uvs[c]);
        let uv = ((ub[0] - ua[0]) * (uc[1] - ua[1]) - (uc[0] - ua[0]) * (ub[1] - ua[1])).abs();
        if world > 1e-12 && uv > 1e-12 { (uv / world).sqrt() } else { 1.0 / (self.radius * 2.0) }
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
