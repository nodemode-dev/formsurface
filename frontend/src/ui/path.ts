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

export function clearOverlay(overlay: HTMLCanvasElement): void {
  overlay.getContext('2d')?.clearRect(0, 0, overlay.width, overlay.height);
}
