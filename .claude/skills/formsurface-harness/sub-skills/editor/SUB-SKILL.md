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
- Styles: `src/styles/tokens.css` (colors, spacing, z-index layers) and `src/styles/app.css`. Every z-index is a `--z-*` token.

## Rules

- Pointer input goes to the engine as coalesced samples with pressure. Never paint in TypeScript.
- Controls change engine state through `syncBrush`, `syncChannels`, `syncBase`, `syncView`. Agent handlers set the same inputs with `setControl`, so the panel always shows what the engine uses.
- DOM stability: write the one property that changed. Reserve width for text that changes length (see `#agent-label`, `#fps`). Never clear a container to re-render it.
- Vite stays `host: false` on port 5176 with the `/api` proxy and `ws: true`.

## Verifying

Typecheck with `npx tsc --noEmit`. Exercise each touched control with CPU throttled 4x and watch for jumps or flashes; state whether the check ran on a real GPU.
