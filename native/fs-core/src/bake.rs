use crate::{add, dot, normalize, scale, sub, Bvh, Mesh, Vec3};

pub struct HighBake {
    pub normal: Vec<u8>,
    pub id: Vec<u8>,
    pub height: Vec<u8>,
    pub opacity: Vec<u8>,
    pub ao: Vec<u8>,
    pub bent: Vec<u8>,
    pub hits: usize,
    pub texels: usize,
}

fn hemisphere(n: Vec3, k: usize, count: usize, spin: f32) -> Vec3 {
    let helper = if n[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let t = normalize(crate::cross(helper, n));
    let b = crate::cross(n, t);
    let u = (k as f32 + 0.5) / count as f32;
    let r = u.sqrt();
    let phi = (k as f32 * 2.399_963) + spin;
    let (x, y) = (r * phi.cos(), r * phi.sin());
    normalize(add(add(scale(t, x), scale(b, y)), scale(n, (1.0 - u).max(0.0).sqrt())))
}

fn gray(pixels: &mut [u8], o: usize, v: f32) {
    let g = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    pixels[o..o + 4].copy_from_slice(&[g, g, g, 255]);
}

struct Texel {
    p: Vec3,
    n: Vec3,
    t: Vec3,
    b: Vec3,
    tri: u32,
    w: [f32; 3],
}

pub fn match_key(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    for suffix in ["_low", "_high", "_lo", "_hi"] {
        if let Some(stem) = lower.strip_suffix(suffix) {
            return stem.to_string();
        }
    }
    lower
}

fn rasterize(low: &Mesh, size: usize) -> Vec<Option<Texel>> {
    let (tangents, bitangents) = low.tangents();
    let mut out: Vec<Option<Texel>> = (0..size * size).map(|_| None).collect();
    let fs = size as f32;
    for (tri_index, tri) in low.indices.chunks_exact(3).enumerate() {
        let i = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
        let uv = i.map(|k| [low.uvs[k][0] * fs, low.uvs[k][1] * fs]);
        let min_x = uv.iter().map(|p| p[0]).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
        let max_x = (uv.iter().map(|p| p[0]).fold(f32::MIN, f32::max).ceil() as usize).min(size);
        let min_y = uv.iter().map(|p| p[1]).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
        let max_y = (uv.iter().map(|p| p[1]).fold(f32::MIN, f32::max).ceil() as usize).min(size);
        let area = (uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1]) - (uv[2][0] - uv[0][0]) * (uv[1][1] - uv[0][1]);
        if area.abs() < 1e-12 {
            continue;
        }
        for y in min_y..max_y {
            for x in min_x..max_x {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((uv[1][0] - px) * (uv[2][1] - py) - (uv[2][0] - px) * (uv[1][1] - py)) / area;
                let w1 = ((uv[2][0] - px) * (uv[0][1] - py) - (uv[0][0] - px) * (uv[2][1] - py)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                    continue;
                }
                let blend = |v: &[Vec3]| add(add(scale(v[i[0]], w0), scale(v[i[1]], w1)), scale(v[i[2]], w2));
                let n = normalize(blend(&low.normals));
                let t = blend(&tangents);
                let t = normalize(sub(t, scale(n, dot(t, n))));
                let mut b = blend(&bitangents);
                b = normalize(sub(sub(b, scale(n, dot(b, n))), scale(t, dot(b, t))));
                out[y * size + x] = Some(Texel { p: blend(&low.positions), n, t, b, tri: tri_index as u32, w: [w0, w1, w2] });
            }
        }
    }
    out
}

fn id_color(part: u32) -> [u8; 3] {
    let mut h = part.wrapping_mul(2654435761).wrapping_add(0x9e37);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b3c6d);
    h ^= h >> 12;
    let hue = (h % 360) as f32 / 60.0;
    let x = 1.0 - (hue % 2.0 - 1.0).abs();
    let (r, g, b) = match hue as u32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    [(r * 230.0 + 25.0) as u8, (g * 230.0 + 25.0) as u8, (b * 230.0 + 25.0) as u8]
}

