import '../styles/home.css';
import { t } from '../i18n';
import { brandMark } from './brand';
import { DEMOS, type DemoId } from './demos';
import { Dialog } from './dialog';
import { el, escapeHtml, patchList, setClass, setDisabled, setHidden, setText, swapMedia, type PatchListOptions } from './dom';
import { icon } from './icons';
import { library, type ProjectSummary, type TemplateSummary, type TrashedProject } from './project-library';
import { dismissToast, pushToast, toast, toastRoot } from './toast';

export type HomePage = 'start' | 'recent' | 'templates' | 'trash';

export interface HomeHooks {
  openProject(id: string): Promise<void>;
  closeProject(id: string): void;
  createProject(template: string, title?: string): Promise<void>;
  openFiles(files: File[]): void;
  pickMesh(): void;
  preferences(): void;
  agents(): void;
  demo(id: DemoId): void;
  hasDocument(): boolean;
  onVisible?(open: boolean): void;
}

const NAV_KEY = 'formsurface.homeNav';
const NAV_HIDDEN_KEY = 'formsurface.homeNavHidden';
const PREF_LAUNCH = 'formsurface.autoShowHome';
const PREF_CLOSE = 'formsurface.homeOnLastClose';

function pref(key: string): boolean {
  try { return localStorage.getItem(key) !== 'false'; } catch { return true; }
}
function setPref(key: string, on: boolean): void {
  try { localStorage.setItem(key, JSON.stringify(on)); } catch { return; }
}

export const homePrefs = {
  autoShow: (): boolean => pref(PREF_LAUNCH),
  afterClose: (): boolean => pref(PREF_CLOSE),
  setAutoShow(on: boolean): void { setPref(PREF_LAUNCH, on); home?.sync(); },
  setAfterClose(on: boolean): void { setPref(PREF_CLOSE, on); home?.sync(); },
};

function tf(pattern: string, ...args: Array<string | number>): string {
  return t(pattern).replace(/\{(\d+)\}/g, (_all, i: string) => String(args[Number(i)] ?? ''));
}

const why = (e: unknown): string => (e instanceof Error ? e.message : String(e));

function ago(iso?: string): string {
  if (!iso) return '';
  const s = (Date.now() - Date.parse(iso)) / 1000;
  if (s < 60) return t('just now');
  if (s < 3600) return tf('{0} min ago', Math.round(s / 60));
  if (s < 86400) return tf('{0} h ago', Math.round(s / 3600));
  return new Date(iso).toLocaleDateString([], { month: 'short', day: 'numeric' });
}

function patch<T>(container: HTMLElement, items: T[], opts: PatchListOptions<T>): void {
  const keep = new Set(items.map(opts.key));
  for (const child of Array.from(container.children) as HTMLElement[]) if (child.dataset.key !== undefined && !keep.has(child.dataset.key)) child.remove();
  patchList(container, items, opts);
}

const layerCount =(n: number | null): string => (n === null ? '' : n === 1 ? t('1 layer') : tf('{0} layers', n));

function formats(): string[] {
  const accept = (document.getElementById('mesh-file') as HTMLInputElement | null)?.accept ?? '';
  return accept.split(',').map((x) => x.trim().replace(/^\./, '').toUpperCase()).filter(Boolean);
}

export class Home {
  readonly root: HTMLElement;
  private view: HomePage = 'start';
  private templates: TemplateSummary[] = [];
  private categories: string[] = ['All'];
  private defaultTemplate = 'sphere';
  private category = 'All';
  private query = '';
  private projects: ProjectSummary[] = [];
  private trashed: TrashedProject[] = [];
  private pinned: string[] = [];
  private hiddenRecent: string[] = [];
  private readonly selected = new Set<string>();
  private anchor = '';
  private readonly tplGrid: HTMLElement;
  private readonly tplGridFull: HTMLElement;
  private readonly recentGrid: HTMLElement;
  private readonly recentGridFull: HTMLElement;
  private readonly trashList: HTMLElement;
  private readonly confirmBar: HTMLElement;
  private readonly menu: HTMLElement;
  private readonly back: HTMLElement;
  private onConfirm: (() => void) | null = null;
  private forced = false;
  private hadDocument = false;
  private loads = 0;
  private busy = false;

