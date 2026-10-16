"""Built-in lighting environments, generated on first use as Radiance .hdr files in the workspace environments folder,
so FormSurface has presentation lighting without downloading anything: a softbox studio, a neon night city and an
overcast sky. Each is an equirectangular image in linear light with real HDR values (light sources well above 1)."""
import math
import random
import struct
from pathlib import Path

WIDTH, HEIGHT = 768, 384


def _write_hdr(path: Path, pixels: list[tuple[float, float, float]]) -> None:
    out = bytearray(b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n" + f"-Y {HEIGHT} +X {WIDTH}\n".encode())
    for r, g, b in pixels:
        top = max(r, g, b)
        if top < 1e-32:
            out += b"\0\0\0\0"
            continue
        mantissa, exponent = math.frexp(top)
        scale = mantissa * 256.0 / top
        out += struct.pack("BBBB", int(r * scale), int(g * scale), int(b * scale), exponent + 128)
    partial = path.with_suffix(".part")
    partial.write_bytes(bytes(out))
    partial.replace(path)


def _directions():
    for y in range(HEIGHT):
        theta = (y + 0.5) / HEIGHT * math.pi
        up = math.cos(theta)
        ring = math.sin(theta)
        for x in range(WIDTH):
            phi = (x + 0.5) / WIDTH * 2.0 * math.pi
            yield up, phi, ring


def _box(up: float, phi: float, center_phi: float, center_up: float, half_phi: float, half_up: float, soft: float) -> float:
    d = (phi - center_phi + math.pi) % (2.0 * math.pi) - math.pi
    fx = max(0.0, abs(d) - half_phi) / soft
    fy = max(0.0, abs(up - center_up) - half_up) / soft
    return math.exp(-(fx * fx + fy * fy))


def studio() -> list[tuple[float, float, float]]:
    """Dark grey studio with a large key softbox, a fill and a strip rim light, and a darker floor."""
    pixels = []
    for up, phi, _ in _directions():
        base = 0.06 + 0.05 * max(up, 0.0) if up > -0.05 else 0.025
        key = 9.0 * _box(up, phi, 0.8, 0.45, 0.35, 0.18, 0.04)
        fill = 2.5 * _box(up, phi, 3.6, 0.25, 0.45, 0.2, 0.06)
        rim = 7.0 * _box(up, phi, 2.3, 0.55, 0.06, 0.3, 0.02)
        top = 1.5 * _box(up, phi, 0.0, 0.97, math.pi, 0.03, 0.05)
        v = base + key + fill + rim + top
        pixels.append((v, v, v * 1.02))
    return pixels


def night_city() -> list[tuple[float, float, float]]:
    """Night city: deep blue-violet sky, a skyline band of neon signs (magenta, cyan, amber, green) at the horizon,
    a strong magenta key and a cyan rim, and a dark wet street that picks up the colors."""
    rng = random.Random(2077)
    signs = []
    palette = [(6.0, 0.4, 3.2), (0.3, 4.5, 6.0), (6.0, 3.0, 0.4), (0.6, 5.5, 2.0), (5.0, 0.6, 1.2)]
    for _ in range(26):
        signs.append((rng.uniform(0, 2 * math.pi), rng.uniform(0.02, 0.35), rng.uniform(0.02, 0.12), rng.uniform(0.01, 0.08), palette[rng.randrange(len(palette))]))
    pixels = []
    for up, phi, _ in _directions():
        if up > 0:
            sky = (0.02 + 0.05 * (1 - up), 0.015 + 0.02 * (1 - up), 0.06 + 0.08 * (1 - up))
        else:
            sky = (0.01, 0.01, 0.015)
        r, g, b = sky
        for center, height, half_w, half_h, color in signs:
            w = _box(up, phi, center, height, half_w, half_h, 0.008)
            if up < 0:
                w += 0.18 * _box(-up, phi, center, height, half_w, half_h * 2.5, 0.05)
            r += color[0] * w
            g += color[1] * w
            b += color[2] * w
        key = _box(up, phi, 0.9, 0.35, 0.25, 0.15, 0.08)
        rim = _box(up, phi, 4.0, 0.3, 0.08, 0.25, 0.04)
        r += 7.0 * key + 0.6 * rim
        g += 0.8 * key + 5.0 * rim
        b += 5.0 * key + 7.0 * rim
        pixels.append((r, g, b))
    return pixels


def overcast() -> list[tuple[float, float, float]]:
    """Soft overcast daylight: a bright even sky, brightest overhead, over grey ground."""
    pixels = []
    for up, _, _ in _directions():
        if up > 0:
            v = 1.2 + 1.8 * up
            pixels.append((v * 0.95, v, v * 1.08))
        else:
            v = 0.18 + 0.1 * (1 + up)
            pixels.append((v, v * 0.97, v * 0.92))
    return pixels


BUILTIN = {"studio_softbox.hdr": studio, "night_city_neon.hdr": night_city, "overcast_sky.hdr": overcast}


def ensure(name: str, folder: Path) -> Path | None:
    """The path of a built-in environment, generated into `folder` the first time it is asked for."""
    make = BUILTIN.get(name)
    if make is None:
        return None
    path = folder / name
    if not path.is_file():
        folder.mkdir(parents=True, exist_ok=True)
        _write_hdr(path, make())
    return path
