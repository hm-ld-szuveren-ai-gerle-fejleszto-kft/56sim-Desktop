#!/usr/bin/env python3
"""Emit a Windows .ico file from PNG sources (PNG-in-ICO container).

Std-lib only. Each entry embeds the PNG verbatim; 256x256 entries are
written with width/height byte 0 per the ICO convention (0 == 256).

Usage:
    python3 packaging/make_icon.py OUT.ico SIZE=PNG [SIZE=PNG ...]
    python3 packaging/make_icon.py assets/icons/56sim.ico \
        256=assets/icons/hicolor/256/apps/hu.szuveren.Sim56.png \
        48=assets/icons/hicolor/48/apps/hu.szuveren.Sim56.png \
        32=assets/icons/hicolor/32/apps/hu.szuveren.Sim56.png \
        16=assets/icons/hicolor/16/apps/hu.szuveren.Sim56.png

A bare PNG path without SIZE= is accepted too; its size is read from the
PNG header.
"""
import struct
import sys

PNG_MAGIC = b"\x89PNG\r\n\x1a\n"


def png_size(data: bytes) -> tuple[int, int]:
    if data[:8] != PNG_MAGIC:
        raise ValueError("not a PNG file")
    if data[12:16] != b"IHDR":
        raise ValueError("PNG lacks IHDR")
    w, h = struct.unpack(">II", data[16:24])

    return w, h

def build_ico(entries: list[tuple[int, bytes]]) -> bytes:
    # ICONDIR: reserved u16=0, type u16=1, count u16.
    # ICONDIRENTRYs all precede the image payloads.
    header = [struct.pack("<HHH", 0, 1, len(entries))]
    dir_size = 6 + 16 * len(entries)
    offset = dir_size
    for _size, png in entries:
        w, h = png_size(png)
        wb = 0 if w >= 256 else w
        hb = 0 if h >= 256 else h
        # planes/bitcount are ignored for PNG payloads; use 1/32.
        header.append(
            struct.pack("<BBBBHHII", wb, hb, 0, 0, 1, 32, len(png), offset))
        offset += len(png)
    # All directory entries precede all payloads.
    return b"".join(header + [png for _size, png in entries])


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    out_path = argv[1]
    entries: list[tuple[int, bytes]] = []
    for spec in argv[2:]:
        if "=" in spec:
            size_s, path = spec.split("=", 1)
            size = int(size_s)
        else:
            path = spec
            size = None
        with open(path, "rb") as fh:
            data = fh.read()
        w, h = png_size(data)
        if w != h:
            raise ValueError(f"{path}: not square ({w}x{h})")
        if size is None:
            size = w
        if size != w:
            raise ValueError(f"{path}: declared size {size} != PNG width {w}")
        entries.append((size, data))
    entries.sort(key=lambda e: e[0], reverse=True)
    with open(out_path, "wb") as fh:
        fh.write(build_ico(entries))
    print(f"{out_path}: {len(entries)} entries "
          f"({', '.join(str(s) for s, _ in entries)})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
