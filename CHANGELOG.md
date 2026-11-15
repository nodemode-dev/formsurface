# Changelog

Versions follow `v*.*.*` with dots. Dates are UTC. Entries before 0.61.0 were reconstructed from the commit history.

## 0.61.0 · 2026-11-15

FormSurface joins the Form family standard for Home, the menu bar and the splash, with a full review of the engine, the editor, the service and the desktop app behind it.

### Added

- Home: a start screen over a project library. A side panel carries Start, Recent, Templates and Trash with counts, the open project, Agents, Local only and Preferences; the main column shows your recent projects with previews and the templates, with filters and search. It shows at launch while no project is open and when the project closes (two preferences, both on by default), Esc leaves it while a project is open, and the brand in the title bar, File > Home and Alt+Ctrl+H (Alt+Cmd+H on macOS) bring it back.
- The project library: the projects folder now lists projects with dates and previews. From Home you can open, duplicate, pin, move to Trash, restore, delete for good and empty the Trash, each with an undo toast or a confirm bar.
- Templates: seven starting projects on meshes the app makes itself, from a plain sphere to a figure with two texture sets and a low and high-poly pair for baking.
- A project stores its mesh, so a project made from a mesh you picked from disk opens again.
- The family menu bar: one menu list feeds the bar in the window, the palette and the native macOS menu bar. The order is File, Edit, Tools, Layer, Bake, Agent, View, Window, Help. File starts with Home and gains Close Project and Exit, Edit (Cut, Copy, Paste, Select All) acts on a focused text field, and a new Window menu holds the panel items and themes. "Menus in Window" keeps the bar in the window on macOS.
- The family splash, with the app's own icon and "Starting formsurface".
- One running window: a second launch of the desktop app goes to the window that is already open.
- Quit, Cmd+Q and the window's close button ask first whenever a project is open. The app does not yet track unsaved changes, so the question also comes right after a save.
- Home, the library and the new menu items in all 25 languages.

### Changed

- The panel items moved from the View menu to the new Window menu.
- The dev service listens on 127.0.0.1 only.
- The desktop window opens once the engine is really listening, so it no longer opens blank on a slow start.

### Fixed

- Export and save work for any mesh file name. Export maps, Export mesh, Export layer or mask, Render and Save Project all failed for a name with a character such as a bracket or an accent, for example "model (1).glb".
- UV tiles: reloading a mesh keeps every UDIM tile's paint; it erased all tiles but the first while reporting that the layers were kept.
- Layers: removing a layer in the middle of a stroke no longer writes the stroke into another layer, and an unknown layer id is refused instead of acting on the bottom layer.
- Projects: a damaged project (no layers, or a layer id of 0) is refused instead of stopping or hanging the engine; projects made from the sample mesh open again.
- The editor says so when the engine download fails, when a mesh fails to load and when the graphics context is lost, instead of staying on "Starting the GPU engine" or freezing silently. After a lost context the engine starts again (layers are not recovered).
- Shortcuts: shortcut capture ends with its dialog instead of rebinding a command on the next key press, and the Path tool's keys no longer take Enter and Backspace from the Python editor.
- Languages: 58 strings that stayed English in every language are translated; nine translation folders were never loaded.
- The UV1 sample can be reached from the File menu.
- Terminals reconnect after a dropped connection, and Chat recovers after a failed load.
- Agents: handlers refuse an unknown layer id instead of reporting success; a headless run that fails to start stops what it started; Stop and exit end Claude Code's whole process tree.
- Service: project names and command arguments are checked (a name ending in a newline or ".." was accepted; bad arguments answered with an error 500).
- Desktop: the engine is asked to stop on quit and removes what it unpacked into the temp folder (Windows still leaves part of it).
- Windows: `formsurface.bat claude` builds valid settings, and the installed app's Terminal tab has a shell again.
- GPU memory is released explicitly on WebGPU.
- Home: the confirm bar (Empty Trash, Delete permanently) no longer covers the page title; it pushes the page down, as formdeck's does.
- Top bar: in a narrow window the version label, the Agent link label and the Export maps label collapse (the buttons keep their icons and tooltips), so nothing runs past the window's edge. The labels collapse below 1400 px, so languages with longer words are never cut.
- `NOTICE` names the single-instance plugin and the crates it brings, with their licences.
- An `.editorconfig`, as the family has it.

### Notes

- Nothing in this release was run on macOS or Windows. The desktop shell was compile-checked on Linux only, and the native macOS menu bar, Cmd+Opt+H beside the system's Hide Others, and the signed app were not run.

## 0.60.4 · 2026-11-05

- README rewritten to the Form family template, with dark screenshots of the main canvas, the v3 icons and the FormSurface app name. `.github/CODEOWNERS` added.

## 0.60.1 · 2026-10-31

- The Windows desktop app's window scripts keep their file types, so the window no longer opens blank.

## 0.60.0 · 2026-10-30

- `formsurface.bat desktop build` builds the desktop app on Windows, and the Windows installs stop on a failed step.
- `build_app` takes `--config`, and Linux builds the AppImage too.
- Headless runs end their whole process tree on Windows too.
- `.gitattributes`: LF in the tree and CRLF for Windows scripts.
- These changes were committed as 0.59.10 and ship as 0.60.0, because the patch number stays one digit.

## 0.59.5 to 0.59.9 · 2026-10-25 to 2026-10-26

- Only this computer's own window and agents reach the service; cross-site requests that carry no Origin are refused.
- The desktop release build keeps its symbols, so macOS builds succeed.
- A test suite for the family layers and the engine core runs in CI, with the window smoke test.
- The fps readout has its space reserved, so the status bar does not shift.

