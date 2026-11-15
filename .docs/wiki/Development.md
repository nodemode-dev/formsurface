# Development

Engine: `native/` (Rust, wgpu, built to WebAssembly by `setup/build_engine`). Editor: `frontend/` (vanilla TypeScript and Vite on port 5176 with host false, three.js only for its file loaders). Service: `backend/` (FastAPI, venv at `backend/venv`). Desktop: `src-tauri/`. The `formsurface-harness` skill in `.claude/skills/` maps the codebase for agents working on it; `CLAUDE.md` is the short rule set.

## Repo rules

- **Surgical edits.** Change one region at a time, by hand. No scripted or patch-applied changes.
- **Commands.** Every editor action is a command in `frontend/src/shell/app-shell.ts` (registry in `frontend/src/shell/commands.ts`); menus, the palette, title bar buttons and shortcuts all run commands.
- **No emoji, no em dashes**, in code, interface text or markdown. Use the interpunct, commas, colons or parentheses.
- **Icons** are inline `currentColor` SVG from `frontend/src/shell/icons.ts`. No icon libraries.
- **z-index** only through the layer tokens in `frontend/src/styles/tokens.css`.
- **Zero DOM flash.** Build structure once and patch nodes; reserve space for content that arrives later; apply the theme before first paint.
- **Languages.** A new user-visible string goes into `strings.json` and every language table under `frontend/src/i18n` in the same change.
- **Names.** Other products are named only for compatibility, export templates, file formats, `docs/PARITY.md` and the trademark notice. FormSurface's features carry its own names.
- **Licences.** Only permissive dependencies, each recorded in `NOTICE` in the same change.

## Checks

```
setup/build_engine debug
(cd frontend && npm run typecheck && npm run build)
(cd backend && python -m pytest tests -q -p no:cacheprovider)
(cd native && cargo check --locked --target wasm32-unknown-unknown -p fs-core -p fs-engine && cargo test --locked -p fs-core)
python -m pytest frontend/tests/window -q -p no:cacheprovider
```

The service suite includes the version check: every version site listed in `backend/tests/conftest.py` must carry the same version. The window smoke test loads the built editor and the splash page under a slow CPU and network and fails on a layout shift, a console error or a failed request.

## Commits and releases

Conventional Commits with the version in the summary, `type(scope): summary (x.y.z)`, and a body. The version bump is the last commit of a release and changes every version site at once. Releases are built by formcompiler from a `compile_vX.Y.Z` branch. The full model is in `.docs/BRANCHING.md`.

## Docs

`docs/PARITY.md` tracks coverage against the reference painter. This wiki is `.docs/wiki/`, published by `.github/workflows/wiki.yml` on push to `main`. Planning documents (the plan, the build notes, the test plan) ship in the planning sidecar, not in this tree.
