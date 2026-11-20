# frontend HASHOUT_LOG

Decisions, reasons and anything left out, newest first. Code carries no comments; the context lives here. Started in 0.64.0; earlier history is not backfilled.

## Unreleased (2026-11-20): every sentence that lists the agent programs names all four (`src/shell/app-shell.ts`, `src/shell/home.ts`, `index.html`, `src/i18n/strings.json`, `src/i18n/locales/*.json`)

- The Connect an Outside Agent dialog's first sentence, Home's Agents entry and the Agent link button's tooltip now say Claude Code, Codex, Cursor and Antigravity, in that order.
- The dialog sentence and the tooltip are lookup keys (the page is translated by matching its English text), so each key changed in `strings.json` and in all 23 tables that carry it, with the translation, in the same commit. `es-MX` and `fr-CA` carry neither key and take `es` and `fr`. Home's entry is product names only and has no table entry. No native speaker has read the new translations.
- The search was for "Cursor" across `src` (outside the shared `dock/` folder) and `index.html`; the remaining hits are the panel module, the menu row named Cursor and the four "Under Cursor" commands, which list no programs.

## Unreleased (2026-11-20): the shared panel module is 2.2.1 (`src/dock/`)

- `CONTRACT.md`, `MANIFEST.json` and `strings.ts` are copied byte for byte from FormVis commit 23b01889 (the file `cp`d whole, as the family vendors the module); the other 13 files were already the same bytes. All 16 match FormVis at that commit, and `setup/agents.py check` finds the folder whole against its manifest.
- The only change a person sees: the first-run guide's two sentences for a computer with Claude Code now also say that Codex, Cursor and Antigravity each get a button of their own when they are on this computer.
- `shell/dock-adapter.ts` keeps `CONTRACT = '2.2.0'`. That string is the handshake with the engine (`backend/app/agent/agent_programs.py`, which reports 2.2.0): when the two differ the adapter shows only Claude Code. The 2.2.1 contract says an adapter written for 2.2.0 needs no change, and the engine's side is not part of this change, so both stay 2.2.0 and move together later.
- FormSurface has no translations of the panel's own sentences (the adapter's `t` looks the English up in the app's tables and falls back to it), so no language table changes here.

## 0.64.0 (2026-11-20): the window check hands each page the key the way the desktop app does (`tests/window/test_window.py`, `tests/window/test_home_and_library.py`)

- The window tests start one engine per module and open a new browser context per test. An engine started from source gives its key to the first page only, so from the second context on the page would be refused. The fixtures now claim the key once, right after the engine answers, and every page gets it through an init script that sets `window.__FORMSURFACE_WINDOW_KEY`, as the shell's initialization script does. The pages then make no bootstrap request at all.

## 0.64.0 (2026-11-20): the window claims its key before its first request (`src/shell/window-key.ts`, `src/main.ts`, `src/shell/events.ts`, `src/ui/agent-link.ts`, `src/shell/dock-adapter.ts`)

- `shell/window-key.ts` is FormVis's `io/window-key.ts` under formsurface's names. It is imported first in `main.ts`, so the key is in hand before any other module's first `fetch`: the shell's global `__FORMSURFACE_WINDOW_KEY` (read once, then deleted), else the key kept in `sessionStorage` for a reload of the same tab (checked with `prefs.get`), else `POST /api/window/key`. It wraps `window.fetch` so every same-origin request carries `X-Formsurface-Window-Key`; other origins are passed through untouched.
- The live socket (Agent link) sends the key message as soon as it opens, after the key has loaded. Terminal sockets send it first, before any input or resize, from `dock-adapter.ts`; the shared panel module is not changed.
- The event stream asks for a one-time ticket and opens `EventSource` with it. On any error it closes and opens again with a fresh ticket after 1.5 s, because the browser's own retry would reuse the spent ticket.
- Checked: `tsc --noEmit` reports only the two lines for the missing WebAssembly build (`src/engine/pkg`), which cannot be built on this machine (no `wasm-bindgen`).

## Unreleased (2026-11-20): Chat pictures are asked for with a ticket (`src/shell/chat-pane.ts`)

- Each picture in a Chat reply asks `streamTicket()` for a one-time ticket and only then sets its `src` with `&ticket=`, because an `<img>` cannot send the window key header. One ticket per picture, spent by its one load.
