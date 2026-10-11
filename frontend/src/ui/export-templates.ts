export type Source = 'basecolor' | 'normal_gl' | 'normal_dx' | 'roughness' | 'smoothness' | 'metallic' | 'height' | 'ao' | 'curvature' | 'thickness' | 'one' | 'zero';

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

export const SOURCE_CHANNEL: Record<string, number> = { basecolor: 0, roughness: 1, smoothness: 1, metallic: 2, height: 3, normal_gl: 4, normal_dx: 4, ao: 5, curvature: 6, thickness: 7 };

export function sourcesOf(maps: MapSpec[]): number[] {
  const needed = new Set<number>();
  for (const map of maps) {
    for (const s of [map.rgb, map.r, map.g, map.b, map.a]) if (s && s in SOURCE_CHANNEL) needed.add(SOURCE_CHANNEL[s]);
  }
  return [...needed];
}

export function validateMaps(value: unknown): MapSpec[] {
  if (!Array.isArray(value) || !value.length || value.length > 16) throw new Error('A custom template is a list of 1 to 16 maps');
  const sources = new Set(['basecolor', 'normal_gl', 'normal_dx', 'roughness', 'smoothness', 'metallic', 'height', 'ao', 'curvature', 'thickness', 'one', 'zero']);
  return value.map((raw) => {
    const map = raw as MapSpec;
    if (typeof map.suffix !== 'string' || !/^[A-Za-z0-9_-]{1,40}$/.test(map.suffix)) throw new Error('Each map needs a suffix of letters, digits, dash or underscore');
    for (const key of ['r', 'g', 'b', 'a'] as const) if (map[key] !== undefined && !sources.has(map[key]!)) throw new Error(`Unknown source ${map[key]} in ${map.suffix}`);
    if (map.rgb !== undefined && !['basecolor', 'normal_gl', 'normal_dx'].includes(map.rgb)) throw new Error(`rgb must be basecolor, normal_gl or normal_dx in ${map.suffix}`);
    if (!map.rgb && !map.r) throw new Error(`Map ${map.suffix} needs rgb or r`);
    return map;
  });
}

export function pack(map: MapSpec, channels: Map<number, Uint8Array>, size: number): Uint8Array {
  const out = new Uint8Array(size * size * 4);
  const read = (source: Source | undefined, i: number, fallback: number): number => {
    if (!source) return fallback;
    if (source === 'one') return 255;
    if (source === 'zero') return 0;
    const data = channels.get(SOURCE_CHANNEL[source]);
    if (!data) return source === 'ao' ? 255 : fallback;
    return source === 'smoothness' ? 255 - data[i] : data[i];
  };
  for (let i = 0; i < size * size * 4; i += 4) {
    if (map.rgb) {
      const data = channels.get(SOURCE_CHANNEL[map.rgb])!;
      out[i] = data[i];
      out[i + 1] = map.rgb === 'normal_dx' ? 255 - data[i + 1] : data[i + 1];
      out[i + 2] = data[i + 2];
    } else {
      const r = read(map.r, i, 0);
      out[i] = r;
      out[i + 1] = map.g ? read(map.g, i, 0) : r;
      out[i + 2] = map.b ? read(map.b, i, 0) : r;
    }
    out[i + 3] = read(map.a, i, 255);
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

export async function encodePng(rgba: Uint8Array, width: number, height: number, alpha: boolean): Promise<Blob> {
  const bpp = alpha ? 4 : 3;
  const row = width * bpp;
  const raw = new Uint8Array((row + 1) * height);
  for (let y = 0; y < height; y++) {
    const base = y * (row + 1);
    raw[base] = 1;
    for (let x = 0; x < width; x++) {
      for (let c = 0; c < bpp; c++) {
        const value = rgba[(y * width + x) * 4 + c];
        const left = x > 0 ? rgba[(y * width + x - 1) * 4 + c] : 0;
        raw[base + 1 + x * bpp + c] = (value - left) & 0xff;
      }
    }
  }
  const compressed = new Uint8Array(await new Response(new Blob([raw]).stream().pipeThrough(new CompressionStream('deflate'))).arrayBuffer());
  const header = new Uint8Array(13);
  const view = new DataView(header.buffer);
  view.setUint32(0, width);
  view.setUint32(4, height);
  header[8] = 8;
  header[9] = alpha ? 6 : 2;
  const signature = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]);
  return new Blob([signature, chunk('IHDR', header), chunk('IDAT', compressed), chunk('IEND', new Uint8Array())], { type: 'image/png' });
}
