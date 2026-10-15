import type { Commands } from './commands';
import { el, q, setClass, setText } from './dom';
import type { MenuDef, MenuItem } from './menu';
import { t } from '../i18n';

export function fuzzyScore(query: string, text: string): number | null {
  const qs = query.toLowerCase().trim(), s = text.toLowerCase();
  if (!qs) return 0;
  let score = 0, at = 0, prev = -2;
  for (const ch of qs) {
    if (ch === ' ') continue;
    const i = s.indexOf(ch, at);
    if (i < 0) return null;
    score += 1 + (i === prev + 1 ? 6 : 0) + (i === 0 || /[\s>(\-]/.test(s[i - 1]) ? 4 : 0);
    prev = i;
    at = i + 1;
  }
  return score - s.length / 8;
}

function pathsOf(defs: MenuDef[]): Map<string, string> {
  const out = new Map<string, string>();
  const walk = (items: MenuItem[], trail: string) => {
    for (const item of items) {
      if (item === '-') continue;
      if (typeof item === 'string') { if (!out.has(item)) out.set(item, trail); continue; }
      walk(item.items, `${trail} > ${t(item.label)}`);
    }
  };
  for (const d of defs) walk(typeof d.items === 'function' ? d.items() : d.items, t(d.label));
  return out;
}

export class CommandPalette {
  private readonly scrim: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly list: HTMLElement;
  private rows: Array<{ id: string; node: HTMLElement }> = [];
  private index = 0;

  constructor(private readonly commands: Commands, private readonly menus: () => MenuDef[]) {
    this.scrim = el(`<div class="cv-scrim cv-palette-scrim" hidden><div class="cv-palette" role="dialog" aria-modal="true" aria-label="Command Palette">
<input class="cv-palette-input" type="text" spellcheck="false" autocomplete="off" placeholder="Type a command">
<div class="cv-palette-list" role="listbox"></div>
</div></div>`);
    this.input = q<HTMLInputElement>(this.scrim, '.cv-palette-input');
    this.list = q(this.scrim, '.cv-palette-list');
    this.input.addEventListener('input', () => this.filter());
    this.input.addEventListener('keydown', (e) => this.key(e));
    this.scrim.addEventListener('pointerdown', (e) => { if (e.target === this.scrim) this.close(); });
    this.list.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest<HTMLElement>('.cv-palette-row');
      if (row?.dataset.id) this.run(row.dataset.id);
    });
    document.getElementById('overlay-root')?.appendChild(this.scrim);
  }

  open(): void {
    const paths = pathsOf(this.menus());
    this.list.replaceChildren();
    this.rows = [];
    for (const cmd of this.commands.all()) {
      if (!this.commands.enabled(cmd)) continue;
      const node = el(`<div class="cv-palette-row" role="option"><span class="cv-palette-label"></span><span class="cv-palette-path"></span><span class="cv-palette-keys"></span></div>`);
      node.dataset.id = cmd.id;
      setText(q(node, '.cv-palette-label'), t(this.commands.label(cmd)));
      setText(q(node, '.cv-palette-path'), paths.get(cmd.id) ?? '');
      setText(q(node, '.cv-palette-keys'), this.commands.keyLabel(cmd));
      this.rows.push({ id: cmd.id, node });
      this.list.appendChild(node);
    }
    this.input.value = '';
    this.scrim.hidden = false;
    this.filter();
    this.input.focus();
  }

  close(): void { this.scrim.hidden = true; }

  private filter(): void {
    const query = this.input.value;
    const scored = this.rows.map((r) => {
      const label = q(r.node, '.cv-palette-label').textContent ?? '', path = q(r.node, '.cv-palette-path').textContent ?? '';
      const a = fuzzyScore(query, label), b = fuzzyScore(query, `${path} ${label}`);
      return { r, s: a === null ? (b === null ? null : b - 3) : a };
    });
    const shown = scored.filter((x) => x.s !== null).sort((x, y) => (y.s ?? 0) - (x.s ?? 0));
    const order = new Set(shown.map((x) => x.r));
    for (const r of this.rows) r.node.hidden = !order.has(r);
    for (const x of shown) this.list.appendChild(x.r.node);
    this.index = 0;
    this.mark();
  }

  private visible(): HTMLElement[] { return [...this.list.querySelectorAll<HTMLElement>('.cv-palette-row:not([hidden])')]; }

  private mark(): void {
    const v = this.visible();
    v.forEach((n, i) => setClass(n, 'is-on', i === this.index));
    v[this.index]?.scrollIntoView({ block: 'nearest' });
  }

  private key(e: KeyboardEvent): void {
    const v = this.visible();
    if (e.key === 'ArrowDown') { e.preventDefault(); this.index = Math.min(v.length - 1, this.index + 1); this.mark(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); this.index = Math.max(0, this.index - 1); this.mark(); }
    else if (e.key === 'Enter') { e.preventDefault(); const id = v[this.index]?.dataset.id; if (id) this.run(id); }
    else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); this.close(); }
  }

  private run(id: string): void {
    this.close();
    this.commands.run(id);
  }
}
