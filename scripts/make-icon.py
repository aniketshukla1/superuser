#!/usr/bin/env python3
"""Draw the Superuser app icon.

The mark is the one the app already draws in its own chrome (`Icon.tsx`, case
"logo"): a terminal window with a root prompt inside. In Unix `$` is a normal user
and `#` is root, so the prompt IS the name — the chevron and the block cursor read
as "you have the shell".

This renders that same 24px-grid geometry at icon scale so the dock matches the
app, on the product's own phosphor-green-on-near-black palette. Regenerate the
platform icon set with:

    python3 scripts/make-icon.py && npx tauri icon src-tauri/icons/source.png

Kept as a script rather than a one-off because the mark is defined in code: when
`Icon.tsx` changes, the icon should be regenerated from the same numbers, not
redrawn by hand in an image editor.
"""

from PIL import Image, ImageDraw

SIZE = 1024          # what `tauri icon` wants as a source
SUPERSAMPLE = 4      # draw big, downsample — Pillow has no antialiased stroking
S = SIZE * SUPERSAMPLE

# The app's own dark palette (styles.css): panel background, phosphor-green mark.
BG = (13, 18, 15, 255)        # --panel  #0d120f
FG = (0, 224, 90, 255)        # --accent #00e05a

# Icon.tsx draws on a 24-unit grid with a 1.7 stroke and rounded caps/joins. The
# glyph occupies most of that grid, so it's inset here to leave the margin macOS
# icons are expected to have.
GRID = 24.0
INSET = 0.17                  # fraction of the canvas kept clear around the glyph


def main() -> None:
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # macOS squircle-ish rounded square. 22.37% of the side is Apple's corner radius.
    d.rounded_rectangle([0, 0, S - 1, S - 1], radius=int(S * 0.2237), fill=BG)

    # Map the 24-unit grid onto the inset area.
    span = S * (1 - 2 * INSET)
    ox = oy = S * INSET
    def p(x: float, y: float) -> tuple[float, float]:
        return (ox + x / GRID * span, oy + y / GRID * span)

    stroke = max(1, int(1.7 / GRID * span))

    # The terminal window.
    x0, y0 = p(2.6, 4.2)
    x1, y1 = p(2.6 + 18.8, 4.2 + 15.6)
    d.rounded_rectangle([x0, y0, x1, y1], radius=int(3.0 / GRID * span), outline=FG, width=stroke)

    # The prompt: chevron then cursor. Drawn as segments with round joins, which
    # Pillow gives via `joint="curve"` plus discs at the ends for the caps.
    def polyline(points: list[tuple[float, float]]) -> None:
        pts = [p(*pt) for pt in points]
        d.line(pts, fill=FG, width=stroke, joint="curve")
        r = stroke / 2
        for cx, cy in pts:
            d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=FG)

    polyline([(7, 10.4), (10.2, 13), (7, 15.6)])   # >
    polyline([(12.4, 15.8), (16.8, 15.8)])         # cursor

    img.resize((SIZE, SIZE), Image.LANCZOS).save("src-tauri/icons/source.png")
    print("wrote src-tauri/icons/source.png")


if __name__ == "__main__":
    main()
