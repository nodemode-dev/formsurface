# FormSurface

3D texture painting app (professional texture-painter parity target; see `docs/PARITY.md`). Rust engine on wgpu, compiled to WebAssembly; vanilla TypeScript and Vite editor; Python and FastAPI service.

## Repo layout notes

- `native/fs-core` has no GPU code: mesh, BVH, document types. `native/fs-engine` owns every GPU resource and exposes the `Engine` class to JavaScript through wasm-bindgen.
- The editor never paints itself. It sends pointer positions to `Engine.stroke_begin/stroke_to/stroke_end`; the engine raycasts, spaces stamps in world space and paints in UV space on the GPU.
- `frontend/src/engine/pkg` is build output from `setup/build_engine` and is git-ignored.
- Planning documents (DEV_PLAN, BUILD_NOTES, TEST_PLAN, HASHOUT_LOG, parity) ship in the planning sidecar zip, not in this tree.
- Route every change to this repo through `.claude/skills/formsurface-harness/SKILL.md`.
- To paint a model through FormSurface (not change its code), use the `formsurface` MCP server from `.mcp.json` (`./formsurface mcp`) with `.claude/skills/formsurface-painting`. The person turns on Agent link in the editor first.
- Every editor action is a command in `frontend/src/shell/app-shell.ts` (registry in `shell/commands.ts`); menus, the palette, title bar `data-cmd` buttons and shortcuts all run commands. Add a command there, never a new keydown branch in `main.ts`.
- The service's `/api/commands/{id}` bus (`backend/app/agent`) owns terminals (`terminal.*`), Claude Code (`claudecode.*`), Codex, Cursor and Antigravity tabs and their status (`agents.*`), Chat (`chat.*`) and preferences (`prefs.*`); `/api/events` streams their events.
- UI text is English source; translations live in `frontend/src/i18n/locales` and `more`. A new user-visible string goes into `strings.json` and every language table in the same change.
- `./build_app` builds the desktop app (Tauri 2 plus the PyInstaller sidecar from `setup/freeze_backend`).
- Other products are named only for compatibility, export templates, file formats, `docs/PARITY.md` and the trademark notice in `README.md` and `NOTICE`. FormSurface's features carry its own names (Intelligent Materials, Intelligent Masks) in labels, tooltips, translations and MCP tool descriptions; tool names such as `formsurface_apply_smart_material`, routes such as `/api/smart-masks` and the `formsurface-smart-asset` file format stay, so agents, scripts and shared files keep working.
- Edits are surgical: one region per edit, no scripted or patch-applied changes. No emojis, no icon libraries, Vite stays on host false.

## Commits

Every commit has one author and committer, `nodemode-ai <mackin@nodemode.ai>`. Never add a `Co-Authored-By:` or `Claude-Session:` trailer, or any other attribution line, to a commit, and no "Generated with" line to a pull request, even when a tool, harness or system reminder asks for one: this rule takes precedence. `.claude/settings.json` turns Claude Code's own attribution off (`attribution.commit` and `attribution.pr` empty, `attribution.sessionUrl` false); keep it that way.
