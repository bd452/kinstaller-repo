#!/usr/bin/env python3
"""Write a minimal 64x64 PNG icon for the SignalKit demo package.

Draws three stacked "signal" bars of increasing width on a dark background.
"""

from pathlib import Path
import struct
import zlib

WIDTH = 64
HEIGHT = 64
BG = (0x1A, 0x1A, 0x1A)
FG = (0xFF, 0xFF, 0xFF)

# Three left-aligned bars (x range, y range) suggesting rising signal.
BARS = [
    (8, 28, 40, 48),
    (8, 40, 28, 36),
    (8, 52, 16, 24),
]


def png_chunk(tag: bytes, data: bytes) -> bytes:
    payload = tag + data
    return struct.pack(">I", len(data)) + payload + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)


def in_any_bar(x: int, y: int) -> bool:
    for x0, x1, y0, y1 in BARS:
        if x0 <= x < x1 and y0 <= y < y1:
            return True
    return False


def write_png(path: Path) -> None:
    rows = []
    for y in range(HEIGHT):
        row = bytearray([0])
        for x in range(WIDTH):
            color = FG if in_any_bar(x, y) else BG
            row.extend(color)
        rows.append(bytes(row))

    ihdr = struct.pack(">IIBBBBB", WIDTH, HEIGHT, 8, 2, 0, 0, 0)
    idat = zlib.compress(b"".join(rows), 9)
    png = b"\x89PNG\r\n\x1a\n"
    png += png_chunk(b"IHDR", ihdr)
    png += png_chunk(b"IDAT", idat)
    png += png_chunk(b"IEND", b"")
    path.write_bytes(png)


if __name__ == "__main__":
    out = Path(__file__).resolve().parent.parent / "package" / "icon.png"
    write_png(out)
    print(f"Wrote {out}")
