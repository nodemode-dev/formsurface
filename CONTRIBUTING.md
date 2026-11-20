# Contributing to FormSurface

Thank you for helping. FormSurface is the Form family's 3D texture painting app: paint
straight onto the model, layer by layer, with fill layers, generators, baking and PBR
export, in one window with no account, running on your own computer, and open to
agents over MCP. It gets better through real use and real contributions.

## Where to start

- **Use it and report what goes wrong.** A clear bug report, with steps, the version,
  your platform and whether the editor ran on WebGPU or WebGL 2, is one of the most
  useful things you can give.
- **Look at the open issues.** Comment on one before starting larger work, so nobody
  builds the same thing twice.
- **Translations** are welcome: the interface ships in English and 25 more languages.
  The source text is English; the tables live in `frontend/src/i18n/locales` and
  `frontend/src/i18n/more`, and a new user-visible string goes into `strings.json`
  and every language table in the same change.
- **Parity** with the reference painter is tracked row by row in
  [docs/PARITY.md](docs/PARITY.md); a few capabilities are partial.

## Setting up

Clone the repository and run `./formsurface install`, then `./formsurface` (on
Windows, `formsurface_oc_install.bat`, then `formsurface.bat`). The first installs the
Rust wasm target, wasm-bindgen, the service's Python environment and the editor's
packages, and builds the engine; the second starts the service on 127.0.0.1:8766 and
the editor at http://localhost:5176. `./formsurface doctor` checks the toolchain, the
builds and the running service, and `./formsurface desktop dev` opens the desktop
window after `./formsurface install-desktop`. The README's "Run it from source"
section has the requirements.

The code lives in:

| Folder | What is there |
| --- | --- |
| `native/fs-core` | mesh, BVH and document types (Rust, no GPU code) |
| `native/fs-engine` | the GPU engine (Rust, wgpu), compiled to WebAssembly |
| `frontend/src` | the editor (TypeScript, Vite): every action is a command in `shell/app-shell.ts` |
| `backend/app` | the service (Python, FastAPI): projects and the project library, maps, OpenColorIO, terminals and Claude Code |
| `backend/tests`, `frontend/tests/window` | the service suite and the window smoke test |
| `mcp` | the MCP server, the JSON batch runner and the Python API |
| `src-tauri` | the desktop shell (Rust, Tauri 2) |
| `setup` | the scripts behind `./formsurface` |
| `docs` | the parity table and the README's pictures |
| `assets` | repo-level art (the README logo and the desktop icon source) |
| `.claude/skills` | agent skills, mirrored into `.agents/skills` by `./formsurface agents sync` (`agents check` checks the kit) |
| `frontend/src/dock` | the shared agent dock module, copied unchanged from FormVis; never edit it here |
| `.docs/wiki` | the wiki's source, published on push |

## Before you open a pull request

Run the same checks CI runs:

```sh
setup/build_engine debug
(cd frontend && npm ci && npm audit --audit-level=low && npm run typecheck && npm run build)
(cd backend && pip install -r requirements.txt -r requirements-test.txt \
  && python -m compileall -q app formsurface_engine.py \
  && python -m pytest tests -q -p no:cacheprovider)
(cd native && cargo check --locked --target wasm32-unknown-unknown -p fs-core -p fs-engine \
  && cargo test --locked -p fs-core)
python -m pytest frontend/tests/window -q -p no:cacheprovider
```

`fs-engine` is built for WebAssembly only, so check it with the wasm target, never on
the host. The window smoke test needs `pytest` and `playwright` with Chromium
installed, and a built editor. CI also boots the service and probes `/api/health`,
runs `bash -n` over the shell scripts, and fails on an em dash or an emoji in tracked
Markdown.

- Keep each pull request to one change.
- Write commit subjects the way the history does, in Conventional Commits with the
  version in the summary: `type(scope): plain sentence (X.Y.Z)`, for example
  `feat(engine): ...`, `fix(editor): ...`, `docs(readme): ...`. Leave the version
  alone; a maintainer bumps every version site together when releasing, and the
  service suite fails when they disagree. See [.docs/BRANCHING.md](.docs/BRANCHING.md).
- Add a command in `frontend/src/shell/app-shell.ts`, never a new keydown branch in
  `main.ts`: the menus, the palette, the title bar buttons and the shortcuts all run
  commands.
- The editor never paints itself. It sends pointer positions to the engine, which
  raycasts and paints in UV space on the GPU.
- Icons are inline `currentColor` SVG in `frontend/src/shell/icons.ts`, `z-index`
  comes only from the layer tokens in `frontend/src/styles/tokens.css`, and Vite
  stays on host false.
- No emoji anywhere, no em dash in Markdown, and keep code comments to a minimum.
- Tool names such as `formsurface_apply_smart_material`, routes such as
  `/api/smart-masks` and the `formsurface-smart-asset` file format stay as they are,
  so agents, scripts and shared files keep working.

## How changes land

Only the maintainers push to `main`. Everyone else forks the repository and opens a
pull request into `main`. A pull request merges when every CI check passes on an
up-to-date branch and a maintainer has approved it after its last push.

## Releases

A release is a version, and only the maintainers make one: the version bump is the last
commit on `main`, a maintainer cuts a `compile_vX.Y.Z` branch from it, and every build
is compiled and released from
[FormCompiler](https://github.com/nodemode-dev/formcompiler), which builds every
platform and publishes them together. This repository holds the source; packaging and
releases stay in FormCompiler. See [.docs/BRANCHING.md](.docs/BRANCHING.md).

## Licensing

FormSurface is licensed under the Apache License 2.0 ([LICENSE](LICENSE), SPDX
`Apache-2.0`). By contributing, you agree that your contribution is licensed under the
same terms. Only add code you have the right to contribute, and name any third-party
code and its licence in the pull request.

## Patents and other people's IP

Some features are deliberately not built because they come too close to someone else's
patent, trademark, copyright or design. The maintainers hold that line with counsel;
contributors do not decide it, and nobody here redesigns a feature on their own reading
of a patent. This is engineering diligence, not legal advice.

### Rules for every contribution

- Patents: do not add a feature, or a dependency that provides one, that reproduces a method you know to be patented. If you are unsure, open an issue that describes what the feature does, not which patent it resembles. Patent numbers stay out of code, commits, comments, logs and changelogs.
- Trademarks: other vendors' product and feature names appear only to name a file format, an export template or compatibility ("opens <format> files", "for use with"), in `docs/PARITY.md` and in the trademark notice. No command, menu, panel, tool, preset, template, theme or agent tool takes another product's feature name, and no text says "like X" or "the way X does it". Describe what the feature does. FormSurface's features carry its own names (Intelligent Materials, Intelligent Masks).
- Copyright: never copy another product's help, tooltip, dialog or documentation text (link to it instead); never trace or import vendor icons; never bundle vendor templates, themes, presets, brushes, swatches, LUTs, materials, sample files, stock media or fonts. Test fixtures and template previews come from this repository's own scripts and the app's own captures.
- Licences: every new dependency, binary, model, font, profile or data file gets a row in `NOTICE` in the same commit that adds it. GPL stays outside the distributed app, AGPL nowhere, LGPL dynamically linked or a separate process and replaceable, MPL files unedited. CC BY credited, CC BY-SA only unmodified, CC NC and ND never.

## Conduct

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md). Security
problems go through [SECURITY.md](SECURITY.md), never a public issue.
