use crate::{add, cross, dot, normalize, scale, sub, Mesh, Vec3};

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub t: f32,
    pub position: Vec3,
    pub normal: Vec3,
    pub triangle: u32,
    pub uv: [f32; 2],
}

struct Node {
    lo: Vec3,
    hi: Vec3,
    start: u32,
    count: u32,
    right: u32,
}

pub struct Bvh {
    nodes: Vec<Node>,
    order: Vec<u32>,
}

const LEAF: usize = 4;

impl Bvh {
    pub fn build(mesh: &Mesh) -> Bvh {
        let tris = mesh.triangle_count();
        let mut centroids = Vec::with_capacity(tris);
        let mut bounds = Vec::with_capacity(tris);
        for t in 0..tris {
            let [a, b, c] = corners(mesh, t);
            let lo = [a[0].min(b[0]).min(c[0]), a[1].min(b[1]).min(c[1]), a[2].min(b[2]).min(c[2])];
            let hi = [a[0].max(b[0]).max(c[0]), a[1].max(b[1]).max(c[1]), a[2].max(b[2]).max(c[2])];
            bounds.push((lo, hi));
            centroids.push(scale(add(lo, hi), 0.5));
        }
        let mut bvh = Bvh { nodes: Vec::with_capacity(tris * 2 / LEAF + 1), order: (0..tris as u32).collect() };
        if tris > 0 {
            bvh.split(0, tris, &centroids, &bounds);
        }
        bvh
    }

    fn split(&mut self, start: usize, end: usize, centroids: &[Vec3], bounds: &[(Vec3, Vec3)]) -> u32 {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut clo = [f32::MAX; 3];
        let mut chi = [f32::MIN; 3];
        for &t in &self.order[start..end] {
            let (blo, bhi) = bounds[t as usize];
            let c = centroids[t as usize];
            for a in 0..3 {
                lo[a] = lo[a].min(blo[a]);
                hi[a] = hi[a].max(bhi[a]);
                clo[a] = clo[a].min(c[a]);
                chi[a] = chi[a].max(c[a]);
            }
        }
        let index = self.nodes.len() as u32;
        self.nodes.push(Node { lo, hi, start: start as u32, count: (end - start) as u32, right: 0 });
        if end - start <= LEAF {
            return index;
        }
        let ext = sub(chi, clo);
        let axis = if ext[0] >= ext[1] && ext[0] >= ext[2] { 0 } else if ext[1] >= ext[2] { 1 } else { 2 };
        let mid = (start + end) / 2;
        self.order[start..end].select_nth_unstable_by(mid - start, |a, b| {
            centroids[*a as usize][axis].total_cmp(&centroids[*b as usize][axis])
        });
        self.split(start, mid, centroids, bounds);
        let right = self.split(mid, end, centroids, bounds);
        let node = &mut self.nodes[index as usize];
        node.count = 0;
        node.right = right;
        index
    }

    pub fn raycast(&self, mesh: &Mesh, origin: Vec3, direction: Vec3) -> Option<Hit> {
        self.raycast_filter(mesh, origin, direction, &|_| true)
    }

    pub fn raycast_filter(&self, mesh: &Mesh, origin: Vec3, direction: Vec3, accept: &dyn Fn(u32) -> bool) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = [1.0 / direction[0], 1.0 / direction[1], 1.0 / direction[2]];
        let mut best: Option<(f32, u32, f32, f32)> = None;
        let mut stack = vec![0u32];
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i as usize];
            let limit = best.map(|b| b.0).unwrap_or(f32::MAX);
            if !slab(node.lo, node.hi, origin, inv, limit) {
                continue;
            }
            if node.count > 0 {
                for &t in &self.order[node.start as usize..(node.start + node.count) as usize] {
                    if !accept(t) {
                        continue;
                    }
                    if let Some((d, u, v)) = intersect(corners(mesh, t as usize), origin, direction) {
                        if best.map(|b| d < b.0).unwrap_or(true) {
                            best = Some((d, t, u, v));
                        }
                    }
                }
            } else {
                stack.push(node.right);
                stack.push(i + 1);
            }
        }
        best.map(|(t, tri, u, v)| {
            let idx = &mesh.indices[tri as usize * 3..tri as usize * 3 + 3];
            let w = 1.0 - u - v;
            let blend3 = |f: &dyn Fn(usize) -> Vec3| add(add(scale(f(idx[0] as usize), w), scale(f(idx[1] as usize), u)), scale(f(idx[2] as usize), v));
            let normal = normalize(blend3(&|k| mesh.normals[k]));
            let uv0 = mesh.uvs[idx[0] as usize];
            let uv1 = mesh.uvs[idx[1] as usize];
            let uv2 = mesh.uvs[idx[2] as usize];
            Hit {
                t,
                position: add(origin, scale(direction, t)),
                normal,
                triangle: tri,
                uv: [uv0[0] * w + uv1[0] * u + uv2[0] * v, uv0[1] * w + uv1[1] * u + uv2[1] * v],
            }
        })
    }
}

fn corners(mesh: &Mesh, t: usize) -> [Vec3; 3] {
    let i = &mesh.indices[t * 3..t * 3 + 3];
    [mesh.positions[i[0] as usize], mesh.positions[i[1] as usize], mesh.positions[i[2] as usize]]
}

fn slab(lo: Vec3, hi: Vec3, origin: Vec3, inv: Vec3, limit: f32) -> bool {
    let mut tmin = 0.0f32;
    let mut tmax = limit;
    for a in 0..3 {
        let t1 = (lo[a] - origin[a]) * inv[a];
        let t2 = (hi[a] - origin[a]) * inv[a];
        tmin = tmin.max(t1.min(t2));
        tmax = tmax.min(t1.max(t2));
    }
    tmin <= tmax
}

fn intersect(tri: [Vec3; 3], origin: Vec3, direction: Vec3) -> Option<(f32, f32, f32)> {
    let e1 = sub(tri[1], tri[0]);
    let e2 = sub(tri[2], tri[0]);
    let p = cross(direction, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(origin, tri[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(direction, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv;
    if t > 1e-6 { Some((t, u, v)) } else { None }
}
