#!/usr/bin/env python3
"""Render the launcher icons `cargo tauri icon` cannot produce correctly.

Three jobs:

1. The adaptive FOREGROUND layer must be the `#` alone on transparency, sized to
   the adaptive safe zone. Tauri bakes the plate into it.
2. mipmap-hdpi must be 72px. Tauri emits 49px for ic_launcher/ic_launcher_round
   while correctly using 72 as the basis for the foreground (162 = 2.25 x 72), so
   only the legacy pair is affected — one density out of five, undersized by a
   third on the bucket most mid-range phones land in.
3. The iOS 20pt and 29pt 1x renditions are too small for the mark to survive being
   resampled, and need an optical correction rather than a straight scale.

Run this after `cargo tauri icon src-tauri/icons/icon-manifest.json`; the manifest
fixes (1) at generation time, this script fixes (2) and can redo (1) directly.

Android composites ic_launcher_foreground over ic_launcher_background and then
applies the OEM's own mask (circle, squircle, teardrop, ...). A foreground that
bakes in its own plate therefore gets the mask cut THROUGH the artwork instead of
around it.

Geometry is derived from the canonical mark rather than redrawn, so this cannot
drift from it: the four strokes are the same ones in surfaces/gui/public/favicon.svg
and Icon.tsx case "logo", on the same 24 grid.

The adaptive layer is 108dp; only the inner 72dp survives masking, and key content
is meant to stay inside a 66dp circle. Mapping the 24-grid tile onto that 72dp
window is `p -> 54 + 1.86*(p - 12)`, which puts the mark's outer bounds at 32.8dp
across — well inside the safe circle at every density.

No third-party imports: this has to run wherever the repo is checked out, and
adding Pillow/cairo as a build dependency to draw four lines is not a trade worth
making. Rasterisation is supersampled distance-to-capsule; PNG is written by hand.
"""
import struct, zlib, sys
from pathlib import Path

MARK = (0x8A, 0x8C, 0xF0)          # iris, matching favicon.svg / Icon.tsx
STROKE_24 = 2.05                    # stroke-width on the 24 grid
GROUP_SCALE = 0.62                  # the mark's own scale inside its tile

# The four strokes, as (x1, y1, x2, y2) on the 24 grid.
STROKES_24 = [
    (9.4, 4.2, 7.2, 19.8),   # left vertical, slanted the way a # is cut
    (16.8, 4.2, 14.6, 19.8), # right vertical
    (4.6, 9.6, 19.4, 9.6),   # upper horizontal
    (3.8, 14.4, 18.6, 14.4), # lower horizontal
]

# 24-grid -> 108dp adaptive canvas, via the 72dp masked window.
def to108(p):
    return 54.0 + (72.0 / 24.0) * GROUP_SCALE * (p - 12.0)

SEGMENTS = [(to108(a), to108(b), to108(c), to108(d)) for a, b, c, d in STROKES_24]
RADIUS_108 = (STROKE_24 * GROUP_SCALE * (72.0 / 24.0)) / 2.0


def coverage(px, py, segs, r):
    """1.0 inside a capsule, 0.0 outside — caller supersamples for the edge."""
    for x1, y1, x2, y2 in segs:
        dx, dy = x2 - x1, y2 - y1
        L2 = dx * dx + dy * dy
        t = 0.0 if L2 == 0 else ((px - x1) * dx + (py - y1) * dy) / L2
        t = 0.0 if t < 0.0 else (1.0 if t > 1.0 else t)   # round caps
        qx, qy = x1 + t * dx, y1 + t * dy
        ex, ey = px - qx, py - qy
        if ex * ex + ey * ey <= r * r:
            return 1.0
    return 0.0


def render(size):
    s = 108.0 / size                     # canvas units per pixel
    segs, r = SEGMENTS, RADIUS_108
    ss = 8 if size <= 220 else 4         # more samples where a pixel is worth more
    step, inv = 1.0 / ss, 1.0 / (ss * ss)
    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            acc = 0.0
            for sy in range(ss):
                py = (y + (sy + 0.5) * step) * s
                for sx in range(ss):
                    acc += coverage((x + (sx + 0.5) * step) * s, py, segs, r)
            a = int(acc * inv * 255.0 + 0.5)
            row += bytes((MARK[0], MARK[1], MARK[2], a)) if a else b"\0\0\0\0"
        rows.append(bytes(row))
    return rows


