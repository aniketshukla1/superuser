#!/usr/bin/env python3
"""The README's images, from the raw captures the demo spec takes.

Two steps, because the first needs a browser and the second does not:

    cd surfaces/gui
    node node_modules/@playwright/test/cli.js test -c playwright.demo.config.ts   # docs/assets/raw/
    python3 ../../scripts/readme-assets.py                                        # docs/assets/

readmeArt.spec.ts draws the hero (a loop of frames, assembled here into an animated WebP), the
buttons, the numbers strip and the architecture; demo.spec.ts takes the screenshots.

The captures are the real UI over a mock backend with invented data (surfaces/gui/e2e-demo),
so nothing in them is anybody's mail, chats or accounts. `--extract` refreshes the part of
that data which is NOT invented: the built-in assistants and the boundary presets, read from
the Rust source, so a screenshot cannot show something the build does not have.

Needs Pillow (`pip install pillow`).
"""
import glob
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RAW = os.path.join(ROOT, "docs", "assets", "raw")
OUT = os.path.join(ROOT, "docs", "assets")

SCREEN_W = 1600  # the README shows them at half a column; 1600 keeps text sharp on a retina display
GIF_W = 1200
FRAME_MS = 2800
HERO_W = 1600  # the hero shows at most ~900px wide
HERO_MS = 83  # 72 frames: one turn of the light in six seconds, at 12 frames a second
# Drawn by readmeArt.spec.ts, kept at the size they were drawn.
DRAWN = ("social-", "btn-", "stats-", "architecture-")


def extract():
    core = os.path.join(ROOT, "src-tauri", "src", "core")
    personas = []
    for f in sorted(glob.glob(os.path.join(core, "personas_builtin", "*.md"))):
        front = open(f, encoding="utf8").read().split("---")[1]

        def field(key, default=""):
            m = re.search(rf"^{key}:\s*(.*)$", front, re.M)
            return m.group(1).strip() if m else default

        tools = [t.strip() for t in field("tools", "[]").strip("[]").split(",") if t.strip()]
        personas.append(
            {
                "id": field("id"),
                "name": field("name"),
                "icon": field("icon", "sparkle"),
                "tagline": field("tagline"),
                "family": field("family", "knowledge"),
                "tools": tools,
                "mode": field("default_permission_mode", "interactive"),
            }
        )

    # The whole module, not one file: since the split into folders (2026-10-12) the presets
    # live in boundaries/presets.rs, and a later split must not break this again.
    module = [os.path.join(core, "boundaries.rs")] + sorted(glob.glob(os.path.join(core, "boundaries", "*.rs")))
    src = "".join(open(p, encoding="utf8").read().split("#[cfg(test)]")[0] for p in module if not p.endswith("tests.rs"))
    presets = [
        {"id": m.group(1), "name": m.group(2), "when": m.group(3), "cost": m.group(4).replace('\\"', '"')}
        for m in re.finditer(
            r'id: "([a-z-]+)",\s*name: "([^"]+)",(?:(?!id: ").)*?when: r#"(.*?)"#,\s*cost: "((?:[^"\\]|\\.)*)"',
            src,
            re.S,
        )
    ]
    block = src[src.index("pub const FIELDS") :]
    block = block[: block.index("];")]
    fields = [
        {"field": a, "means": b.replace('\\"', '"')}
        for a, b in re.findall(r'\("([a-z_.]+)",\s*"((?:[^"\\]|\\.)*)"\)', block)
    ]

    target = os.path.join(ROOT, "surfaces", "gui", "e2e-demo", "from-source.json")
    with open(target, "w", encoding="utf8") as fh:
        json.dump({"personas": personas, "presets": presets, "fields": fields}, fh, indent=1, ensure_ascii=False)
        fh.write("\n")
    print(f"{len(personas)} assistants, {len(presets)} presets, {len(fields)} rule fields -> {os.path.relpath(target, ROOT)}")


def build():
    from PIL import Image

    if not os.path.isdir(RAW):
        sys.exit(f"no captures in {RAW} — run the demo spec first (see this file's header)")

    def fit(im, width):
        if im.width <= width:
            return im
        return im.resize((width, round(im.height * width / im.width)), Image.LANCZOS)

    made = []
    for path in sorted(glob.glob(os.path.join(RAW, "*.png"))):
        name = os.path.basename(path)
        if name.startswith("frame-"):
            continue
        im = Image.open(path)
        if name.startswith(DRAWN):
            target = os.path.join(OUT, name)
            im.save(target, optimize=True)
        else:
            target = os.path.join(OUT, "screen-" + name)
            fit(im.convert("RGB"), SCREEN_W).save(target, optimize=True)
        made.append(target)

    for theme in ("dark", "light"):
        paths = sorted(glob.glob(os.path.join(RAW, "hero", f"hero-{theme}-*.png")))
        if not paths:
            continue
        target = os.path.join(OUT, f"hero-{theme}.webp")
        loop = [fit(Image.open(p).convert("RGBA"), HERO_W) for p in paths]
        # Not minimize_size: its partial frames left blocky seams in the dark around the headline.
        loop[0].save(target, save_all=True, append_images=loop[1:], duration=HERO_MS, loop=0, quality=78, method=6)
        made.append(target)

    frames = [
        fit(Image.open(p).convert("RGB"), GIF_W).quantize(colors=255, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
        for p in sorted(glob.glob(os.path.join(RAW, "frame-*.png")))
    ]
    if frames:
        target = os.path.join(OUT, "demo.gif")
        frames[0].save(target, save_all=True, append_images=frames[1:], duration=FRAME_MS, loop=0, optimize=True)
        made.append(target)

    for target in made:
        print(f"{os.path.getsize(target) / 1024:8.0f} KB  {os.path.relpath(target, ROOT)}")


if __name__ == "__main__":
    extract() if "--extract" in sys.argv else build()
