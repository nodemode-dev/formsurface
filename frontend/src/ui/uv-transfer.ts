import { emptyPixels, type Pixels } from './export-templates';

/**
 * Moves a square RGBA map painted in the first UV layout onto the second: every texel inside a triangle's UV1 footprint
 * takes the bilinear sample at the same surface point in UV0. Texels of the one-texel seam ring that uvMask adds take the
 * nearest written neighbor, so padding afterwards starts from a closed island edge.
 * Tangent-space normal maps keep UV0's tangent frame, so they are only right where UV1 islands are not rotated or mirrored.
 */
export function transferToUv1(px: Pixels, size: number, uv0: Float32Array, uv1: Float32Array, indices: Uint32Array, max: number, cover1: Uint8Array): Pixels {
  const out = emptyPixels(max, px.length);
  const written = new Uint8Array(size * size);
  const sample = (u: number, v: number, at: number) => {
    const fx = Math.min(Math.max(u * size - 0.5, 0), size - 1), fy = Math.min(Math.max(v * size - 0.5, 0), size - 1);
    const x0 = Math.floor(fx), y0 = Math.floor(fy), x1 = Math.min(x0 + 1, size - 1), y1 = Math.min(y0 + 1, size - 1);
    const tx = fx - x0, ty = fy - y0;
    const a = (y0 * size + x0) * 4, b = (y0 * size + x1) * 4, c = (y1 * size + x0) * 4, d = (y1 * size + x1) * 4;
    for (let k = 0; k < 4; k++) {
      const top = px[a + k] + (px[b + k] - px[a + k]) * tx;
      const bottom = px[c + k] + (px[d + k] - px[c + k]) * tx;
      const value = top + (bottom - top) * ty;
      out[at + k] = max === 255 || max === 65535 ? Math.round(value) : value;
    }
  };
  for (let t = 0; t + 2 < indices.length; t += 3) {
    const [a, b, c] = [indices[t], indices[t + 1], indices[t + 2]];
    const ax = uv1[a * 2] * size, ay = uv1[a * 2 + 1] * size;
    const bx = uv1[b * 2] * size, by = uv1[b * 2 + 1] * size;
    const cx = uv1[c * 2] * size, cy = uv1[c * 2 + 1] * size;
    const area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    if (Math.abs(area) < 1e-12) continue;
    const x0 = Math.max(0, Math.floor(Math.min(ax, bx, cx))), x1 = Math.min(size - 1, Math.ceil(Math.max(ax, bx, cx)));
    const y0 = Math.max(0, Math.floor(Math.min(ay, by, cy))), y1 = Math.min(size - 1, Math.ceil(Math.max(ay, by, cy)));
    for (let y = y0; y <= y1; y++) {
      const py = y + 0.5;
      for (let x = x0; x <= x1; x++) {
        const qx = x + 0.5;
        const w0 = ((bx - qx) * (cy - py) - (by - py) * (cx - qx)) / area;
        const w1 = ((cx - qx) * (ay - py) - (cy - py) * (ax - qx)) / area;
        const w2 = 1 - w0 - w1;
        if (w0 < -1e-6 || w1 < -1e-6 || w2 < -1e-6) continue;
        const u = w0 * uv0[a * 2] + w1 * uv0[b * 2] + w2 * uv0[c * 2];
        const v = w0 * uv0[a * 2 + 1] + w1 * uv0[b * 2 + 1] + w2 * uv0[c * 2 + 1];
        sample(u, v, (y * size + x) * 4);
        written[y * size + x] = 1;
      }
    }
  }
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = y * size + x;
      if (written[i] || !cover1[i]) continue;
      for (let dy = -1; dy <= 1 && !written[i]; dy++) for (let dx = -1; dx <= 1; dx++) {
        const nx = x + dx, ny = y + dy;
        if (nx < 0 || ny < 0 || nx >= size || ny >= size || written[ny * size + nx] !== 1) continue;
        for (let k = 0; k < 4; k++) out[i * 4 + k] = out[(ny * size + nx) * 4 + k];
        written[i] = 2;
        break;
      }
    }
  }
  return out;
}
