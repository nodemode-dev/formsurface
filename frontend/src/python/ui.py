"""Plugin buttons for embedded scripts: a script adds named buttons to the Python panel, and each click runs its function."""

import js
from pyodide.ffi import create_proxy

_buttons = {}


def add_button(label: str, action) -> None:
    """Add (or replace) a button in the Python panel that runs action() when clicked."""
    remove_button(label)
    proxy = create_proxy(action)
    _buttons[label] = proxy
    js.formsurfaceButton(label, proxy)


def remove_button(label: str) -> None:
    """Remove a plugin button."""
    proxy = _buttons.pop(label, None)
    if proxy is not None:
        js.formsurfaceButton(label, None)
        proxy.destroy()


def log(*parts) -> None:
    """Write a line to the Python panel's output."""
    js.formsurfaceLog(" ".join(str(p) for p in parts))
