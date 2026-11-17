#!/usr/bin/env python3
"""Generates ui/app-icon.slint (the AppIcon component) from assets/app-icon.svg, so the SVG stays the
single vector source and the binary needs no SVG decoder. Handles exactly the subset the icon uses:
rect, path, g, translate/scale transforms, linear and radial gradients, solid fills, opacity, a stroke.
Default: writes the file. --check: exits 1 if the file differs from the rendered text."""

import argparse
import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "assets" / "app-icon.svg"
TARGET = ROOT / "ui" / "app-icon.slint"
NS = "{http://www.w3.org/2000/svg}"
HEADER = (
    "// Generated from assets/app-icon.svg by scripts/gen_app_icon.py, never edited by hand.\n"
    "// Gradients are mapped onto Slint's angle-based gradients over each item box.\n"
)
# Local canvas of each <g> that holds paths (the Fluent art's native size); gradient boxes are taken from it.
CANVAS = {"clipboard": 48.0, "lines": 32.0, "sparkle": 32.0}
# Slint's radial gradient is a circle: stands in for the clipshade ellipse with this radius (SVG units).
RADIAL_CIRCLE_RADIUS = 9.0


def num(v: float) -> str:
    return f"{v:.4f}".rstrip("0").rstrip(".") or "0"


def color(value: str, opacity: float) -> str:
    h = value.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    return f"#{h}{round(opacity * 255):02x}"


class Affine:
    """Uniform scale `a` plus offset (bx, by): global = a * local + b."""

    def __init__(self, a=1.0, bx=0.0, by=0.0):
        self.a, self.bx, self.by = a, bx, by

    def apply(self, transform: str | None) -> "Affine":
        out = Affine(self.a, self.bx, self.by)
        for op, args in re.findall(r"(\w+)\(([^)]*)\)", transform or ""):
            v = [float(x) for x in args.replace(",", " ").split()]
            if op == "translate":
                out.bx += out.a * v[0]
                out.by += out.a * (v[1] if len(v) > 1 else 0.0)
            elif op == "scale" and len(v) == 1:
                out.a *= v[0]
            else:
                sys.exit(f"unsupported transform: expected translate(x y) or scale(s), got {op}({args}) in {SOURCE}")
        return out


class Gradients:
    def __init__(self, root):
        self.defs = {g.get("id"): g for g in root.iter() if g.tag in (NS + "linearGradient", NS + "radialGradient")}

    def stops(self, g, opacity):
        out = []
        for s in g.findall(NS + "stop"):
            off = float(s.get("offset", "0"))
            out.append((off, color(s.get("stop-color"), float(s.get("stop-opacity", "1")) * opacity)))
        return out

    def linear(self, g, box, opacity):
        """box = (x, y, w, h) in the gradient's user space; returns a Slint @linear-gradient."""
        x1, y1, x2, y2 = (float(g.get(k)) for k in ("x1", "y1", "x2", "y2"))
        dx, dy = x2 - x1, y2 - y1
        length = math.hypot(dx, dy)
        ux, uy = dx / length, dy / length
        angle = math.degrees(math.atan2(ux, -uy)) % 360  # CSS: 0deg points up, clockwise
        bx, by, bw, bh = box
        line = abs(bw * ux) + abs(bh * uy)
        cx, cy = bx + bw / 2, by + bh / 2

        def pos(px, py):
            return ((px - cx) * ux + (py - cy) * uy) / line + 0.5

        t0, t1 = pos(x1, y1), pos(x2, y2)
        stops = [(t0 + off * (t1 - t0), c) for off, c in self.stops(g, opacity)]
        if any(not -1e-6 <= p <= 1 + 1e-6 for p, _ in stops):
            sys.exit(f"gradient {g.get('id')}: stop positions {[round(p, 3) for p, _ in stops]} leave the box {box}")
        body = ", ".join(f"{c} {num(max(0.0, min(1.0, p)) * 100)}%" for p, c in stops)
        return f"@linear-gradient({num(angle)}deg, {body})"

    def radial(self, g, opacity):
        body = ", ".join(f"{c} {num(off * 100)}%" for off, c in self.stops(g, opacity))
        return f"@radial-gradient(circle, {body})"


def fill_value(fill, opacity, grads, box, radial_box_out):
    if fill in (None, "none"):
        return None
    m = re.fullmatch(r"url\(#(\w+)\)", fill)
    if not m:
        return color(fill, opacity)
    g = grads.defs[m.group(1)]
    if g.tag == NS + "radialGradient":
        cx, cy = float(g.get("cx")), float(g.get("cy"))
        radial_box_out.append((cx, cy))
        return grads.radial(g, opacity)
    return grads.linear(g, box, opacity)


