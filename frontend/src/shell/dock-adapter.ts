import type { DockAdapter, DockAgentStatus, DockChannel, DockKind, DockTerminal } from '../dock/adapter';
import { jsonLegacy } from '../dock/state';
import { api, ApiError } from './api';
import { ChatPane } from './chat-pane';
import { isMac } from './dom';
import { onLanguage, t } from '../i18n';

export type AgentKind = 'antigravity';
export const DOCK_STORE = 'formsurface.agentDock';
const CONTRACT = '2.2.0';
const EXIT = '{"type": "exit"}';
const LEAD = "Ask Claude about your model in the line below: texture a material, set up layers, bake maps, export. It works through FormSurface's tools, every change shows here and in the viewport, and each one can be undone.";

export interface DockHost {
  linkAgent(): void;
  context(): string;
  fail(err: unknown): void;
  claude(prompt?: string, resume?: string): Promise<boolean>;
}

function channel(id: string): DockChannel {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const socket = new WebSocket(`${proto}//${location.host}/api/terminal/${encodeURIComponent(id)}`);
  let data = (_chunk: string | Uint8Array): void => undefined;
  let exit = (): void => undefined;
  let closed = (): void => undefined;
  socket.addEventListener('open', () => data(''));
  socket.addEventListener('message', (msg) => {
    const text = String(msg.data);
    if (text === EXIT) exit(); else data(text);
  });
  socket.addEventListener('close', () => closed());
  const send = (text: string): void => {
    if (socket.readyState === WebSocket.OPEN) socket.send(text);
    else if (socket.readyState === WebSocket.CONNECTING) socket.addEventListener('open', () => socket.send(text), { once: true });
  };
  return {
    send: (chunk) => send(JSON.stringify({ type: 'input', data: chunk })),
    resize: (cols, rows) => send(JSON.stringify({ type: 'resize', cols, rows })),
    onData: (handler) => { data = handler; },
    onExit: (handler) => { exit = handler; },
    onClose: (handler) => { closed = handler; },
    close: () => socket.close(),
  };
}

export function formsurfaceDock(host: DockHost): DockAdapter<AgentKind> {
  let pane: ChatPane | null = null;
  const tell = (text: string, tab: DockKind | AgentKind | null): boolean | Promise<boolean> => {
    host.linkAgent();
    const extra = host.context();
    if (tab === 'chat' && pane) {
      if (pane.isBusy()) return false;
      return pane.send(text, extra).then(() => true);
    }
    return host.claude(extra ? `${text} (${extra})` : text);
  };
  return {
    words: { app: 'FormSurface', noun: 'model', lead: LEAD, example: 'give the body a worn painted metal look', start: './formsurface' },
    placement: {
      mode: 'own',
      storage: { key: DOCK_STORE, legacy: jsonLegacy(['formsurface.dock'], true) },
      sizes: { minHeight: 140, maxHeightShare: 0.7, defaultHeight: 280, minWidth: 320, maxWidthShare: 0.5, defaultWidth: 440 },
    },
    isDockChord: (e) => (isMac ? e.metaKey : e.ctrlKey) && !e.shiftKey && !e.altKey && e.code === 'KeyJ',
    chordLabel: () => `${isMac ? 'Cmd' : 'Ctrl'}+J`,
    passKey: (e) => (isMac ? e.metaKey : e.ctrlKey) && !e.altKey && (e.code === 'KeyJ' || e.code === 'KeyK' || (!e.shiftKey && e.key === ',')),
    tabs: () => ({ builtin: ['chat', 'shell', 'claude', 'codex', 'cursor', 'antigravity'] }),
    classes: { iconButton: 'cv-tt-btn', button: 'cv-btn', primaryButton: 'cv-btn', input: 'cv-input', select: 'cv-input', note: 'cv-note' },
    t: (_key, english) => t(english),
    onTheme: (handler) => {
      document.addEventListener('nm:ui-settings-changed', handler);
      onLanguage(handler);
    },
    errors: {
      message: (err) => (err instanceof ApiError ? err.message : null),
      kind: (err) => {
        if (err instanceof TypeError) return 'unreachable';
        if (!(err instanceof ApiError)) return null;
        if (err.status === 403) return 'refused';
        if (err.status === 424) return 'notInstalled';
        if (err.status === 404 && /^Unknown command/.test(err.message)) return 'oldEngine';
        if (err.status === 409) return 'busy';
        return err.status >= 502 && err.status <= 504 ? 'unreachable' : null;
      },
    },
    terminal: {
      list: async () => (await api.cmd<{ terminals: DockTerminal<AgentKind>[] }>('terminal.list')).terminals,
      open: async (kind, o) => {
        if (kind === 'shell') return api.cmd<DockTerminal<AgentKind>>('terminal.open', { cols: o.cols, rows: o.rows });
        host.linkAgent();
        if (kind === 'claude') return api.cmd<DockTerminal<AgentKind>>('claudecode.open', { cols: o.cols, rows: o.rows, ...(o.prompt ? { prompt: o.prompt } : {}), ...(o.resume ? { resume: o.resume } : {}) });
        return api.cmd<DockTerminal<AgentKind>>('agents.open', { kind, cols: o.cols, rows: o.rows });
      },
      restart: async (id) => { await api.cmd('terminal.restart', { id }); },
      close: async (id) => { await api.cmd('terminal.close', { id }); },
      channel,
    },
    agentStatus: async () => {
      try {
        const s = await api.cmd<DockAgentStatus<AgentKind> & { contract?: string }>('agents.status');
        const { contract, ...programs } = s;
        return contract === CONTRACT ? programs : { claude: programs.claude ?? { installed: false, version: null, install: '', signedIn: null } };
      } catch (err) {
        if (err instanceof TypeError) return null;
        const old = await api.cmd<{ installed: boolean; version: string | null }>('claudecode.status').catch(() => null);
        return old ? { claude: { installed: old.installed, version: old.version, install: '', signedIn: null } } : null;
      }
    },
    chat: {
      pane: (box) => {
        const chat = new ChatPane(api);
        chat.onFail = host.fail;
        chat.onHandoff = (session) => { void host.claude(undefined, session); };
        chat.root.hidden = false;
        box.appendChild(chat.root);
        pane = chat;
        return {
          shown: () => { host.linkAgent(); void chat.load(); },
          busy: () => chat.isBusy(),
          send: (text) => tell(text, 'chat'),
        };
      },
      ask: { send: (text, tab) => tell(text, tab) },
    },
  };
}
