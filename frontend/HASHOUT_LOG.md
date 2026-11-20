# frontend HASHOUT_LOG

Decisions, reasons and anything left out, newest first. Code carries no comments; the context lives here. Started in 0.64.0; earlier history is not backfilled.

## 0.64.0 (2026-11-20): the window check hands each page the key the way the desktop app does (`tests/window/test_window.py`, `tests/window/test_home_and_library.py`)

- The window tests start one engine per module and open a new browser context per test. An engine started from source gives its key to the first page only, so from the second context on the page would be refused. The fixtures now claim the key once, right after the engine answers, and every page gets it through an init script that sets `window.__FORMSURFACE_WINDOW_KEY`, as the shell's initialization script does. The pages then make no bootstrap request at all.

## 0.64.0 (2026-11-20): the window claims its key before its first request (`src/shell/window-key.ts`, `src/main.ts`, `src/shell/events.ts`, `src/ui/agent-link.ts`, `src/shell/dock-adapter.ts`)

- `shell/window-key.ts` is FormVis's `io/window-key.ts` under formsurface's names. It is imported first in `main.ts`, so the key is in hand before any other module's first `fetch`: the shell's global `__FORMSURFACE_WINDOW_KEY` (read once, then deleted), else the key kept in `sessionStorage` for a reload of the same tab (checked with `prefs.get`), else `POST /api/window/key`. It wraps `window.fetch` so every same-origin request carries `X-Formsurface-Window-Key`; other origins are passed through untouched.
- The live socket (Agent link) sends the key message as soon as it opens, after the key has loaded. Terminal sockets send it first, before any input or resize, from `dock-adapter.ts`; the shared panel module is not changed.
- The event stream asks for a one-time ticket and opens `EventSource` with it. On any error it closes and opens again with a fresh ticket after 1.5 s, because the browser's own retry would reuse the spent ticket.
- Checked: `tsc --noEmit` reports only the two lines for the missing WebAssembly build (`src/engine/pkg`), which cannot be built on this machine (no `wasm-bindgen`).

## Unreleased (2026-11-20): Chat pictures are asked for with a ticket (`src/shell/chat-pane.ts`)

- Each picture in a Chat reply asks `streamTicket()` for a one-time ticket and only then sets its `src` with `&ticket=`, because an `<img>` cannot send the window key header. One ticket per picture, spent by its one load.
