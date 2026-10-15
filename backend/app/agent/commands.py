from dataclasses import dataclass, field
from typing import Any, Callable


class CommandError(Exception):
    def __init__(self, message: str, status: int = 400):
        super().__init__(message)
        self.status = status


@dataclass
class Spec:
    id: str
    label: str
    fn: Callable[..., Any]
    params: dict[str, dict] = field(default_factory=dict)
    description: str = ""


REGISTRY: dict[str, Spec] = {}


def command(cid: str, label: str, params: dict[str, dict] | None = None, description: str = "", mutates: bool = False) -> Callable:
    def wrap(fn: Callable) -> Callable:
        if cid in REGISTRY:
            raise RuntimeError(f"command registered twice: {cid}")
        REGISTRY[cid] = Spec(cid, label, fn, params or {}, description or label)
        return fn
    return wrap


def dispatch(cid: str, params: dict) -> Any:
    spec = REGISTRY.get(cid)
    if spec is None:
        raise CommandError(f"Unknown command {cid}; GET /api/commands lists them", 404)
    known = {k: v for k, v in params.items() if k in spec.params}
    return spec.fn(**known)
