#!/usr/bin/env python3
"""Generate icon.png from icon.svg with blur applied to background shapes.

cairosvg doesn't support SVG filters, so we:
1. Render background shapes (only) to a separate PNG
2. Apply Gaussian blur with Pillow
3. Render foreground shapes to another PNG
4. Composite: blurred background + sharp foreground

Usage: python3 gen_icon.py [blur_radius]
  blur_radius: Pillow blur radius (default: 2.0, 0 = no blur)
"""

import cairosvg
from PIL import Image, ImageFilter
import io
import os
import sys
import re

HERE = os.path.dirname(os.path.abspath(__file__))
SVG_PATH = os.path.join(HERE, "icon.svg")
PNG_PATH = os.path.join(HERE, "icon.png")
SIZE = 256

BLUR_RADIUS = float(sys.argv[1]) if len(sys.argv) > 1 else 2.0


def read_svg():
    with open(SVG_PATH) as f:
        return f.read()


def extract_layers(svg):
    """Split SVG into background-only and foreground-only versions."""
    # Background: everything before "<!-- Foreground:"
    # Foreground: everything from "<!-- Foreground:" onward

    # Find the split point
    fg_marker = "<!-- Foreground:"
    idx = svg.index(fg_marker)

    # Find the preceding whitespace/newlines to split cleanly
    # We need valid SVG for each layer, so wrap in the same outer structure

    # Extract the outer wrapper (everything up to and including the main <g>)
    # and closing tags
    header_match = re.search(r'(.*?<g transform="translate\(23\.95,23\.93\)">\s*)', svg, re.DOTALL)
    header = header_match.group(1)
    footer = "\n  </g>\n</svg>"

    # Background content: between header and foreground marker
    bg_content = svg[header_match.end():idx]

    # Foreground content: from marker to closing tags
    fg_end = svg.index("</g>\n</svg>")
    fg_content = svg[idx:fg_end]

    bg_svg = header + bg_content + footer
    fg_svg = header + fg_content + footer

    return bg_svg, fg_svg


def svg_to_pil(svg_str):
    """Render SVG string to PIL RGBA image."""
    png_bytes = cairosvg.svg2png(bytestring=svg_str.encode(),
                                  output_width=SIZE, output_height=SIZE)
    return Image.open(io.BytesIO(png_bytes)).convert('RGBA')


def main():
    svg = read_svg()
    bg_svg, fg_svg = extract_layers(svg)

    # Render layers
    bg_img = svg_to_pil(bg_svg)
    fg_img = svg_to_pil(fg_svg)

    # Apply blur to background
    if BLUR_RADIUS > 0:
        bg_img = bg_img.filter(ImageFilter.GaussianBlur(radius=BLUR_RADIUS))

    # Composite: blurred background + sharp foreground
    result = Image.new('RGBA', (SIZE, SIZE), (0, 0, 0, 0))
    result = Image.alpha_composite(result, bg_img)
    result = Image.alpha_composite(result, fg_img)

    result.save(PNG_PATH, 'PNG')
    fsize = os.path.getsize(PNG_PATH)
    print(f"PNG saved: {PNG_PATH} ({fsize} bytes, {SIZE}x{SIZE}, blur={BLUR_RADIUS})")


if __name__ == '__main__':
    main()
