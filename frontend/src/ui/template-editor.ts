import type { MapSpec, Source, Template } from './export-templates';

const RGB_SOURCES = ['basecolor', 'normal_gl', 'normal_dx'] as const;
const GRAY_SOURCES: Source[] = ['roughness', 'smoothness', 'metallic', 'height', 'ao', 'curvature', 'thickness', 'one', 'zero'];

export async function loadTemplates(): Promise<Template[]> {
  try {
    const response = await fetch('/api/templates');
    if (!response.ok) return [];
    return ((await response.json()).templates ?? []) as Template[];
  } catch {
    return [];
  }
}

export async function saveTemplate(template: Template): Promise<string> {
  if (!/^[A-Za-z0-9 _.-]{1,80}$/.test(template.name) || template.name.startsWith('.')) throw new Error('Template names use letters, digits, spaces, dot, dash and underscore');
  const response = await fetch(`/api/templates/${encodeURIComponent(template.name)}`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ label: template.label, maps: template.maps }) });
  if (!response.ok) throw new Error(`Saving the template failed (is ./formsurface running?): ${await response.text()}`);
  return (await response.json()).file as string;
}

interface Row {
  root: HTMLDivElement;
  suffix: HTMLInputElement;
  r: HTMLSelectElement;
  g: HTMLSelectElement;
  b: HTMLSelectElement;
  a: HTMLSelectElement;
}

function select(label: string, options: [string, string][]): HTMLSelectElement {
  const s = document.createElement('select');
  s.title = label;
  s.setAttribute('aria-label', label);
  s.style.minWidth = '0';
  for (const [value, text] of options) s.add(new Option(text, value));
  return s;
}

export class TemplateEditor {
  private rows: Row[] = [];

  constructor(private readonly host: HTMLElement) {}

  private add(map?: MapSpec): void {
    const root = document.createElement('div');
    root.className = 'row';
    root.style.gridTemplateColumns = 'minmax(0,1.3fr) repeat(4, minmax(0,1fr)) 24px';
    root.style.gap = '4px';
    const suffix = document.createElement('input');
    suffix.type = 'text';
    suffix.placeholder = 'suffix';
    suffix.style.minWidth = '0';
    suffix.setAttribute('aria-label', 'Map suffix');
    const gray: [string, string][] = GRAY_SOURCES.map((s) => [s, s]);
    const r = select('Red, or RGB from a color source', [...RGB_SOURCES.map((s) => [`rgb:${s}`, `${s} (RGB)`] as [string, string]), ...gray]);
    const g = select('Green (same as red when empty)', [['', 'as R'], ...gray]);
    const b = select('Blue (same as red when empty)', [['', 'as R'], ...gray]);
    const a = select('Alpha', [['', 'no A'], ...gray]);
    const remove = document.createElement('button');
    remove.className = 'icon-btn';
    remove.title = 'Remove this map';
    remove.setAttribute('aria-label', 'Remove this map');
    const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
    use.setAttribute('href', '#i-trash');
    icon.appendChild(use);
    remove.appendChild(icon);
    const row: Row = { root, suffix, r, g, b, a };
    const sync = () => { const rgb = r.value.startsWith('rgb:'); g.disabled = rgb; b.disabled = rgb; };
    r.addEventListener('change', sync);
    remove.addEventListener('click', () => { root.remove(); this.rows = this.rows.filter((x) => x !== row); });
    root.append(suffix, r, g, b, a, remove);
    if (map) {
      suffix.value = map.suffix;
      r.value = map.rgb ? `rgb:${map.rgb}` : map.r ?? 'roughness';
      g.value = map.g ?? '';
      b.value = map.b ?? '';
      a.value = map.a ?? '';
    } else {
      suffix.value = `Map${this.rows.length + 1}`;
      r.value = 'roughness';
    }
    sync();
    this.rows.push(row);
    this.host.appendChild(root);
  }

  addMap(): void {
    if (this.rows.length >= 16) throw new Error('A template holds up to 16 maps');
    this.add();
  }

  load(template: Template): void {
    for (const row of this.rows) row.root.remove();
    this.rows = [];
    for (const map of template.maps) this.add(map);
  }

  maps(): MapSpec[] {
    return this.rows.map((row) => {
      const map: MapSpec = { suffix: row.suffix.value.trim() };
      if (row.r.value.startsWith('rgb:')) map.rgb = row.r.value.slice(4) as MapSpec['rgb'];
      else {
        map.r = row.r.value as Source;
        if (row.g.value) map.g = row.g.value as Source;
        if (row.b.value) map.b = row.b.value as Source;
      }
      if (row.a.value) map.a = row.a.value as Source;
      return map;
    });
  }
}
