import { encodePng, type Pixels } from './export-templates';

export type Format = 'png' | 'tif' | 'exr' | 'hdr' | 'jpg' | 'psd';

export const FORMAT_DEPTHS: Record<Format, number[]> = { png: [8, 16], tif: [8, 16, 32], exr: [16, 32], hdr: [32], jpg: [8], psd: [8, 16, 32] };
export const FORMAT_ALPHA: Record<Format, boolean> = { png: true, tif: true, exr: true, hdr: false, jpg: false, psd: true };
const MIME: Record<Format, string> = { png: 'image/png', tif: 'image/tiff', exr: 'image/x-exr', hdr: 'image/vnd.radiance', jpg: 'image/jpeg', psd: 'image/vnd.adobe.photoshop' };

export function isFormat(value: string): value is Format {
  return value in FORMAT_DEPTHS;
}

export function checkFormat(format: string, depth: number): Format {
  if (!isFormat(format)) throw new Error(`format is one of ${Object.keys(FORMAT_DEPTHS).join(', ')}`);
  if (!FORMAT_DEPTHS[format].includes(depth)) throw new Error(`${format.toUpperCase()} is written at ${FORMAT_DEPTHS[format].join(' or ')} bit`);
  return format;
}

export function sampleMax(format: Format, depth: number): number {
  if (depth === 8) return 255;
  if (format === 'exr' || format === 'hdr' || depth === 32) return 1;
  return 65535;
}

export function gammaToLinear(px: Float32Array): void {
  for (let i = 0; i < px.length; i += 4) {
    for (let c = 0; c < 3; c++) px[i + c] = Math.pow(Math.max(0, px[i + c]), 2.2);
  }
}

const f32 = new Float32Array(1);
const u32 = new Uint32Array(f32.buffer);

function toHalf(value: number): number {
  f32[0] = value;
  const x = u32[0];
  const sign = (x >>> 16) & 0x8000;
  const exp = (x >>> 23) & 0xff;
  let mant = x & 0x7fffff;
  if (exp === 0xff) return sign | 0x7c00 | (mant ? 0x200 : 0);
  let e = exp - 127 + 15;
  if (e >= 31) return sign | 0x7c00;
  if (e <= 0) {
    if (e < -10) return sign;
    mant |= 0x800000;
    const shift = 14 - e;
    let half = mant >> shift;
    if ((mant >> (shift - 1)) & 1) half += 1;
    return sign | half;
  }
  let half = sign | (e << 10) | (mant >> 13);
  if (mant & 0x1000) half += 1;
  return half;
}

class Writer {
  bytes: Uint8Array<ArrayBuffer>;
  view: DataView;
  at = 0;
  constructor(size: number, readonly little: boolean) {
    this.bytes = new Uint8Array(size);
    this.view = new DataView(this.bytes.buffer);
  }
  u8(v: number) { this.view.setUint8(this.at, v); this.at += 1; }
  u16(v: number) { this.view.setUint16(this.at, v, this.little); this.at += 2; }
  i32(v: number) { this.view.setInt32(this.at, v, this.little); this.at += 4; }
  u32(v: number) { this.view.setUint32(this.at, v, this.little); this.at += 4; }
  f32(v: number) { this.view.setFloat32(this.at, v, this.little); this.at += 4; }
  u64(v: number) { this.view.setBigUint64(this.at, BigInt(v), this.little); this.at += 8; }
  text(s: string) { for (let i = 0; i < s.length; i++) this.u8(s.charCodeAt(i)); }
}

function bitsOf(px: Pixels): number {
  return px instanceof Uint8Array ? 8 : px instanceof Uint16Array ? 16 : 32;
}

function sample(w: Writer, px: Pixels, i: number): void {
  if (px instanceof Uint8Array) w.u8(px[i]);
  else if (px instanceof Uint16Array) w.u16(px[i]);
  else w.f32(px[i]);
}

