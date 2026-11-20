---
name: editor
description: The FormSurface TypeScript and Vite editor. Load for panels, controls, pointer and pen input, mesh import, sample meshes, export and screenshot buttons, the Agent link toggle and DOM stability.
---

# Editor

## Shape

- `index.html` holds the whole shell, including an inline SVG symbol sheet. Icons are an inline svg element whose use child points at `#i-name`; add new symbols to the sheet, never an icon package.
- `src/main.ts` boots the engine, wires controls and owns the agent command handlers. `src/ui/layers.ts` patches layer rows by id. `src/ui/agent-link.ts` owns the WebSocket. `src/engine/meshes.ts` flattens three.js loader output into one indexed mesh with V flipped.
- Export: `renderMaps` reads each channel from the engine as u8, u16 or f32 (`sampleMax` by format and depth), packs templates in `src/ui/export-templates.ts`, pads in `src/ui/padding.ts` (island mask rasterized from `texture_set_mesh` UVs and grown one texel, nearest-island index by BFS once per set, then per map) and encodes in `src/ui/image-formats.ts` (PNG, TIFF, EXR, HDR, PSD written by hand; JPEG through OffscreenCanvas). Float formats store base color linear (pow 2.2, matching the engine's 1/2.2 export). Saved templates come from `/api/templates`; `src/ui/template-editor.ts` appends and removes its own rows. Verify new encoders with an outside reader (tifffile, OpenEXR, OpenCV, psd-tools), never only with the writer itself; the sample meshes fill the whole UV square, so test padding on a mesh with real islands.
- Path and text: `src/ui/path.ts` (Catmull-Rom on surface points, overlay canvas), `src/ui/text.ts` (FontFace loading, text rasters).
- Embedded Python (v0.29.0): `src/ui/python.ts` loads Pyodide lazily (dev serves `/node_modules/pyodide/`; the build copies its five runtime files to `dist/pyodide`), writes `mcp/server.py` and `mcp/formsurface_api` (imported as raw text; `server.fs.allow` lists only those) plus the stubs in `src/python` (httpx bridging to the agent handlers through `run_sync`, a pydantic subset that checks Annotated Field limits, Literal, lists and models, an MCPServer registry), so the MCP tool functions and their checks run unchanged in the page. `api_init.py` replaces the package `__init__` and adds `event` and `ui`. Python callbacks are invoked with `callPromising` so they can call tools; `emitPython` fires events from the editor.
- Smart asset browser (v0.30.0): `src/ui/asset-browser.ts` keeps one tile per asset and patches it (src, label, hidden) instead of re-rendering. Built-in thumbnails are 96 px WebP files in `src/assets/smart` named `material-` or `mask-` plus the asset's slug (lowercase, dashes), rendered by the app itself (materials on the sphere, masks as an orange fill on the torus knot, every other layer removed first because a reload carries layers); regenerate them when a built-in changes. Saved materials and masks store a viewport thumbnail as a data URL (the service checks the prefix and a 200 KB cap). Shared files are `{format: formsurface-smart-asset, version, kind, name, layers or settings, thumbnail}`.
- Styles: `src/styles/tokens.css` (colors, spacing, z-index layers) and `src/styles/app.css`. Every z-index is a `--z-*` token.

## Rules

- Pointer input goes to the engine as coalesced samples with pressure. Never paint in TypeScript.
- Controls change engine state through `syncBrush`, `syncChannels`, `syncBase`, `syncView`. Agent handlers set the same inputs with `setControl`, so the panel always shows what the engine uses.
- DOM stability: write the one property that changed. Reserve width for text that changes length (see `#agent-label`, `#fps`). Never clear a container to re-render it.
- Vite stays `host: false` on port 5176 with the `/api` proxy and `ws: true`.

## Shell, commands and languages

- `frontend/src/shell/app-shell.ts` registers every command (id, English label, keys, run, enabled, checked) and builds the menu bar, panel tabs and folds, dialogs and the terminal dock. Commands drive the existing controls by id, so `main.ts` keeps its handlers. A new action is a new command plus a menu entry; its default keys must not collide with an existing combo.
- Menus, the palette and dialogs render into `#overlay-root` with `--z-menu` and `--z-modal`; never a raw z-index.
- `html.shell-pending` and `html.i18n-pending` hide the app until the shell and the page translator have run, so tabs, folds and translations never flash. Keep that order in `main.ts`: `startLanguage`, `startShell`, `startPageTranslator`.
- Text is written in English. Add each new string to `frontend/src/i18n/strings.json` and to all 25 tables (`locales` or `more`) in the same change; `{0}` placeholders stay intact. Elements whose text must not translate carry `translate="no"`.
- The terminal dock and chat pane talk to `/api/commands/*`, `/api/terminal/{id}` (WebSocket) and `/api/events` (SSE). Opening Claude Code or Chat turns Agent link on, because the MCP tools reach the editor through it.

## Verifying

Typecheck with `npx tsc --noEmit`. Exercise each touched control with CPU throttled 4x and watch for jumps or flashes; state whether the check ran on a real GPU.
