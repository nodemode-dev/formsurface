/**
 * Share of a texture set's used UV area that more than one triangle maps to: mirrored halves, stacked copies or
 * overlapping islands. Paint in that area appears on every surface that shares it, so lettering painted on one side
 * of a mirrored model reads backwards on the other. Triangles are tested at texel centers strictly inside their edges,
 * so neighbors that only share an edge are not counted as overlapping.
 */
export function sharedUvShare(uvs: Float32Array, indices: Uint32Array, size = 256): number {
  const count = new Uint8Array(size * size);
  for (let t = 0; t + 2 < indices.length; t += 3) {
    const a = indices[t], b = indices[t + 1], c = indices[t + 2];
    const ax = uvs[a * 2] * size, ay = uvs[a * 2 + 1] * size;
    const bx = uvs[b * 2] * size, by = uvs[b * 2 + 1] * size;
    const cx = uvs[c * 2] * size, cy = uvs[c * 2 + 1] * size;
    const area = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
    if (Math.abs(area) < 1e-9) continue;
    const x0 = Math.max(0, Math.floor(Math.min(ax, bx, cx)));
    const y0 = Math.max(0, Math.floor(Math.min(ay, by, cy)));
    const x1 = Math.min(size - 1, Math.ceil(Math.max(ax, bx, cx)));
    const y1 = Math.min(size - 1, Math.ceil(Math.max(ay, by, cy)));
    for (let y = y0; y <= y1; y++) {
      const py = y + 0.5;
      for (let x = x0; x <= x1; x++) {
        const px = x + 0.5;
        const w0 = ((bx - px) * (cy - py) - (by - py) * (cx - px)) / area;
        const w1 = ((cx - px) * (ay - py) - (cy - py) * (ax - px)) / area;
        const w2 = 1 - w0 - w1;
        if (w0 <= 1e-4 || w1 <= 1e-4 || w2 <= 1e-4) continue;
        const i = y * size + x;
        if (count[i] < 255) count[i]++;
      }
    }
  }
  let used = 0, shared = 0;
  for (let i = 0; i < count.length; i++) {
    if (count[i] > 0) used++;
    if (count[i] > 1) shared++;
  }
  return used ? shared / used : 0;
}
