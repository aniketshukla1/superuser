# Icons

The mark is a bare `#` — root's shell prompt — in iris `#8A8CF0` on near-black
`#0B0D0F`. `source.png` is the master; the same four strokes are drawn in
`surfaces/gui/public/favicon.svg` and `Icon.tsx` case `"logo"`, on a 24 grid.
If the mark ever changes, those three move together.

## Regenerating

Two steps, in this order. Both are required — step 1 alone leaves two defects
in the Android output.

```bash
# 1. everything, from the manifest (pins source, background colour and the
#    adaptive foreground master, so the foreground is not re-plated)
cargo tauri icon src-tauri/icons/icon-manifest.json

# 2. correct the Android launcher tiles that step 1 gets wrong
python3 src-tauri/icons/src/render_android_icons.py --launchers

# 3. redraw the two iOS renditions too small to survive being resampled
python3 src-tauri/icons/src/render_android_icons.py --ios-small
```

## Why steps 2 and 3 exist

`cargo tauri icon` has two bugs on the Android side, and one limitation that is
not really a bug.

**It emits `mipmap-hdpi` at 49x49.** The density ladder is 48/72/96/144/192, so
hdpi should be 72. Tauri knows this elsewhere — it derives the adaptive
foreground correctly as 162 = 2.25 x 72 — but the legacy `ic_launcher` and
`ic_launcher_round` come out a third undersized, on the bucket a lot of
mid-range phones land in.

**It bakes the plate into `ic_launcher_foreground.png`.** Android composites the
foreground over `ic_launcher_background` and then applies the launcher's own
mask (circle, squircle, teardrop, ...). A foreground carrying its own plate gets
that mask cut *through* the artwork instead of around it. `icon-manifest.json`
heads this off by supplying `src/adaptive-foreground.png` — the mark alone on
transparency — as the foreground master, so step 1 already gets this right;
`--densities` re-renders those five files directly if you need it.

**The iOS ladder is one artwork scaled, and that stops working at the bottom.**
The mark's stroke is 5.3% of the tile, so the 20pt and 29pt 1x slots land at
1.06px and 1.54px. Under about two device pixels a stroke cannot hold a solid
edge: antialiasing spreads it across two columns at partial opacity and the `#`
reads as a violet smudge. At 20px only 8 pixels in the whole glyph reached full
mark colour.

Step 3 applies the standard optical-size correction, as two floors in device
pixels rather than a redrawn glyph — stroke at least 2.0px, and the counters
inside the `#` at least 1.0px. Thickening alone would close those counters (at
20px the horizontals are 2.48px apart centre to centre), so the mark is scaled up
by exactly enough to keep 1px of daylight and no more: 1.21x at 20px, nothing at
29px where the stroke floor alone suffices. Every rendition at 40px and up is
already over 2px and is left exactly as Tauri resampled it.

## What the script draws

`src/render_android_icons.py` derives everything from the canonical mark rather
than redrawing it: one table of four strokes, one stroke width, one group scale,
all read from `favicon.svg`. It cannot drift from the mark.

Plate proportions were measured off the assets Tauri got right, so the corrected
hdpi sits on the same ladder as its neighbours: the square plate is inset 1/12
of the canvas per side, the round plate 1/24, and the mark spans 45.6% of the
plate in both. Its circle is also antialiased, where Tauri's is hard-edged.

Stdlib only — supersampled coverage and a hand-written PNG encoder. Pulling in a
rasteriser to draw four lines was not a trade worth making.
