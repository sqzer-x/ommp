#!/usr/bin/env python3
"""Generate the hero's background: ommp's splash-screen note field.

The splash fills the terminal with dark and scatters music notes across it
(src/ui/widgets/about_modal.rs, render_splash_screen). This reproduces that
exact scatter — the same hash, the same four glyphs, the same three colours —
as a tile the page can repeat.

The scatter is periodic. A note falls on cell (x, y) when

    (7x + 13y) mod 37 == 0

and shifting x or y by 37 adds 7*37 or 13*37 to the hash, both multiples of 37.
So a 37x37 tile repeats seamlessly. Which glyph and which colour a note gets
depends on hash/37, whose period is 444 rather than 37, so those cycle sooner
here than in the app. At this contrast — the brightest of the three is #3C1450
on #0A051E — that is not a difference anyone can see.

Rendered to a raster on this machine rather than shipped as text: the glyphs
live in the CJK fonts, not in the page's latin-subset JetBrains Mono, and a
visitor without them would get four kinds of tofu.
"""

import subprocess
import sys

# src/ui/widgets/about_modal.rs
NOTES = ["♪", "♫", "♩", "◆"]
DIM_COLORS = ["#3C1450", "#143C50", "#501432"]  # DIM_PURPLE, DIM_CYAN, DIM_PINK
PERIOD = 37

# One terminal cell, at 2x so the tile stays sharp on a hidpi screen. The 1:2
# ratio is roughly what a monospace cell is.
CELL_W, CELL_H = 20, 40
FONT_SIZE = 26

OUT = sys.argv[1] if len(sys.argv) > 1 else "static/img/note-field.png"


def font():
    """A font with all four glyphs. Prefer the one the terminal itself falls
    back to for U+266A, so the page's notes are the shapes ommp actually draws."""
    listing = subprocess.run(
        ["fc-list", ":charset=266a 266b 2669 25c6", "file"],
        capture_output=True, text=True, check=True,
    ).stdout
    files = [l.split(":")[0].strip() for l in listing.splitlines() if l.strip()]
    for preferred in ("NotoSansMonoCJK", "NotoSansCJK", "DejaVuSans"):
        for f in files:
            if preferred in f:
                return f
    if not files:
        sys.exit("no font on this system has all four glyphs")
    return files[0]


def main():
    args = ["magick", "-size", f"{PERIOD * CELL_W}x{PERIOD * CELL_H}", "xc:none",
            "-font", font(), "-pointsize", str(FONT_SIZE), "-gravity", "none"]

    drawn = 0
    for y in range(PERIOD):
        for x in range(PERIOD):
            h = (x * 7 + y * 13) & 0xFFFFFFFF
            if h % PERIOD:
                continue
            glyph = NOTES[(h // PERIOD) % len(NOTES)]
            colour = DIM_COLORS[(h // PERIOD + 1) % len(DIM_COLORS)]
            # annotate's origin is the baseline; sit the glyph in its cell.
            args += ["-fill", colour,
                     "-annotate", f"+{x * CELL_W + 2}+{y * CELL_H + CELL_H - 12}", glyph]
            drawn += 1

    args += ["-strip", OUT]
    subprocess.run(args, check=True)
    print(f"  {OUT}: {drawn} notes in a {PERIOD}x{PERIOD} cell tile")


if __name__ == "__main__":
    main()
