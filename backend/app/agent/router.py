import asyncio
import json
import urllib.parse
from contextlib import asynccontextmanager
from pathlib import Path

from fastapi import APIRouter, HTTPException, Request, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse, StreamingResponse

from app import window_key
from app.agent import claude_code, cmd_agents, cmd_chat, cmd_prefs, cmd_terminal, config, events, external, terminal
from app.agent.commands import REGISTRY, CommandError, dispatch

router = APIRouter()
router.include_router(external.router)
LOCAL = ("localhost", "127.0.0.1", "::1", "tauri.localhost")
MODULES = (claude_code, cmd_agents, cmd_chat, cmd_prefs, cmd_terminal)
TAB_HEADER = "x-formsurface-tab"
WINDOW_ONLY = "This route belongs to the FormSurface window; an agent tab's token is not accepted here."


def _local(origin: str | None) -> bool:
    if not origin:
        return True
    return (urllib.parse.urlparse(origin).hostname or "") in LOCAL or origin.startswith("tauri://")


@asynccontextmanager
async def lifespan(_app):
    events.bind(asyncio.get_running_loop())
    await terminal.start()
    try:
        yield
    finally:
        cmd_chat.cancel()
        await terminal.stop()


@router.get("/api/commands")
def list_commands() -> dict:
    return {"commands": [{"id": s.id, "label": s.label, "params": s.params, "description": s.description} for s in REGISTRY.values()]}


@router.post("/api/commands/{cid}")
async def run_command(cid: str, request: Request) -> dict:
    if not _local(request.headers.get("origin")):
        raise HTTPException(403, "FormSurface only answers this computer")
    if TAB_HEADER in request.headers:
        raise HTTPException(403, WINDOW_ONLY)
    try:
        params = await request.json()
    except ValueError:
        params = {}
    if not isinstance(params, dict):
        raise HTTPException(400, "The body must be a JSON object")
    try:
        result = await asyncio.to_thread(dispatch, cid, params)
    except CommandError as exc:
        raise HTTPException(exc.status, str(exc)) from None
    return {"ok": True, "command": cid, "result": result}


@router.get("/api/events")
async def stream(request: Request) -> StreamingResponse:
    if TAB_HEADER in request.headers:
        raise HTTPException(403, WINDOW_ONLY)
    window_key.held_or_ticket(request)
    queue = events.subscribe()

    async def gen():
        try:
            yield f"retry: 1500\ndata: {json.dumps({'type': 'hello'})}\n\n"
            while True:
                if await request.is_disconnected():
                    break
                try:
                    message = await asyncio.wait_for(queue.get(), timeout=15.0)
                except asyncio.TimeoutError:
                    yield ": ping\n\n"
                    continue
                yield f"data: {message}\n\n"
        finally:
            events.unsubscribe(queue)

    return StreamingResponse(gen(), media_type="text/event-stream", headers={"Cache-Control": "no-store", "X-Accel-Buffering": "no"})


@router.get("/api/chat/image")
def chat_image(path: str, request: Request) -> FileResponse:
    window_key.held_or_ticket(request)
    folder = (config.data_dir() / "Chat").resolve()
    target = Path(path).resolve()
    if target.parent != folder or not target.is_file():
        raise HTTPException(404, "No such chat image")
    return FileResponse(target)


@router.websocket("/api/terminal/{sid}")
async def attach(ws: WebSocket, sid: str) -> None:
    if not _local(ws.headers.get("origin")) or TAB_HEADER in ws.headers:
        await ws.close(code=4403)
        return
    await ws.accept()
    if not await window_key.socket_admitted(ws):
        await ws.close(code=4403)
        return
    try:
        session = terminal.get(sid)
    except KeyError:
        await ws.close(code=4404)
        return
    q: asyncio.Queue = asyncio.Queue()
    session.subscribers.add(q)

    async def outbound() -> None:
        backlog = session.scrollback()
        if backlog:
            await ws.send_text(backlog)
        if not session.alive:
            await ws.send_text(json.dumps({"type": "exit"}))
        while True:
            text = await q.get()
            await ws.send_text(json.dumps({"type": "exit"}) if text is None else text)

    sender = asyncio.create_task(outbound())
    try:
        while True:
            raw = await ws.receive_text()
            try:
                msg = json.loads(raw)
            except ValueError:
                continue
            if msg.get("type") == "input" and session.alive:
                terminal.write(sid, str(msg.get("data", "")))
            elif msg.get("type") == "resize":
                terminal.resize(sid, int(msg.get("cols", 80)), int(msg.get("rows", 24)))
    except WebSocketDisconnect:
        pass
    finally:
        sender.cancel()
        session.subscribers.discard(q)
