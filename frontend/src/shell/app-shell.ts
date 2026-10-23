import '../styles/shell.css';
import '../styles/dock.css';
import { api } from './api';
import { Commands, comboOf, formatKeys, type Command } from './commands';
import { el, isMac, q, revealApp, setClass, setHidden, setText } from './dom';
import { events } from './events';
import { icon } from './icons';
import { MenuBar, type MenuDef, type MenuItem } from './menu';
import { TerminalDock } from './terminal-dock';
import { CommandPalette } from './command-palette';
import { Dialog } from './dialog';
import { THEMES, currentTheme, setTheme } from './themes';
import { language, languages, setLanguage, t, type Lang } from '../i18n';
import { mountColorWheel } from '../ui/color-wheel';

const SHORTCUTS_KEY = 'formsurface.shortcuts';
const DOCK_KEY = 'formsurface.dock';
const TAB_KEY = 'formsurface.panelTab';
const FOLD_KEY = 'formsurface.folded';
const PANEL_KEY = 'formsurface.panel';
const SWATCH_KEY = 'formsurface.swatches';
const SWATCH_LOCK_KEY = 'formsurface.swatchesLocked';
const SWATCHES = 12;

const byId = <T extends HTMLElement = HTMLElement>(id: string): T | null => document.getElementById(id) as T | null;
const click = (id: string): void => { byId(id)?.click(); };
const pressed = (id: string): boolean => byId(id)?.getAttribute('aria-pressed') === 'true';

function read<T>(key: string, fallback: T): T {
  try { const v = localStorage.getItem(key); return v ? (JSON.parse(v) as T) : fallback; } catch { return fallback; }
}
function write(key: string, value: unknown): void {
  try { localStorage.setItem(key, JSON.stringify(value)); } catch { return; }
}

function setSelect(id: string, value: string): void {
  const s = byId<HTMLSelectElement>(id);
  if (!s || s.value === value) return;
  s.value = value;
  s.dispatchEvent(new Event('change', { bubbles: true }));
}

