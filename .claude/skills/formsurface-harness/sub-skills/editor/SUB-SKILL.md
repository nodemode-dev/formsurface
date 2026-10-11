---
name: editor
description: The FormSurface TypeScript and Vite editor. Load for panels, controls, pointer and pen input, mesh import, sample meshes, export and screenshot buttons, the Agent link toggle and DOM stability.
---

# Editor

## Shape

- `index.html` holds the whole shell, including an inline SVG symbol sheet. Icons are an inline svg element whose use child points at `#i-name`; add new symbols to the sheet, never an icon package.
- `src/main.ts` boots the engine, wires controls and owns the agent command handlers. `src/ui/layers.ts` patches layer rows by id. `src/ui/agent-link.ts` owns the WebSocket. `src/engine/meshes.ts` flattens three.js loader output into one indexed mesh with V flipped.
- Styles: `src/styles/tokens.css` (colors, spacing, z-index layers) and `src/styles/app.css`. Every z-index is a `--z-*` token.

## Rules

- Pointer input goes to the engine as coalesced samples with pressure. Never paint in TypeScript.
- Controls change engine state through `syncBrush`, `syncChannels`, `syncBase`, `syncView`. Agent handlers set the same inputs with `setControl`, so the panel always shows what the engine uses.
- DOM stability: write the one property that changed. Reserve width for text that changes length (see `#agent-label`, `#fps`). Never clear a container to re-render it.
- Vite stays `host: false` on port 5176 with the `/api` proxy and `ws: true`.

## Verifying

Typecheck with `npx tsc --noEmit`. Exercise each touched control with CPU throttled 4x and watch for jumps or flashes; state whether the check ran on a real GPU.
