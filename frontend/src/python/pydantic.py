import re
import types
import typing
from inspect import isclass


class FieldInfo:
    def __init__(self, default=None, *, ge=None, gt=None, le=None, lt=None, min_length=None, max_length=None, pattern=None, **_):
        self.default = default
        self.ge, self.gt, self.le, self.lt = ge, gt, le, lt
        self.min_length, self.max_length, self.pattern = min_length, max_length, pattern

    def verify(self, value, name):
        if value is None:
            return
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            if self.ge is not None and value < self.ge:
                raise ValueError(f"{name} must be at least {self.ge}")
            if self.gt is not None and value <= self.gt:
                raise ValueError(f"{name} must be greater than {self.gt}")
            if self.le is not None and value > self.le:
                raise ValueError(f"{name} must be at most {self.le}")
            if self.lt is not None and value >= self.lt:
                raise ValueError(f"{name} must be less than {self.lt}")
        if isinstance(value, (list, tuple, str)):
            if self.min_length is not None and len(value) < self.min_length:
                raise ValueError(f"{name} needs at least {self.min_length} items")
            if self.max_length is not None and len(value) > self.max_length:
                raise ValueError(f"{name} takes at most {self.max_length} items")
        if self.pattern is not None and isinstance(value, str) and not re.match(self.pattern, value):
            raise ValueError(f"{name} does not match {self.pattern}")


def Field(default=None, **kwargs):
    return FieldInfo(default, **kwargs)


def check(value, hint, name):
    origin = typing.get_origin(hint)
    args = typing.get_args(hint)
    if origin is typing.Annotated:
        value = check(value, args[0], name)
        for meta in args[1:]:
            if isinstance(meta, FieldInfo):
                meta.verify(value, name)
        return value
    if origin in (typing.Union, types.UnionType):
        if value is None and type(None) in args:
            return None
        last = None
        for option in args:
            if option is type(None):
                continue
            try:
                return check(value, option, name)
            except ValueError as error:
                last = error
        raise last or ValueError(f"{name} has the wrong type")
    if origin is typing.Literal:
        if value not in args:
            raise ValueError(f"{name} must be one of {', '.join(repr(a) for a in args)}")
        return value
    if origin is list:
        if not isinstance(value, (list, tuple)):
            raise ValueError(f"{name} must be a list")
        return [check(v, args[0], name) for v in value] if args else list(value)
    if origin is tuple:
        if not isinstance(value, (list, tuple)) or (args and len(value) != len(args)):
            raise ValueError(f"{name} must be a pair of numbers")
        return tuple(check(v, a, name) for v, a in zip(value, args))
    if origin is dict or hint is dict:
        if not isinstance(value, dict):
            raise ValueError(f"{name} must be an object")
        return value
    if hint is bool:
        if not isinstance(value, bool):
            raise ValueError(f"{name} must be true or false")
        return value
    if hint is int:
        if isinstance(value, bool) or not isinstance(value, int):
            if isinstance(value, float) and value.is_integer():
                return int(value)
            raise ValueError(f"{name} must be a whole number")
        return value
    if hint is float:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ValueError(f"{name} must be a number")
        return float(value)
    if hint is str:
        if not isinstance(value, str):
            raise ValueError(f"{name} must be text")
        return value
    if isclass(hint) and issubclass(hint, BaseModel):
        if isinstance(value, hint):
            return value
        if not isinstance(value, dict):
            raise ValueError(f"{name} must be an object")
        return hint(**value)
    return value


class BaseModel:
    def __init__(self, **values):
        hints = typing.get_type_hints(type(self), include_extras=True)
        unknown = set(values) - set(hints)
        if unknown:
            raise ValueError(f"unknown fields {', '.join(sorted(unknown))}")
        for key, hint in hints.items():
            if key in values:
                setattr(self, key, check(values[key], hint, key))
            elif hasattr(type(self), key):
                setattr(self, key, getattr(type(self), key))
            else:
                raise ValueError(f"{key} is required")

    def model_dump(self, exclude_none=False):
        hints = typing.get_type_hints(type(self))
        return {k: getattr(self, k) for k in hints if not (exclude_none and getattr(self, k) is None)}