def write_png(path, size, rows):
    raw = b"".join(b"\0" + r for r in rows)          # filter 0 on every scanline
    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)
    png = (b"\x89PNG\r\n\x1a\n"
           + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(raw, 9))
           + chunk(b"IEND", b""))
    Path(path).write_bytes(png)


# density bucket -> foreground size (2.25x the launcher ladder, per the adaptive spec)
DENSITIES = {"mdpi": 108, "hdpi": 162, "xhdpi": 216, "xxhdpi": 324, "xxxhdpi": 432}

PLATE = (0x0B, 0x0D, 0x0F)   # the near-black plate, as in favicon.svg

# Legacy (pre-adaptive) launcher tiles. These SHOULD carry their own plate — they
# are self-contained, no background layer and no OS mask behind them.
#
# The fractions are measured off the existing xxxhdpi assets rather than invented,
# so the corrected hdpi sits on the same ladder as the four densities that were
# already right: the square plate is inset 1/12 of the canvas per side, the round
# plate 1/24, and in both the mark spans 45.6% of the plate — the same proportion
# it has inside its own tile in favicon.svg.
SQUARE_INSET = 1.0 / 12.0
ROUND_INSET = 1.0 / 24.0
CORNER_R_24 = 5.37           # rect rx on the 24 grid, from favicon.svg

# The launcher ladder. Tauri's own table says 49 for hdpi; the platform says 72.
LAUNCHER = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}


def _rounded_rect_cov(px, py, cx, cy, half, r):
    dx = abs(px - cx) - (half - r)
    dy = abs(py - cy) - (half - r)
    ox, oy = max(dx, 0.0), max(dy, 0.0)
    return (ox * ox + oy * oy) ** 0.5 + min(max(dx, dy), 0.0) - r <= 0.0


def render_tile(size, round_plate=False):
    """A self-contained launcher tile: plate plus the mark, on transparency."""
    inset = (ROUND_INSET if round_plate else SQUARE_INSET) * size
    plate = size - 2 * inset
    c = size / 2.0
    corner = plate * (CORNER_R_24 / 24.0)
    # the mark's 24 grid mapped onto the plate, same transform as the tile artwork
    def to_px(p):
        return inset + plate * (12.0 + GROUP_SCALE * (p - 12.0)) / 24.0
    segs = [(to_px(a), to_px(b), to_px(cc), to_px(d)) for a, b, cc, d in STROKES_24]
    r = (STROKE_24 * GROUP_SCALE * plate / 24.0) / 2.0
    ss = 8 if size <= 220 else 4
    step, inv = 1.0 / ss, 1.0 / (ss * ss)
    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            pacc = macc = 0.0
            for sy in range(ss):
                py = y + (sy + 0.5) * step
                for sx in range(ss):
                    px = x + (sx + 0.5) * step
                    on_plate = ((px - c) ** 2 + (py - c) ** 2 <= (plate / 2.0) ** 2
                                if round_plate else
                                _rounded_rect_cov(px, py, c, c, plate / 2.0, corner))
                    if on_plate:
                        pacc += 1.0
                        if coverage(px, py, segs, r):
                            macc += 1.0
            pa, ma = pacc * inv, macc * inv
            if pa <= 0.0:
                row += b"\0\0\0\0"
            else:
                # mark over plate, then the whole thing against transparency
                rr = int(MARK[0] * ma + PLATE[0] * (pa - ma) + 0.5)
                gg = int(MARK[1] * ma + PLATE[1] * (pa - ma) + 0.5)
                bb = int(MARK[2] * ma + PLATE[2] * (pa - ma) + 0.5)
                a = int(pa * 255.0 + 0.5)
                row += bytes((min(rr, 255), min(gg, 255), min(bb, 255), a))
        rows.append(bytes(row))
    return rows


