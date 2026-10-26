---
name: formsurface-harness
description: Use when building, fixing or extending the FormSurface repo, the Form family 3D texture painter (Rust wgpu engine in WebAssembly, TypeScript and Vite editor, FastAPI service, MCP server). Use whenever work touches the FormSurface repo, including continue formsurface, fix formsurface painting, brush or stroke bugs, projection painting, seams, layers, blend modes, undo, the PBR viewport, export maps, mesh import, the agent link relay, formsurface MCP tools, formsurface releases, the planning sidecar, or Substance Painter parity rows. Also trigger on fs-engine, fs-core, setup/build_engine, ./formsurface mcp or formsurface-harness. Not for using FormSurface to paint a model through MCP, which is the formsurface-painting skill, and not for FormVis or FormCut.
license: Apache-2.0
metadata:
  version: "0.59.9"
  author: nodemode-ai
---

# FormSurface harness

FormSurface is meant to let a person paint on a 3D model the way Substance 3D Painter does. Every change is judged against that: a feature that exists in a menu but does not paint, or a parity row graded on a toy fixture, is the failure this repo was rebuilt to escape. The previous 0.2.0 build painted flat circles in UV space through an HTTP round trip and graded itself 100 percent; the rebuild started over at v0.1.0.

## Architecture in one screen

```
native/fs-core     mesh, BVH picking, brush, channel and layer types; no GPU
native/fs-engine   wgpu 30 on WebGPU: paint pass, apply, composite, dilation, PBR view, export, snapshot
frontend/          vanilla TS + Vite; calls the Engine class from frontend/src/engine/pkg
backend/app        FastAPI on 127.0.0.1:8766: projects, exports, /api/live relay, /api/agent
mcp/server.py      MCPServer (mcp 2.3.0) over stdio, tools call /api/agent
```

The editor never paints. Pointer samples go to `stroke_begin`, `stroke_to` and `stroke_end`; the engine raycasts through its BVH, spaces stamps in world units and paints in UV space on the GPU. Agents reach the same engine calls through the relay, so a hand stroke and an agent stroke are the same operation and undo covers both.

## Routing

| The work is about | Load |
| --- | --- |
| Rust, WGSL, painting, projection, layers, compositing, undo, export, viewport shading, wasm builds | `sub-skills/engine/SUB-SKILL.md` |
| Editor UI, panels, pointer handling, mesh loaders, DOM stability, the Agent link button | `sub-skills/editor/SUB-SKILL.md` |
| FastAPI service, the WebSocket relay, MCP tools, `.mcp.json`, `./formsurface mcp` | `sub-skills/service-mcp/SUB-SKILL.md` |
| Versions, commits, the two zips, the planning sidecar, parity grading, skill packs | `sub-skills/release/SUB-SKILL.md` |

Load one specialist for a narrow change; for a feature that crosses layers (a new engine capability exposed to agents, for example) load engine, then editor, then service-mcp, in that order, because each layer consumes the one before it.

## Laws that apply everywhere

- Surgical edits: one hand-made region per edit against a file read this turn. No sed, patch, codemod or script that rewrites source.
- Zero DOM flash: patch the node that changed, reserve space before async content, never rebuild a panel.
- Version strings read `v` digit dot digit dot digit and match across `frontend/package.json`, `native/Cargo.toml`, `backend/app/main.py` and `VERSION`.
- Planning documents and every HASHOUT_LOG live in the planning sidecar, never in the tree.
- No emojis, no em dashes in markdown, no icon libraries, Vite host stays false, z-index only from tokens.
- No self-written tests; testing intent goes to the sidecar TEST_PLAN.md for a fresh context.
- Never claim a capability that was not exercised. State what ran and on what hardware.
- GPU readbacks (screenshot, layer pick, bake, export) need a WebGPU adapter that can map buffers: headless SwiftShader cannot. On a machine without a GPU, run Chromium headful under `xvfb-run` on Mesa lavapipe (`VK_ICD_FILENAMES=.../lvp_icd.json`, flags `--enable-unsafe-webgpu --enable-features=Vulkan --use-vulkan=native --ignore-gpu-blocklist --disable-gpu-watchdog`). Without `--disable-gpu-watchdog` Chrome kills the slow software GPU process during heavy passes (path cover, bakes) and every later map fails with "Error occurred when trying to async map a buffer" and the console warning "A valid external Instance reference no longer exists"; that is the watchdog, not an engine fault.

## Evaluations

`EVALS.json` lists the requests this pack is checked against, with observable expected behaviour.
