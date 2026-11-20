# Painting with Agents

An agent paints in the editor the person has open. Nothing reaches the editor until the person presses Agent link in the top bar; it is off by default.

| Where | What lives there |
| --- | --- |
| `mcp/server.py` | the MCP server: 68 `formsurface_*` tools over stdio |
| `mcp/launch` | what `./formsurface mcp` runs: starts the service on 127.0.0.1:8766 when it is down, then the server |
| `mcp/batch.py`, `mcp/headless.py` | the JSON batch runner and the headless run (service, editor and a windowless browser) |
| `mcp/formsurface_api`, `mcp/run_script.py` | the Python API and `./formsurface script` |
| `.mcp.json`, `.codex/config.toml`, `.cursor/mcp.json` | the MCP registration for Claude Code, Codex and Cursor |
| `.claude/skills/formsurface-painting`, `.agents/skills/formsurface-painting` | the painting skill and its tool reference |
| `.claude-plugin/plugin.json` | the same server and skill as a Claude Code plugin |

## Setting up

1. Run `./formsurface` and open http://localhost:5176.
2. Press Agent link in the top bar.
3. Start the agent in this folder. Claude Code reads `.mcp.json`, Codex reads `.codex/config.toml`, Cursor reads `.cursor/mcp.json`; each runs `./formsurface mcp`. `./formsurface claude` starts Claude Code already connected, with a 30 minute limit per tool call.

To use an agent outside this folder, `./formsurface agents install` registers the server for your user with Claude Code, Codex, Cursor and Antigravity (`--dry-run` first says what it would change). Inside the window, the agent dock (Ctrl+J, Cmd+J on a Mac) runs the same four programs in tabs, each connected with a token that ends when its tab closes, and opening one turns Agent link on.

## The flow

`formsurface_status` first: it reports the texture sets, layers, brush and camera, or says that no editor is linked. Then one idea per layer: set the base material, add a layer or a fill layer, bake the mesh maps once and drive wear with generators, take a screenshot, paint strokes with viewport points, check from a second angle, save the project, export the maps. The skill's `references/tools.md` lists every tool with its arguments.

## Without an open editor

`./formsurface batch --headless job.json` and `./formsurface script --headless job.py` start the service, the editor and a windowless Chrome, Chromium or Edge, run, and stop what they started.

## Contract for changes here

- Tool names, their arguments, routes such as `/api/smart-masks` and the `formsurface-smart-asset` file format stay as they are, so agents, scripts and shared files keep working.
- A new tool is added to `mcp/server.py` and to the tool reference in both skill folders in the same change, and its handler in the editor refuses an unknown layer id instead of reporting success.
- Commands run one at a time in the editor, and a stroke is always closed.
