export interface TextImage {
  w: number;
  h: number;
  data: Uint8Array;
}

const loadedFonts = new Map<string, string>();

export async function loadFont(file: File): Promise<string> {
  const family = `FS ${file.name.replace(/\.[^.]+$/, '').replace(/[^A-Za-z0-9 _-]/g, '').slice(0, 40) || 'font'}`;
  const face = new FontFace(family, await file.arrayBuffer());
  await face.load();
  document.fonts.add(face);
  loadedFonts.set(family, file.name);
  return family;
}

function cssFamily(font: string): string {
  return ['sans-serif', 'serif', 'monospace'].includes(font) ? font : `"${font}"`;
}

export function textImage(text: string, font: string, bold: boolean, mode: 'alpha' | 'stencil' | 'fill'): TextImage {
  const value = text.trim() || ' ';
  const px = 160;
  const fontSpec = `${bold ? '700 ' : ''}${px}px ${cssFamily(font)}`;
  const probe = new OffscreenCanvas(8, 8).getContext('2d')!;
  probe.font = fontSpec;
  const metrics = probe.measureText(value);
  const ascent = metrics.actualBoundingBoxAscent || px * 0.8;
  const descent = metrics.actualBoundingBoxDescent || px * 0.2;
  const textW = Math.ceil(Math.max(metrics.width, metrics.actualBoundingBoxLeft + metrics.actualBoundingBoxRight));
  const textH = Math.ceil(ascent + descent);
  const pad = Math.round(px * 0.15);
  let w = Math.min(2048, textW + pad * 2);
  let h = Math.min(2048, textH + pad * 2);
  if (mode === 'alpha') w = h = 512;
  if (mode === 'fill') h = w;
  const surface = new OffscreenCanvas(w, h);
  const ctx = surface.getContext('2d')!;
  if (mode !== 'fill') {
    ctx.fillStyle = '#000000';
    ctx.fillRect(0, 0, w, h);
  }
  const fit = Math.min(1, (w - pad * 2) / Math.max(1, textW), (h - pad * 2) / Math.max(1, textH));
  ctx.translate(w / 2, h / 2);
  ctx.scale(fit, fit);
  ctx.font = fontSpec;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'alphabetic';
  ctx.fillStyle = '#ffffff';
  ctx.fillText(value, 0, (ascent - descent) / 2);
  const pixels = ctx.getImageData(0, 0, w, h).data;
  if (mode !== 'fill') {
    for (let i = 0; i < pixels.length; i += 4) {
      const l = Math.round(pixels[i] * 0.2126 + pixels[i + 1] * 0.7152 + pixels[i + 2] * 0.0722);
      pixels[i] = pixels[i + 1] = pixels[i + 2] = l;
      pixels[i + 3] = 255;
    }
  }
  return { w, h, data: new Uint8Array(pixels.buffer) };
}

export function fontLabel(family: string): string {
  return loadedFonts.get(family) ?? family;
}
