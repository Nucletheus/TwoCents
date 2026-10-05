#!/usr/bin/env python3
"""Generate ``assets/twocents.ico`` from ``src/ui/balanced-icon.svg``.

The SVG is a flat stack of ``<rect>`` elements on a 1024x1024 canvas, so it
rasterises deterministically without a full SVG engine. This script is pure
standard library (``re``, ``math``, ``struct``, ``zlib``) and needs no
third-party packages, so the committed icon can be regenerated on any machine
with Python alone:

    python tools/gen_icon.py            # write assets/twocents.ico
    python tools/gen_icon.py --check    # fail if the committed icon is stale

The build never runs this script: the icon is a committed artifact embedded by
``build.rs``. ``--check`` exists for CI, which regenerates and compares pixel
data (not raw bytes, so a different zlib version cannot cause false drift).

The icon holds seven resolutions (16/24/32/48/64/128/256 px). Sizes up to 64 px
are stored as 32-bit DIBs (maximum shell compatibility); 128 and 256 px are
stored as PNG, which Windows has accepted since Vista.
"""

import math
import re
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SVG_PATH = ROOT / "src" / "ui" / "balanced-icon.svg"
OUT_PATH = ROOT / "assets" / "twocents.ico"

SIZES = (16, 24, 32, 48, 64, 128, 256)
SUPERSAMPLE = 4  # render at 4x then box-downsample for anti-aliasing
CANVAS = 1024.0  # the SVG viewBox
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def _hex_rgb(value):
    value = value.strip().lstrip("#")
    return tuple(int(value[i : i + 2], 16) for i in (0, 2, 4))


def parse_svg(path):
    """Return (width, height, [(x, y, w, h, rx, rgba), ...]) in painter order."""
    text = path.read_text(encoding="utf-8")
    header = re.search(r'width="([\d.]+)"\s+height="([\d.]+)"', text)
    canvas_w, canvas_h = (
        (float(header.group(1)), float(header.group(2))) if header else (CANVAS, CANVAS)
    )
    rects = []
    for tag in re.findall(r"<rect\b[^>]*>", text):

        def attr(name, default="0"):
            m = re.search(rf'{name}="([^"]+)"', tag)
            return m.group(1) if m else default

        r, g, b = _hex_rgb(attr("fill", "#000000"))
        rects.append(
            (
                float(attr("x")),
                float(attr("y")),
                float(attr("width")),
                float(attr("height")),
                float(attr("rx")),
                (r, g, b, 255),
            )
        )
    return canvas_w, canvas_h, rects


def fill_rounded(buf, size, x, y, w, h, r, rgba):
    """Paint an opaque rounded rectangle into an RGBA bytearray (centre rule)."""
    y0, y1 = y, y + h
    x1 = x + w
    start_row = max(0, int(math.floor(y0)))
    end_row = min(size, int(math.ceil(y1)))
    span = bytes(rgba)
    for py in range(start_row, end_row):
        cy = py + 0.5
        inset = 0.0
        if r > 0.0:
            if cy < y0 + r:
                d = (y0 + r) - cy
                if d < r:
                    inset = r - math.sqrt(max(0.0, r * r - d * d))
            elif cy > y1 - r:
                d = cy - (y1 - r)
                if d < r:
                    inset = r - math.sqrt(max(0.0, r * r - d * d))
        xs, xe = x + inset, x1 - inset
        px0 = max(0, int(math.ceil(xs - 0.5)))
        px1 = min(size, int(math.ceil(xe - 0.5)))
        if px1 <= px0:
            continue
        a = (py * size + px0) * 4
        b = (py * size + px1) * 4
        buf[a:b] = span * (px1 - px0)


def render(size, canvas_w, canvas_h, rects):
    """Rasterise the icon at ``size`` px and return straight RGBA bytes."""
    ss = size * SUPERSAMPLE
    sx = ss / canvas_w
    sy = ss / canvas_h
    hi = bytearray(ss * ss * 4)
    for (x, y, w, h, r, rgba) in rects:
        fill_rounded(hi, ss, x * sx, y * sy, w * sx, h * sy, r * min(sx, sy), rgba)

    out = bytearray(size * size * 4)
    inv = 1.0 / (SUPERSAMPLE * SUPERSAMPLE)
    for oy in range(size):
        for ox in range(size):
            ar = ag = ab = aa = 0
            for dy in range(SUPERSAMPLE):
                row = ((oy * SUPERSAMPLE + dy) * ss + ox * SUPERSAMPLE) * 4
                for dx in range(SUPERSAMPLE):
                    i = row + dx * 4
                    a = hi[i + 3]
                    ar += hi[i] * a
                    ag += hi[i + 1] * a
                    ab += hi[i + 2] * a
                    aa += a
            o = (oy * size + ox) * 4
            if aa:
                out[o] = round(ar / aa)
                out[o + 1] = round(ag / aa)
                out[o + 2] = round(ab / aa)
                out[o + 3] = round(aa * inv)
    return bytes(out)


