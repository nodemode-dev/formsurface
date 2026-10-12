export interface MaskEffectSpec {
  kind: string;
  blend: string;
  opacity: number;
  visible: boolean;
  value: number;
  in_low: number;
  in_high: number;
  out_low: number;
  out_high: number;
  radius: number;
  layer: number;
  fill: Record<string, unknown>;
}

export const EFFECT_KINDS: [string, string][] = [['fill', 'Fill'], ['generator', 'Generator'], ['levels', 'Levels'], ['invert', 'Invert'], ['blur', 'Blur'], ['compare', 'Compare mask']];
const BLENDS: [string, string][] = [['normal', 'Normal'], ['multiply', 'Multiply'], ['add', 'Add'], ['subtract', 'Subtract'], ['lighten', 'Lighten (max)'], ['darken', 'Darken (min)'], ['screen', 'Screen']];
const GENERATORS: [string, string][] = [['cavity', 'Cavity'], ['edges', 'Edges'], ['thin', 'Thin parts'], ['top', 'From top'], ['light', 'Light'], ['position', 'Position'], ['noise', 'Noise'], ['dirt', 'Dirt'], ['metal_edge_wear', 'Metal edge wear'], ['mask_builder', 'Mask builder']];

export function newEffect(kind: string): MaskEffectSpec {
  return {
    kind, blend: kind === 'generator' || kind === 'compare' ? 'multiply' : 'normal', opacity: 1, visible: true, value: kind === 'fill' ? 0 : 1,
    in_low: 0, in_high: 1, out_low: 0, out_high: 1, radius: 4, layer: 0, fill: { generator: 'cavity', generator_level: 0.5, generator_softness: 0.1 },
  };
}

function select(options: [string, string][], value: string, label: string): HTMLSelectElement {
  const s = document.createElement('select');
  s.setAttribute('aria-label', label);
  for (const [v, t] of options) { const o = document.createElement('option'); o.value = v; o.textContent = t; s.append(o); }
  s.value = value;
  return s;
}

function range(min: number, max: number, step: number, value: number): HTMLInputElement {
  const r = document.createElement('input');
  r.type = 'range';
  r.min = String(min); r.max = String(max); r.step = String(step); r.value = String(value);
  return r;
}

function row(label: string, control: HTMLElement, out?: HTMLOutputElement): HTMLDivElement {
  const d = document.createElement('div');
  d.className = 'row';
  const l = document.createElement('label');
  l.textContent = label;
  d.append(l, control, out ?? document.createElement('output'));
  return d;
}

export function renderEffects(host: HTMLElement, specs: MaskEffectSpec[], layers: { id: number; name: string }[], onChange: (specs: MaskEffectSpec[]) => void): void {
  const nodes: HTMLElement[] = [];
  specs.forEach((spec, i) => {
    const box = document.createElement('div');
    box.className = 'mfx';
    const head = document.createElement('div');
    head.className = 'row';
    head.style.gridTemplateColumns = '1fr auto auto';
    const name = document.createElement('label');
    name.textContent = `${i + 1}. ${EFFECT_KINDS.find(([k]) => k === spec.kind)?.[1] ?? spec.kind}`;
    const vis = document.createElement('input');
    vis.type = 'checkbox';
    vis.checked = spec.visible;
    vis.setAttribute('aria-label', 'Effect visible');
    vis.addEventListener('change', () => { spec.visible = vis.checked; onChange(specs); });
    const remove = document.createElement('button');
    remove.className = 'btn';
    remove.textContent = 'Remove';
    remove.addEventListener('click', () => { specs.splice(i, 1); renderEffects(host, specs, layers, onChange); onChange(specs); });
    head.append(name, vis, remove);
    box.append(head);
    const bind = (input: HTMLInputElement, apply: (v: number) => void, label: string) => {
      const out = document.createElement('output');
      out.textContent = Number(input.value).toFixed(2);
      input.addEventListener('input', () => { out.textContent = Number(input.value).toFixed(2); apply(Number(input.value)); onChange(specs); });
      box.append(row(label, input, out));
    };
    if (spec.kind === 'fill' || spec.kind === 'generator' || spec.kind === 'compare') {
      const b = select(BLENDS, spec.blend, 'Effect blend');
      b.addEventListener('change', () => { spec.blend = b.value; onChange(specs); });
      box.append(row('Blend', b));
    }
    if (spec.kind === 'fill') bind(range(0, 1, 0.01, spec.value), (v) => { spec.value = v; }, 'Value');
    if (spec.kind === 'generator') {
      const g = select(GENERATORS, String(spec.fill.generator ?? 'cavity'), 'Generator');
      g.addEventListener('change', () => { spec.fill = { ...spec.fill, generator: g.value }; onChange(specs); });
      box.append(row('Generator', g));
      bind(range(0, 1, 0.01, Number(spec.fill.generator_level ?? 0.5)), (v) => { spec.fill = { ...spec.fill, generator_level: v }; }, 'Level');
    }
    if (spec.kind === 'levels') {
      bind(range(0, 1, 0.01, spec.in_low), (v) => { spec.in_low = v; }, 'In low');
      bind(range(0, 1, 0.01, spec.in_high), (v) => { spec.in_high = v; }, 'In high');
      bind(range(0, 1, 0.01, spec.out_low), (v) => { spec.out_low = v; }, 'Out low');
      bind(range(0, 1, 0.01, spec.out_high), (v) => { spec.out_high = v; }, 'Out high');
    }
    if (spec.kind === 'blur') bind(range(0, 32, 1, spec.radius), (v) => { spec.radius = v; }, 'Radius px');
    if (spec.kind === 'compare') {
      const options: [string, string][] = [['0', 'Choose a layer'], ...layers.map((l) => [String(l.id), l.name] as [string, string])];
      const c = select(options, String(spec.layer), 'Compare with');
      c.addEventListener('change', () => { spec.layer = Number(c.value); onChange(specs); });
      box.append(row('Mask of', c));
    }
    bind(range(0, 1, 0.01, spec.opacity), (v) => { spec.opacity = v; }, 'Opacity');
    nodes.push(box);
  });
  host.replaceChildren(...nodes);
}
