export type AssetKind = 'material' | 'mask';

export interface AssetItem {
  kind: AssetKind;
  name: string;
  thumbnail?: string;
  builtIn: boolean;
}

const BUILT_IN_THUMBS = import.meta.glob('../assets/smart/*.webp', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;

export function slug(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

export function builtInThumb(kind: AssetKind, name: string): string | undefined {
  return BUILT_IN_THUMBS[`../assets/smart/${kind}-${slug(name)}.webp`];
}

export class AssetBrowser {
  private items: AssetItem[] = [];
  private selected?: { kind: AssetKind; name: string };
  private tiles = new Map<string, HTMLButtonElement>();

  constructor(private readonly host: HTMLElement, private readonly onPick: (item: AssetItem) => void) {}

  get selection(): AssetItem | undefined {
    return this.items.find((i) => i.kind === this.selected?.kind && i.name === this.selected?.name);
  }

  setItems(items: AssetItem[]): void {
    this.items = items;
    const wanted = new Set(items.map((i) => `${i.kind}:${i.name}`));
    for (const [key, tile] of this.tiles) if (!wanted.has(key)) { tile.remove(); this.tiles.delete(key); }
    for (const item of items) {
      const key = `${item.kind}:${item.name}`;
      let tile = this.tiles.get(key);
      if (!tile) {
        tile = document.createElement('button');
        tile.className = 'asset-tile';
        tile.type = 'button';
        const img = document.createElement('img');
        img.alt = '';
        img.width = 72;
        img.height = 72;
        const label = document.createElement('span');
        tile.append(img, label);
        tile.addEventListener('click', () => this.pick(item.kind, item.name));
        this.tiles.set(key, tile);
        this.host.append(tile);
      }
      const img = tile.querySelector('img')!;
      const src = item.thumbnail ?? builtInThumb(item.kind, item.name) ?? '';
      if (img.getAttribute('src') !== src) { if (src) img.src = src; else img.removeAttribute('src'); }
      const label = tile.querySelector('span')!;
      const text = item.builtIn ? item.name : `${item.name} (saved)`;
      if (label.textContent !== text) label.textContent = text;
      tile.title = `${item.kind === 'material' ? 'Smart material' : 'Smart mask'}: ${item.name}`;
      tile.dataset.kind = item.kind;
    }
    this.filter(this.query, this.kind);
  }

  private query = '';
  private kind: AssetKind | 'all' = 'all';

  filter(query: string, kind: AssetKind | 'all'): void {
    this.query = query;
    this.kind = kind;
    const q = query.trim().toLowerCase();
    for (const item of this.items) {
      const tile = this.tiles.get(`${item.kind}:${item.name}`);
      if (!tile) continue;
      const show = (kind === 'all' || item.kind === kind) && (!q || item.name.toLowerCase().includes(q));
      if (tile.hidden === show) tile.hidden = !show;
    }
  }

  pick(kind: AssetKind, name: string): void {
    this.selected = { kind, name };
    for (const [key, tile] of this.tiles) tile.setAttribute('aria-pressed', String(key === `${kind}:${name}`));
    const item = this.selection;
    if (item) this.onPick(item);
  }
}