def png_bytes(rgba, size):
    raw = bytearray()
    stride = size * 4
    for y in range(size):
        raw.append(0)  # filter: none
        raw += rgba[y * stride : (y + 1) * stride]
    chunk = zlib.compress(bytes(raw), 9)

    def block(tag, data):
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return PNG_SIGNATURE + block(b"IHDR", ihdr) + block(b"IDAT", chunk) + block(b"IEND", b"")


def dib_bytes(rgba, size):
    """A 32-bit bottom-up DIB with an empty AND mask (alpha carries shape)."""
    header = struct.pack(
        "<IiiHHIIiiII", 40, size, size * 2, 1, 32, 0, size * size * 4, 0, 0, 0, 0
    )
    xor = bytearray()
    for y in range(size - 1, -1, -1):
        row = rgba[y * size * 4 : (y + 1) * size * 4]
        for x in range(size):
            i = x * 4
            xor += bytes((row[i + 2], row[i + 1], row[i], row[i + 3]))  # BGRA
    mask_stride = ((size + 31) // 32) * 4
    return header + bytes(xor) + bytes(mask_stride * size)


def build_ico(images):
    out = bytearray(struct.pack("<HHH", 0, 1, len(images)))
    offset = 6 + 16 * len(images)
    for size, data in images:
        dimension = 0 if size >= 256 else size
        out += struct.pack(
            "<BBBBHHII", dimension, dimension, 0, 0, 1, 32, len(data), offset
        )
        offset += len(data)
    for _, data in images:
        out += data
    return bytes(out)


def read_ico(path):
    """Decode a committed ICO back to {size: RGBA bytes} for comparison."""
    data = path.read_bytes()
    reserved, kind, count = struct.unpack("<HHH", data[:6])
    if (reserved, kind) != (0, 1):
        raise ValueError("not an ICO file")
    frames = {}
    off = 6
    for _ in range(count):
        w, _h, _cc, _r, _planes, _bpp, ln, o = struct.unpack(
            "<BBBBHHII", data[off : off + 16]
        )
        off += 16
        size = w or 256
        blob = data[o : o + ln]
        frames[size] = _decode_frame(blob, size)
    return frames


def _decode_frame(blob, size):
    if blob[:8] == PNG_SIGNATURE:
        return _decode_png(blob, size)
    return _decode_dib(blob, size)


def _decode_dib(blob, size):
    bi_size = struct.unpack("<I", blob[:4])[0]
    xor = blob[bi_size : bi_size + size * size * 4]
    out = bytearray(size * size * 4)
    for y in range(size):
        for x in range(size):
            i = (y * size + x) * 4
            s = ((size - 1 - y) * size + x) * 4
            out[i] = xor[s + 2]
            out[i + 1] = xor[s + 1]
            out[i + 2] = xor[s]
            out[i + 3] = xor[s + 3]
    return bytes(out)


def _decode_png(blob, size):
    off = 8
    idat = bytearray()
    while off < len(blob):
        ln = struct.unpack(">I", blob[off : off + 4])[0]
        tag = blob[off + 4 : off + 8]
        body = blob[off + 8 : off + 8 + ln]
        if tag == b"IDAT":
            idat += body
        off += 12 + ln
    raw = zlib.decompress(bytes(idat))
    stride = size * 4
    out = bytearray(size * size * 4)
    for y in range(size):
        assert raw[y * (stride + 1)] == 0, "unexpected PNG filter"
        out[y * stride : (y + 1) * stride] = raw[y * (stride + 1) + 1 : (y + 1) * (stride + 1)]
    return bytes(out)


def generate():
    canvas_w, canvas_h, rects = parse_svg(SVG_PATH)
    images = []
    for size in SIZES:
        rgba = render(size, canvas_w, canvas_h, rects)
        data = png_bytes(rgba, size) if size >= 128 else dib_bytes(rgba, size)
        images.append((size, data))
    return build_ico(images)


def check():
    frames = read_ico(OUT_PATH)
    canvas_w, canvas_h, rects = parse_svg(SVG_PATH)
    problems = []
    if set(frames) != set(SIZES):
        problems.append(f"sizes {sorted(frames)} != expected {list(SIZES)}")
    for size in SIZES:
        if size not in frames:
            continue
        expected = render(size, canvas_w, canvas_h, rects)
        if frames[size] != expected:
            problems.append(f"{size}px frame differs from the SVG")
    if problems:
        print("icon is stale:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print("run: python tools/gen_icon.py", file=sys.stderr)
        return 1
    print("icon is up to date")
    return 0


def main(argv):
    if "--check" in argv:
        return check()
    OUT_PATH.parent.mkdir(parents=True, exist_ok=True)
    OUT_PATH.write_bytes(generate())
    print(
        f"wrote {OUT_PATH.relative_to(ROOT)} "
        f"({OUT_PATH.stat().st_size} bytes, {len(SIZES)} sizes)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