# --- iOS small renditions -------------------------------------------------
#
# iOS icons are full-bleed: the 24 grid maps to the whole canvas, no plate inset,
# no transparency (the OS applies its own corner mask).
#
# Every rendition is the same artwork scaled, which stops working at the bottom of
# the ladder. The mark's stroke is 5.3% of the tile, so at the 20pt and 29pt 1x
# slots it lands at 1.06px and 1.54px — under a device pixel and a half. Antialiasing
# then spreads each stroke across two columns at partial opacity and the `#` reads as
# a violet smudge rather than a glyph. Every rendition at 40px and up is already over
# 2px and is left exactly as `cargo tauri icon` resampled it.
#
# The correction is the usual optical-size one, expressed as two floors in device
# pixels rather than a redrawn glyph:
#
#   stroke  >= 2.0px   below this a stroke cannot hold a solid edge
#   counter >= 1.0px   the openings inside the # have to survive the thicker stroke
#
# Thickening alone would close the counters — at 20px the horizontals are only
# 2.48px apart centre to centre — so the mark is scaled up by exactly the amount
# needed to keep 1px of daylight, and no more. That works out to 1.21x at 20px and
# no scaling at all at 29px, where the floor on its own is enough.
MIN_STROKE_PX = 2.0
MIN_COUNTER_PX = 1.0
COUNTER_24 = 4.8          # centre-to-centre of the two horizontals, on the 24 grid


def ios_optical(size):
    """(stroke width, extra glyph scale) for a full-bleed iOS tile of `size` px."""
    w = max(size * STROKE_24 * GROUP_SCALE / 24.0, MIN_STROKE_PX)
    natural_counter = COUNTER_24 * GROUP_SCALE * size / 24.0
    k = max(1.0, (w + MIN_COUNTER_PX) / natural_counter)
    return w, k


def render_ios(size):
    """A full-bleed iOS rendition, optically corrected if it is small enough to need it."""
    w, k = ios_optical(size)
    def to_px(p):
        return size * (12.0 + GROUP_SCALE * k * (p - 12.0)) / 24.0
    segs = [(to_px(a), to_px(b), to_px(c), to_px(d)) for a, b, c, d in STROKES_24]
    r = w / 2.0
    ss = 8 if size <= 220 else 4
    step, inv = 1.0 / ss, 1.0 / (ss * ss)
    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            acc = 0.0
            for sy in range(ss):
                py = y + (sy + 0.5) * step
                for sx in range(ss):
                    acc += coverage(x + (sx + 0.5) * step, py, segs, r)
            a = acc * inv
            row += bytes((int(MARK[0] * a + PLATE[0] * (1 - a) + 0.5),
                          int(MARK[1] * a + PLATE[1] * (1 - a) + 0.5),
                          int(MARK[2] * a + PLATE[2] * (1 - a) + 0.5), 255))
        rows.append(bytes(row))
    return rows


# The renditions whose natural stroke falls under the floor. Everything else on the
# iOS ladder is left as generated.
IOS_SMALL = {"AppIcon-20x20@1x.png": 20, "AppIcon-29x29@1x.png": 29}


MASTER_SIZE = 1024   # what icon-manifest.json feeds to `cargo tauri icon`

if __name__ == "__main__":
    # Default: write the master the icon manifest points at. `cargo tauri icon
    # src-tauri/icons/icon-manifest.json` resamples it down to the five densities,
    # so the foreground survives a regeneration instead of being re-plated.
    #   --densities [dir]   write the five foreground density files directly
    #   --launchers  [dir]   write ic_launcher / ic_launcher_round at correct sizes
    #   --ios-small  [dir]   redraw the iOS renditions too small to resample
    args = sys.argv[1:]
    if args and args[0] == "--launchers":
        base = Path(args[1] if len(args) > 1 else "src-tauri/icons/android")
        for d, size in LAUNCHER.items():
            for name, rnd in (("ic_launcher", False), ("ic_launcher_round", True)):
                out = base / f"mipmap-{d}" / f"{name}.png"
                write_png(out, size, render_tile(size, round_plate=rnd))
                print(f"  {d:8s} {name:18s} {size}x{size}")
    elif args and args[0] == "--ios-small":
        base = Path(args[1] if len(args) > 1 else "src-tauri/icons/ios")
        for name, size in IOS_SMALL.items():
            w, k = ios_optical(size)
            write_png(base / name, size, render_ios(size))
            print(f"  {name:24s} {size}x{size}  stroke {w:.2f}px  glyph x{k:.3f}")
    elif args and args[0] == "--densities":
        base = Path(args[1] if len(args) > 1 else "src-tauri/icons/android")
        for d, size in DENSITIES.items():
            out = base / f"mipmap-{d}" / "ic_launcher_foreground.png"
            write_png(out, size, render(size))
            print(f"  {d:8s} {size}x{size}  {out}")
    else:
        out = Path(args[0] if args else "src-tauri/icons/src/adaptive-foreground.png")
        write_png(out, MASTER_SIZE, render(MASTER_SIZE))
        print(f"  master   {MASTER_SIZE}x{MASTER_SIZE}  {out}")
