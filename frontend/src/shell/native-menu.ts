import type { Commands } from './commands';
import type { MenuDef, MenuItem as MenuSpec } from './menu';
import { t } from '../i18n';

export const MENUS_IN_WINDOW_KEY = 'formsurface.menusInWindow';
export function menusInWindow(): boolean {
  try { return localStorage.getItem(MENUS_IN_WINDOW_KEY) === '1'; } catch { return false; }
}
export const isMacDesktop = (): boolean => '__TAURI_INTERNALS__' in window && /Mac/.test(navigator.platform);
export const wantsNativeMenus = (): boolean => isMacDesktop() && !menusInWindow();

const NAMES: Record<string, string> = { Mod: 'CmdOrCtrl', Alt: 'Alt', Shift: 'Shift', Ctrl: 'Ctrl', Plus: 'Plus', '=': '=', '-': '-', '[': '[', ']': ']', ',': ',', '.': '.', '/': '/', ';': ';', "'": "'", '\\': '\\', '`': '`', Delete: 'Delete', Backspace: 'Backspace', Escape: 'Escape', Enter: 'Enter', Tab: 'Tab', Space: 'Space', ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', Up: 'Up', Down: 'Down', Left: 'Left', Right: 'Right', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown' };

export function accelerator(combo: string | undefined): string | null {
  if (!combo) return null;
  const parts = combo.endsWith('++') ? [...combo.slice(0, -2).split('+'), 'Plus'] : combo.split('+');
  const key = parts[parts.length - 1];
  const fkey = /^F([1-9]|1[0-2])$/.test(key);
  if (!fkey && !parts.some((p) => p === 'Mod' || p === 'Ctrl')) return null;
  const out: string[] = [];
  for (const p of parts) {
    if (NAMES[p]) out.push(NAMES[p]);
    else if (/^[A-Za-z0-9]$/.test(p)) out.push(p.toUpperCase());
    else if (/^F([1-9]|1[0-2])$/.test(p)) out.push(p);
    else return null;
  }
  return out.join('+');
}

const TEXT_EDITS = new Map<string, string>([['edit.undo', 'undo'], ['edit.redo', 'redo'], ['edit.cut', 'cut'], ['edit.copy', 'copy'], ['edit.paste', 'paste'], ['edit.selectAll', 'select-all']]);
const NOT_TEXT = new Set(['button', 'checkbox', 'radio', 'range', 'color', 'file', 'submit', 'reset', 'image']);

export function textFieldFocused(): boolean {
  const a = document.activeElement;
  if (a instanceof HTMLTextAreaElement) return !a.readOnly && !a.disabled;
  if (a instanceof HTMLInputElement) return !NOT_TEXT.has(a.type) && !a.readOnly && !a.disabled;
  return a instanceof HTMLElement && a.isContentEditable;
}

const dialogOpen = (): boolean => !!document.querySelector('.cv-scrim:not([hidden])');

type Api = typeof import('@tauri-apps/api/menu');
type Live = { setEnabled(on: boolean): Promise<void>; setChecked?(on: boolean): Promise<void> };

export class NativeMenus {
  allow: ((id: string) => boolean) | null = null;
  appMenu: { name: string; about: string; preferences: string; copyright: string; license: string } = { name: 'FormSurface', about: 'About FormSurface', preferences: 'file.preferences', copyright: 'Copyright 2026 nodemode-ai. Apache-2.0', license: 'Apache-2.0' };
  private items: Array<{ id: string; item: Live; on: boolean; tick: boolean | null }> = [];
  private readonly clicked = new Set<string>();
  private building = false;
  private again = false;
  private stateTimer = 0;
  private shape = '';
  private inflight = '';

  constructor(private readonly commands: Commands, private readonly menus: () => MenuDef[]) {
    for (const ev of ['pointerup', 'keyup', 'focusin', 'focusout'] as const) window.addEventListener(ev, () => { if (this.items.length) this.refresh(); }, { passive: true });
  }

  private signature(): string {
    const walk = (list: MenuSpec[]): unknown[] => list.map((i) => {
      if (i === '-') return i;
      if (typeof i === 'object') return [i.label, walk(i.items)];
      const cmd = this.commands.get(i);
      return cmd ? [i, this.commands.label(cmd), cmd.keys?.[0] ?? '', !!cmd.checked] : null;
    });
    return JSON.stringify(this.menus().map((d) => [d.label, walk(typeof d.items === 'function' ? d.items() : d.items)]));
  }

