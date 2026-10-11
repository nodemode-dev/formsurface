import type { Pixels } from './export-templates';

export type PaddingMode = 'infinite' | 'transparent' | 'color' | 'diffusion';

export interface Padding {
  mode: PaddingMode;
  width: number;
  color: [number, number, number];
}

export const PADDING_MODES: PaddingMode[] = ['infinite', 'transparent', 'color', 'diffusion'];

export function uvMask(uvs: Float32Array, indices: Uint32Array, size: number): Uint8Array {
  const inside = new Uint8Array(size * size);
  for (let t = 0; t + 2 < indices.length; t += 3) {
    const [a, b, c] = [indices[t], indices[t + 1], indices[t + 2]];
    const ax = uvs[a * 2] * size, ay = uvs[a * 2 + 1] * size;
    const bx = uvs[b * 2] * size, by = uvs[b * 2 + 1] * size;
    const cx = uvs[c * 2] * size, cy = uvs[c * 2 + 1] * size;
    const area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    if (Math.abs(area) < 1e-12) continue;
    const x0 = Math.max(0, Math.floor(Math.min(ax, bx, cx))), x1 = Math.min(size - 1, Math.ceil(Math.max(ax, bx, cx)));
    const y0 = Math.max(0, Math.floor(Math.min(ay, by, cy))), y1 = Math.min(size - 1, Math.ceil(Math.max(ay, by, cy)));
    for (let y = y0; y <= y1; y++) {
      const py = y + 0.5;
      for (let x = x0; x <= x1; x++) {
        const px = x + 0.5;
        const w0 = ((bx - px) * (cy - py) - (by - py) * (cx - px)) / area;
        const w1 = ((cx - px) * (ay - py) - (cy - py) * (ax - px)) / area;
        if (w0 >= -1e-6 && w1 >= -1e-6 && 1 - w0 - w1 >= -1e-6) inside[y * size + x] = 1;
      }
    }
  }
  const mask = new Uint8Array(size * size);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      if (!inside[y * size + x]) continue;
      for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
        const nx = x + dx, ny = y + dy;
        if (nx >= 0 && ny >= 0 && nx < size && ny < size) mask[ny * size + nx] = 1;
      }
    }
  }
  return mask;
}

export function nearestSource(mask: Uint8Array, size: number, limit: number): Int32Array {
  const n = size * size;
  const src = new Int32Array(n).fill(-1);
  const dist = new Int32Array(n);
  const queue = new Int32Array(n);
  let head = 0, tail = 0;
  for (let i = 0; i < n; i++) if (mask[i]) { src[i] = i; queue[tail++] = i; }
  while (head < tail) {
    const i = queue[head++];
    if (dist[i] >= limit) continue;
    const x = i % size, y = (i - x) / size;
    for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
      if (!dx && !dy) continue;
      const nx = x + dx, ny = y + dy;
      if (nx < 0 || ny < 0 || nx >= size || ny >= size) continue;
      const j = ny * size + nx;
      if (src[j] >= 0) continue;
      src[j] = src[i];
      dist[j] = dist[i] + 1;
      queue[tail++] = j;
    }
  }
  return src;
}

function pullPush(px: Pixels, size: number, known: Uint8Array): void {
  const levels: { n: number; v: Float32Array; w: Float32Array }[] = [];
  let n = size;
  const v0 = new Float32Array(size * size * 4);
  const w0 = new Float32Array(size * size);
  for (let i = 0; i < size * size; i++) {
    if (!known[i]) continue;
    w0[i] = 1;
    for (let c = 0; c < 4; c++) v0[i * 4 + c] = px[i * 4 + c];
  }
  levels.push({ n, v: v0, w: w0 });
  while (n > 1) {
    const m = n >> 1;
    const prev = levels[levels.length - 1];
    const v = new Float32Array(m * m * 4);
    const w = new Float32Array(m * m);
    for (let y = 0; y < m; y++) for (let x = 0; x < m; x++) {
      let sw = 0;
      const acc = [0, 0, 0, 0];
      for (let dy = 0; dy < 2; dy++) for (let dx = 0; dx < 2; dx++) {
        const j = (y * 2 + dy) * n + x * 2 + dx;
        const wj = prev.w[j];
        if (!wj) continue;
        sw += wj;
        for (let c = 0; c < 4; c++) acc[c] += prev.v[j * 4 + c] * wj;
      }
      const i = y * m + x;
      if (sw > 0) for (let c = 0; c < 4; c++) v[i * 4 + c] = acc[c] / sw;
      w[i] = Math.min(1, sw);
    }
    levels.push({ n: m, v, w });
    n = m;
  }
  for (let k = levels.length - 2; k >= 0; k--) {
    const fine = levels[k];
    const coarse = levels[k + 1];
    for (let y = 0; y < fine.n; y++) for (let x = 0; x < fine.n; x++) {
      const i = y * fine.n + x;
      const wf = fine.w[i];
      if (wf >= 1) continue;
      const fx = Math.min(coarse.n - 1, Math.max(0, (x + 0.5) / 2 - 0.5));
      const fy = Math.min(coarse.n - 1, Math.max(0, (y + 0.5) / 2 - 0.5));
      const x0 = Math.floor(fx), y0 = Math.floor(fy);
      const x1 = Math.min(coarse.n - 1, x0 + 1), y1 = Math.min(coarse.n - 1, y0 + 1);
      const tx = fx - x0, ty = fy - y0;
      for (let c = 0; c < 4; c++) {
        const s = (xx: number, yy: number) => coarse.v[(yy * coarse.n + xx) * 4 + c];
        const up = (s(x0, y0) * (1 - tx) + s(x1, y0) * tx) * (1 - ty) + (s(x0, y1) * (1 - tx) + s(x1, y1) * tx) * ty;
        fine.v[i * 4 + c] = fine.v[i * 4 + c] * wf + up * (1 - wf);
      }
      fine.w[i] = 1;
    }
  }
  const round = !(px instanceof Float32Array);
  for (let i = 0; i < size * size; i++) {
    if (known[i]) continue;
    for (let c = 0; c < 4; c++) px[i * 4 + c] = round ? Math.round(v0[i * 4 + c]) : v0[i * 4 + c];
  }
}

export function applyPadding(px: Pixels, size: number, src: Int32Array, padding: Padding, max: number): boolean {
  const n = size * size;
  const unresolved = new Uint8Array(n);
  let any = false;
  for (let i = 0; i < n; i++) {
    const s = src[i];
    if (s === i) continue;
    if (s >= 0) {
      for (let c = 0; c < 4; c++) px[i * 4 + c] = px[s * 4 + c];
      continue;
    }
    any = true;
    unresolved[i] = 1;
    if (padding.mode === 'transparent') {
      px[i * 4] = px[i * 4 + 1] = px[i * 4 + 2] = px[i * 4 + 3] = 0;
    } else if (padding.mode === 'color') {
      const round = max === 1 ? (v: number) => v : (v: number) => Math.round(v * max);
      px[i * 4] = round(padding.color[0]);
      px[i * 4 + 1] = round(padding.color[1]);
      px[i * 4 + 2] = round(padding.color[2]);
      px[i * 4 + 3] = max;
    }
  }
  if (any && padding.mode === 'diffusion') {
    const known = new Uint8Array(n);
    for (let i = 0; i < n; i++) known[i] = unresolved[i] ? 0 : 1;
    pullPush(px, size, known);
  }
  return any && padding.mode === 'transparent';
}
