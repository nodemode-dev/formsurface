export type Source = 'basecolor' | 'normal_gl' | 'normal_dx' | 'roughness' | 'smoothness' | 'metallic' | 'height' | 'ao' | 'curvature' | 'thickness' | 'ao_high' | 'height_high' | 'opacity' | 'one' | 'zero';

export interface MapSpec {
  suffix: string;
  rgb?: 'basecolor' | 'normal_gl' | 'normal_dx';
  r?: Source;
  g?: Source;
  b?: Source;
  a?: Source;
}

export interface Template {
  name: string;
  label: string;
  maps: MapSpec[];
}

export const TEMPLATES: Template[] = [
  { name: 'pbr', label: 'PBR metallic roughness (separate maps)', maps: [] },
  {
    name: 'gltf', label: 'glTF 2.0 (packed occlusion, roughness, metallic)', maps: [
      { suffix: 'baseColor', rgb: 'basecolor' },
      { suffix: 'occlusionRoughnessMetallic', r: 'ao', g: 'roughness', b: 'metallic' },
      { suffix: 'normal', rgb: 'normal_gl' },
    ],
  },
  {
    name: 'unreal', label: 'Unreal Engine (ORM, DirectX normal)', maps: [
      { suffix: 'BaseColor', rgb: 'basecolor' },
      { suffix: 'OcclusionRoughnessMetallic', r: 'ao', g: 'roughness', b: 'metallic' },
      { suffix: 'Normal', rgb: 'normal_dx' },
    ],
  },
  {
    name: 'unity_hdrp', label: 'Unity HDRP (mask map)', maps: [
      { suffix: 'BaseMap', rgb: 'basecolor' },
      { suffix: 'MaskMap', r: 'metallic', g: 'ao', b: 'zero', a: 'smoothness' },
      { suffix: 'Normal', rgb: 'normal_gl' },
    ],
  },
  {
    name: 'unity_urp', label: 'Unity URP (metallic smoothness)', maps: [
      { suffix: 'BaseMap', rgb: 'basecolor' },
      { suffix: 'MetallicSmoothness', r: 'metallic', g: 'metallic', b: 'metallic', a: 'smoothness' },
      { suffix: 'Occlusion', r: 'ao', g: 'ao', b: 'ao' },
      { suffix: 'Normal', rgb: 'normal_gl' },
    ],
  },
];

export const SOURCE_CHANNEL: Record<string, number> = { basecolor: 0, roughness: 1, smoothness: 1, metallic: 2, height: 3, normal_gl: 4, normal_dx: 4, ao: 5, curvature: 6, thickness: 7, ao_high: 100, height_high: 101, opacity: 102 };

export function sourcesOf(maps: MapSpec[]): number[] {
  const needed = new Set<number>();
  for (const map of maps) {
    for (const s of [map.rgb, map.r, map.g, map.b, map.a]) if (s && s in SOURCE_CHANNEL) needed.add(SOURCE_CHANNEL[s]);
  }
  return [...needed];
}

export function validateMaps(value: unknown): MapSpec[] {
  if (!Array.isArray(value) || !value.length || value.length > 16) throw new Error('A custom template is a list of 1 to 16 maps');
  const sources = new Set(['basecolor', 'normal_gl', 'normal_dx', 'roughness', 'smoothness', 'metallic', 'height', 'ao', 'curvature', 'thickness', 'ao_high', 'height_high', 'opacity', 'one', 'zero']);
  return value.map((raw) => {
    const map = raw as MapSpec;
    if (typeof map.suffix !== 'string' || !/^[A-Za-z0-9_-]{1,40}$/.test(map.suffix)) throw new Error('Each map needs a suffix of letters, digits, dash or underscore');
    for (const key of ['r', 'g', 'b', 'a'] as const) if (map[key] !== undefined && !sources.has(map[key]!)) throw new Error(`Unknown source ${map[key]} in ${map.suffix}`);
    if (map.rgb !== undefined && !['basecolor', 'normal_gl', 'normal_dx'].includes(map.rgb)) throw new Error(`rgb must be basecolor, normal_gl or normal_dx in ${map.suffix}`);
    if (!map.rgb && !map.r) throw new Error(`Map ${map.suffix} needs rgb or r`);
    return map;
  });
}

export type Pixels = Uint8Array | Uint16Array | Float32Array;

