# Architecture

formsurface is four parts: a Rust engine that owns the GPU, a TypeScript editor around it, a Python service for files, and a Tauri shell for the desktop.

| Where | What lives there |
| --- | --- |
| `native/fs-core` | mesh, BVH and document types; no GPU code; has the engine's tests |
| `native/fs-engine` | every GPU resource, on wgpu; exposes the `Engine` class to JavaScript through wasm-bindgen; WebAssembly only |
| `frontend/src/engine/pkg` | build output of `setup/build_engine`, git-ignored |
| `frontend/src/main.ts` | the editor: canvas, pointer handling, panels, document state |
| `frontend/src/shell` | commands, menus, palette, Home, dialogs, the dock adapter, Chat, themes, icons |
| `frontend/src/dock` | the Form family's shared agent dock module 2.2.1, copied unchanged from FormVis (hashes in `MANIFEST.json`) |
| `frontend/src/i18n` | English source strings and the language tables |
| `backend/app/main.py` | the FastAPI service: projects, maps, meshes, exports, version |
| `backend/app/project_library.py`, `backend/app/projects.py` | the project library behind Home |
| `backend/app/agent` | the command bus: terminals, the agent programs (Claude Code, Codex, Cursor, Antigravity), Chat, preferences, events |
| `backend/app/local_guard.py` | the loopback, Host and Origin checks on every request |
| `backend/app/ocio.py`, `backend/app/environments.py` | OpenColorIO and HDR environments |
| `mcp` | the MCP server, the batch runner and the Python API |
| `src-tauri/src` | the desktop shell: splash and reveal, the engine sidecar, the quit gate, text-field Edit routing |

## How painting flows

The editor never paints itself. It sends pointer positions to the engine's `stroke_begin`, `stroke_to` and `stroke_end`; the engine raycasts onto the mesh, spaces the stamps in world space and paints in UV space on the GPU. Compositing of the layer stack stays on the GPU too. The engine runs on WebGPU in the browser, with a WebGL 2 path; path tracing and the ray-traced bake need WebGPU.

## How the desktop app starts

The shell shows the splash, starts the frozen service (a PyInstaller program built by `setup/freeze_backend`), waits until it listens, then opens the main window on the service's own page. A second launch is handed to the running window. Quit, Cmd+Q and the close button ask the page first; on quit the shell asks the service to stop before it ends it.

## Contract for changes here

- `fs-core` stays free of GPU code, and `fs-engine` is checked with `--target wasm32-unknown-unknown` only.
- The service binds 127.0.0.1 only, and every route passes the local guard.
- A file name from a request is validated before it touches the disk, and writes are atomic.
- Every editor action is a command in `frontend/src/shell/app-shell.ts`.
