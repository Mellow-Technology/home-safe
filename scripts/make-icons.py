"""Generates Home Safe's icons: a teal tile with a white house and a heart inside.

No third-party deps. Writes:
  extension/icons/icon{16,48,128}.png   browser extension (full-bleed tile)
  crates/app/assets/icon.png            1024px macOS app icon (Dock, set at runtime)
  crates/app/assets/logo.png            48px toolbar logo (no macOS margins)
  crates/app/assets/AppIcon.icns        bundle icon (macOS only; needs `iconutil`)

Run: python3 scripts/make-icons.py
"""
import pathlib, shutil, struct, subprocess, tempfile, zlib

ROOT = pathlib.Path(__file__).resolve().parent.parent

TEAL_TOP = (20, 145, 135)
TEAL_BOTTOM = (12, 98, 92)
WHITE = (255, 255, 255)
HEART = (244, 63, 94)


def heart(s, t):
    # Classic implicit heart (x²+y²-1)³ - x²y³ <= 0, y up, centered in the house body.
    x = (s - 0.5) / 0.115
    y = -(t - 0.615) / 0.115 + 0.12
    a = x * x + y * y - 1
    return a * a * a - x * x * y * y * y <= 0


def shade(s, t):
    """Color at tile coords (s, t) in [0, 1], t down. None = transparent."""
    # Rounded-rect tile.
    r = 0.225
    cx, cy = min(max(s, r), 1 - r), min(max(t, r), 1 - r)
    if (s - cx) ** 2 + (t - cy) ** 2 > r * r:
        return None
    roof = 0.18 <= t <= 0.47 and abs(s - 0.5) <= (t - 0.18) * 1.2
    body = 0.45 <= t <= 0.8 and 0.27 <= s <= 0.73
    chimney = 0.24 <= t <= 0.4 and 0.63 <= s <= 0.7
    if roof or body or chimney:
        return HEART if heart(s, t) else WHITE
    k = t
    return tuple(round(a + (b - a) * k) for a, b in zip(TEAL_TOP, TEAL_BOTTOM))


def sample(u, v, margin):
    s, t = (u - margin) / (1 - 2 * margin), (v - margin) / (1 - 2 * margin)
    if not (0 <= s <= 1 and 0 <= t <= 1):
        return None
    return shade(s, t)


def render(size, margin=0.0, ss=4):
    rows = []
    for y in range(size):
        row = bytearray([0])
        for x in range(size):
            # Only supersample pixels whose corners disagree (edges).
            probe = {sample((x + dx) / size, (y + dy) / size, margin)
                     for dx, dy in ((0, 0), (1, 0), (0, 1), (1, 1), (.5, .5))}
            if len(probe) == 1:
                c = probe.pop()
                row += bytes([0, 0, 0, 0] if c is None else [*c, 255])
                continue
            acc = [0, 0, 0, 0]
            for j in range(ss):
                for i in range(ss):
                    c = sample((x + (i + .5) / ss) / size, (y + (j + .5) / ss) / size, margin)
                    if c is not None:
                        acc[0] += c[0]; acc[1] += c[1]; acc[2] += c[2]; acc[3] += 1
            n = acc[3]
            if n == 0:
                row += bytes([0, 0, 0, 0])
            else:
                row += bytes([round(acc[0] / n), round(acc[1] / n), round(acc[2] / n),
                              round(255 * n / (ss * ss))])
        rows.append(bytes(row))

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr)
            + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b""))


# macOS icon grid: 824px tile on a 1024px canvas.
MAC_MARGIN = 100 / 1024

ext = ROOT / "extension" / "icons"
ext.mkdir(exist_ok=True)
for s in (16, 48, 128):
    (ext / f"icon{s}.png").write_bytes(render(s))

assets = ROOT / "crates" / "app" / "assets"
assets.mkdir(exist_ok=True)
(assets / "icon.png").write_bytes(render(1024, MAC_MARGIN))
# Full-bleed logo for the app's toolbar (shown at 24pt, so 2x for Retina).
(assets / "logo.png").write_bytes(render(48))

if shutil.which("iconutil"):
    with tempfile.TemporaryDirectory() as tmp:
        iconset = pathlib.Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for base in (16, 32, 128, 256, 512):
            (iconset / f"icon_{base}x{base}.png").write_bytes(render(base, MAC_MARGIN))
            (iconset / f"icon_{base}x{base}@2x.png").write_bytes(render(base * 2, MAC_MARGIN))
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(assets / "AppIcon.icns")], check=True)
else:
    print("iconutil not found; skipped AppIcon.icns")