const HALF = (() => {
  const table = new Float32Array(65536);
  for (let h = 0; h < 65536; h++) {
    const s = h & 0x8000 ? -1 : 1;
    const e = (h >> 10) & 0x1f;
    const f = h & 0x3ff;
    table[h] = e === 0 ? s * f * 2 ** -24 : e === 31 ? (f ? NaN : s * Infinity) : s * (1 + f / 1024) * 2 ** (e - 15);
  }
  return table;
})();

export function halfToUnit16(bytes: Uint8Array): Uint16Array {
  const halves = new Uint16Array(bytes.buffer, bytes.byteOffset, bytes.byteLength / 2);
  const out = new Uint16Array(halves.length);
  for (let i = 0; i < halves.length; i++) {
    const v = HALF[halves[i]];
    out[i] = v > 0 ? Math.round(Math.min(v, 1) * 65535) : 0;
  }
  return out;
}

export function halfToFloat(bytes: Uint8Array): Float32Array {
  const halves = new Uint16Array(bytes.buffer, bytes.byteOffset, bytes.byteLength / 2);
  const out = new Float32Array(halves.length);
  for (let i = 0; i < halves.length; i++) out[i] = HALF[halves[i]];
  return out;
}

export function emptyPixels(max: number, length: number): Pixels {
  return max === 255 ? new Uint8Array(length) : max === 65535 ? new Uint16Array(length) : new Float32Array(length);
}

export function pack(map: MapSpec, channels: Map<number, Pixels>, size: number, max = 255): Pixels {
  const out = emptyPixels(max, size * size * 4);
  const read = (source: Source | undefined, i: number, fallback: number): number => {
    if (!source) return fallback;
    if (source === 'one') return max;
    if (source === 'zero') return 0;
    const data = channels.get(SOURCE_CHANNEL[source]);
    if (!data) return source === 'ao' || source === 'ao_high' ? max : fallback;
    return source === 'smoothness' ? max - data[i] : data[i];
  };
  for (let i = 0; i < size * size * 4; i += 4) {
    if (map.rgb) {
      const data = channels.get(SOURCE_CHANNEL[map.rgb])!;
      out[i] = data[i];
      out[i + 1] = map.rgb === 'normal_dx' ? max - data[i + 1] : data[i + 1];
      out[i + 2] = data[i + 2];
    } else {
      const r = read(map.r, i, 0);
      out[i] = r;
      out[i + 1] = map.g ? read(map.g, i, 0) : r;
      out[i + 2] = map.b ? read(map.b, i, 0) : r;
    }
    out[i + 3] = read(map.a, i, max);
  }
  return out;
}

const CRC = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) c = CRC[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type: string, data: Uint8Array): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(12 + data.length);
  const view = new DataView(out.buffer);
  view.setUint32(0, data.length);
  for (let i = 0; i < 4; i++) out[4 + i] = type.charCodeAt(i);
  out.set(data, 8);
  view.setUint32(8 + data.length, crc32(out.subarray(4, 8 + data.length)));
  return out;
}

export async function encodePng(rgba: Uint8Array | Uint16Array, width: number, height: number, alpha: boolean): Promise<Blob> {
  const channels = alpha ? 4 : 3;
  const depth = rgba instanceof Uint16Array ? 16 : 8;
  const bytesPer = depth / 8;
  const bpp = channels * bytesPer;
  const row = width * bpp;
  const raw = new Uint8Array((row + 1) * height);
  const line = new Uint8Array(row);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      for (let c = 0; c < channels; c++) {
        const value = rgba[(y * width + x) * 4 + c];
        const at = x * bpp + c * bytesPer;
        if (depth === 16) { line[at] = value >> 8; line[at + 1] = value & 0xff; } else line[at] = value;
      }
    }
    const base = y * (row + 1);
    raw[base] = 1;
    for (let i = 0; i < row; i++) raw[base + 1 + i] = (line[i] - (i >= bpp ? line[i - bpp] : 0)) & 0xff;
  }
  const compressed = new Uint8Array(await new Response(new Blob([raw]).stream().pipeThrough(new CompressionStream('deflate'))).arrayBuffer());
  const header = new Uint8Array(13);
  const view = new DataView(header.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  header[8] = depth;
  header[9] = alpha ? 6 : 2;
  const signature = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);
  return new Blob([signature, chunk('IHDR', header), chunk('IDAT', compressed), chunk('IEND', new Uint8Array())], { type: 'image/png' });
}
