# formsurface

formsurface is the Form family's 3D texture painting app. The brush hits the mesh in world space and paints into the UV textures on the GPU, in a stack of paint, fill and filter layers with masks, generators and baking, and it exports PBR maps for game engines and renderers. It runs in the browser or as a Tauri desktop app, with a local service that holds projects and maps, and an MCP server so Claude Code, Codex or Cursor can paint in the editor you have open.

| Page | What it covers |
| --- | --- |
| [Getting Started](Getting-Started) | Install, boot, every launcher command, ports and the workspace folder |
| [Home and Projects](Home-and-Projects) | The Home screen, the project library, templates and Trash, the menu bar |
| [Painting with Agents](Painting-with-Agents) | Agent link, the MCP server, the batch runner and the Python API |
| [Architecture](Architecture) | The engine, the editor, the service and the desktop shell, with their source paths |
| [Development](Development) | Repo rules, checks, commits and releases |

The full documents live in the repository:

- [docs/PARITY.md](https://github.com/nodemode-dev/formsurface/blob/main/docs/PARITY.md): coverage against the reference painter, row by row.
- [README.md](https://github.com/nodemode-dev/formsurface/blob/main/README.md): what ships, and running from source.
- [CHANGELOG.md](https://github.com/nodemode-dev/formsurface/blob/main/CHANGELOG.md): what each version changed.

| Area | State | Note |
| --- | --- | --- |
| Painting, layers, masks, generators, baking | in use | on WebGPU, with a WebGL 2 path; path tracing and the ray-traced bake need WebGPU |
| Export, UDIM, texture sets | in use | very large export sizes need a GPU with the memory for them |
| Home, project library, menu bar | new in 0.61.0 | exercised in a browser on Linux |
| Desktop app | not run on macOS or Windows for 0.62.0 | the shell was compile-checked on Linux only; the native macOS menu bar was not run |
| Agents: MCP, batch, Python API | in use | Agent link is off by default |

These pages are the source of truth in the repository under `.docs/wiki/`. Edit them there; the wiki workflow publishes them. Writing rules: full words, real paths, no emoji and no em dash (use the interpunct).
