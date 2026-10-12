use crate::mesh::Mesh;
use crate::{cross, dot, normalize, sub, Vec3};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Triangle,
    Polygon,
    Mesh,
    UvChunk,
}

impl Mode {
    pub fn from_code(code: u32) -> Mode {
        match code {
            1 => Mode::Polygon,
            2 => Mode::Mesh,
            3 => Mode::UvChunk,
            _ => Mode::Triangle,
        }
    }
}

pub fn polygon_loop(mesh: &Mesh, keys: &[u64], tris: &[u32]) -> Option<Vec<u32>> {
    let mut edges: HashMap<(u64, u64), u32> = HashMap::new();
    for &t in tris {
        let i = &mesh.indices[t as usize * 3..t as usize * 3 + 3];
        for k in 0..3 {
            edges.insert((keys[i[k] as usize], keys[i[(k + 1) % 3] as usize]), i[k]);
        }
    }
    let mut next: HashMap<u64, (u64, u32)> = HashMap::new();
    for (&(a, b), &v) in &edges {
        if !edges.contains_key(&(b, a)) {
            if next.insert(a, (b, v)).is_some() {
                return None;
            }
        }
    }
    let (&start, _) = next.iter().min_by_key(|(k, _)| **k)?;
    let mut out = Vec::with_capacity(next.len());
    let mut at = start;
    loop {
        let (to, v) = *next.get(&at)?;
        out.push(v);
        at = to;
        if at == start || out.len() > next.len() {
            break;
        }
    }
    (out.len() == next.len() && out.len() >= 3).then_some(out)
}

pub fn welded(mesh: &Mesh) -> Vec<u64> {
    let eps = (mesh.radius * 1e-5).max(1e-9);
    let mut keys: HashMap<(i64, i64, i64), u64> = HashMap::new();
    mesh.positions
        .iter()
        .map(|p| {
            let k = ((p[0] / eps).round() as i64, (p[1] / eps).round() as i64, (p[2] / eps).round() as i64);
            let next = keys.len() as u64;
            *keys.entry(k).or_insert(next)
        })
        .collect()
}

fn face_normal(mesh: &Mesh, t: u32) -> Vec3 {
    let i = &mesh.indices[t as usize * 3..t as usize * 3 + 3];
    let (a, b, c) = (mesh.positions[i[0] as usize], mesh.positions[i[1] as usize], mesh.positions[i[2] as usize]);
    normalize(cross(sub(b, a), sub(c, a)))
}

pub fn select(mesh: &Mesh, seed: u32, first: u32, last: u32, mode: Mode) -> Vec<u32> {
    if mode == Mode::Triangle || seed < first || seed >= last {
        return vec![seed];
    }
    let keys: Vec<u64> = if mode == Mode::UvChunk { (0..mesh.positions.len() as u64).collect() } else { welded(mesh) };
    let key = |t: u32, k: usize| keys[mesh.indices[t as usize * 3 + k] as usize];
    let mut links: HashMap<(u64, u64), Vec<u32>> = HashMap::new();
    for t in first..last {
        for k in 0..3 {
            let (a, b) = (key(t, k), key(t, (k + 1) % 3));
            if mode == Mode::Polygon {
                links.entry((a.min(b), a.max(b))).or_default().push(t);
            } else {
                links.entry((a, a)).or_default().push(t);
            }
        }
    }
    let seed_normal = face_normal(mesh, seed);
    let mut seen = vec![false; (last - first) as usize];
    let mut out = Vec::new();
    let mut stack = vec![seed];
    seen[(seed - first) as usize] = true;
    while let Some(t) = stack.pop() {
        out.push(t);
        for k in 0..3 {
            let (a, b) = (key(t, k), key(t, (k + 1) % 3));
            let link = if mode == Mode::Polygon { (a.min(b), a.max(b)) } else { (a, a) };
            for &n in links.get(&link).map(|v| v.as_slice()).unwrap_or(&[]) {
                let slot = (n - first) as usize;
                if seen[slot] {
                    continue;
                }
                if mode == Mode::Polygon && dot(face_normal(mesh, n), seed_normal) < 0.9995 {
                    continue;
                }
                seen[slot] = true;
                stack.push(n);
            }
        }
    }
    out
}