pub fn bake_high(low: &Mesh, high: &Mesh, high_bvh: &Bvh, high_parts: &[u32], size: usize, front: f32, rear: f32) -> HighBake {
    bake_high_matched(low, high, high_bvh, high_parts, size, front, rear, None, 0, None, None)
}

pub struct Reprojection {
    pub map: Vec<f32>,
    pub texels: usize,
    pub found: usize,
}

pub fn reproject_map(new_low: &Mesh, old: &Mesh, old_bvh: &Bvh, old_triangles: (u32, u32), size: usize, reach: f32) -> Reprojection {
    let texels = rasterize(new_low, size);
    let mut map = vec![0.0f32; size * size * 4];
    let (mut count, mut found) = (0usize, 0usize);
    let keep = |t: u32| t >= old_triangles.0 && t < old_triangles.1;
    for (index, texel) in texels.iter().enumerate() {
        let Some(tx) = texel else { continue };
        count += 1;
        let front = old_bvh.raycast_filter(old, add(tx.p, scale(tx.n, reach)), scale(tx.n, -1.0), &keep).filter(|h| h.t <= reach * 2.0);
        let back = old_bvh.raycast_filter(old, sub(tx.p, scale(tx.n, reach)), tx.n, &keep).filter(|h| h.t <= reach * 2.0);
        let best = match (front, back) {
            (Some(a), Some(b)) => Some(if (a.t - reach).abs() <= (b.t - reach).abs() { a } else { b }),
            (a, b) => a.or(b),
        };
        if let Some(hit) = best {
            found += 1;
            let o = index * 4;
            let tile = |x: f32| if x > 1.0 { x - x.floor() } else { x };
            map[o] = tile(hit.uv[0]);
            map[o + 1] = tile(hit.uv[1]);
            map[o + 2] = 1.0;
        }
    }
    Reprojection { map, texels: count, found }
}

fn cage_ray(cage: Option<&[f32]>, tx: &Texel, front: f32) -> (Vec3, Vec3, f32) {
    let fallback = (add(tx.p, scale(tx.n, front)), scale(tx.n, -1.0), front);
    let Some(c) = cage else { return fallback };
    let o = tx.tri as usize * 9;
    if c.len() < o + 9 {
        return fallback;
    }
    let corner = |k: usize| [c[o + k * 3], c[o + k * 3 + 1], c[o + k * 3 + 2]];
    let cp = add(add(scale(corner(0), tx.w[0]), scale(corner(1), tx.w[1])), scale(corner(2), tx.w[2]));
    let v = sub(tx.p, cp);
    let d = dot(v, v).sqrt();
    if d < 1e-6 {
        return fallback;
    }
    (cp, scale(v, 1.0 / d), d)
}

