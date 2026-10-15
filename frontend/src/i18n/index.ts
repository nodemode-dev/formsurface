import { LANG_NAMES, LANG_PARENT, isLang, type Lang } from './langs';

export type { Lang } from './langs';

const KEY = 'formsurface.language';
const loaders = import.meta.glob<{ default: Record<string, string> }>(['./locales/*.json', './more/*.json', './more2/*.json', './more3/*.json', './more4/*.json', './more5/*.json', './more6/*.json', './more7/*.json', './more8/*.json', './more9/*.json', './more10/*.json', './more11/*.json', './more12/*.json']);
const SKIP = 'script, style, textarea, code, pre, canvas, [translate="no"], [data-no-i18n], [contenteditable="true"], .xterm, .fl-chat-msg, .fl-md, .layer .name, .set-row .name';
const ATTRS = ['title', 'placeholder', 'aria-label', 'alt'] as const;

let current: Lang = 'en';
let table = new Map<string, string>();
let patterns: Array<{ re: RegExp; out: string }> = [];
const cache = new Map<string, string>();
const listeners = new Set<() => void>();
const textSource = new WeakMap<Text, { en: string; shown: string }>();
const attrSource = new WeakMap<Element, Map<string, { en: string; shown: string }>>();
let observer: MutationObserver | null = null;

function stored(): Lang {
  try {
    const v = localStorage.getItem(KEY);
    if (isLang(v)) return v;
  } catch { return 'en'; }
  return 'en';
}

export const languages = (): ReadonlyArray<readonly [Lang, string]> => LANG_NAMES;
export const language = (): Lang => current;
export function locale(): string { return current; }

async function load(code: Lang): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  for (const folder of ['locales', 'more', 'more2', 'more3', 'more4', 'more5']) {
    const loader = loaders[`./${folder}/${code}.json`];
    if (!loader) continue;
    try { Object.assign(out, (await loader()).default); } catch { continue; }
  }
  return out;
}

function escapeRe(s: string): string { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

function compile(entries: Record<string, string>): void {
  table = new Map(Object.entries(entries));
  patterns = [];
  const keyed = Object.keys(entries).filter((k) => /\{[a-z0-9]+\}/i.test(k)).sort((a, b) => b.length - a.length);
  for (const k of keyed) {
    const names: string[] = [];
    const src = k.split(/(\{[a-z0-9]+\})/i).map((part) => {
      const m = /^\{([a-z0-9]+)\}$/i.exec(part);
      if (!m) return escapeRe(part);
      names.push(m[1]);
      return '(.+?)';
    }).join('');
    const out = entries[k].replace(/\{([a-z0-9]+)\}/gi, (_all, n: string) => `$${names.indexOf(n) + 1}`);
    patterns.push({ re: new RegExp(`^${src}$`), out });
  }
  cache.clear();
}

function lookup(text: string): string | undefined {
  const hit = table.get(text);
  if (hit !== undefined) return hit;
  const trail = /^(.*?)(\.\.\.|:|…)$/.exec(text);
  if (trail) {
    const base = table.get(trail[1]);
    if (base !== undefined) return base + trail[2];
  }
  for (const p of patterns) if (p.re.test(text)) return text.replace(p.re, p.out);
  return undefined;
}

export function t(english: string): string {
  if (current === 'en' || !english) return english;
  const cached = cache.get(english);
  if (cached !== undefined) return cached;
  const lead = /^\s*/.exec(english)?.[0] ?? '', tail = /\s*$/.exec(english)?.[0] ?? '';
  const core = english.trim();
  const found = core ? lookup(core) : undefined;
  const out = found === undefined ? english : lead + found + tail;
  cache.set(english, out);
  return out;
}

function translateText(node: Text): void {
  const raw = node.data;
  if (!raw.trim()) return;
  const prev = textSource.get(node);
  const en = prev && prev.shown === raw ? prev.en : raw;
  const shown = t(en);
  if (shown !== raw) node.data = shown;
  if (shown !== en || prev) textSource.set(node, { en, shown });
}

function translateAttrs(node: Element): void {
  for (const a of ATTRS) {
    const raw = node.getAttribute(a);
    if (!raw) continue;
    let map = attrSource.get(node);
    const prev = map?.get(a);
    const en = prev && prev.shown === raw ? prev.en : raw;
    const shown = t(en);
    if (shown !== raw) node.setAttribute(a, shown);
    if (shown !== en || prev) { if (!map) { map = new Map(); attrSource.set(node, map); } map.set(a, { en, shown }); }
  }
}

function walk(root: Node): void {
  if (root.nodeType === Node.TEXT_NODE) {
    const parent = root.parentElement;
    if (parent && !parent.closest(SKIP)) translateText(root as Text);
    return;
  }
  if (!(root instanceof Element) || root.closest(SKIP)) return;
  translateAttrs(root);
  if (root instanceof HTMLSelectElement) {
    for (const o of Array.from(root.options)) for (const c of Array.from(o.childNodes)) if (c.nodeType === Node.TEXT_NODE) translateText(c as Text);
  }
  const it = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => (n instanceof Element && n.matches(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT),
  });
  for (let n = it.nextNode(); n; n = it.nextNode()) {
    if (n.nodeType === Node.TEXT_NODE) translateText(n as Text);
    else translateAttrs(n as Element);
  }
}

export function startPageTranslator(): void {
  if (observer || current === 'en') { document.documentElement.classList.remove('i18n-pending'); return; }
  walk(document.body);
  document.documentElement.classList.remove('i18n-pending');
  observer = new MutationObserver((records) => {
    observer?.disconnect();
    for (const r of records) {
      if (r.type === 'characterData') walk(r.target);
      else if (r.type === 'attributes' && r.target instanceof Element) { if (!r.target.closest(SKIP)) translateAttrs(r.target); }
      else for (const n of Array.from(r.addedNodes)) walk(n);
    }
    observer?.observe(document.body, OPTIONS);
  });
  observer.observe(document.body, OPTIONS);
}

const OPTIONS: MutationObserverInit = { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: [...ATTRS] };

export async function startLanguage(): Promise<void> {
  const code = stored();
  if (code === 'en') return;
  const parent = LANG_PARENT[code];
  const merged = { ...(parent ? await load(parent) : {}), ...(await load(code)) };
  current = code;
  compile(merged);
  document.documentElement.lang = code;
}

export function setLanguage(code: Lang): void {
  try { localStorage.setItem(KEY, code); } catch { return; }
  for (const fn of listeners) fn();
  window.location.reload();
}

export function onLanguage(fn: () => void): () => void {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
