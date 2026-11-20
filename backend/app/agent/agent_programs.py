import hmac
import json
import os
import secrets
import time
import uuid
from pathlib import Path

from app.agent import agent_lookup, claude_code, config, terminal

CONTRACT = "2.2.0"
NOT_INSTALLED = 424
PROGRAMS = {
    "codex": {"title": "Codex", "override": "FORMSURFACE_CODEX_BIN"},
    "cursor": {"title": "Cursor", "override": "FORMSURFACE_CURSOR_BIN"},
    "antigravity": {"title": "Antigravity", "override": "FORMSURFACE_ANTIGRAVITY_BIN"},
}
TAB_ENTRY = "formsurface_tab"
TOKEN_VAR = "FORMSURFACE_TAB_TOKEN"
TOKEN_HEADER = "x-formsurface-tab"
CLOSED_REPLY = "This FormSurface tab has closed, so its access has ended."
TOKENS: set[str] = set()
_cache: dict = {"at": 0.0, "value": None}


def issue() -> str:
    token = secrets.token_urlsafe(32)
    TOKENS.add(token)
    return token


def revoke(token: str | None) -> None:
    TOKENS.discard(token or "")


def valid(token: str | None) -> bool:
    return isinstance(token, str) and bool(token) and any(hmac.compare_digest(token.encode(), t.encode()) for t in list(TOKENS))


def _write_private(path: Path, text: str) -> None:
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    part = path.with_name(path.name + f".part-{os.getpid()}")
    with os.fdopen(os.open(part, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w", encoding="utf-8") as fh:
        fh.write(text)
    os.replace(part, path)


def tab_folder(kind: str) -> Path:
    folder = config.data_dir() / "agent-tabs" / kind
    folder.mkdir(parents=True, exist_ok=True)
    return folder


def private_file(kind: str, text: str) -> Path:
    path = tab_folder(kind) / f"mcp-{uuid.uuid4().hex[:12]}.json"
    _write_private(path, text)
    return path


def _env(path: str, token: str) -> dict:
    base = claude_code.child_env()
    return agent_lookup.child_env(path, base=base, front=[str(config.REPO_DIR)] if not config.FROZEN else [],
                                  extra={"FORMSURFACE_SERVICE": config.service_url(), TOKEN_VAR: token})


def _located(kind: str, fresh: bool = False) -> agent_lookup.Found:
    found = agent_lookup.find(kind, setting=None, override_vars=(PROGRAMS[kind]["override"],), fresh=fresh)
    return agent_lookup.check(found, agent_lookup.child_env(found.path), 15.0, fresh)


def _program(kind: str, fresh: bool) -> dict:
    found = _located(kind, fresh)
    install = agent_lookup.install(kind)
    out = {"installed": found.state == "runs", "version": found.version, "install": "" if found.state == "runs" else install,
           "signedIn": None, "label": PROGRAMS[kind]["title"]}
    if found.state == "broken":
        out["note"] = agent_lookup.sentence(found, install)
    return out


def status(fresh: bool = False) -> dict:
    if fresh or _cache["value"] is None or time.time() - _cache["at"] >= 60:
        _cache.update(at=time.time(), value={kind: _program(kind, fresh) for kind in PROGRAMS})
    claude = claude_code.info(fresh=fresh)
    return {"claude": {"installed": claude["installed"], "version": claude["version"], "signedIn": None,
                       "install": "" if claude["installed"] else agent_lookup.install("claude")}, **_cache["value"], "contract": CONTRACT}


def _servers(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8") or "{}") if path.exists() else {}
    if not isinstance(data, dict) or not isinstance(data.get("mcpServers", {}), dict):
        raise OSError(f"{path} is not an MCP servers file FormSurface can read, so it was left alone")
    return data


def _entry(env: dict) -> dict:
    command, args = claude_code.launcher()
    return {"command": command, "args": args, "env": env}


def antigravity_file(folder: Path) -> Path:
    return folder / ".agents" / "mcp_config.json"


def write_tab_entry(folder: Path, token: str) -> None:
    path = antigravity_file(folder)
    data = _servers(path)
    data.setdefault("mcpServers", {})[TAB_ENTRY] = _entry({"FORMSURFACE_SERVICE": config.service_url(), TOKEN_VAR: token})
    _write_private(path, json.dumps(data, indent=2) + "\n")


def remove_tab_entry(folder: Path, token: str) -> None:
    path = antigravity_file(folder)
    try:
        data = _servers(path)
        entry = data.get("mcpServers", {}).get(TAB_ENTRY)
        if not isinstance(entry, dict) or (entry.get("env") or {}).get(TOKEN_VAR) != token:
            return
        del data["mcpServers"][TAB_ENTRY]
        _write_private(path, json.dumps(data, indent=2) + "\n")
    except (OSError, ValueError):
        return


def cursor_config(folder: Path) -> None:
    path = folder / ".cursor" / "mcp.json"
    entry = _entry({"FORMSURFACE_SERVICE": "${env:FORMSURFACE_SERVICE}", TOKEN_VAR: "${env:" + TOKEN_VAR + "}"})
    try:
        data = _servers(path)
    except (OSError, ValueError):
        return
    if data.get("mcpServers", {}).get(TAB_ENTRY) == entry:
        return
    data.setdefault("mcpServers", {})[TAB_ENTRY] = entry
    _write_private(path, json.dumps(data, indent=2) + "\n")


def codex_args() -> list[str]:
    command, args = claude_code.launcher()
    key = f"mcp_servers.{TAB_ENTRY}"
    return ["-c", f"{key}.command={json.dumps(command)}", "-c", f"{key}.args={json.dumps(args)}",
            "-c", f'{key}.env_vars=["FORMSURFACE_SERVICE","{TOKEN_VAR}"]', "-c", f"{key}.tool_timeout_sec=1800"]


def open_terminal(kind: str, cols: int, rows: int) -> dict:
    if kind not in PROGRAMS:
        raise KeyError(kind)
    found = _located(kind)
    if found.state != "runs":
        found = _located(kind, fresh=True)
    if found.state != "runs":
        raise LookupError(agent_lookup.sentence(found, agent_lookup.install(kind)))
    own = tab_folder(kind) if kind == "antigravity" else None
    folder = own or claude_code.workdir()
    if kind == "cursor":
        cursor_config(folder)
    token = issue()
    argv = agent_lookup.start_argv([found.path] + (codex_args() if kind == "codex" else []))
    try:
        if own:
            write_tab_entry(own, token)
        session = terminal.open_session(str(folder), cols, rows, PROGRAMS[kind]["title"], argv, _env(found.path, token), kind)
    except BaseException:
        revoke(token)
        if own:
            remove_tab_entry(own, token)
        raise
    session.on_close.append(lambda: revoke(token))
    if own:
        session.on_close.append(lambda: remove_tab_entry(own, token))
    return session.describe()
