export interface SmartLayer {
  kind: 'fill' | 'filter';
  name: string;
  settings: Record<string, unknown>;
  mask?: 'white' | 'black';
  opacity?: number;
  blend?: string;
}

export interface SmartMaterial {
  name: string;
  layers: SmartLayer[];
  builtIn?: boolean;
  thumbnail?: string;
}

export const MASK_THUMBS: Record<string, string> = {};

const hex = (value: string): number[] => [1, 3, 5].map((i) => Math.pow(parseInt(value.slice(i, i + 2), 16) / 255, 2.2));

export const BUILT_IN_MATERIALS: SmartMaterial[] = [
  {
    name: 'Worn painted metal',
    builtIn: true,
    layers: [
      { kind: 'fill', name: 'Bare steel', settings: { base_color: hex('#8c9096'), roughness: 0.35, metallic: 1 } },
      { kind: 'fill', name: 'Paint', settings: { base_color: hex('#7a1f1a'), roughness: 0.55, metallic: 0, generator: 'edges', generator_level: 0.62, generator_softness: 0.06, generator_invert: true } },
      { kind: 'fill', name: 'Grime', settings: { base_color: hex('#2b2118'), roughness: 0.9, metallic: 0, generator: 'cavity', generator_level: 0.45, generator_softness: 0.15, opacity: 0.8 } },
    ],
  },
  {
    name: 'Rusty iron',
    builtIn: true,
    layers: [
      { kind: 'fill', name: 'Iron', settings: { base_color: hex('#5d5f63'), roughness: 0.45, metallic: 1 } },
      { kind: 'fill', name: 'Rust patches', settings: { base_color: hex('#7c3a16'), roughness: 0.95, metallic: 0, projection: 'triplanar', pattern: 'noise', scale: 3, sharpness: 6 } },
      { kind: 'fill', name: 'Rust in crevices', settings: { base_color: hex('#5a2610'), roughness: 1, metallic: 0, generator: 'cavity', generator_level: 0.4, generator_softness: 0.2 } },
    ],
  },
  {
    name: 'Dusty plastic',
    builtIn: true,
    layers: [
      { kind: 'fill', name: 'Plastic', settings: { base_color: hex('#2f6fb3'), roughness: 0.4, metallic: 0 } },
      { kind: 'fill', name: 'Dust', settings: { base_color: hex('#bfb6a3'), roughness: 0.95, metallic: 0, generator: 'top', generator_level: 0.7, generator_softness: 0.15, opacity: 0.7 } },
      { kind: 'filter', name: 'Fade', settings: { saturation: 0.85, brightness: 0.02 } },
    ],
  },
  {
    name: 'Brushed gold',
    builtIn: true,
    layers: [
      { kind: 'fill', name: 'Gold', settings: { base_color: hex('#d4a64a'), roughness: 0.28, metallic: 1 } },
      { kind: 'fill', name: 'Brushing', settings: { base_color: hex('#e6c477'), roughness: 0.4, metallic: 1, pattern: 'stripes', scale: 160, opacity: 0.35 } },
    ],
  },
];

export const SMART_MASKS: Record<string, Record<string, unknown>> = {
  'Edge wear': { generator: 'edges', generator_level: 0.6, generator_softness: 0.06, generator_invert: false },
  'Cavity dirt': { generator: 'cavity', generator_level: 0.45, generator_softness: 0.15, generator_invert: false },
  'Dust from top': { generator: 'top', generator_level: 0.7, generator_softness: 0.15, generator_invert: false },
  'Thin parts': { generator: 'thin', generator_level: 0.6, generator_softness: 0.1, generator_invert: false },
  'Protect edges': { generator: 'edges', generator_level: 0.6, generator_softness: 0.06, generator_invert: true },
  'Grunge dirt': { generator: 'dirt', generator_level: 0.4, generator_softness: 0.12, generator_invert: false, noise_scale: 6 },
  'Metal edge wear': { generator: 'metal_edge_wear', generator_level: 0.45, generator_softness: 0.08, generator_invert: false, noise_scale: 10 },
  'Sun bleached': { generator: 'light', generator_level: 0.6, generator_softness: 0.2, generator_invert: false, generator_direction: [0.3, 0.9, 0.3], generator_noise: 0.2 },
};

export const GENERATOR_KEYS = ['generator', 'generator_level', 'generator_softness', 'generator_invert', 'generator_noise', 'noise_scale', 'generator_direction', 'builder_ao', 'builder_curvature', 'builder_noise'];

export function generatorPart(settings: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const key of GENERATOR_KEYS) if (settings[key] !== undefined) out[key] = settings[key];
  return out;
}

export async function savedMasks(): Promise<Record<string, Record<string, unknown>>> {
  try {
    const response = await fetch('/api/smart-masks');
    if (!response.ok) return {};
    const list = ((await response.json()).masks ?? []) as { name: string; settings: Record<string, unknown>; thumbnail?: string }[];
    for (const m of list) if (m.thumbnail) MASK_THUMBS[m.name] = m.thumbnail;
    return Object.fromEntries(list.map((m) => [m.name, generatorPart(m.settings)]));
  } catch {
    return {};
  }
}

export async function saveMask(name: string, settings: Record<string, unknown>, thumbnail?: string): Promise<string> {
  if (!/^[A-Za-z0-9 _.-]{1,80}$/.test(name) || name.startsWith('.')) throw new Error('Intelligent mask names use letters, digits, spaces, dot, dash and underscore');
  const response = await fetch(`/api/smart-masks/${encodeURIComponent(name)}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ settings: generatorPart(settings), thumbnail }) });
  if (!response.ok) throw new Error(`Saving the intelligent mask failed (is ./formsurface running?): ${await response.text()}`);
  return (await response.json()).file as string;
}

export async function savedMaterials(): Promise<SmartMaterial[]> {
  try {
    const response = await fetch('/api/library');
    if (!response.ok) return [];
    return ((await response.json()).materials ?? []) as SmartMaterial[];
  } catch {
    return [];
  }
}

export async function saveMaterial(material: SmartMaterial): Promise<string> {
  const response = await fetch(`/api/library/${encodeURIComponent(material.name)}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(material) });
  if (!response.ok) throw new Error(`Saving the intelligent material failed: ${await response.text()}`);
  return (await response.json()).file as string;
}