## 0.59.1 to 0.59.4 · 2026-10-22 to 2026-10-24

- Releases move to formcompiler, and the desktop app can take updates from it.
- Intelligent Materials and Intelligent Masks are the feature names in every language; a trademark notice; the parity table moved to `docs/PARITY.md`.
- Licence detail for components, frozen packages and the shell's crates; the copyright holder is NodeAgentc, Inc.; "by nodemode.ai" in About.

## 0.57.0 to 0.59.0 · 2026-10-22

- The Form family's 28 themes, chosen in Preferences and applied before first paint.
- The family splash card and the concealed reveal of the desktop window; the page side of the macOS overlay title bar.

## 0.56.0 to 0.56.3 · 2026-10-16 to 2026-10-21

- Save and reopen whole documents, with their layers and pixels.
- Built-in HDR environments for presentation.
- A new app icon.
- Fill names the texture set that owns a clicked part; image brush tips keep their proportions.

## 0.55.0 to 0.55.5 · 2026-10-15 to 2026-10-16

- Mirror across several axes at once, and file name templates for exported maps (`$mesh`, `$textureSet`, `$map`, `$udim` and more).
- Undo history follows a GPU memory budget per texture set.
- glTF meshes keep their texture orientation; export templates write an emissive map for emissive texture sets.
- Agent commands run one at a time and always close their strokes; Claude Code gets 30 minutes per tool call.

## 0.50.0 to 0.54.0 · 2026-10-15

- Paint across texture sets with one stroke.
- A docked HSV colour wheel under the base colour.
- The WebGL 2 path: painting, path cover, mesh bakes and depth of field work there; path tracing and the ray-traced bake are disabled with a reason.
- Layers whose mask is known to be blank are skipped when compositing.
- The desktop engine exits when the app is gone.

## 0.46.0 to 0.49.0 · 2026-10-15

- A second UV channel: import it, project fills through it and export maps onto it.
- Mask effect groups with nesting.
- A paintable cage offset for high-poly baking, and editing a layer in an external app.

## 0.43.0 to 0.45.0 · 2026-10-15

- Lasso and rectangle fills and limits, precise to the pixel; recent colours.
- Select the layer under the cursor; hide and isolate mesh parts.
- Menu commands for agents, and the Windows commands.

## 0.42.0 · 2026-10-15

- The new shell: a command registry, a menu bar, a command palette, a terminal dock and Chat, with Claude Code in the app.
- The interface in 25 more languages.
- The desktop app: `build_app`, a Tauri shell and the frozen service.

## 0.38.0 to 0.41.0 · 2026-10-12

- Export at another size, up to 8192, without touching the working textures, one layer at a time so memory does not grow with the stack.
- Two more named user channels per texture set.
- Idle texture sets can be paged out of GPU memory.

## 0.36.0 to 0.37.0 · 2026-10-12

- OpenColorIO: configs, displays, views and looks in the viewport, the background and the path tracer.
- An ACEScg working space.

## 0.32.0 to 0.35.0 · 2026-10-12

- A progressive path tracer in the viewport, with HDR importance sampling and a denoiser.
- Path render to a file at any size, depth of field, shadows through opacity cut-outs.
- Glass and subsurface scattering in the path tracer.

## 0.26.0 to 0.31.0 · 2026-10-12

- Instancing for every layer kind and for folders, across texture sets.
- A resolution per texture set or UV tile.
- Mask effect stacks.
- Python in the editor, with editor events and plugin buttons.
- An asset browser for saved materials and masks, with thumbnails and shared asset files.
- A fifth channel per texture set: user, opacity, ambient occlusion or emissive.

## 0.21.0 to 0.25.0 · 2026-10-12

- Layers are reprojected onto a reloaded mesh, and texture sets are carried across the reload.
- ACES, Standard, AgX and Filmic view transforms; headless batch and script runs.
- Ray-traced ambient occlusion and thickness baking.
- True polygon fill; n-gon, tessellated and displaced mesh export.
- Path ribbon and filled modes; warp projection and one fill image per UDIM tile.

## 0.14.0 to 0.20.0 · 2026-10-11

- The Path tool, the 2D UV view and text as a brush tip, stencil or fill.
- Export formats (TIFF, EXR, HDR, JPEG, PSD), padding modes and saved export templates.
- HDR environment lighting, more generators and saved masks.
- Merge down, flatten folder, export a layer, clone from a layer.
- 30 blend modes; levels, invert, posterize and pixelate filters; more high-poly bakes.
- Viewport post effects and custom cage meshes.
- USD, PLY and Collada import, a colour space per resource and the Python API.

## 0.7.0 to 0.13.0 · 2026-10-11

- Texture sets per material and UDIM tiles.
- Layer folders and export templates with channel packing.
- Polygon fill, 16-bit PNG export and baking in a worker.
- Quick mask, blur and sharpen filters.
- Mask sources; clone, flatten, mask export and mesh export.
- Matching low and high parts by name when baking, the batch runner and layer instancing.

## 0.2.0 to 0.6.0 · 2026-10-11

- Agent link, the MCP server and the agent skills.
- The brush engine: alignment, tips, jitter, lazy mouse, stencils and symmetry.
- Fill layers with projections, and layer masks.
- GPU mesh map baking, generators and filter layers.
- High-poly baking of normals and IDs with a cage; saved materials and masks.

## 0.1.0 · 2026-10-11

- The first version: a wgpu engine with 3D projection painting, the painting editor and the project and export service.