  constructor(private readonly hooks: HomeHooks) {
    const nav = (v: HomePage, ic: string, label: string, count = ''): string => `<button class="fs-home-nav-item${v === 'start' ? ' is-active' : ''}" type="button" data-v="${v}">${icon(ic, 16)}<span>${label}</span>${count ? `<span class="fs-badge is-count ${count}">0</span>` : ''}</button>`;
    const chips = formats().map((f) => `<span class="fs-home-format">${escapeHtml(f)}</span>`).join('');
    const demos = DEMOS.length ? `<section class="fs-home-section" data-section="demos"><div class="fs-home-section-head"><h2 class="fs-home-section-title">Demos</h2><span class="fs-card-meta">Each opens as a new project in your library</span></div><div class="fs-card-grid fs-demo-grid">${DEMOS.map((d) => `<button class="fs-card" type="button" data-demo="${escapeHtml(d.id)}"><span class="fs-card-preview" aria-hidden="true"><img alt="" decoding="async" draggable="false" src="${escapeHtml(d.picture)}"></span><span class="fs-card-body"><span class="fs-card-name" translate="no">${escapeHtml(d.title)}</span><span class="fs-card-meta" title="${escapeHtml(d.line)}">${escapeHtml(d.line)}</span></span></button>`).join('')}</div></section>` : '';
    this.root = el(`<section class="fs-home fs-family" aria-label="Home" tabindex="-1" hidden>
<div class="fs-home-drag" data-tauri-drag-region></div>
<nav class="fs-home-nav" aria-label="Home">
<div class="fs-home-brand" translate="no">${brandMark(26)}FormSurface <span>by nodemode.ai</span></div>
${nav('start', 'home', 'Start')}${nav('recent', 'layers', 'Recent projects', 'fs-project-count')}${nav('templates', 'template', 'Templates', 'fs-tpl-count')}${nav('trash', 'trash', 'Trash', 'fs-trash-count')}
<div class="fs-home-open" hidden><div class="fs-home-nav-sep"></div><div class="fs-home-nav-title">Open project</div><div class="fs-home-open-list"></div></div>
<div class="fs-home-nav-sep"></div>
<div class="fs-home-nav-title">Agents</div>
<button class="fs-home-nav-item" type="button" data-a="agents">${icon('agent', 16)}<span>Claude Code, Codex, Cursor</span></button>
<div class="fs-home-nav-foot">
<button class="fs-home-nav-item fs-home-back" type="button" data-a="back" hidden>${icon('chevronLeft', 16)}<span>Back to Workspace</span></button>
<button class="fs-home-nav-item" type="button" data-a="local" title="Your projects stay on this computer">${icon('cloudOff', 16)}<span>Local only</span></button>
<button class="fs-home-nav-item" type="button" data-a="prefs">${icon('settings', 16)}<span>Preferences</span></button>
</div></nav>
<div class="fs-home-resizer" role="separator" aria-orientation="vertical" aria-label="Resize the side panel"></div>
<button class="fs-home-nav-hide" type="button" title="Hide the side panel" aria-label="Hide the side panel">${icon('chevronLeft', 14)}</button>
<button class="fs-home-nav-show" type="button" title="Show the side panel" aria-label="Show the side panel">${icon('chevronRight', 14)}</button>
<div class="fs-home-main">
<div class="fs-home-narrow"><button class="fs-home-nav-item is-active" type="button" data-v="start">${icon('home', 16)}<span>Start</span></button><button class="fs-home-nav-item" type="button" data-v="recent">${icon('layers', 16)}<span>Recent</span></button><button class="fs-home-nav-item" type="button" data-v="templates">${icon('template', 16)}<span>Templates</span></button><button class="fs-home-nav-item" type="button" data-v="trash">${icon('trash', 16)}<span>Trash</span></button></div>
<div class="fs-home-confirm" role="alertdialog" aria-live="assertive" hidden><span class="fs-home-confirm-text"></span><button class="fs-btn fs-btn-sm" type="button" data-a="confirm-no">Cancel</button><button class="fs-btn fs-btn-sm fs-btn-danger" type="button" data-a="confirm-yes"></button></div>
<div data-page="start">
<div class="fs-home-head"><div><h1 class="fs-home-title"></h1><p class="fs-home-sub">Paint a mesh of your own, or start from a template.</p></div><div class="fs-home-head-actions"><button class="fs-btn" type="button" data-a="open">${icon('folderOpen', 14)}Open mesh</button><button class="fs-btn" type="button" data-a="blank">${icon('add', 14)}New project</button></div></div>
<div class="fs-home-hero">
<span class="fs-home-hero-icon">${icon('inbox', 26)}</span>
<div class="fs-home-hero-text"><div class="fs-home-hero-title">Start from your mesh</div>
<div class="fs-home-hero-sub">Drop a model anywhere in this window, or choose one. FormSurface paints it in 3D with layers, masks and materials, and your agent can take it from there.</div>
<div class="fs-home-formats" translate="no">${chips}</div></div>
<div class="fs-home-hero-actions"><button class="fs-btn fs-btn-primary" type="button" data-a="choose">${icon('upload', 14)}Choose a mesh</button><button class="fs-btn" type="button" data-v="templates">${icon('template', 14)}Start from a template</button><button class="fs-btn fs-btn-ghost" type="button" data-a="agents">${icon('agent', 14)}Use your agent</button></div>
</div>
<section class="fs-home-section"><div class="fs-home-section-head"><h2 class="fs-home-section-title">Recent projects</h2><button class="fs-btn fs-btn-ghost fs-btn-sm" type="button" data-v="recent">See all</button></div><div class="fs-card-grid fs-recent-grid"><button class="fs-card is-skeleton" type="button" tabindex="-1" data-key="__skeleton" aria-hidden="true"><span class="fs-card-preview"></span><span class="fs-card-body"><span class="fs-card-name">&nbsp;</span><span class="fs-card-meta">&nbsp;</span></span></button><button class="fs-card is-empty" type="button" tabindex="-1" hidden><span class="fs-card-preview is-blank">${icon('cube', 28)}</span><span class="fs-card-body"><span class="fs-card-name">No projects yet</span><span class="fs-card-meta">Start from your mesh or a template</span></span></button></div></section>
${demos}
<section class="fs-home-section"><div class="fs-home-section-head"><h2 class="fs-home-section-title">Templates</h2><div class="fs-home-filters fs-cats"></div><label class="fs-search">${icon('search', 14)}<input type="search" placeholder="Search templates" aria-label="Search templates" class="fs-tpl-search"></label></div><div class="fs-card-grid fs-tpl-grid"></div><p class="fs-note fs-tpl-none" hidden>No templates match.</p></section>
</div>
<div data-page="recent" hidden><div class="fs-home-head"><div><h1 class="fs-home-title">Recent projects</h1><p class="fs-home-sub">Everything in your library, newest first. A project is saved when you choose Save Project.</p></div></div><div class="fs-card-grid fs-recent-full"></div><p class="fs-note fs-recent-none" hidden>No projects yet. Start from your mesh or a template, then save.</p></div>
<div data-page="trash" hidden><div class="fs-home-head"><div><h1 class="fs-home-title">Trash</h1><p class="fs-home-sub">Deleted projects wait here until you restore them or delete them permanently.</p></div><div class="fs-home-head-actions"><button class="fs-btn fs-btn-empty-trash" type="button" data-a="empty-trash" disabled>${icon('trash', 14)}Empty Trash</button></div></div><div class="fs-trash-list"></div><p class="fs-note fs-trash-none">Trash is empty.</p></div>
<div data-page="templates" hidden><div class="fs-home-head"><div><h1 class="fs-home-title">Templates</h1><p class="fs-home-sub">Starting projects on meshes FormSurface makes itself. Each one opens as a new project in your library.</p></div></div>
<div class="fs-home-section-head"><div class="fs-home-filters fs-cats"></div><label class="fs-search">${icon('search', 14)}<input type="search" placeholder="Search templates" aria-label="Search templates" class="fs-tpl-search"></label></div><div class="fs-card-grid fs-tpl-full"></div><p class="fs-note fs-tpl-none" hidden>No templates match.</p></div>
</div></section>`);
    const find = (sel: string): HTMLElement => this.root.querySelector(sel) as HTMLElement;
    this.tplGrid = find('.fs-tpl-grid');
    this.tplGridFull = find('.fs-tpl-full');
    this.recentGrid = find('.fs-recent-grid');
    this.recentGridFull = find('.fs-recent-full');
    this.trashList = find('.fs-trash-list');
    this.confirmBar = find('.fs-home-confirm');
    this.back = find('.fs-home-back');
    this.menu = el('<div class="fs-menu fs-home-menu fs-family" role="menu" hidden></div>');
    document.getElementById('overlay-root')?.appendChild(this.menu);
    toastRoot();
    this.greet();
    (document.querySelector('.app') ?? document.body).appendChild(this.root);
    this.wire();
    this.sidePanel();
    this.renderTemplates();
  }

