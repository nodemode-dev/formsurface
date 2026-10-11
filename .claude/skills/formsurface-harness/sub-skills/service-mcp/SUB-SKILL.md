---
name: service-mcp
description: The FormSurface FastAPI service, the editor relay and the MCP server. Load for endpoints, /api/live and /api/agent, MCP tool changes, .mcp.json and ./formsurface mcp.
---

# Service and MCP

## Flow

MCP client, then `mcp/server.py` (stdio, MCPServer from mcp 2.3.0), then `POST /api/agent {command, args, timeout}`, then the relay sends `{id, command, args}` over `/api/live` to the editor, which runs the handler and answers `{id, ok, result | error}`. No editor linked returns 409, an editor refusal 422, a timeout 504, each with a sentence the agent can act on.

## Adding or changing a tool

1. Add or change the editor handler in `frontend/src/main.ts` (the `AgentLink` map). It must call the same engine methods and controls a person uses.
2. Add or change the `formsurface_*` tool in `mcp/server.py` with typed, bounded parameters and a docstring that says what changes and what the coordinates mean.
3. Update the tool table in `.claude/skills/formsurface-painting/references/tools.md` and its mirror under `.agents/skills`.
4. Exercise it: link an editor, call the command over `curl -X POST 127.0.0.1:8766/api/agent`, then through an MCP stdio client.

## Rules

- Local only: the HTTP middleware and the WebSocket both refuse non-local clients; the MCP server refuses a non-local service URL.
- Tools never execute code or read arbitrary files. Exports land under `workspace/`.
- Keep `requirements.txt` and `requirements_macos.txt` identical unless a platform needs a difference.
