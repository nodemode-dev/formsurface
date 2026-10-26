import asyncio
import json
import re
import subprocess
import sys
import urllib.request

import pytest
from mcp import Client
from mcp.client.stdio import StdioServerParameters
from websockets.asyncio.client import connect

from conftest import MCP_SERVER, REPO, WS_PATH


def params(base):
    return StdioServerParameters(command=sys.executable, args=["-I", str(MCP_SERVER)], env={"FORMSURFACE_SERVICE": base}, cwd=str(REPO))


def declared_tools():
    return re.findall(r"^@server\.tool\(\)\ndef (\w+)\(", MCP_SERVER.read_text(encoding="utf-8"), re.MULTILINE)


def run(coro):
    return asyncio.run(asyncio.wait_for(coro, 90))


def text_of(result):
    return "".join(getattr(part, "text", "") for part in result.content)


def linked(engine):
    with urllib.request.urlopen(engine["base"] + "/api/health", timeout=2) as r:
        return json.loads(r.read())["editor_linked"]


async def call_with_editor(engine, reply):
    seen = []
    ready = asyncio.Event()

    async def editor():
        async with connect(f"ws://127.0.0.1:{engine['port']}{WS_PATH}") as ws:
            ready.set()
            message = json.loads(await ws.recv())
            seen.append(message)
            await ws.send(json.dumps({"id": message["id"], **reply}))
            await asyncio.sleep(0.2)

    task = asyncio.create_task(editor())
    await ready.wait()
    for _ in range(50):
        if await asyncio.to_thread(linked, engine):
            break
        await asyncio.sleep(0.1)
    async with Client(params(engine["base"])) as client:
        result = await client.call_tool("formsurface_status", {})
    await task
    return result, seen


def test_tools_list_names_exactly_the_declared_catalogue(engine):
    async def go():
        async with Client(params(engine["base"])) as client:
            return (await client.list_tools()).tools

    tools = run(go())
    names = [t.name for t in tools]
    declared = declared_tools()
    assert len(declared) >= 60
    assert sorted(names) == sorted(declared)
    assert len(set(names)) == len(names)
    for tool in tools:
        assert tool.name.startswith("formsurface_")
        assert tool.description
        assert tool.input_schema.get("type") == "object"


def test_status_without_a_linked_editor_is_a_tool_error_that_says_so(engine):
    async def go():
        async with Client(params(engine["base"])) as client:
            return await client.call_tool("formsurface_status", {})

    result = run(go())
    assert result.is_error
    assert "No FormSurface editor is linked" in text_of(result)


def test_status_reaches_a_linked_editor_and_returns_its_answer(engine):
    result, seen = run(call_with_editor(engine, {"ok": True, "result": {"layers": 3, "size": 2048}}))
    assert not result.is_error
    assert json.loads(text_of(result)) == {"layers": 3, "size": 2048}
    assert seen[0]["command"] == "status"


def test_editor_refusal_comes_back_as_a_tool_error(engine):
    result, _ = run(call_with_editor(engine, {"ok": False, "error": "no mesh is loaded"}))
    assert result.is_error
    assert "no mesh is loaded" in text_of(result)


@pytest.mark.parametrize("service", ["http://evil.example:8766", "http://192.168.1.10:8766"])
def test_mcp_server_refuses_to_talk_to_a_non_local_service(service):
    done = subprocess.run([sys.executable, "-I", str(MCP_SERVER)], env={"FORMSURFACE_SERVICE": service}, capture_output=True, text=True, timeout=60, stdin=subprocess.DEVNULL)
    assert done.returncode != 0
    assert "local FormSurface service" in done.stderr