function encodeTiff(px: Pixels, width: number, height: number, alpha: boolean): Uint8Array<ArrayBuffer> {
  const spp = alpha ? 4 : 3;
  const bits = bitsOf(px);
  const bytes = width * height * spp * (bits / 8);
  const tags: [number, number, number, number[]][] = [
    [256, 4, 1, [width]], [257, 4, 1, [height]], [258, 3, spp, new Array(spp).fill(bits)], [259, 3, 1, [1]], [262, 3, 1, [2]],
    [273, 4, 1, [0]], [277, 3, 1, [spp]], [278, 4, 1, [height]], [279, 4, 1, [bytes]], [284, 3, 1, [1]],
    ...(alpha ? [[338, 3, 1, [2]] as [number, number, number, number[]]] : []),
    [339, 3, spp, new Array(spp).fill(bits === 32 ? 3 : 1)],
  ];
  const ifdSize = 2 + tags.length * 12 + 4;
  let extra = 8 + ifdSize;
  const extraAt = new Map<number, number>();
  for (const [tag, , count] of tags) if (count * 2 > 4) { extraAt.set(tag, extra); extra += count * 2; }
  const dataAt = extra;
  const w = new Writer(dataAt + bytes, true);
  w.text('II'); w.u16(42); w.u32(8);
  w.u16(tags.length);
  for (const [tag, type, count, values] of tags) {
    w.u16(tag); w.u16(type); w.u32(count);
    if (extraAt.has(tag)) { w.u32(extraAt.get(tag)!); continue; }
    const value = tag === 273 ? dataAt : values[0];
    if (type === 3) { w.u16(value); w.u16(0); } else w.u32(value);
  }
  w.u32(0);
  for (const [tag, , , values] of tags) if (extraAt.has(tag)) for (const v of values) w.u16(v);
  for (let p = 0; p < width * height; p++) for (let c = 0; c < spp; c++) sample(w, px, p * 4 + c);
  return w.bytes;
}

function encodeExr(px: Float32Array, width: number, height: number, alpha: boolean, half: boolean): Uint8Array<ArrayBuffer> {
  const names = alpha ? ['A', 'B', 'G', 'R'] : ['B', 'G', 'R'];
  const offsetOf: Record<string, number> = { R: 0, G: 1, B: 2, A: 3 };
  const sampleBytes = half ? 2 : 4;
  const lineBytes = width * names.length * sampleBytes;
  const attrs: [string, string, (w: Writer) => void, number][] = [
    ['channels', 'chlist', (w) => { for (const n of names) { w.text(n); w.u8(0); w.i32(half ? 1 : 2); w.u8(0); w.u8(0); w.u8(0); w.u8(0); w.i32(1); w.i32(1); } w.u8(0); }, names.length * 18 + 1],
    ['compression', 'compression', (w) => w.u8(0), 1],
    ['dataWindow', 'box2i', (w) => { w.i32(0); w.i32(0); w.i32(width - 1); w.i32(height - 1); }, 16],
    ['displayWindow', 'box2i', (w) => { w.i32(0); w.i32(0); w.i32(width - 1); w.i32(height - 1); }, 16],
    ['lineOrder', 'lineOrder', (w) => w.u8(0), 1],
    ['pixelAspectRatio', 'float', (w) => w.f32(1), 4],
    ['screenWindowCenter', 'v2f', (w) => { w.f32(0); w.f32(0); }, 8],
    ['screenWindowWidth', 'float', (w) => w.f32(1), 4],
  ];
  const headerSize = 8 + attrs.reduce((sum, [name, type, , size]) => sum + name.length + 1 + type.length + 1 + 4 + size, 0) + 1;
  const tableSize = height * 8;
  const total = headerSize + tableSize + height * (8 + lineBytes);
  const w = new Writer(total, true);
  w.u8(0x76); w.u8(0x2f); w.u8(0x31); w.u8(0x01); w.i32(2);
  for (const [name, type, write, size] of attrs) { w.text(name); w.u8(0); w.text(type); w.u8(0); w.i32(size); write(w); }
  w.u8(0);
  for (let y = 0; y < height; y++) w.u64(headerSize + tableSize + y * (8 + lineBytes));
  for (let y = 0; y < height; y++) {
    w.i32(y); w.i32(lineBytes);
    for (const n of names) {
      const c = offsetOf[n];
      for (let x = 0; x < width; x++) {
        const v = px[(y * width + x) * 4 + c];
        if (half) w.u16(toHalf(v)); else w.f32(v);
      }
    }
  }
  return w.bytes;
}