  async build(force = false): Promise<void> {
    const sigNow = this.signature();
    if (this.building) {
      if (!force && sigNow === this.inflight) return;
      this.again = true;
      return;
    }
    if (!force && this.items.length && sigNow === this.shape) return;
    this.building = true;
    this.inflight = sigNow;
    try {
      const api: Api = await import('@tauri-apps/api/menu');
      const items: typeof this.items = [];
      const taken = new Set<string>();
      const sep = (): Promise<unknown> => api.PredefinedMenuItem.new({ item: 'Separator' });
      const make = async (spec: MenuSpec): Promise<unknown> => {
        if (spec === '-') return sep();
        if (typeof spec === 'object') return api.Submenu.new({ text: t(spec.label), items: (await list(spec.items)) as never[] });
        const cmd = this.commands.get(spec);
        if (!cmd) return null;
        const text = t(this.commands.label(cmd));
        const edit = TEXT_EDITS.get(spec);
        const found = accelerator(cmd.keys?.[0]);
        const acc = found && !taken.has(found) ? found : null;
        if (acc) taken.add(acc);
        const opts = { text, enabled: this.enabled(spec), action: () => {
          if (edit && textFieldFocused()) void import('@tauri-apps/api/core').then((m) => m.invoke('text_edit', { action: edit })).catch(() => { document.execCommand(edit === 'select-all' ? 'selectAll' : edit); });
          else if (this.enabled(spec)) this.commands.run(spec);
          this.clicked.add(spec);
          this.refresh();
        } };
        const on = opts.enabled, tick = cmd.checked ? !!cmd.checked() : null;
        let item: Live;
        if (tick !== null) {
          const copts = { ...opts, checked: tick };
          try { item = await api.CheckMenuItem.new(acc ? { ...copts, accelerator: acc } : { ...copts }); } catch { item = await api.CheckMenuItem.new(copts); }
        } else {
          try { item = await api.MenuItem.new(acc ? { ...opts, accelerator: acc } : { ...opts }); } catch { item = await api.MenuItem.new(opts); }
        }
        items.push({ id: spec, item, on, tick });
        return item;
      };
      const list = async (specs: MenuSpec[]): Promise<unknown[]> => (await Promise.all(specs.map(make))).filter(Boolean);
      const app = this.appMenu;
      const top: unknown[] = [];
      top.push(await api.Submenu.new({ text: app.name, items: [
        await api.PredefinedMenuItem.new({ item: { About: { name: app.name, copyright: app.copyright, license: app.license } }, text: t(app.about) }),
        await sep(),
        ...(this.commands.get(app.preferences) ? [await make(app.preferences)] : []),
        await sep(),
        await api.PredefinedMenuItem.new({ item: 'Services' }),
        await sep(),
        await api.PredefinedMenuItem.new({ item: 'Hide' }),
        await api.PredefinedMenuItem.new({ item: 'HideOthers' }),
        await api.PredefinedMenuItem.new({ item: 'ShowAll' }),
        await sep(),
        await api.PredefinedMenuItem.new({ item: 'Quit' }),
      ] as never[] }));
      const windowItems = async (): Promise<unknown[]> => [
        await api.PredefinedMenuItem.new({ item: 'Minimize' }),
        await api.PredefinedMenuItem.new({ item: 'Maximize' }),
        await api.PredefinedMenuItem.new({ item: 'Fullscreen' }),
        await sep(),
        await api.PredefinedMenuItem.new({ item: 'BringAllToFront' }),
      ];
      const defs = this.menus();
      const ownWindow = defs.some((d) => d.label === 'Window');
      let help: { setAsHelpMenuForNSApp(): Promise<void> } | null = null;
      let win: { setAsWindowsMenuForNSApp(): Promise<void> } | null = null;
      for (const def of defs) {
        const made = await list(typeof def.items === 'function' ? def.items() : def.items);
        if (def.label === 'Help' && !ownWindow) {
          win = await api.Submenu.new({ text: t('Window'), items: (await windowItems()) as never[] });
          top.push(win);
        }
        if (def.label === 'Window') {
          if (made.length) made.push(await sep());
          made.push(...(await windowItems()));
        }
        const sub = await api.Submenu.new({ text: t(def.label), items: made as never[] });
        if (def.label === 'Help') help = sub;
        if (def.label === 'Window') win = sub;
        top.push(sub);
      }
      const menu = await api.Menu.new({ items: top as never[] });
      await menu.setAsAppMenu();
      if (win) void win.setAsWindowsMenuForNSApp().catch(() => undefined);
      if (help) void help.setAsHelpMenuForNSApp().catch(() => undefined);
      this.items = items;
      this.shape = sigNow;
      document.documentElement.classList.add('has-native-menus');
    } catch (err) {
      if (!this.items.length) document.documentElement.classList.remove('has-native-menus');
      throw err;
    } finally {
      this.building = false;
      if (this.again) { this.again = false; void this.build().catch(() => undefined); }
    }
  }

  private enabled(id: string): boolean {
    if (TEXT_EDITS.has(id) && textFieldFocused()) return true;
    if (dialogOpen() || (this.allow && !this.allow(id))) return false;
    const cmd = this.commands.get(id);
    return !!cmd && this.commands.enabled(cmd);
  }

  refresh(): void {
    if (this.stateTimer || !this.items.length) return;
    this.stateTimer = window.setTimeout(() => {
      this.stateTimer = 0;
      if (this.signature() !== this.shape) { void this.build().catch(() => undefined); return; }
      for (const entry of this.items) {
        const cmd = this.commands.get(entry.id);
        if (!cmd) continue;
        const on = this.enabled(entry.id);
        if (on !== entry.on) { entry.on = on; void entry.item.setEnabled(on).catch(() => undefined); }
        if (!cmd.checked || !entry.item.setChecked) continue;
        const tick = !!cmd.checked();
        if (tick !== entry.tick || this.clicked.has(entry.id)) { entry.tick = tick; void entry.item.setChecked(tick).catch(() => undefined); }
      }
      this.clicked.clear();
    }, 150);
  }
}
