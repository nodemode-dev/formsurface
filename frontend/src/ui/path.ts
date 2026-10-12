export type Vec3 = [number, number, number];

export function pathSamples(points: Vec3[], closed: boolean, smooth: boolean, perSegment = 48): Vec3[] {
  const n = points.length;
  if (n < 2) return points.slice();
  const at = (i: number): Vec3 => (closed ? points[(i + n) % n] : points[Math.max(0, Math.min(n - 1, i))]);
  const segments = closed ? n : n - 1;
  const out: Vec3[] = [];
  for (let s = 0; s < segments; s++) {
    const [p0, p1, p2, p3] = [at(s - 1), at(s), at(s + 1), at(s + 2)];
    for (let k = 0; k < perSegment; k++) {
      const t = k / perSegment;
      if (!smooth) {
        out.push([0, 1, 2].map((i) => p1[i] + (p2[i] - p1[i]) * t) as Vec3);
        continue;
      }
      const t2 = t * t;
      const t3 = t2 * t;
      out.push([0, 1, 2].map((i) => 0.5 * (2 * p1[i] + (p2[i] - p0[i]) * t + (2 * p0[i] - 5 * p1[i] + 4 * p2[i] - p3[i]) * t2 + (3 * p1[i] - p0[i] - 3 * p2[i] + p3[i]) * t3)) as Vec3);
    }
  }
  out.push(closed ? points[0] : points[n - 1]);
  return out;
}

export function drawPath(overlay: HTMLCanvasElement, curve: ([number, number] | undefined)[], handles: ([number, number] | undefined)[]): void {
  const rect = overlay.getBoundingClientRect();
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const w = Math.max(1, Math.round(rect.width * ratio));
  const h = Math.max(1, Math.round(rect.height * ratio));
  if (overlay.width !== w || overlay.height !== h) {
    overlay.width = w;
    overlay.height = h;
  }
  const ctx = overlay.getContext('2d');
  if (!ctx) return;
  ctx.clearRect(0, 0, w, h);
  if (!handles.length) return;
  ctx.lineWidth = 2 * ratio;
  ctx.strokeStyle = 'rgba(240, 150, 80, 0.9)';
  ctx.beginPath();
  let open = false;
  for (const p of curve) {
    if (!p) { open = false; continue; }
    if (open) ctx.lineTo(p[0], p[1]); else ctx.moveTo(p[0], p[1]);
    open = true;
  }
  ctx.stroke();
  ctx.fillStyle = '#ffffff';
  for (const p of handles) {
    if (!p) continue;
    ctx.beginPath();
    ctx.arc(p[0], p[1], 4 * ratio, 0, Math.PI * 2);
    ctx.fill();
  }
}

export interface Cover { width: number; height: number; rgba: Uint8Array; bounds: [number, number, number, number] }

type Pt = [number, number];

function coverCanvas(points: Pt[], pad: number): { ctx: OffscreenCanvasRenderingContext2D; bounds: [number, number, number, number]; width: number; height: number } {
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  const x0 = Math.floor(Math.min(...xs) - pad);
  const y0 = Math.floor(Math.min(...ys) - pad);
  const width = Math.max(1, Math.min(8192, Math.ceil(Math.max(...xs) + pad) - x0));
  const height = Math.max(1, Math.min(8192, Math.ceil(Math.max(...ys) + pad) - y0));
  const ctx = new OffscreenCanvas(width, height).getContext('2d');
  if (!ctx) throw new Error('No 2D canvas for the path');
  ctx.translate(-x0, -y0);
  return { ctx, bounds: [x0, y0, x0 + width, y0 + height], width, height };
}

function finish(ctx: OffscreenCanvasRenderingContext2D, width: number, height: number, bounds: [number, number, number, number]): Cover {
  const data = ctx.getImageData(0, 0, width, height).data;
  return { width, height, rgba: new Uint8Array(data.buffer, data.byteOffset, data.byteLength), bounds };
}

export function filledCover(points: Pt[]): Cover {
  if (points.length < 3) throw new Error('A filled path needs at least three points');
  const { ctx, bounds, width, height } = coverCanvas(points, 2);
  ctx.fillStyle = '#ffffff';
  ctx.beginPath();
  points.forEach((p, i) => (i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1])));
  ctx.closePath();
  ctx.fill('nonzero');
  return finish(ctx, width, height, bounds);
}