function nudgeBrush(factor: number): void {
  const input = byId<HTMLInputElement>('b-size');
  if (!input) return;
  input.value = String(Math.max(2, Math.min(400, Math.round(Number(input.value) * factor))));
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function select(kind: 'lasso' | 'rect', limit: boolean): void {
  byId('canvas')?.dispatchEvent(new CustomEvent('fs-select', { detail: { kind, limit } }));
}

function hidePart(mode: number, isolate: boolean): void {
  byId('canvas')?.dispatchEvent(new CustomEvent('fs-hide-part', { detail: { mode, isolate } }));
}

async function pickColor(): Promise<void> {
  const Picker = (window as unknown as { EyeDropper?: new () => { open: () => Promise<{ sRGBHex: string }> } }).EyeDropper;
  const input = byId<HTMLInputElement>('c-color');
  if (!Picker || !input) return;
  try {
    const { sRGBHex } = await new Picker().open();
    input.value = sRGBHex;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    input.dispatchEvent(new Event('change', { bubbles: true }));
  } catch { return; }
}

function fail(err: unknown): void {
  const msg = byId('status');
  if (!msg) return;
  setText(msg, err instanceof Error ? err.message : String(err));
  setClass(msg, 'error', true);
}

const SAMPLES: Array<[string, string]> = [
  ['sphere', 'Sphere'], ['knot', 'Torus Knot'], ['torus', 'Torus'], ['capsule', 'Capsule'],
  ['sets', 'Figure with Two Texture Sets'], ['pair', 'Sphere Pair with High-Poly'],
];

const PANEL_TABS: Array<{ id: string; label: string; sections: string[] }> = [
  { id: 'paint', label: 'Paint', sections: ['Brush', 'Symmetry', 'Stencil', 'Path', 'Text', 'Material'] },
  { id: 'layers', label: 'Layers', sections: ['Texture sets', 'Layers', 'Intelligent materials', 'Fill layer', 'Filter layer'] },
  { id: 'bake', label: 'Bake', sections: ['Baking', 'Base material'] },
  { id: 'display', label: 'Display', sections: ['Display', 'Post effects', 'Resources'] },
  { id: 'export', label: 'Export', sections: ['Export', 'Python'] },
];

export class Shell {
  readonly commands = new Commands();
  private dock: TerminalDock | null = null;
  private readonly dockHost: HTMLElement;
  private readonly app: HTMLElement;
  private palette: CommandPalette | null = null;
  private tab = read<string>(TAB_KEY, 'paint');
  private readonly folded = new Set(read<string[]>(FOLD_KEY, []));
  private readonly sections = new Map<string, { node: HTMLElement; tab: string }>();
  private readonly tabButtons = new Map<string, HTMLButtonElement>();

  constructor() {
    this.app = q(document, '.app');
    this.dockHost = q(document, '.fl-dock');
    this.register();
    this.loadKeys();
    this.buildPanel();
    new MenuBar(q(document, '.cv-menubar'), this.commands, this.menus());
    document.addEventListener('click', (e) => {
      const b = (e.target as HTMLElement).closest<HTMLElement>('[data-cmd]');
      if (b?.dataset.cmd) { e.preventDefault(); this.commands.run(b.dataset.cmd); }
    });
    window.addEventListener('keydown', (e) => this.onKey(e), true);
    const dock = read<{ open: boolean; h: number }>(DOCK_KEY, { open: false, h: 280 });
    this.app.style.setProperty('--fs-dock-open-h', `${dock.h}px`);
    if (dock.open) void this.setDock(true);
    events.start();
  }

  private menus(): MenuDef[] {
    const sample: MenuItem = { label: 'Sample Mesh', items: SAMPLES.map(([id]) => `file.sample.${id}`) };
    return [
      { label: 'File', items: ['file.open', sample, '-', 'file.save', 'file.screenshot', '-', 'file.exportMaps', 'file.exportMesh', '-', 'file.preferences'] },
      { label: 'Edit', items: ['edit.undo', 'edit.redo', '-', 'edit.palette', 'edit.shortcuts'] },
      { label: 'Tools', items: ['tool.paint', 'tool.erase', 'tool.clone', 'tool.polyfill', 'tool.path', '-', 'tool.mirror', 'tool.quickMask', '-', 'tool.lassoFill', 'tool.rectFill', 'tool.lassoLimit', 'tool.rectLimit', '-', 'tool.brushBigger', 'tool.brushSmaller', 'tool.pickColor'] },
      { label: 'Layer', items: ['layer.addPaint', 'layer.addFill', 'layer.addFillBlack', 'layer.addFilter', 'layer.addFolder', '-', 'layer.maskBlack', 'layer.maskWhite', 'layer.maskNone', '-', 'layer.pickUnderCursor', '-', 'layer.mergeDown', 'layer.flattenFolder', 'layer.flatten', '-', 'layer.export', 'layer.exportMask', 'layer.editExternal', '-', 'layer.delete'] },
      { label: 'Bake', items: ['bake.mesh', 'bake.loadHigh', 'bake.high'] },
      { label: 'View', items: ['view.frame', 'view.2d', '-', 'view.hidePart', 'view.hideIsland', 'view.isolatePart', 'view.showParts', '-', ...PANEL_TABS.map((p) => `view.tab.${p.id}`), 'view.panel', '-', 'view.render'] },
      { label: 'Agent', items: ['view.terminal', 'view.newTerminal', 'agent.claude', 'agent.chat', '-', 'agent.link', 'agent.connect'] },
      { label: 'Help', items: ['edit.shortcuts', 'agent.connect', '-', 'help.about'] },
    ];
  }

  private register(): void {
    const c = this.commands;
    const btn = (id: string, label: string, target: string, keys?: string[], extra: Partial<Command> = {}): Command => ({ id, label, keys, run: () => click(target), ...extra });
    c.add(
      btn('file.open', 'Open Mesh...', 'open-mesh', ['Mod+O']),
      ...SAMPLES.map(([id, label]) => ({ id: `file.sample.${id}`, label, run: () => (document.querySelector<HTMLElement>(`[data-sample="${id}"]`))?.click() })),
      btn('file.save', 'Save Project', 'save-project', ['Mod+S']),
      btn('file.screenshot', 'Save Screenshot', 'screenshot', ['Shift+Mod+S']),
      btn('file.exportMaps', 'Export Texture Maps', 'export', ['Shift+Mod+E']),
      btn('file.exportMesh', 'Export Mesh (OBJ)', 'x-mesh'),
      { id: 'file.preferences', label: 'Preferences...', keys: ['Mod+,'], run: () => void this.preferences() },
      btn('edit.undo', 'Undo', 'undo', ['Mod+Z']),
      btn('edit.redo', 'Redo', 'redo', ['Shift+Mod+Z', 'Mod+Y']),
      { id: 'edit.palette', label: 'Command Palette...', keys: ['Mod+K', 'Shift+Mod+P'], run: () => this.openPalette() },
      { id: 'edit.shortcuts', label: 'Keyboard Shortcuts...', keys: ['Mod+/'], run: () => this.shortcuts() },
      btn('tool.paint', 'Paint', 'tool-paint', ['B'], { checked: () => pressed('tool-paint') }),
      btn('tool.erase', 'Eraser', 'tool-erase', ['E'], { checked: () => pressed('tool-erase') }),
      btn('tool.clone', 'Clone', 'tool-clone', ['C'], { checked: () => pressed('tool-clone') }),
      btn('tool.polyfill', 'Polygon Fill', 'tool-polyfill', ['P'], { checked: () => pressed('tool-polyfill') }),
      btn('tool.path', 'Path', 'tool-path', ['L'], { checked: () => pressed('tool-path') }),
      btn('tool.mirror', 'Mirror X Symmetry', 'tool-mirror', ['M'], { checked: () => pressed('tool-mirror') }),
      { id: 'tool.quickMask', label: 'Toggle Quick Mask', keys: ['T'], run: () => setSelect('qm-mode', byId<HTMLSelectElement>('qm-mode')?.value === '1' ? '2' : '1') },
      { id: 'tool.brushBigger', label: 'Increase Brush Size', keys: [']'], run: () => nudgeBrush(1.15) },
      { id: 'tool.brushSmaller', label: 'Decrease Brush Size', keys: ['['], run: () => nudgeBrush(0.87) },
      { id: 'tool.lassoFill', label: 'Lasso Fill', keys: ['Q'], run: () => select('lasso', false) },
      { id: 'tool.rectFill', label: 'Rectangle Fill', keys: ['Shift+Q'], run: () => select('rect', false) },
      { id: 'tool.lassoLimit', label: 'Limit Strokes to Lasso', keys: ['Alt+Q'], run: () => select('lasso', true) },
      { id: 'tool.rectLimit', label: 'Limit Strokes to Rectangle', keys: ['Alt+Shift+Q'], run: () => select('rect', true) },
      { id: 'tool.pickColor', label: 'Pick Paint Color from Screen', keys: ['I'], enabled: () => 'EyeDropper' in window, run: () => void pickColor() },
      btn('layer.addPaint', 'Add Paint Layer', 'layer-add', ['Shift+Mod+N']),
      btn('layer.addFill', 'Add Fill Layer', 'layer-add-fill', ['Shift+Mod+F']),
      { id: 'layer.addFillBlack', label: 'Add Fill Layer with Black Mask', keys: ['Alt+Shift+Mod+F'], run: () => { click('layer-add-fill'); setSelect('l-mask', 'black'); } },
      btn('layer.addFilter', 'Add Filter Layer', 'layer-add-filter'),
      btn('layer.addFolder', 'Add Folder', 'layer-add-folder', ['Mod+G']),
      { id: 'layer.maskBlack', label: 'Add Black Mask', keys: ['Alt+Mod+M'], run: () => setSelect('l-mask', 'black') },
      { id: 'layer.maskWhite', label: 'Add White Mask', run: () => setSelect('l-mask', 'white') },
      { id: 'layer.maskNone', label: 'Remove Mask', run: () => setSelect('l-mask', 'none') },
      btn('layer.mergeDown', 'Merge Down', 'l-merge', ['Mod+E']),
      btn('layer.flattenFolder', 'Flatten Folder', 'l-flatten-folder'),
      btn('layer.flatten', 'Flatten Stack', 'l-flatten'),
      btn('layer.export', 'Export Layer', 'l-export'),
      btn('layer.exportMask', 'Export Mask', 'l-mask-export'),
      btn('layer.delete', 'Delete Layer', 'layer-delete', ['Mod+Backspace']),
      { id: 'layer.editExternal', label: 'Edit Layer in External App...', keys: ['Alt+Mod+E'], run: () => { byId('canvas')?.dispatchEvent(new Event('fs-edit-external')); } },
      { id: 'layer.pickUnderCursor', label: 'Select Layer Under Cursor', keys: ['Shift+S'], run: () => { byId('canvas')?.dispatchEvent(new Event('fs-pick-layer')); } },
      btn('bake.mesh', 'Bake Mesh Maps', 'k-bake', ['Shift+Mod+B']),
      btn('bake.loadHigh', 'Load High-Poly Mesh...', 'k-high-load'),
      btn('bake.high', 'Bake Maps from High-Poly', 'k-bake-high'),
      btn('view.frame', 'Frame Mesh', 'frame', ['F']),
      btn('view.2d', '2D View', 'view-2d', ['U'], { checked: () => pressed('view-2d') }),
      btn('view.render', 'Render to File', 'd-render'),
      { id: 'view.hidePart', label: 'Hide Part Under Cursor', keys: ['H'], run: () => hidePart(2, false) },
      { id: 'view.hideIsland', label: 'Hide UV Island Under Cursor', keys: ['Shift+H'], run: () => hidePart(3, false) },
      { id: 'view.isolatePart', label: 'Isolate Part Under Cursor', keys: ['Alt+Shift+H'], run: () => hidePart(2, true) },
      { id: 'view.showParts', label: 'Show All Parts', keys: ['Alt+H'], run: () => { byId('canvas')?.dispatchEvent(new Event('fs-show-parts')); } },
      ...PANEL_TABS.map((p, i) => ({ id: `view.tab.${p.id}`, label: `${p.label} Panel`, keys: [`Alt+${i + 1}`], checked: () => this.tab === p.id, run: () => this.showTab(p.id) })),
      { id: 'view.panel', label: 'Show Properties Panel', keys: ['Mod+\\'], checked: () => !this.app.classList.contains('is-panel-hidden'), run: () => this.togglePanel() },
      { id: 'view.terminal', label: 'Terminal', keys: ['Mod+J'], checked: () => this.app.classList.contains('is-dock-open'), run: () => void this.setDock(!this.app.classList.contains('is-dock-open')) },
      { id: 'view.newTerminal', label: 'New Terminal', run: () => void this.withDock((d) => d.open()) },
      { id: 'agent.claude', label: 'Claude Code', keys: ['Shift+Mod+J'], run: () => void this.withDock((d) => { this.linkAgent(); return d.claude(); }) },
      { id: 'agent.chat', label: 'Chat with Claude', keys: ['Alt+Mod+J'], run: () => void this.withDock((d) => { this.linkAgent(); return d.showChat(); }) },
      btn('agent.link', 'Agent Link', 'agent-link', undefined, { checked: () => pressed('agent-link') }),
      { id: 'agent.connect', label: 'Connect an Outside Agent...', run: () => this.connectHelp() },
      { id: 'help.about', label: 'About FormSurface', run: () => this.about() },
    );
  }

  private loadKeys(): void {
    const saved = read<Record<string, string[]>>(SHORTCUTS_KEY, {});
    for (const [id, keys] of Object.entries(saved)) if (Array.isArray(keys)) this.commands.setKeys(id, keys);
    this.syncTitles();
  }

  private syncTitles(): void {
    for (const node of document.querySelectorAll<HTMLElement>('[data-cmd]')) {
      const cmd = this.commands.get(node.dataset.cmd ?? '');
      if (!cmd) continue;
      const keys = this.commands.keyLabel(cmd);
      const label = t(this.commands.label(cmd));
      node.title = keys ? `${label} (${keys})` : label;
      node.setAttribute('aria-label', label);
    }
  }

  private linkAgent(): void {
    if (!pressed('agent-link')) click('agent-link');
  }

  private typing(e: KeyboardEvent): boolean {
    const target = e.target as HTMLElement | null;
    if (!target) return false;
    if (target.closest('.fl-dock-stage, .cv-dialog, .cv-palette')) return true;
    if (target.isContentEditable || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT') return true;
    return target.tagName === 'INPUT' && !['range', 'checkbox', 'radio', 'color', 'button'].includes((target as HTMLInputElement).type);
  }

  private onKey(e: KeyboardEvent): void {
    if (e.isComposing || ['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return;
    const cmd = this.commands.forEvent(e);
    if (!cmd) return;
    const plain = !e.ctrlKey && !e.metaKey && !e.altKey;
    if (this.typing(e) && (plain || !['edit.palette', 'view.terminal', 'agent.claude', 'agent.chat'].includes(cmd.id))) return;
    if (document.querySelector('.cv-scrim:not([hidden])') && cmd.id !== 'edit.palette') return;
    e.preventDefault();
    e.stopPropagation();
    if (e.repeat && !['edit.undo', 'edit.redo', 'tool.brushBigger', 'tool.brushSmaller'].includes(cmd.id)) return;
    this.commands.run(cmd.id);
  }

  private buildPanel(): void {
    const panel = q(document, '.panel');
    const strip = el<HTMLElement>('<div class="fs-tabs" role="tablist" aria-label="Panels"></div>');
    for (const p of PANEL_TABS) {
      const b = el<HTMLButtonElement>(`<button type="button" class="fs-tab" role="tab" data-tab="${p.id}"></button>`);
      setText(b, p.label);
      b.addEventListener('click', () => this.showTab(p.id));
      this.tabButtons.set(p.id, b);
      strip.appendChild(b);
    }
    panel.prepend(strip);
    const tabOf = new Map<string, string>();
    for (const p of PANEL_TABS) for (const s of p.sections) tabOf.set(s, p.id);
    for (const section of panel.querySelectorAll<HTMLElement>(':scope > .section')) {
      const head = section.querySelector('h2');
      const name = (head?.firstChild?.textContent ?? '').trim();
      const tab = tabOf.get(name) ?? 'export';
      this.sections.set(name, { node: section, tab });
      if (!head) continue;
      const fold = el<HTMLButtonElement>(`<button type="button" class="fs-fold" aria-expanded="true">${icon('chevronDown', 12)}</button>`);
      fold.setAttribute('aria-label', name);
      head.prepend(fold);
      fold.addEventListener('click', () => this.fold(name, !this.folded.has(name)));
      head.addEventListener('dblclick', (e) => { if (!(e.target as HTMLElement).closest('button:not(.fs-fold), select, input')) this.fold(name, !this.folded.has(name)); });
      if (this.folded.has(name)) this.applyFold(section, fold, true);
    }
    this.buildSwatches();
    if (!PANEL_TABS.some((p) => p.id === this.tab)) this.tab = 'paint';
    this.showTab(this.tab);
    if (read<boolean>(PANEL_KEY, true) === false) this.app.classList.add('is-panel-hidden');
  }

  private buildSwatches(): void {
    const input = byId<HTMLInputElement>('c-color');
    const row = input?.closest('.channel');
    if (!input || !row) return;
    let colors = read<string[]>(SWATCH_KEY, []).filter((c) => /^#[0-9a-f]{6}$/i.test(c)).slice(0, SWATCHES);
    let locked = read<boolean>(SWATCH_LOCK_KEY, false);
    const strip = el<HTMLElement>(`<div class="fs-swatches" role="group" aria-label="Recent colors"></div>`);
    const chips: HTMLButtonElement[] = [];
    for (let i = 0; i < SWATCHES; i++) {
      const chip = el<HTMLButtonElement>('<button type="button" class="fs-swatch" aria-label="Recent color"></button>');
      chip.addEventListener('click', () => {
        const c = colors[i];
        if (!c) return;
        input.value = c;
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
      });
      chips.push(chip);
      strip.appendChild(chip);
    }
    const lock = el<HTMLButtonElement>(`<button type="button" class="fs-swatch-lock" aria-pressed="false" title="Lock the palette: picked colors no longer replace these">${icon('lock', 12)}</button>`);
    strip.appendChild(lock);
    const sync = () => {
      chips.forEach((chip, i) => {
        const c = colors[i] ?? '';
        if (chip.style.backgroundColor !== c) chip.style.backgroundColor = c;
        setClass(chip, 'is-empty', !c);
        chip.title = c;
      });
      lock.setAttribute('aria-pressed', String(locked));
    };
    lock.addEventListener('click', () => { locked = !locked; write(SWATCH_LOCK_KEY, locked); sync(); });
    input.addEventListener('change', () => {
      if (locked) return;
      const c = input.value.toLowerCase();
      colors = [c, ...colors.filter((x) => x !== c)].slice(0, SWATCHES);
      write(SWATCH_KEY, colors);
      sync();
    });
    row.after(strip);
    sync();
    mountColorWheel(input, strip);
  }

  private applyFold(section: HTMLElement, fold: HTMLElement, on: boolean): void {
    setClass(section, 'is-folded', on);
    fold.setAttribute('aria-expanded', String(!on));
  }

  private fold(name: string, on: boolean): void {
    const s = this.sections.get(name);
    if (!s) return;
    if (on) this.folded.add(name); else this.folded.delete(name);
    this.applyFold(s.node, q(s.node, '.fs-fold'), on);
    write(FOLD_KEY, [...this.folded]);
  }

  showTab(id: string): void {
    this.tab = id;
    write(TAB_KEY, id);
    for (const [tid, b] of this.tabButtons) { setClass(b, 'is-on', tid === id); b.setAttribute('aria-selected', String(tid === id)); }
    for (const s of this.sections.values()) setHidden(s.node, s.tab !== id);
    const panel = q(document, '.panel');
    if (this.app.classList.contains('is-panel-hidden')) this.togglePanel();
    panel.scrollTop = 0;
  }

  private togglePanel(): void {
    const hidden = !this.app.classList.contains('is-panel-hidden');
    setClass(this.app, 'is-panel-hidden', hidden);
    write(PANEL_KEY, !hidden);
    window.dispatchEvent(new Event('resize'));
  }

  private async withDock(fn: (d: TerminalDock) => Promise<void>): Promise<void> {
    await this.setDock(true);
    if (this.dock) await fn(this.dock);
  }

  private async setDock(open: boolean): Promise<void> {
    if (open && !this.dock) {
      this.dock = new TerminalDock(this.dockHost, api, () => void this.setDock(false));
      this.dock.onFail = fail;
      this.dockHost.addEventListener('click', (e) => { if ((e.target as HTMLElement).closest('.fl-dock-chat, .fl-dock-claude')) this.linkAgent(); }, true);
      this.dockHost.addEventListener('submit', () => this.linkAgent(), true);
      this.dock.context = () => {
        const set = document.querySelector('.set-row.active .name')?.textContent ?? '';
        const layer = document.querySelector('.layer.active .name')?.textContent ?? '';
        const parts = [set && `texture set ${set}`, layer && `active layer ${layer}`].filter(Boolean).join(', ');
        return parts ? `FormSurface window: ${parts}` : '';
      };
      this.resizer();
    }
    setHidden(this.dockHost, !open);
    setClass(this.app, 'is-dock-open', open);
    const h = parseInt(this.app.style.getPropertyValue('--fs-dock-open-h'), 10) || 280;
    write(DOCK_KEY, { open, h });
    window.dispatchEvent(new Event('resize'));
    if (open && this.dock) await this.dock.ensure();
    else (byId('canvas'))?.focus();
  }

  private resizer(): void {
    const grip = this.dockHost.querySelector<HTMLElement>('.fl-dock-rsz');
    if (!grip) return;
    grip.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      grip.setPointerCapture(e.pointerId);
      const startY = e.clientY;
      const startH = this.dockHost.getBoundingClientRect().height;
      const move = (ev: PointerEvent) => {
        const h = Math.round(Math.max(140, Math.min(window.innerHeight * 0.7, startH + startY - ev.clientY)));
        this.app.style.setProperty('--fs-dock-open-h', `${h}px`);
      };
      const up = () => {
        grip.removeEventListener('pointermove', move);
        grip.removeEventListener('pointerup', up);
        write(DOCK_KEY, { open: true, h: parseInt(this.app.style.getPropertyValue('--fs-dock-open-h'), 10) || 280 });
        window.dispatchEvent(new Event('resize'));
      };
      grip.addEventListener('pointermove', move);
      grip.addEventListener('pointerup', up);
    });
  }

  private openPalette(): void {
    if (!this.palette) this.palette = new CommandPalette(this.commands, () => this.menus());
    this.palette.open();
  }

  private shortcuts(): void {
    const d = new Dialog('Keyboard Shortcuts', 'wide');
    const list = el<HTMLElement>('<div class="fs-keys"></div>');
    const note = el<HTMLElement>('<p class="cv-note">Click a shortcut, then press the new keys. Escape cancels, Backspace clears.</p>');
    d.body.append(note, list);
    const rows = this.commands.all().filter((c) => !c.id.startsWith('file.sample.'));
    for (const cmd of rows) {
      const row = el<HTMLElement>('<div class="fs-keys-row"><span class="fs-keys-label"></span><button type="button" class="fs-keys-combo"></button></div>');
      setText(q(row, '.fs-keys-label'), t(this.commands.label(cmd)));
      const combo = q<HTMLButtonElement>(row, '.fs-keys-combo');
      const show = () => setText(combo, (cmd.keys ?? []).map(formatKeys).join(', ') || t('None'));
      show();
      combo.addEventListener('click', () => {
        setText(combo, t('Press keys'));
        const grab = (e: KeyboardEvent) => {
          if (['Shift', 'Control', 'Alt', 'Meta'].includes(e.key)) return;
          e.preventDefault(); e.stopPropagation();
          window.removeEventListener('keydown', grab, true);
          if (e.key === 'Escape') { show(); return; }
          this.commands.setKeys(cmd.id, e.key === 'Backspace' ? [] : [comboOf(e)]);
          write(SHORTCUTS_KEY, this.commands.overrides());
          this.syncTitles();
          for (const b of list.querySelectorAll<HTMLButtonElement>('.fs-keys-combo')) b.dispatchEvent(new Event('fs-refresh'));
        };
        window.addEventListener('keydown', grab, true);
      });
      combo.addEventListener('fs-refresh', show);
      list.appendChild(row);
    }
    d.action('Reset All', () => { this.commands.resetKeys(); write(SHORTCUTS_KEY, {}); this.syncTitles(); for (const b of list.querySelectorAll<HTMLButtonElement>('.fs-keys-combo')) b.dispatchEvent(new Event('fs-refresh')); }, false);
    d.action('Done', () => d.close(), true);
    d.open();
  }

  private async preferences(): Promise<void> {
    const d = new Dialog('Preferences');
    const prefs = await api.cmd<Record<string, string>>('prefs.get').catch(() => ({} as Record<string, string>));
    const form = el<HTMLElement>(`<div class="fs-form">
<h3>Interface</h3>
<label class="fs-field"><span>Language</span><select class="fs-lang" translate="no"></select></label>
<label class="fs-field"><span>Theme</span><select class="fs-theme"></select></label>
<h3>AI agents</h3>
<label class="fs-field"><span>Claude Code program</span><input class="fs-claude-bin" type="text" spellcheck="false" placeholder="Found automatically"></label>
<label class="fs-field"><span>Starts in folder</span><input class="fs-claude-folder" type="text" spellcheck="false" placeholder="FormSurface Agent folder"></label>
<label class="fs-field"><span>Start with message</span><input class="fs-claude-first" type="text" spellcheck="false" placeholder="Nothing"></label>
<h3>Other apps</h3>
<label class="fs-field"><span>External image editor</span><input class="fs-external" type="text" spellcheck="false" placeholder="The default app for PNG files"></label>
<p class="cv-note fs-claude-state"></p>
</div>`);
    const lang = q<HTMLSelectElement>(form, '.fs-lang');
    for (const [code, name] of languages()) { const o = document.createElement('option'); o.value = code; o.textContent = name; lang.appendChild(o); }
    lang.value = language();
    const themePick = q<HTMLSelectElement>(form, '.fs-theme');
    for (const [id, label] of THEMES) { const o = document.createElement('option'); o.value = id; o.textContent = label; themePick.appendChild(o); }
    themePick.value = currentTheme();
    themePick.addEventListener('change', () => setTheme(themePick.value));
    q<HTMLInputElement>(form, '.fs-claude-bin').value = prefs.claudeBin ?? '';
    q<HTMLInputElement>(form, '.fs-claude-folder').value = prefs.claudeFolder ?? '';
    q<HTMLInputElement>(form, '.fs-claude-first').value = prefs.claudeFirstMessage ?? '';
    q<HTMLInputElement>(form, '.fs-external').value = prefs.externalEditor ?? '';
    d.body.appendChild(form);
    void api.cmd<{ installed: boolean; path: string | null; version: string | null; folder: string }>('claudecode.status').then((s) => {
      setText(q(form, '.fs-claude-state'), s.installed ? `${t('Claude Code')} ${s.version ?? ''} · ${s.path ?? ''} · ${s.folder}` : t('Claude Code was not found on this computer: install it from claude.com/claude-code and sign in once in a terminal.'));
    }).catch(() => undefined);
    d.action('Cancel', () => d.close(), false);
    d.action('Save', async () => {
      try {
        await api.cmd('prefs.set', { values: {
          claudeBin: q<HTMLInputElement>(form, '.fs-claude-bin').value.trim(),
          claudeFolder: q<HTMLInputElement>(form, '.fs-claude-folder').value.trim(),
          claudeFirstMessage: q<HTMLInputElement>(form, '.fs-claude-first').value.trim(),
          externalEditor: q<HTMLInputElement>(form, '.fs-external').value.trim(),
        } });
      } catch (err) { fail(err); }
      d.close();
      if (lang.value !== language()) setLanguage(lang.value as Lang);
    }, true);
    d.open();
  }

  private connectHelp(): void {
    const d = new Dialog('Connect an Outside Agent');
    const body = el<HTMLElement>(`<div class="fs-form">
<p>Claude Code, Codex and Cursor drive FormSurface through its MCP server. Turn on Agent Link in this window, then register the server once:</p>
<pre class="fs-code" translate="no">claude mcp add --scope user formsurface -- /path/to/formsurface/formsurface mcp</pre>
<p>Inside this repo, the .mcp.json file already registers it for Claude Code. The Terminal, Claude Code and Chat buttons in the title bar start Claude Code right here, already connected.</p>
</div>`);
    d.body.appendChild(body);
    d.action('Turn On Agent Link', () => { this.linkAgent(); d.close(); }, false);
    d.action('Done', () => d.close(), true);
    d.open();
  }

  private about(): void {
    const d = new Dialog('About FormSurface');
    const body = el<HTMLElement>('<div class="fs-form"><p class="fs-about-version"></p><p>3D texture painting on a Rust and WebGPU engine. Apache License 2.0.</p></div>');
    setText(q(body, '.fs-about-version'), `FormSurface ${byId('version')?.textContent ?? ''}`);
    d.body.appendChild(body);
    d.action('Done', () => d.close(), true);
    d.open();
  }
}

export function startShell(): Shell {
  const shell = new Shell();
  void Promise.race([
    (document.fonts?.ready ?? Promise.resolve()).catch(() => undefined),
    new Promise((resolve) => setTimeout(resolve, 2500)),
  ]).then(revealApp);
  return shell;
}

export const MOD_LABEL = isMac ? 'Cmd' : 'Ctrl';
