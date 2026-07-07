#!/usr/bin/env python3
"""Write a minimal 64x64 PNG icon for the demo package."""

from pathlib import Path
import struct
import zlib

WIDTH = 64
HEIGHT = 64
BG = (0x1A, 0x1A, 0x1A)
FG = (0xFF, 0xFF, 0xFF)


def png_chunk(tag: bytes, data: bytes) -> bytes:
    payload = tag + data
    return struct.pack(">I", len(data)) + payload + struct.pack(">I", zlib.crc32(payload) & 0xFFFFFFFF)


def write_png(path: Path) -> None:
    rows = []
    for y in range(HEIGHT):
        row = bytearray([0])
        for x in range(WIDTH):
            in_box = 8 <= x < 56 and 8 <= y < 56
            in_letter = (
                (24 <= x < 32 and 16 <= y < 48)
                or (24 <= x < 40 and 16 <= y < 24)
                or (24 <= x < 40 and 30 <= y < 38)
                or (32 <= x < 40 and 16 <= y < 48)
            )
            color = FG if in_box and in_letter else BG
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
