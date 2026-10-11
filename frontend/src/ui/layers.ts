export interface LayerInfo {
  id: number;
  name: string;
  visible: boolean;
  opacity: number;
  blend: string;
}

interface Handlers {
  select: (id: number) => void;
  toggle: (layer: LayerInfo) => void;
  move: (id: number, offset: number) => void;
}

interface Row {
  root: HTMLDivElement;
  name: HTMLSpanElement;
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
      if (row.name.textContent !== layer.name) row.name.textContent = layer.name;
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
    const [up] = button('i-up', 'Move layer up');
    const [down] = button('i-down', 'Move layer down');
    root.append(eyeBtn, name, up, down);
    const row: Row = { root, name, eye, layer };
    root.addEventListener('click', () => this.handlers.select(row.layer.id));
    eyeBtn.addEventListener('click', (e) => { e.stopPropagation(); this.handlers.toggle(row.layer); });
    up.addEventListener('click', (e) => { e.stopPropagation(); this.handlers.move(row.layer.id, 1); });
    down.addEventListener('click', (e) => { e.stopPropagation(); this.handlers.move(row.layer.id, -1); });
    this.rows.set(layer.id, row);
    return row;
  }
}
