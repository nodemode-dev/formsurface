export interface TextureSetInfo {
  index: number;
  name: string;
  material: string;
  udim: number;
  tiled: boolean;
  triangles: number;
  layers: number;
}

interface Row {
  root: HTMLDivElement;
  name: Text;
  detail: HTMLSpanElement;
}

export class TextureSetList {
  private rows = new Map<number, Row>();

  constructor(private readonly host: HTMLElement, private readonly select: (index: number) => void) {}

  render(sets: TextureSetInfo[], active: number): void {
    for (const [index, row] of this.rows) {
      if (index >= sets.length) {
        row.root.remove();
        this.rows.delete(index);
      }
    }
    for (const set of sets) {
      const row = this.rows.get(set.index) ?? this.create(set.index);
      if (row.name.textContent !== set.name) row.name.textContent = set.name;
      const detail = `${set.triangles} tris · ${set.layers} layer${set.layers === 1 ? '' : 's'}`;
      if (row.detail.textContent !== detail) row.detail.textContent = detail;
      row.root.classList.toggle('active', set.index === active);
    }
  }

  private create(index: number): Row {
    const root = document.createElement('div');
    root.className = 'set-row';
    root.title = 'Paint and edit this texture set';
    const label = document.createElement('span');
    label.className = 'name';
    const name = document.createTextNode('');
    label.appendChild(name);
    const detail = document.createElement('span');
    detail.className = 'kind';
    root.append(label, detail);
    root.addEventListener('click', () => this.select(index));
    this.host.appendChild(root);
    const row: Row = { root, name, detail };
    this.rows.set(index, row);
    return row;
  }
}
