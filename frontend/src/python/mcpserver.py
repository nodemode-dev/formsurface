import inspect
import typing

from pydantic import check


class ToolError(Exception):
    pass


class Image:
    def __init__(self, data=b"", format="png"):
        self.data = data
        self.format = format


class MCPServer:
    def __init__(self, name, instructions=""):
        self.name = name
        self.instructions = instructions
        self.registry = {}

    def tool(self, *_, **__):
        def register(fn):
            self.registry[fn.__name__] = fn
            return fn
        return register

    def names(self):
        return sorted(self.registry)

    def call(self, name, args):
        fn = self.registry.get(name)
        if fn is None:
            raise ToolError(f"No tool {name}")
        signature = inspect.signature(fn)
        hints = typing.get_type_hints(fn, include_extras=True)
        unknown = set(args) - set(signature.parameters)
        if unknown:
            raise ToolError(f"{name} has no argument {', '.join(sorted(unknown))}")
        clean = {}
        for key, param in signature.parameters.items():
            if key in args:
                try:
                    clean[key] = check(args[key], hints.get(key, typing.Any), key)
                except ValueError as error:
                    raise ToolError(f"{name}: {error}") from error
            elif param.default is inspect.Parameter.empty:
                raise ToolError(f"{name}: {key} is required")
        result = fn(**clean)
        if isinstance(result, Image):
            return bytes(result.data)
        return result
