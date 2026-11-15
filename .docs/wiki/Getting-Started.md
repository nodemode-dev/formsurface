# Getting Started

## Requirements

Rust through rustup, Node 22.12 or later, Python (the README's "Run it from source" section names the version), and a browser with WebGPU or WebGL 2. `./formsurface doctor` checks the toolchain, the builds and the running service.

## Install and boot

```
./formsurface install      # wasm target, wasm-bindgen, service venv, editor packages, engine build
./formsurface              # service on 127.0.0.1:8766, editor on http://localhost:5176
```

Windows: `formsurface_oc_install.bat`, then `formsurface.bat`.

## Every command

| Command | What it does |
| --- | --- |
| `./formsurface` or `./formsurface boot` | builds the engine if needed, then the service and the editor together |
| `./formsurface backend`, `./formsurface frontend` | one half only |
| `./formsurface engine` | rebuild the Rust engine into `frontend/src/engine/pkg` |
| `./formsurface build`, `./formsurface serve` | build the engine and the editor, serve the built editor from the service |
| `./formsurface mcp` | the MCP server over stdio |
| `./formsurface batch FILE` | run a JSON list of tool calls against the linked editor (`--headless` runs without an open editor) |
| `./formsurface script FILE` | run a Python script that imports `formsurface_api` |
| `./formsurface claude` | Claude Code in this terminal, connected to the MCP server |
| `./formsurface install-desktop`, `./formsurface desktop dev`, `./formsurface desktop build` | the Tauri desktop app (`desktop build` is `./build_app`) |
| `./formsurface freeze` | freeze the service into the desktop sidecar |
| `./formsurface doctor` | checks |

## Ports and folders

The service listens on 127.0.0.1:8766 (`FORMSURFACE_PORT`), the Vite editor on localhost:5176. From source the workspace folder is `workspace/` in the repository; `FORMSURFACE_DATA` moves it. It holds meshes, images, environments, exports and `projects/`, the project library that Home lists.

## First paint

On Home choose a template, or drop a mesh with UVs (glTF, GLB, OBJ, FBX, USD, PLY or Collada). Paint on the model, add layers in the Layers tab, then File, Save Project. The project appears under Recent on Home with a preview. See [Home and Projects](Home-and-Projects) next.