function profile(d: number, hardness: number): number {
  if (d >= 1) return 0;
  if (d <= hardness || hardness >= 1) return 1;
  const t = (1 - d) / (1 - hardness);
  return t * t * (3 - 2 * t);
}

function sampleAlpha(alpha: ImageData, u: number, v: number): number {
  const x = Math.min(alpha.width - 1, Math.max(0, Math.floor(u * alpha.width)));
  const y = Math.min(alpha.height - 1, Math.max(0, Math.floor(v * alpha.height)));
  return alpha.data[(y * alpha.width + x) * 4] / 255;
}

export function ribbonCover(samples: Pt[], width: number, hardness: number, alpha?: ImageData): Cover {
  if (samples.length < 2) throw new Error('A ribbon needs at least two points');
  const half = Math.max(0.5, width * 0.5);
  const xs = samples.map((p) => p[0]);
  const ys = samples.map((p) => p[1]);
  const x0 = Math.floor(Math.min(...xs) - half - 2);
  const y0 = Math.floor(Math.min(...ys) - half - 2);
  const w = Math.max(1, Math.min(8192, Math.ceil(Math.max(...xs) + half + 2) - x0));
  const h = Math.max(1, Math.min(8192, Math.ceil(Math.max(...ys) + half + 2) - y0));
  const cover = new Float32Array(w * h);
  const claimed = new Uint8Array(w * h);
  const repeat = alpha ? width * (alpha.width / alpha.height) : 0;
  const [s0, s1, e0, e1] = [samples[0], samples[1], samples[samples.length - 2], samples[samples.length - 1]];
  const beyondEnds = (x: number, y: number) =>
    (x - s0[0]) * (s1[0] - s0[0]) + (y - s0[1]) * (s1[1] - s0[1]) < 0 || (x - e1[0]) * (e1[0] - e0[0]) + (y - e1[1]) * (e1[1] - e0[1]) > 0;
  for (const joins of [false, true]) {
    let along = 0;
    for (let i = 1; i < samples.length; i++) {
      const [a, b] = [samples[i - 1], samples[i]];
      const dx = b[0] - a[0];
      const dy = b[1] - a[1];
      const length = Math.hypot(dx, dy);
      if (length < 1e-4) continue;
      const [tx, ty] = [dx / length, dy / length];
      const left = Math.max(0, Math.floor(Math.min(a[0], b[0]) - half) - x0);
      const right = Math.min(w - 1, Math.ceil(Math.max(a[0], b[0]) + half) - x0);
      const top = Math.max(0, Math.floor(Math.min(a[1], b[1]) - half) - y0);
      const bottom = Math.min(h - 1, Math.ceil(Math.max(a[1], b[1]) + half) - y0);
      for (let y = top; y <= bottom; y++) {
        for (let x = left; x <= right; x++) {
          const k = y * w + x;
          if (joins && claimed[k]) continue;
          const px = x + x0 + 0.5 - a[0];
          const py = y + y0 + 0.5 - a[1];
          const along_t = px * tx + py * ty;
          const inside = along_t >= 0 && along_t <= length;
          if (!joins && !inside) continue;
          if (alpha && joins && beyondEnds(x + x0 + 0.5, y + y0 + 0.5)) continue;
          const t = Math.max(0, Math.min(length, along_t));
          const side = px * -ty + py * tx;
          const d = Math.hypot(px - tx * t, py - ty * t) / half;
          if (d >= 1) continue;
          if (!joins) claimed[k] = 1;
          let c: number;
          if (alpha) {
            const s = along + t;
            c = sampleAlpha(alpha, (s % repeat) / repeat, 0.5 + 0.5 * Math.max(-1, Math.min(1, side / half)));
          } else c = profile(d, hardness);
          if (c > cover[k]) cover[k] = c;
        }
      }
      along += length;
    }
  }
  const rgba = new Uint8Array(w * h * 4);
  for (let k = 0; k < w * h; k++) {
    const c = Math.round(cover[k] * 255);
    rgba[k * 4] = c;
    rgba[k * 4 + 1] = c;
    rgba[k * 4 + 2] = c;
    rgba[k * 4 + 3] = c;
  }
  return { width: w, height: h, rgba, bounds: [x0, y0, x0 + w, y0 + h] };
}

export function clearOverlay(overlay: HTMLCanvasElement): void {
  overlay.getContext('2d')?.clearRect(0, 0, overlay.width, overlay.height);
}