#[allow(clippy::too_many_arguments)]
pub fn bake_high_matched(low: &Mesh, high: &Mesh, high_bvh: &Bvh, high_parts: &[u32], size: usize, front: f32, rear: f32, keys: Option<(&[u32], &[u32])>, ao_rays: usize, cage: Option<&[f32]>, offset: Option<&[f32]>) -> HighBake {
    let base_front = front;
    let base_rear = rear;
    let texels = rasterize(low, size);
    let mut normal = vec![0u8; size * size * 4];
    let mut id = vec![0u8; size * size * 4];
    let mut height = vec![0u8; size * size * 4];
    let mut opacity = vec![0u8; size * size * 4];
    let mut ao = vec![0u8; size * size * 4];
    let mut bent = vec![0u8; size * size * 4];
    let ao_reach = high.radius * 0.5;
    let mut covered = vec![false; size * size];
    let mut hits = 0usize;
    let mut count = 0usize;
    for (index, texel) in texels.iter().enumerate() {
        let Some(tx) = texel else { continue };
        count += 1;
        covered[index] = true;
        let k = offset.and_then(|o| o.get(index)).map(|v| 1.0 + 3.0 * v.clamp(0.0, 1.0)).unwrap_or(1.0);
        let rear = base_rear * k;
        let (origin, dir, front) = cage_ray(cage, tx, base_front * k);
        let reach = (front + rear).max(1e-6);
        let mut ts = [0.0, 0.0, 1.0];
        let mut bent_ts = [0.0, 0.0, 1.0];
        let mut part_color = [128u8, 128, 128];
        let mut height_value = 0.5;
        let mut hit_value = 0.0;
        let mut ao_value = 1.0;
        let found = match keys {
            Some((low_keys, high_keys)) => {
                let want = low_keys.get(tx.tri as usize).copied().unwrap_or(u32::MAX);
                high_bvh.raycast_filter(high, origin, dir, &|t| high_keys.get(t as usize).copied().unwrap_or(u32::MAX) == want)
            }
            None => high_bvh.raycast(high, origin, dir),
        };
        if let Some(hit) = found {
            if hit.t <= reach {
                hits += 1;
                let hn = normalize(hit.normal);
                let hn = if dot(hn, tx.n) < 0.0 { scale(hn, -1.0) } else { hn };
                ts = [dot(hn, tx.t), dot(hn, tx.b), dot(hn, tx.n).max(0.0)];
                let part = high_parts.get(hit.triangle as usize).copied().unwrap_or(0);
                part_color = id_color(part);
                hit_value = 1.0;
                height_value = ((front - hit.t) + rear) / reach;
                if ao_rays > 0 {
                    let at = add(add(origin, scale(dir, hit.t)), scale(hn, ao_reach * 0.002));
                    let spin = (index as f32 * 0.618_034).fract() * std::f32::consts::TAU;
                    let mut open = 0usize;
                    let mut sum = [0.0f32; 3];
                    for k in 0..ao_rays {
                        let d = hemisphere(hn, k, ao_rays, spin);
                        let blocked = high_bvh.raycast(high, at, d).is_some_and(|h| h.t < ao_reach);
                        if !blocked {
                            open += 1;
                            sum = add(sum, d);
                        }
                    }
                    ao_value = open as f32 / ao_rays as f32;
                    let bn = if open > 0 { normalize(sum) } else { hn };
                    bent_ts = [dot(bn, tx.t), dot(bn, tx.b), dot(bn, tx.n).max(0.0)];
                } else {
                    bent_ts = ts;
                }
            }
        }
        let o = index * 4;
        normal[o] = ((ts[0] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
        normal[o + 1] = ((ts[1] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
        normal[o + 2] = ((ts[2] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
        normal[o + 3] = 255;
        id[o..o + 3].copy_from_slice(&part_color);
        id[o + 3] = 255;
        gray(&mut height, o, height_value);
        gray(&mut opacity, o, hit_value);
        gray(&mut ao, o, ao_value);
        for c in 0..3 {
            bent[o + c] = ((bent_ts[c] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
        bent[o + 3] = 255;
    }
    dilate(&mut normal, &mut covered.clone(), size, [128, 128, 255, 255]);
    dilate(&mut height, &mut covered.clone(), size, [128, 128, 128, 255]);
    dilate(&mut opacity, &mut covered.clone(), size, [0, 0, 0, 255]);
    dilate(&mut ao, &mut covered.clone(), size, [255, 255, 255, 255]);
    dilate(&mut bent, &mut covered.clone(), size, [128, 128, 255, 255]);
    dilate(&mut id, &mut covered, size, [128, 128, 128, 255]);
    HighBake { normal, id, height, opacity, ao, bent, hits, texels: count }
}

fn dilate(pixels: &mut [u8], covered: &mut [bool], size: usize, empty: [u8; 4]) {
    for _ in 0..8 {
        let snapshot = covered.to_vec();
        let source = pixels.to_vec();
        for y in 0..size {
            for x in 0..size {
                let i = y * size + x;
                if snapshot[i] {
                    continue;
                }
                let mut found = None;
                for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx < 0 || ny < 0 || nx >= size as i32 || ny >= size as i32 {
                        continue;
                    }
                    let j = ny as usize * size + nx as usize;
                    if snapshot[j] {
                        found = Some(j);
                        break;
                    }
                }
                if let Some(j) = found {
                    pixels[i * 4..i * 4 + 4].copy_from_slice(&source[j * 4..j * 4 + 4]);
                    covered[i] = true;
                }
            }
        }
    }
    for i in 0..size * size {
        if !covered[i] {
            pixels[i * 4..i * 4 + 4].copy_from_slice(&empty);
        }
    }
}