function encodeHdr(px: Float32Array, width: number, height: number): Uint8Array<ArrayBuffer> {
  const header = `#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y ${height} +X ${width}\n`;
  const w = new Writer(header.length + width * height * 4, true);
  w.text(header);
  for (let p = 0; p < width * height; p++) {
    const r = Math.max(0, px[p * 4]);
    const g = Math.max(0, px[p * 4 + 1]);
    const b = Math.max(0, px[p * 4 + 2]);
    const m = Math.max(r, g, b);
    if (m < 1e-32) { w.u32(0); continue; }
    const e = Math.floor(Math.log2(m)) + 1;
    const scale = 256 / 2 ** e;
    w.u8(Math.min(255, Math.floor(r * scale))); w.u8(Math.min(255, Math.floor(g * scale))); w.u8(Math.min(255, Math.floor(b * scale))); w.u8(e + 128);
  }
  return w.bytes;
}

function encodePsd(px: Pixels, width: number, height: number, alpha: boolean): Uint8Array<ArrayBuffer> {
  const channels = alpha ? 4 : 3;
  const bits = bitsOf(px);
  const plane = width * height * (bits / 8);
  const w = new Writer(26 + 12 + 2 + plane * channels, false);
  w.text('8BPS'); w.u16(1); for (let i = 0; i < 6; i++) w.u8(0);
  w.u16(channels); w.u32(height); w.u32(width); w.u16(bits); w.u16(3);
  w.u32(0); w.u32(0); w.u32(0);
  w.u16(0);
  for (let c = 0; c < channels; c++) for (let p = 0; p < width * height; p++) sample(w, px, p * 4 + c);
  return w.bytes;
}

async function encodeJpeg(px: Uint8Array, width: number, height: number): Promise<Blob> {
  const rgba = new Uint8ClampedArray(px.length);
  for (let i = 0; i < px.length; i += 4) { rgba[i] = px[i]; rgba[i + 1] = px[i + 1]; rgba[i + 2] = px[i + 2]; rgba[i + 3] = 255; }
  const surface = new OffscreenCanvas(width, height);
  surface.getContext('2d')!.putImageData(new ImageData(rgba, width, height), 0, 0);
  return surface.convertToBlob({ type: 'image/jpeg', quality: 0.95 });
}

export async function encodeImage(format: Format, px: Pixels, width: number, height: number, alpha: boolean, depth = 8): Promise<Blob> {
  const keepAlpha = alpha && FORMAT_ALPHA[format];
  if (format === 'png') {
    if (px instanceof Float32Array) throw new Error('PNG is 8 or 16 bit');
    return encodePng(px, width, height, keepAlpha);
  }
  if (format === 'jpg') {
    if (!(px instanceof Uint8Array)) throw new Error('JPEG is 8 bit');
    return encodeJpeg(px, width, height);
  }
  if (format === 'exr' || format === 'hdr') {
    if (!(px instanceof Float32Array)) throw new Error(`${format.toUpperCase()} needs float samples`);
  }
  const bytes = format === 'tif' ? encodeTiff(px, width, height, keepAlpha)
    : format === 'psd' ? encodePsd(px, width, height, keepAlpha)
    : format === 'hdr' ? encodeHdr(px as Float32Array, width, height)
    : encodeExr(px as Float32Array, width, height, keepAlpha, depth === 16);
  return new Blob([bytes], { type: MIME[format] });
}
