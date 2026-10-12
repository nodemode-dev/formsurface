from pathlib import Path

try:
    import PyOpenColorIO as OCIO
except ImportError:
    OCIO = None

LUT_SIZE = 65
LOW = -10.0
HIGH = 6.5
VIEWPORT_SPACES = ("linear rec.709 (srgb)", "lin_rec709_srgb", "lin_rec709", "linear srgb", "utility - linear - srgb")


def available() -> bool:
    return OCIO is not None


def version() -> str:
    return OCIO.__version__ if OCIO else ""


def builtin_names() -> list[str]:
    if not OCIO:
        return []
    return [entry[0] for entry in OCIO.BuiltinConfigRegistry().getBuiltinConfigs()]


def load(name: str, folder: Path):
    if not OCIO:
        raise RuntimeError("OpenColorIO is not installed in the FormSurface service (pip install opencolorio)")
    if name.endswith(".ocio"):
        path = folder / name
        if not path.is_file():
            raise FileNotFoundError(f"{name} is not in the workspace ocio folder")
        return OCIO.Config.CreateFromFile(str(path))
    if name not in builtin_names() and name not in ("studio-config-latest", "cg-config-latest", "default"):
        raise FileNotFoundError(f"{name} is not a built-in OpenColorIO config or an .ocio file in the workspace ocio folder")
    return OCIO.Config.CreateFromBuiltinConfig(name)


def viewport_space(config) -> str:
    for want in VIEWPORT_SPACES:
        for cs in config.getColorSpaces():
            if cs.getName().lower() == want or any(alias.lower() == want for alias in cs.getAliases()):
                return cs.getName()
    if config.hasRole("scene_linear"):
        return config.getColorSpace("scene_linear").getName()
    return next(iter(config.getColorSpaces())).getName()


def describe(config) -> dict:
    displays = {d: list(config.getViews(d)) for d in config.getDisplays()}
    return {
        "name": config.getName(),
        "description": config.getDescription().strip(),
        "displays": displays,
        "default_display": config.getDefaultDisplay(),
        "default_views": {d: config.getDefaultView(d) for d in displays},
        "looks": [look.getName() for look in config.getLooks()],
        "color_spaces": [cs.getName() for cs in config.getColorSpaces()],
        "viewport_space": viewport_space(config),
    }


def processor(config, display: str, view: str, look: str = "", source: str = ""):
    source = source or viewport_space(config)
    transform = OCIO.DisplayViewTransform(src=source, display=display, view=view)
    if not look:
        return source, config.getProcessor(transform)
    group = OCIO.GroupTransform()
    group.appendTransform(OCIO.LookTransform(src=source, dst=source, looks=look))
    group.appendTransform(transform)
    return source, config.getProcessor(group)


def convert(config, src: str, rgb: list[float]) -> list[float]:
    if src not in [cs.getName() for cs in config.getColorSpaces()]:
        raise ValueError(f"{src} is not a color space in {config.getName()}")
    proc = config.getProcessor(src, viewport_space(config))
    return list(proc.getDefaultCPUProcessor().applyRGB(rgb))


def bake(config, display: str, view: str, look: str = "", source: str = "") -> tuple[list[float], dict]:
    source, proc = processor(config, display, view, look, source)
    n = LUT_SIZE
    axis = [2.0 ** (LOW + (HIGH - LOW) * i / (n - 1)) for i in range(n)]
    data = []
    for b in range(n):
        for g in range(n):
            for r in range(n):
                data.extend((axis[r], axis[g], axis[b]))
    out = proc.getDefaultCPUProcessor().applyRGB(data)
    return list(out), {"size": n, "low": LOW, "high": HIGH, "source": source, "display": display, "view": view, "look": look}
