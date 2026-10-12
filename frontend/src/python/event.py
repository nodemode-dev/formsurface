"""Editor events for embedded scripts, after Painter's event module: connect a function to an event name and it runs each time the event fires, with a dict of details."""

import json

import js
from pyodide.ffi import create_proxy

EVENTS = ("mesh_loaded", "stroke_end", "layer_stack_changed", "texture_set_changed", "export_finished", "bake_finished")

_handlers = {}


def connect(name: str, handler) -> None:
    """Run handler(details) whenever the event fires. Events: mesh_loaded, stroke_end, layer_stack_changed, texture_set_changed, export_finished, bake_finished."""
    if name not in EVENTS:
        raise ValueError(f"Unknown event {name}. Events: {', '.join(EVENTS)}")
    proxy = create_proxy(lambda details: handler(json.loads(details)))
    _handlers.setdefault(name, []).append((handler, proxy))
    js.formsurfaceOn(name, proxy)


def disconnect(name: str, handler) -> None:
    """Stop calling handler for the event."""
    kept = []
    for h, proxy in _handlers.get(name, []):
        if h is handler:
            js.formsurfaceOff(name, proxy)
            proxy.destroy()
        else:
            kept.append((h, proxy))
    _handlers[name] = kept