def render() -> str:
    root = ET.parse(SOURCE).getroot()
    size = float(root.get("width"))
    grads = Gradients(root)
    out: list[str] = []

    def emit_rect(el, aff):
        x, y, w, h = (float(el.get(k)) for k in ("x", "y", "width", "height"))
        rx = float(el.get("rx", "0"))
        opacity = float(el.get("opacity", "1"))
        stroke, sw = el.get("stroke"), float(el.get("stroke-width", "0"))
        props = []
        fill = fill_value(el.get("fill"), opacity, grads, (x, y, w, h), [])
        if fill:
            props.append(f"background: {fill};")
        if stroke:
            # SVG strokes straddle the edge, Slint borders sit inside: grow the box by half the stroke.
            x, y, w, h, rx = x - sw / 2, y - sw / 2, w + sw, h + sw, rx + sw / 2
            so = float(el.get("stroke-opacity", "1")) * opacity
            props.append(f"border-width: {num(sw)} * u; border-color: {color(stroke, so)};")
        a = aff.a
        out.append(
            f"    Rectangle {{ x: {num(aff.bx + a * x)} * u; y: {num(aff.by + a * y)} * u; "
            f"width: {num(a * w)} * u; height: {num(a * h)} * u; border-radius: {num(a * rx)} * u; {' '.join(props)} }}"
        )

    def emit_path(el, aff, canvas):
        aff = aff.apply(el.get("transform"))
        opacity = float(el.get("opacity", "1"))
        d = el.get("d")
        box = (0.0, 0.0, canvas, canvas)
        radial: list = []
        fill = fill_value(el.get("fill"), opacity, grads, box, radial)
        if radial:
            cx, cy = radial[0]
            h = RADIAL_CIRCLE_RADIUS / math.sqrt(2)
            box = (cx - h, cy - h, 2 * h, 2 * h)
        bx, by, bw, bh = box
        geometry = (
            f"x: {num(aff.bx + aff.a * bx)} * u; y: {num(aff.by + aff.a * by)} * u; "
            f"width: {num(aff.a * bw)} * u; height: {num(aff.a * bh)} * u; "
            f"viewbox-x: {num(bx)}; viewbox-y: {num(by)}; viewbox-width: {num(bw)}; viewbox-height: {num(bh)}; "
            f'commands: "{d}"; '
        )
        if el.get("stroke"):
            # paint-order="stroke": stroke under fill, so only the outer half shows. Slint paints a stroke over its fill.
            sw = float(el.get("stroke-width"))
            out.append(f"    Path {{ {geometry}stroke: {color(el.get('stroke'), opacity)}; stroke-width: {num(sw * aff.a)} * u; }}")
        if fill:
            out.append(f"    Path {{ {geometry}fill: {fill}; }}")

    def walk(el, aff, canvas):
        for ch in el:
            tag = ch.tag.removeprefix(NS)
            if tag == "rect":
                emit_rect(ch, aff)
            elif tag == "path":
                emit_path(ch, aff, canvas)
            elif tag == "g":
                inner = aff.apply(ch.get("transform"))
                walk(ch, inner, canvas if ch.get("transform") is None else group_canvas(ch))
            elif tag in ("defs", "linearGradient", "radialGradient"):
                continue
            else:
                sys.exit(f"unsupported element <{tag}> in {SOURCE}")

    canvases = iter(CANVAS.values())
    cache: dict[int, float] = {}

    def group_canvas(g):
        # Groups with a scale() transform are the three Fluent art groups, in document order.
        if id(g) not in cache:
            cache[id(g)] = next(canvases) if "scale" in g.get("transform", "") else 0.0
        return cache[id(g)]

    walk(root, Affine(), 0.0)
    body = "\n".join(out)
    return (
        f"{HEADER}export component AppIcon inherits Rectangle {{\n"
        f"    // Square, drawn on the SVG's {num(size)} unit canvas scaled to the component's width.\n"
        f"    property <length> u: self.width / {num(size)};\n{body}\n}}\n"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare instead of writing")
    args = parser.parse_args()
    rendered = render()
    if args.check:
        if TARGET.read_text(encoding="utf-8") != rendered:
            sys.exit(f"{TARGET} differs from the rendered icon: run scripts/gen_app_icon.py")
        print(f"{TARGET.relative_to(ROOT)}: OK")
    else:
        TARGET.write_text(rendered, encoding="utf-8")


if __name__ == "__main__":
    main()
