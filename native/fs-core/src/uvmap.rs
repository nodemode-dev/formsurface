//! Lookup of the second UV channel at every texel of the first, so passes that run in UV0 texture space
//! (fills, generators) can project through UV1.

/// Rasterizes the triangles `indices` in UV0 space at `size` and writes, per texel, the interpolated UV1 as
/// RGBA floats (u1, v1, 0, 1). Texels outside every triangle stay (0, 0, 0, 0); the islands are then grown by
/// `grow` texels from their nearest covered neighbor so bilinear lookups and seams never read an empty texel.
pub fn uv1_lookup(uv0: &[[f32; 2]], uv1: &[[f32; 2]], indices: &[u32], size: usize, grow: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; size * size * 4];
    let s = size as f32;
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        if a >= uv0.len() || b >= uv0.len() || c >= uv0.len() || a >= uv1.len() || b >= uv1.len() || c >= uv1.len() {
            continue;
        }
        let (ax, ay) = (uv0[a][0] * s, uv0[a][1] * s);
        let (bx, by) = (uv0[b][0] * s, uv0[b][1] * s);
        let (cx, cy) = (uv0[c][0] * s, uv0[c][1] * s);
        let area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
        if area.abs() < 1e-12 {
            continue;
        }
        let x0 = ax.min(bx).min(cx).floor().max(0.0) as usize;
        let y0 = ay.min(by).min(cy).floor().max(0.0) as usize;
        let x1 = (ax.max(bx).max(cx).ceil() as isize).clamp(0, size as isize - 1) as usize;
        let y1 = (ay.max(by).max(cy).ceil() as isize).clamp(0, size as isize - 1) as usize;
        for y in y0..=y1 {
            let py = y as f32 + 0.5;
            for x in x0..=x1 {
                let px = x as f32 + 0.5;
                let w0 = ((bx - px) * (cy - py) - (by - py) * (cx - px)) / area;
                let w1 = ((cx - px) * (ay - py) - (cy - py) * (ax - px)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-5 || w1 < -1e-5 || w2 < -1e-5 {
                    continue;
                }
                let i = (y * size + x) * 4;
                out[i] = w0 * uv1[a][0] + w1 * uv1[b][0] + w2 * uv1[c][0];
                out[i + 1] = w0 * uv1[a][1] + w1 * uv1[b][1] + w2 * uv1[c][1];
                out[i + 3] = 1.0;
            }
        }
    }
    for _ in 0..grow {
        let before = out.clone();
        for y in 0..size {
            for x in 0..size {
                let i = (y * size + x) * 4;
                if before[i + 3] > 0.0 {
                    continue;
                }
                'near: for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= size as i32 || ny >= size as i32 {
                            continue;
                        }
                        let j = (ny as usize * size + nx as usize) * 4;
                        if before[j + 3] > 0.0 {
                            out[i] = before[j];
                            out[i + 1] = before[j + 1];
                            out[i + 3] = 0.5;
                            break 'near;
                        }
                    }
                }
            }
        }
    }
    out
}
