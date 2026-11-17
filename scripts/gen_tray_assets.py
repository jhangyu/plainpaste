#!/usr/bin/env python3
"""Renders the tray pixel assets in assets/ as raw straight-alpha RGBA (needs PIL and macOS sips):
design/tray.svg -> tray-macos-64.rgba, assets/app-icon.svg -> tray-color-32.rgba.
Both glyphs are drawn at GLYPH_SCALE of the canvas, centered. Intermediate files go to the gitignored build/icons/."""

import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
PNG_DIR = ROOT / "build" / "icons"
BYTES_PER_PIXEL = 4
GLYPH_SCALE = 0.9
GLYPH_MARGIN = (1 - GLYPH_SCALE) / 2  # fraction of the canvas on each side
SVG_CLOSE = "</svg>"
# (source svg, pixel size, output file name)
TRAY_ASSETS = (
    (ROOT / "design" / "tray.svg", 64, "tray-macos-64.rgba"),
    (ASSETS / "app-icon.svg", 32, "tray-color-32.rgba"),
)


def scaled_svg(svg: Path) -> str:
    """The SVG with its content wrapped in a group scaled by GLYPH_SCALE about the canvas center."""
    text = svg.read_text(encoding="utf-8")
    root = ET.fromstring(text)
    margin = float(root.get("width")) * GLYPH_MARGIN
    body_start = text.index(">") + 1
    body_end = text.rindex(SVG_CLOSE)
    group = f'<g transform="translate({margin:g} {margin:g}) scale({GLYPH_SCALE})">'
    return text[:body_start] + group + text[body_start:body_end] + "</g>" + text[body_end:]


def main() -> None:
    PNG_DIR.mkdir(parents=True, exist_ok=True)
    for svg, size, out in TRAY_ASSETS:
        png = PNG_DIR / f"{out}.png"
        scaled = PNG_DIR / f"{out}.svg"
        scaled.write_text(scaled_svg(svg), encoding="utf-8")
        subprocess.run(["sips", "-s", "format", "png", "-z", str(size), str(size), str(scaled), "--out", str(png)], check=True, capture_output=True)
        im = Image.open(png).convert("RGBA")
        assert im.size == (size, size), f"{png}: expected {size}x{size}, got {im.size}"
        (ASSETS / out).write_bytes(im.tobytes())
        print(f"{out} {size}x{size} {size * size * BYTES_PER_PIXEL} bytes")


if __name__ == "__main__":
    main()
