"""Generates the extension's toolbar icons (teal tile, white house). No deps."""
import struct, zlib, pathlib

def png(size):
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            u, v = (x + .5) / size, (y + .5) / size
            r = .18  # corner radius
            cx, cy = min(max(u, r), 1 - r), min(max(v, r), 1 - r)
            inside = (u - cx) ** 2 + (v - cy) ** 2 <= r * r
            roof = v >= .22 and v <= .5 and abs(u - .5) <= (v - .22) * 1.15
            body = .48 <= v <= .78 and .3 <= u <= .7
            door = .6 <= v <= .78 and .44 <= u <= .56
            if not inside:
                row += bytes([0, 0, 0, 0])
            elif (roof or body) and not door:
                row += bytes([255, 255, 255, 255])
            else:
                row += bytes([15, 118, 110, 255])
        rows.append(bytes(row))
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b"")

out = pathlib.Path(__file__).resolve().parent.parent / "icons"
out.mkdir(exist_ok=True)
for s in (16, 48, 128):
    (out / f"icon{s}.png").write_bytes(png(s))
