from app.agent import agent_programs
from app.agent.commands import CommandError, command

KINDS = ", ".join(agent_programs.PROGRAMS)


@command("agents.status", "Agent Programs", {"fresh": {"type": "boolean"}}, mutates=False,
         description="Whether Claude Code, Codex, Cursor and Antigravity are installed on this computer, each with its version and an install line, and the dock contract version.")
def status(fresh: bool | None = None) -> dict:
    return agent_programs.status(bool(fresh))


@command("agents.open", "Open an Agent Program", {"kind": {"type": "string", "description": KINDS}, "cols": {"type": "integer"}, "rows": {"type": "integer"}}, mutates=False,
         description="Starts Codex, Cursor or Antigravity in a terminal tab of FormSurface's dock, connected to FormSurface's MCP tools with a token that ends when the tab closes. The window only.")
def open_(kind: str, cols: int | None = None, rows: int | None = None) -> dict:
    try:
        return agent_programs.open_terminal(kind, max(10, min(int(cols or 100), 500)), max(4, min(int(rows or 30), 200)))
    except KeyError:
        raise CommandError(f"No agent program called {kind}; the dock offers {KINDS}", 400) from None
    except LookupError as exc:
        raise CommandError(str(exc), agent_programs.NOT_INSTALLED) from None
    except (OSError, FileNotFoundError) as exc:
        raise CommandError(str(exc), 400) from None