  private greet(): void {
    const h = new Date().getHours();
    setText(this.root.querySelector('[data-page="start"] .fs-home-title'), t(h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening'));
  }

  private sidePanel(): void {
    const root = this.root;
    const bar = root.querySelector<HTMLElement>('.fs-home-resizer');
    if (!bar) return;
    const save = (): void => {
      try {
        localStorage.setItem(NAV_KEY, root.style.getPropertyValue('--fs-home-nav-w'));
        localStorage.setItem(NAV_HIDDEN_KEY, root.classList.contains('is-nav-hidden') ? '1' : '0');
      } catch { return; }
    };
    const hide = (on: boolean): void => { setClass(root, 'is-nav-hidden', on); save(); };
    try {
      const w = localStorage.getItem(NAV_KEY);
      if (w && /^\d{2,4}px$/.test(w.trim())) root.style.setProperty('--fs-home-nav-w', `${Math.max(180, Math.min(420, parseInt(w, 10)))}px`);
      if (localStorage.getItem(NAV_HIDDEN_KEY) === '1') root.classList.add('is-nav-hidden');
    } catch { void 0; }
    root.querySelector('.fs-home-nav-hide')?.addEventListener('click', () => hide(true));
    root.querySelector('.fs-home-nav-show')?.addEventListener('click', () => hide(false));
    bar.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      const x0 = e.clientX;
      const start = root.querySelector('.fs-home-nav')?.getBoundingClientRect().width || 250;
      bar.classList.add('is-dragging');
      const mv = (ev: PointerEvent): void => {
        const raw = start + ev.clientX - x0;
        const collapse = raw < 120;
        if (root.classList.contains('is-nav-hidden') !== collapse) setClass(root, 'is-nav-hidden', collapse);
        if (!collapse) root.style.setProperty('--fs-home-nav-w', `${Math.round(Math.max(180, Math.min(420, raw)))}px`);
      };
      const up = (): void => {
        bar.classList.remove('is-dragging');
        window.removeEventListener('pointermove', mv);
        window.removeEventListener('pointerup', up);
        window.removeEventListener('pointercancel', up);
        save();
      };
      window.addEventListener('pointermove', mv);
      window.addEventListener('pointerup', up);
      window.addEventListener('pointercancel', up);
    });
  }

