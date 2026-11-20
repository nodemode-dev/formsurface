const HEADER = 'X-Formsurface-Window-Key';
const STORE = 'formsurface-window-key';
const inPage = typeof window !== 'undefined' && typeof window.fetch === 'function';
const plain: typeof fetch = inPage ? window.fetch.bind(window) : (...args: Parameters<typeof fetch>) => fetch(...args);

let key = '';

function remembered(): string {
  try { return sessionStorage.getItem(STORE) ?? ''; } catch { return ''; }
}

function remember(value: string): void {
  try { sessionStorage.setItem(STORE, value); } catch { return; }
}

async function keyWorks(value: string): Promise<boolean> {
  try { return (await plain('/api/commands/prefs.get', { method: 'POST', headers: { [HEADER]: value, 'content-type': 'application/json' }, body: '{}' })).ok; } catch { return false; }
}

async function load(): Promise<void> {
  const holder = window as unknown as { __FORMSURFACE_WINDOW_KEY?: unknown };
  const given = holder.__FORMSURFACE_WINDOW_KEY;
  delete holder.__FORMSURFACE_WINDOW_KEY;
  if (typeof given === 'string' && given) { key = given; return; }
  const kept = remembered();
  if (kept && await keyWorks(kept)) { key = kept; return; }
  try {
    const res = await plain('/api/window/key', { method: 'POST' });
    const body = res.ok ? await res.json() as { key?: unknown } : {};
    if (typeof body.key === 'string' && body.key) { key = body.key; remember(key); return; }
  } catch { key = ''; }
  key = remembered();
}

const ready: Promise<void> = inPage ? load() : Promise.resolve();

function ours(input: RequestInfo | URL): boolean {
  const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
  try { return new URL(url, location.href).origin === location.origin; } catch { return false; }
}

if (inPage) {
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    if (!ours(input)) return plain(input, init);
    await ready;
    if (!key) return plain(input, init);
    const headers = new Headers(init?.headers ?? (input instanceof Request ? input.headers : undefined));
    headers.set(HEADER, key);
    return plain(input, { ...init, headers });
  };
}

export const windowKeyReady = (): Promise<void> => ready;

export const keyMessage = (): string => JSON.stringify({ type: 'key', key });

export async function streamTicket(): Promise<string> {
  await ready;
  try {
    const res = await window.fetch('/api/window/ticket', { method: 'POST' });
    const body = res.ok ? await res.json() as { ticket?: unknown } : {};
    return typeof body.ticket === 'string' ? body.ticket : '';
  } catch { return ''; }
}
