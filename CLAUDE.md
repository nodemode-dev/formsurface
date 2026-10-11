# FormSurface

3D texture painting app (Substance 3D Painter parity target). Rust engine on wgpu, compiled to WebAssembly; vanilla TypeScript and Vite editor; Python and FastAPI service.

## Repo layout notes

- `native/fs-core` has no GPU code: mesh, BVH, document types. `native/fs-engine` owns every GPU resource and exposes the `Engine` class to JavaScript through wasm-bindgen.
- The editor never paints itself. It sends pointer positions to `Engine.stroke_begin/stroke_to/stroke_end`; the engine raycasts, spaces stamps in world space and paints in UV space on the GPU.
- `frontend/src/engine/pkg` is build output from `setup/build_engine` and is git-ignored.
- Planning documents (DEV_PLAN, BUILD_NOTES, TEST_PLAN, HASHOUT_LOG, parity) ship in the planning sidecar zip, not in this tree.
- Edits are surgical: one region per edit, no scripted or patch-applied changes. No emojis, no icon libraries, Vite stays on host false.
