# frontend HASHOUT_LOG

Decisions, reasons and anything left out, newest first. Code carries no comments; the context lives here. Started in 0.64.0; earlier history is not backfilled.

## Unreleased (2026-11-20): AMOLED Black, the theme profile (`src/shell/themes.ts`, `src/i18n/strings.json`, `src/i18n/more14/*.json`)

- The family's AMOLED Black (`/home/claude/form/work/AMOLED_SPEC.md`, FormTxt eb5bacd the reference) in FormSurface's nine token names: `--bg` and `--surface` `#000000` (the spec's `--bg-primary` and `--bg-secondary`), `--surface-2` `#141414` (`--bg-tertiary`: inputs, menus and selected rows use it as their only fill), `--border` `#262626`, `--surface-3` `#3a3a3a` (`--border-light`), `--text` `#ededed` (17.9 to 1 on black), `--text-muted` `#a6a6a6` (8.6 to 1), `--text-faint` `#858585` (5.7 to 1), `--canvas-bg` `#000000`. The accent stays the user's (orange by default).
- Id `amoled`, listed after High Contrast, so the Window menu's Theme list and the Preferences picker show it with no other change. It is not High Contrast: High Contrast keeps white text and grey panels.
- `OLED` names the profiles for OLED screens. `applyTheme` sets `data-oled` on html for them and removes it for every other, and touches the attribute only when it changes, so a switch between two other themes writes nothing.
- The theme's name is a lookup key, so it is in `strings.json` and in all 25 tables of `more14`, beside the other theme names, with the family's wording (FormRef's tables). No speaker has read them.

## Unreleased (2026-11-20): AMOLED Black, the true-black rules (`src/styles/tokens.css`, `src/styles/home.css`)

- Every rule hangs on `data-oled`, never on the theme id, so another OLED profile needs only the flag.
- `--bg-node` and `--surface-1` mix `--surface-2` into the background: `#0c0c0c` and `#090909` under AMOLED. `--bg-node` fills dialogs, the command palette, the glass menus (82% of it), the title bar's top and the plain buttons; `--surface-1` the title bar's foot and the agent panel's bar (`--dock-bar-bg` in `dock-tokens.css`). Both are the background under `data-oled`. The spec puts `--bg-node` at `#000000` too.
- `--line-0` mixes the background with black, so it was black on black: the dividers under the title bar, above the status bar, under the agent panel's head and beside Home's list vanished, and borders are what part the panels here. It is `--border` under `data-oled`, as `theme-light` already does.
- `--bg-hover` 10% white and `--border-soft` 12% white: the usual 6% and 8% give 1.10 and 1.14 to 1 on black.
- Home's drop area draws a 10% accent glow from its top left corner (`#180b02` measured at that corner under AMOLED). Under `data-oled` it is plain black with its dashed border; dragging a file over it still lights it.
- Left as they are, on purpose: `--surface-raised` (hover, the active tab and chips: a fill that marks something), the `#141414` fills of inputs, menus in the panel and the ask line (the spec keeps them), the scrim behind dialogs, Home's template pictures (content), and the 3D view's backdrop `#131416`, which the engine clears to in native code (`native/fs-engine/src/gpu.rs`) under every theme; it is not themed, and changing it is native work outside this change. The view's frame around it (`.viewport`, `--canvas-bg`) is black.
- The agent panel (`src/dock`, copied from FormVis) is not changed: it reads its colours through `dock-tokens.css`, whose sources are black under AMOLED.

## Unreleased (2026-11-20): AMOLED Black before first paint (`index.html`)

- The boot script's profile table has the `amoled` row and an `OLED = { 'amoled': 1 }` table and sets `data-oled` beside the tokens before first paint. A theme in one table and not the other would paint Studio Dark first and then switch.
- The critical style paints `html, body` `#0a0a0a` until the stylesheets arrive, and the boot script repaints only `html`; `html[data-oled] body` is `#000000` there, so the body is black from the first frame.
- Checked in headless Chromium at 1440 by 900, 4x CPU throttle, cache off, 150 ms latency and 4 Mbit/s: cold load and hard reload with `amoled` saved, every frame after navigation black or its settled value at 65 points (corners and centres of 13 surfaces).

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