  private wire(): void {
    this.root.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      this.closeMenu();
      const v = (target.closest('[data-v]') as HTMLElement | null)?.dataset.v as HomePage | undefined;
      if (v) { this.page(v); return; }
      if (target.closest('[data-open-project]')) { this.leave(); return; }
      const a = (target.closest('[data-a]') as HTMLElement | null)?.dataset.a;
      const row = (target.closest('[data-entry]') as HTMLElement | null)?.dataset.entry;
      if (a === 'confirm-yes') { const go = this.onConfirm; this.askClose(); go?.(); return; }
      if (a === 'confirm-no') { this.askClose(); return; }
      if (a === 'empty-trash') { this.ask(tf('Permanently delete everything in the Trash ({0})? This cannot be undone.', this.trashed.length), t('Empty Trash'), () => void this.purge(null)); return; }
      if (a === 'restore' && row) { void this.restore([row]); return; }
      if (a === 'purge' && row) { const x = this.trashed.find((d) => d.entry === row); this.ask(tf('Permanently delete "{0}"? This cannot be undone.', x?.title ?? row), t('Delete permanently'), () => void this.purge(row)); return; }
      const more = target.closest('.fs-card-more') as HTMLElement | null;
      if (more) {
        e.stopPropagation();
        const id = (more.closest('.fs-card') as HTMLElement).dataset.project;
        const r = more.getBoundingClientRect();
        if (id) this.openMenu(id, r.left, r.bottom + 4);
        return;
      }
      const hit = target.closest('.fs-card') as HTMLElement | null;
      if (hit?.dataset.project && (e.ctrlKey || e.metaKey || e.shiftKey)) { this.pickCard(hit, e.shiftKey); return; }
      if (a === 'blank') { void this.create(this.defaultTemplate); return; }
      if (a === 'open' || a === 'choose') { this.hooks.pickMesh(); return; }
      if (a === 'prefs') { this.hooks.preferences(); return; }
      if (a === 'agents') { this.hooks.agents(); return; }
      if (a === 'back') { this.leave(); return; }
      if (a === 'local') { toast(t('Your projects and textures stay in your FormSurface folder on this computer.')); return; }
      if (hit?.dataset.demo) { this.hooks.demo(hit.dataset.demo); return; }
      if (hit?.dataset.project) { void this.openProject(hit.dataset.project, hit); return; }
      if (hit?.dataset.template) {
        const tpl = this.templates.find((x) => x.id === hit.dataset.template);
        if (tpl) this.useTemplate(tpl);
        return;
      }
      const chip = target.closest('.fs-chip') as HTMLElement | null;
      if (chip?.dataset.cat) { this.category = chip.dataset.cat; this.renderTemplates(); }
    });
    for (const input of Array.from(this.root.querySelectorAll<HTMLInputElement>('.fs-tpl-search'))) {
      input.addEventListener('input', () => {
        this.query = input.value.trim().toLowerCase();
        for (const other of Array.from(this.root.querySelectorAll<HTMLInputElement>('.fs-tpl-search'))) if (other !== input && other.value !== input.value) other.value = input.value;
        this.renderTemplates();
      });
    }
    this.root.addEventListener('contextmenu', (e) => {
      const card = (e.target as HTMLElement).closest('.fs-card') as HTMLElement | null;
      if (!card?.dataset.project) return;
      e.preventDefault();
      if (!this.selected.has(card.dataset.project)) { this.clearSelection(); this.setSelected(card.dataset.project, true); }
      this.openMenu(card.dataset.project, e.clientX, e.clientY);
    });
    this.root.addEventListener('keydown', (e) => {
      if (e.key !== 'Delete') return;
      const card = (e.target as HTMLElement).closest('.fs-card') as HTMLElement | null;
      const id = card?.dataset.project;
      if (!id) return;
      e.preventDefault();
      this.requestDelete(this.selected.has(id) ? [...this.selected] : [id]);
    });
    document.addEventListener('keydown', (e) => {
      if (e.key !== 'Escape' || !this.isOpen || e.defaultPrevented) return;
      if (document.querySelector('.cv-scrim:not([hidden])')) return;
      if (!this.menu.hidden) { this.closeMenu(); return; }
      if (!this.confirmBar.hidden) { this.askClose(); return; }
      if (this.selected.size) { this.clearSelection(); return; }
      if (this.hooks.hasDocument()) this.leave();
    });
    this.menu.addEventListener('contextmenu', (e) => e.preventDefault());
    this.menu.addEventListener('keydown', (e) => {
      if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
      e.preventDefault();
      const rows = Array.from(this.menu.querySelectorAll<HTMLElement>('.fs-menu-item'));
      const at = rows.indexOf(document.activeElement as HTMLElement);
      rows[(at + (e.key === 'ArrowDown' ? 1 : rows.length - 1)) % rows.length]?.focus();
    });
    document.addEventListener('pointerdown', (e) => { if (!this.menu.hidden && !this.menu.contains(e.target as Node)) this.closeMenu(); }, true);
    const hero = this.root.querySelector('.fs-home-hero') as HTMLElement;
    hero.addEventListener('dragover', (e) => { e.preventDefault(); setClass(hero, 'is-drag-over', true); });
    hero.addEventListener('dragleave', () => setClass(hero, 'is-drag-over', false));
    hero.addEventListener('drop', () => setClass(hero, 'is-drag-over', false));
    this.root.addEventListener('dragover', (e) => { if (e.dataTransfer?.types.includes('Files')) e.preventDefault(); });
    this.root.addEventListener('drop', (e) => {
      const files = Array.from(e.dataTransfer?.files ?? []);
      if (!files.length) return;
      e.preventDefault();
      this.hooks.openFiles(files);
    });
  }

  private async openProject(id: string, card?: HTMLElement): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    card?.setAttribute('aria-busy', 'true');
    try {
      await this.hooks.openProject(id);
    } catch (e) {
      toast(tf('Could not open {0}: {1}', this.projects.find((d) => d.id === id)?.title ?? id, why(e)), 'error');
    } finally {
      this.busy = false;
      card?.removeAttribute('aria-busy');
    }
  }

  private async create(template: string, title?: string): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    const cards = Array.from(this.root.querySelectorAll<HTMLElement>(`.fs-card[data-template="${CSS.escape(template)}"]`));
    for (const c of cards) c.setAttribute('aria-busy', 'true');
    try {
      await this.hooks.createProject(template, title);
    } catch (e) {
      toast(tf('Could not start the project: {0}', why(e)), 'error');
    } finally {
      this.busy = false;
      for (const c of cards) c.removeAttribute('aria-busy');
    }
  }

  private useTemplate(tpl: TemplateSummary): void {
    const d = new Dialog(tpl.name);
    const form = el<HTMLElement>('<div class="fs-form"><p class="fs-tpl-about"></p><label class="fs-field"><span>Title</span><input class="fs-tpl-title" type="text" maxlength="80" spellcheck="false" placeholder="Project title"></label></div>');
    setText(form.querySelector('.fs-tpl-about'), tpl.description);
    const title = form.querySelector('.fs-tpl-title') as HTMLInputElement;
    d.body.appendChild(form);
    const go = (): void => { d.close(); void this.create(tpl.id, title.value.trim() || undefined); };
    title.addEventListener('keydown', (e) => { if (e.key === 'Enter') { e.preventDefault(); go(); } });
    d.action('Cancel', () => d.close(), false);
    d.action('Use template', go, true);
    d.open();
  }

  async load(): Promise<void> {
    const run = ++this.loads;
    const [tp, pr, tr, st] = await Promise.all([
      library.templates().catch(() => null),
      library.projects().catch(() => null),
      library.trash().catch(() => null),
      library.state().catch(() => null),
    ]);
    if (run !== this.loads) return;
    if (st) { this.pinned = st.settings.pinned; this.hiddenRecent = st.settings.recentHidden; }
    if (tp) { this.templates = tp.templates; this.categories = ['All', ...tp.categories]; this.defaultTemplate = tp.default; }
    if (pr) this.projects = pr.projects;
    if (tr) this.trashed = tr.projects;
    for (const id of [...this.selected]) if (!this.projects.some((d) => d.id === id)) this.selected.delete(id);
    this.renderTemplates();
    this.renderProjects();
    this.renderTrash();
  }

  private ask(text: string, action: string, go: () => void): void {
    this.onConfirm = go;
    setText(this.confirmBar.querySelector('.fs-home-confirm-text'), text);
    setText(this.confirmBar.querySelector('[data-a="confirm-yes"]'), action);
    setHidden(this.confirmBar, false);
    (this.confirmBar.querySelector('[data-a="confirm-no"]') as HTMLElement).focus();
  }

  private askClose(): void {
    this.onConfirm = null;
    setHidden(this.confirmBar, true);
  }

  private closeMenu(): void { setHidden(this.menu, true); }

  private openMenu(id: string, x: number, y: number): void {
    const ids = this.selected.has(id) && this.selected.size > 1 ? [...this.selected] : [id];
    const many = ids.length > 1;
    const rows: Array<[string, string, () => void]> = [];
    if (!many) {
      rows.push([t('Open'), 'open', () => void this.openProject(id)]);
      rows.push([t('Duplicate'), 'dup', () => { void library.duplicate(id).then(() => this.load()).then(() => toast(t('Duplicated.'))).catch((e) => toast(tf('Could not duplicate: {0}', why(e)), 'error')); }]);
    }
    const allPinned = ids.every((x) => this.pinned.includes(x));
    rows.push([allPinned ? t('Unpin from list') : t('Pin to list'), 'pin', () => this.setPinned(ids, !allPinned)]);
    rows.push([t('Remove from list'), 'hide', () => this.setHiddenRecent(ids, true)]);
    rows.push([many ? tf('Delete {0} projects', ids.length) : t('Delete'), 'del', () => this.requestDelete(ids)]);
    patch(this.menu, rows, {
      key: (r) => r[1],
      create: (r) => {
        const b = el<HTMLButtonElement>(`<button class="fs-menu-item" role="menuitem" type="button" data-k="${r[1]}"><span class="fs-menu-label" translate="no"></span></button>`);
        setText(b.querySelector('.fs-menu-label'), r[0]);
        b.addEventListener('click', () => { this.closeMenu(); this.menuRun.get(r[1])?.(); });
        return b;
      },
      update: (b, r) => setText(b.querySelector('.fs-menu-label'), r[0]),
    });
    this.menuRun.clear();
    for (const [, key, run] of rows) this.menuRun.set(key, run);
    this.menu.style.visibility = 'hidden';
    setHidden(this.menu, false);
    const w = this.menu.offsetWidth, h = this.menu.offsetHeight;
    this.menu.style.left = `${Math.round(Math.max(4, Math.min(x, window.innerWidth - w - 4)))}px`;
    this.menu.style.top = `${Math.round(Math.max(4, Math.min(y, window.innerHeight - h - 4)))}px`;
    this.menu.style.visibility = '';
    (this.menu.firstElementChild as HTMLElement | null)?.focus();
  }

  private readonly menuRun = new Map<string, () => void>();

  private saveRecent(): void {
    this.renderProjects();
    void library.setSettings({ pinned: this.pinned, recentHidden: this.hiddenRecent }).catch((e) => toast(tf('Could not save the list: {0}', why(e)), 'error'));
  }

  private setPinned(ids: string[], on: boolean): void {
    this.pinned = on ? [...this.pinned.filter((x) => !ids.includes(x)), ...ids] : this.pinned.filter((x) => !ids.includes(x));
    if (on) this.hiddenRecent = this.hiddenRecent.filter((x) => !ids.includes(x));
    this.saveRecent();
  }

  private setHiddenRecent(ids: string[], on: boolean): void {
    this.hiddenRecent = on ? [...this.hiddenRecent.filter((x) => !ids.includes(x)), ...ids] : this.hiddenRecent.filter((x) => !ids.includes(x));
    if (on) this.pinned = this.pinned.filter((x) => !ids.includes(x));
    this.saveRecent();
  }

  private setSelected(id: string, on: boolean): void {
    if (on) this.selected.add(id); else this.selected.delete(id);
    for (const grid of [this.recentGrid, this.recentGridFull]) {
      const n = grid.querySelector<HTMLElement>(`.fs-card[data-project="${CSS.escape(id)}"]`);
      if (n) setClass(n, 'is-selected', on);
    }
  }

  private clearSelection(): void {
    for (const id of [...this.selected]) this.setSelected(id, false);
    this.anchor = '';
  }

  private pickCard(card: HTMLElement, range: boolean): void {
    const id = card.dataset.project as string;
    if (range && this.anchor && this.anchor !== id) {
      const order = Array.from((card.parentElement as HTMLElement).querySelectorAll<HTMLElement>('.fs-card[data-project]')).map((n) => n.dataset.project as string);
      const a = order.indexOf(this.anchor), b = order.indexOf(id);
      if (a >= 0 && b >= 0) for (let i = Math.min(a, b); i <= Math.max(a, b); i++) this.setSelected(order[i], true);
      return;
    }
    this.setSelected(id, !this.selected.has(id));
    this.anchor = id;
  }

  private requestDelete(ids: string[]): void {
    if (!ids.length) return;
    if (ids.length === 1) { void this.trashProjects(ids); return; }
    this.ask(tf('Move {0} projects to the Trash?', ids.length), tf('Move {0} to Trash', ids.length), () => void this.trashProjects(ids));
  }

  private async trashProjects(ids: string[]): Promise<void> {
    const done: string[] = [];
    const entries: string[] = [];
    for (const id of ids) {
      try { entries.push((await library.remove(id)).entry); done.push(id); } catch (e) { toast(tf('Could not delete: {0}', why(e)), 'error'); }
    }
    if (!done.length) return;
    for (const id of done) { this.selected.delete(id); this.hooks.closeProject(id); }
    const titles = done.map((id) => this.projects.find((d) => d.id === id)?.title ?? t('Untitled project'));
    this.projects = this.projects.filter((d) => !done.includes(d.id));
    this.pinned = this.pinned.filter((x) => !done.includes(x));
    this.hiddenRecent = this.hiddenRecent.filter((x) => !done.includes(x));
    this.renderProjects();
    const tr = await library.trash().catch(() => null);
    if (tr) this.trashed = tr.projects;
    this.renderTrash();
    this.undoToast(done.length === 1 ? tf('Moved "{0}" to the Trash.', titles[0]) : tf('Moved {0} projects to the Trash.', done.length), entries);
  }

  private undoToast(text: string, entries: string[]): void {
    if (!entries.length) { toast(text); return; }
    const node = el('<div class="fs-toast fs-toast-undo" role="status"><span class="fs-toast-text" translate="no"></span><button class="fs-btn fs-btn-ghost fs-btn-sm" type="button">Undo</button></div>');
    setText(node.querySelector('.fs-toast-text'), text);
    node.querySelector('button')?.addEventListener('click', () => { dismissToast(node); void this.restore(entries); });
    pushToast(toastRoot(), node, 8000);
  }

  private async restore(entries: string[]): Promise<void> {
    let n = 0;
    for (const entry of entries) {
      try { await library.restore(entry); n++; } catch (e) { toast(tf('Could not restore: {0}', why(e)), 'error'); }
    }
    if (!n) return;
    const [l, tr] = await Promise.all([library.projects().catch(() => null), library.trash().catch(() => null)]);
    if (l) this.projects = l.projects;
    if (tr) this.trashed = tr.projects;
    this.renderProjects();
    this.renderTrash();
    toast(n === 1 ? t('Project restored.') : tf('{0} projects restored.', n));
  }

  private async purge(entry: string | null): Promise<void> {
    try {
      if (entry) await library.purge(entry); else await library.emptyTrash();
    } catch (e) { toast(tf('Could not delete: {0}', why(e)), 'error'); }
    const tr = await library.trash().catch(() => null);
    this.trashed = tr ? tr.projects : entry ? this.trashed.filter((d) => d.entry !== entry) : [];
    this.renderTrash();
  }

  private renderTrash(): void {
    setText(this.root.querySelector('.fs-trash-count'), String(this.trashed.length));
    setText(this.root.querySelector('.fs-project-count'), String(this.projects.length));
    const empty = this.root.querySelector<HTMLButtonElement>('.fs-btn-empty-trash');
    if (empty) setDisabled(empty, !this.trashed.length);
    const none = this.root.querySelector<HTMLElement>('.fs-trash-none');
    if (none) setHidden(none, this.trashed.length > 0);
    const update = (n: HTMLElement, d: TrashedProject): void => {
      setText(n.querySelector('.fs-card-name'), d.title);
      setText(n.querySelector('.fs-card-meta'), [layerCount(d.layers), tf('Deleted {0}', ago(new Date(d.deletedAt * 1000).toISOString()))].filter(Boolean).join(' · '));
    };
    const create = (d: TrashedProject): HTMLElement => {
      const n = el(`<div class="fs-trash-row" data-entry="${escapeHtml(d.entry)}"><span class="fs-trash-icon">${icon('cube', 18)}</span><span class="fs-trash-text"><span class="fs-card-name" translate="no"></span><span class="fs-card-meta" translate="no"></span></span><button class="fs-btn fs-btn-sm" type="button" data-a="restore">${icon('undo', 14)}Restore</button><button class="fs-btn fs-btn-sm fs-btn-danger" type="button" data-a="purge">${icon('trash', 14)}Delete permanently</button></div>`);
      update(n, d);
      return n;
    };
    patch(this.trashList, this.trashed, { key: (d) => d.entry, create, update });
  }

  private page(v: HomePage): void {
    this.view = v;
    for (const page of Array.from(this.root.querySelectorAll<HTMLElement>('[data-page]'))) setHidden(page, page.dataset.page !== v);
    for (const b of Array.from(this.root.querySelectorAll<HTMLElement>('.fs-home-nav-item[data-v]'))) setClass(b, 'is-active', b.dataset.v === v);
  }

  show(page?: HomePage): void {
    if (page) this.page(page);
    this.forced = true;
    if (this.isOpen) void this.load();
    this.sync();
  }

  leave(): void {
    this.forced = false;
    this.sync();
  }

  documentOpened(): void {
    this.forced = false;
    this.hadDocument = true;
    this.sync();
  }

  documentClosed(): void {
    this.sync();
  }

  sync(): void {
    const has = this.hooks.hasDocument();
    const on = this.forced || (!has && (this.hadDocument ? homePrefs.afterClose() : homePrefs.autoShow()));
    setHidden(this.back, !has);
    if (on) this.open(); else this.close();
  }

  private open(): void {
    if (this.isOpen) return;
    this.greet();
    setHidden(this.root, false);
    document.body.classList.add('nm-home');
    void this.load();
    this.root.focus({ preventScroll: true });
    this.hooks.onVisible?.(true);
  }

  private close(): void {
    if (!this.isOpen) return;
    setHidden(this.root, true);
    this.closeMenu();
    this.askClose();
    this.clearSelection();
    document.body.classList.remove('nm-home');
    this.hooks.onVisible?.(false);
  }

  refresh(): void {
    if (this.isOpen) void this.load();
  }

  get isOpen(): boolean { return !this.root.hidden; }

  setOpenProjects(list: Array<{ id: string; title: string }>): void {
    const group = this.root.querySelector('.fs-home-open') as HTMLElement;
    setHidden(group, !list.length);
    const name = (d: { title: string }): string => d.title.trim() || t('Untitled project');
    patchList(group.querySelector('.fs-home-open-list') as HTMLElement, list, {
      key: (d) => d.id || '__mesh',
      create: (d) => {
        const node = el(`<button class="fs-home-nav-item" type="button" data-open-project="">${icon('cube', 16)}<span class="fs-home-open-name" translate="no"></span></button>`);
        node.dataset.openProject = d.id;
        setText(node.querySelector('.fs-home-open-name'), name(d));
        return node;
      },
      update: (node, d) => setText(node.querySelector('.fs-home-open-name'), name(d)),
    });
  }

  private renderTemplates(): void {
    const q = this.query;
    const hay = (x: TemplateSummary): string => `${x.name} ${t(x.name)} ${x.category} ${t(x.category)} ${x.tags.join(' ')} ${x.description} ${t(x.description)}`.toLowerCase();
    const list = this.templates.filter((x) => (this.category === 'All' || x.category === this.category) && (!q || hay(x).includes(q)));
    setText(this.root.querySelector('.fs-tpl-count'), String(this.templates.length));
    for (const host of Array.from(this.root.querySelectorAll<HTMLElement>('.fs-cats'))) {
      const chip = (n: HTMLElement, c: string): void => { setClass(n, 'is-active', c === this.category); setText(n.querySelector('.fs-chip-count'), !this.templates.length ? '' : c === 'All' ? String(this.templates.length) : String(this.templates.filter((x) => x.category === c).length)); };
      patchList(host, this.categories, {
        key: (c) => c,
        create: (c) => { const n = el(`<button class="fs-chip" type="button" data-cat="${escapeHtml(c)}"><span class="fs-chip-name">${escapeHtml(c)}</span><span class="fs-chip-count" translate="no"></span></button>`); chip(n, c); return n; },
        update: chip,
      });
    }
    const card = (x: TemplateSummary): HTMLElement => {
      const n = el(`<button class="fs-card" type="button" data-template="${escapeHtml(x.id)}"><span class="fs-card-preview" aria-hidden="true"><img alt="" decoding="async" draggable="false"></span><span class="fs-card-body"><span class="fs-card-name"></span><span class="fs-card-meta"></span></span></button>`);
      setText(n.querySelector('.fs-card-name'), x.name);
      setText(n.querySelector('.fs-card-meta'), x.category);
      n.title = x.description;
      const img = n.querySelector('img') as HTMLImageElement;
      img.onerror = () => this.blank(n);
      img.src = `/home/templates/${encodeURIComponent(x.id)}.webp`;
      return n;
    };
    const all = new Set(list.map((x) => x.id));
    const top = new Set(list.slice(0, 15).map((x) => x.id));
    const grid = (host: HTMLElement, shown: Set<string>): void => patch(host, this.templates, {
      key: (x) => x.id,
      create: (x) => { const n = card(x); n.hidden = !shown.has(x.id); return n; },
      update: (n, x) => setHidden(n, !shown.has(x.id)),
    });
    grid(this.tplGrid, top);
    grid(this.tplGridFull, all);
    for (const none of Array.from(this.root.querySelectorAll<HTMLElement>('.fs-tpl-none'))) setHidden(none, list.length > 0 || !this.templates.length);
  }

  private blank(card: HTMLElement): void {
    const prev = card.querySelector('.fs-card-preview') as HTMLElement;
    if (prev.classList.contains('is-blank')) return;
    prev.classList.add('is-blank');
    prev.replaceChildren(el(icon('cube', 28)));
  }

  private preview(card: HTMLElement, d: ProjectSummary): void {
    const want = d.thumb ? library.thumbUrl(d.id, d.modified) : '';
    if (card.dataset.thumb === want) return;
    const first = card.dataset.thumb === undefined;
    card.dataset.thumb = want;
    if (!want) { this.blank(card); return; }
    const prev = card.querySelector('.fs-card-preview') as HTMLElement;
    let img = prev.querySelector('img');
    if (!img) {
      img = el<HTMLImageElement>('<img alt="" decoding="async" draggable="false">');
      prev.classList.remove('is-blank');
      prev.replaceChildren(img);
    }
    const node = img;
    node.onerror = () => { if (card.dataset.thumb === want) this.blank(card); };
    if (first || !node.getAttribute('src')) node.src = want;
    else void swapMedia(node, new URL(want, location.href).href).catch(() => undefined);
  }

  private renderProjects(): void {
    const card = (d: ProjectSummary): HTMLElement => el(`<button class="fs-card" type="button" data-project="${escapeHtml(d.id)}"><span class="fs-card-preview"><img alt="" decoding="async" draggable="false"></span><span class="fs-card-body"><span class="fs-card-name" translate="no"></span><span class="fs-card-meta" translate="no"></span></span><span class="fs-card-more" role="button" tabindex="-1" title="More actions" aria-label="More actions">${icon('more3', 16)}</span></button>`);
    const update = (n: HTMLElement, d: ProjectSummary): void => {
      setText(n.querySelector('.fs-card-name'), d.title);
      setText(n.querySelector('.fs-card-meta'), [this.pinned.includes(d.id) ? t('Pinned') : '', layerCount(d.layers), ago(d.modified)].filter(Boolean).join(' · '));
      setClass(n, 'is-pinned', this.pinned.includes(d.id));
      setClass(n, 'is-selected', this.selected.has(d.id));
      this.preview(n, d);
    };
    const make = (d: ProjectSummary): HTMLElement => { const n = card(d); update(n, d); return n; };
    const pinnedFirst = (list: ProjectSummary[]): ProjectSummary[] => [...list.filter((d) => this.pinned.includes(d.id)), ...list.filter((d) => !this.pinned.includes(d.id))];
    const recent = pinnedFirst(this.projects.filter((d) => !this.hiddenRecent.includes(d.id)));
    patch(this.recentGrid, recent.slice(0, 5), { key: (d) => d.id, create: make, update });
    patch(this.recentGridFull, pinnedFirst(this.projects), { key: (d) => d.id, create: make, update });
    const none = this.recentGrid.querySelector<HTMLElement>('.fs-card.is-empty');
    if (none) setHidden(none, recent.length > 0);
    const noneFull = this.root.querySelector<HTMLElement>('.fs-recent-none');
    if (noneFull) setHidden(noneFull, this.projects.length > 0);
    setText(this.root.querySelector('.fs-project-count'), String(this.projects.length));
  }
}

let home: Home | null = null;

export function mountHome(hooks: HomeHooks): Home {
  if (!home) home = new Home(hooks);
  return home;
}

export function showHome(page?: HomePage): void { home?.show(page); }
export function hideHome(): void { home?.leave(); }
export function isHomeOpen(): boolean { return !!home && home.isOpen; }
