export interface LayerInfo {
  id: number;
  name: string;
  visible: boolean;
  opacity: number;
  blend: string;
  kind?: 'paint' | 'fill' | 'filter' | 'folder';
  mask?: boolean;
  parent?: number;
  instance?: boolean;
}

export function depthOf(layers: LayerInfo[], layer: LayerInfo): number {
  let depth = 0;
  let parent = layer.parent ?? 0;
  while (parent && depth < 16) {
    depth++;
    parent = layers.find((l) => l.id === parent)?.parent ?? 0;
  }
  return depth;
}

interface Handlers {
  select: (id: number) => void;
  toggle: (layer: LayerInfo) => void;
  move: (id: number, offset: number) => void;
}

interface Row {
  root: HTMLDivElement;
  name: HTMLSpanElement;
  kind: HTMLSpanElement;
  eye: SVGUseElement;
  layer: LayerInfo;
}

function icon(name: string): [SVGSVGElement, SVGUseElement] {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
  use.setAttribute('href', `#${name}`);
  svg.appendChild(use);
  return [svg, use];
}

function button(name: string, title: string): [HTMLButtonElement, SVGUseElement] {
  const b = document.createElement('button');
  b.className = 'icon-btn';
  b.title = title;
  const [svg, use] = icon(name);
  b.appendChild(svg);
  return [b, use];
}

export class LayerList {
  private rows = new Map<number, Row>();

  constructor(private readonly host: HTMLElement, private readonly handlers: Handlers) {}

  render(layers: LayerInfo[], active: number): void {
    const keep = new Set(layers.map((l) => l.id));
    for (const [id, row] of this.rows) {
      if (!keep.has(id)) {
        row.root.remove();
        this.rows.delete(id);
      }
    }
    layers.forEach((layer, index) => {
      const row = this.rows.get(layer.id) ?? this.create(layer);
      row.layer = layer;
      if (row.name.firstChild?.textContent !== layer.name) row.name.firstChild!.textContent = layer.name;
      const kind = `${layer.kind ?? 'paint'}${layer.mask ? ' + mask' : ''}${layer.instance ? ' · instance' : ''}`;
      if (row.kind.textContent !== kind) row.kind.textContent = kind;
      const depth = String(depthOf(layers, layer));
      if (row.root.style.getPropertyValue('--depth') !== depth) row.root.style.setProperty('--depth', depth);
      row.root.classList.toggle('folder', layer.kind === 'folder');
      const eye = layer.visible ? '#i-eye' : '#i-eye-off';
      if (row.eye.getAttribute('href') !== eye) row.eye.setAttribute('href', eye);
      row.root.classList.toggle('active', layer.id === active);
      if (this.host.children[index] !== row.root) this.host.insertBefore(row.root, this.host.children[index] ?? null);
    });
  }

  private create(layer: LayerInfo): Row {
    const root = document.createElement('div');
    root.className = 'layer';
    const [eyeBtn, eye] = button('i-eye', 'Show or hide layer');
    const name = document.createElement('span');
    name.className = 'name';
    name.appendChild(document.createTextNode(''));
    const kind = document.createElement('span');
    kind.className = 'kind';
    name.appendChild(kind);
    const [up] = button('i-up', 'Move layer up');
    const [down] = button('i-down', 'Move layer down');
    root.append(eyeBtn, name, up, down);
    const row: Row = { root, name, kind, eye, layer };
    root.addEventListener('click', () => this.handlers.select(row.layer.id));
    eyeBtn.addEventListener('click', (e) => { e.stopPropagation(); this.handlers.toggle(row.layer); });
    up.addEventListener('click', (e) => { e.stopPropagation(); this.handlers.move(row.layer.id, 1); });
    down.addEventListener('click', (e) => { e.stopPropagation(); this.handlers.move(row.layer.id, -1); });
    this.rows.set(layer.id, row);
    return row;
  }
}
