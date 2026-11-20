import json
import os
from pathlib import Path

import pytest

pytestmark = pytest.mark.skipif(os.name == "nt", reason="the stand-in agy is a POSIX shell script")
STAND_IN = '#!/bin/sh\nif [ "$1" = "--version" ]; then echo "0.0.0 (stand-in)"; exit 0; fi\nexec sleep 30\n'


@pytest.fixture(autouse=True)
def stand_in_agy(tmp_path, monkeypatch):
    path = tmp_path / "bin" / "agy"
    path.parent.mkdir()
    path.write_text(STAND_IN, encoding="utf-8")
    path.chmod(0o755)
    monkeypatch.setenv("FORMSURFACE_ANTIGRAVITY_BIN", str(path))
    return path


def entry(folder: Path) -> dict:
    data = json.loads((folder / ".agents" / "mcp_config.json").read_text(encoding="utf-8"))
    return data["mcpServers"]["formsurface_tab"]


def test_two_antigravity_tabs_each_keep_their_own_entry():
    from app.agent import agent_programs, terminal

    first = agent_programs.open_terminal("antigravity", 80, 24)
    second = agent_programs.open_terminal("antigravity", 80, 24)
    try:
        one, two = Path(first["cwd"]), Path(second["cwd"])
        assert one != two
        assert one.parent == two.parent == agent_programs.tab_folder("antigravity")
        token_one = entry(one)["env"][agent_programs.TOKEN_VAR]
        token_two = entry(two)["env"][agent_programs.TOKEN_VAR]
        assert token_one != token_two
        assert agent_programs.valid(token_one) and agent_programs.valid(token_two)
        assert (one / ".agents" / "mcp_config.json").stat().st_mode & 0o777 == 0o600

        terminal.close(second["id"])
        assert not two.exists()
        assert not agent_programs.valid(token_two)
        assert entry(one)["env"][agent_programs.TOKEN_VAR] == token_one
        assert agent_programs.valid(token_one)
    finally:
        terminal.close(first["id"])
        terminal.close(second["id"])
    assert not Path(first["cwd"]).exists()


def test_a_tab_that_fails_to_start_leaves_no_folder(monkeypatch):
    from app.agent import agent_programs, terminal

    before = set(agent_programs.tab_folder("antigravity").iterdir())

    def refuse(*args, **kwargs):
        raise OSError("no terminal")

    monkeypatch.setattr(terminal, "open_session", refuse)
    with pytest.raises(OSError):
        agent_programs.open_terminal("antigravity", 80, 24)
    assert set(agent_programs.tab_folder("antigravity").iterdir()) == before
