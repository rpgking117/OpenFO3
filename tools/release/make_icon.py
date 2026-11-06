"""Draws OpenFO3's icon: a burnt-orange gear ring and hub on a dark disc.

Writes engine/launcher/assets/openfo3.ico (every Windows icon size) and
openfo3.png (256 px: the launcher's window icon and the README's), and the
installer's wizard pictures (wizard*.bmp; the text in Segoe UI when Windows
has it). The pictures aren't committed (binary files are made, not kept):
release.py draws them into the installer folder before building it.

    python tools/release/make_icon.py                  the icon and the pictures
    python tools/release/make_icon.py --wizard DIR     only the pictures, into DIR
"""

import math
import os
import sys

from PIL import Image, ImageDraw, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, "engine", "launcher", "assets")
INSTALLER = os.path.join(ROOT, "engine", "installer")
FONTS = os.path.join(os.environ.get("WINDIR", r"C:\Windows"), "Fonts")

DARK = (0x1C, 0x1E, 0x22, 255)
EDGE = (0x34, 0x38, 0x3F, 255)
ORANGE = (0xCC, 0x55, 0x00, 255)
LIGHT = (0xFF, 0x8A, 0x3D, 255)


def draw(size: int) -> Image.Image:
    s = 1024  # drawn large, then scaled down smoothly
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    c = s / 2
    # The disc.
    d.ellipse([c - 500, c - 500, c + 500, c + 500], fill=EDGE)
    d.ellipse([c - 484, c - 484, c + 484, c + 484], fill=DARK)
    # The gear: a toothed ring.
    teeth = 10
    outer, root, inner = 420, 352, 262
    points = []
    for i in range(teeth * 4):
        a = 2 * math.pi * i / (teeth * 4) - math.pi / 2
        r = outer if (i % 4) in (1, 2) else root
        points.append((c + r * math.cos(a), c + r * math.sin(a)))
    d.polygon(points, fill=ORANGE)
    d.ellipse([c - inner, c - inner, c + inner, c + inner], fill=DARK)
    # The hub.
    hub = 132
    d.ellipse([c - hub, c - hub, c + hub, c + hub], fill=LIGHT)
    d.ellipse([c - 52, c - 52, c + 52, c + 52], fill=DARK)
    return img.resize((size, size), Image.LANCZOS)


def font(name: str, size: int):
    try:
        return ImageFont.truetype(os.path.join(FONTS, name), size)
    except OSError:
        return ImageFont.load_default()


def wizard(scale: int) -> Image.Image:
    """The tall picture down the side of the installer's first and last pages."""
    w, h = 164 * scale, 314 * scale
    img = Image.new("RGB", (w, h))
    top, bottom = (0x17, 0x18, 0x1B), (0x52, 0x24, 0x06)
    for y in range(h):
        t = (y / h) ** 1.6
        row = tuple(int(a + (b - a) * t) for a, b in zip(top, bottom))
        ImageDraw.Draw(img).line([(0, y), (w, y)], fill=row)
    d = ImageDraw.Draw(img, "RGBA")
    for k in range(9):  # faint hazard stripes low down
        x = k * 34 * scale - 40 * scale
        d.line([(x, h), (x + 90 * scale, h - 150 * scale)], fill=(0xCC, 0x55, 0x00, 22), width=14 * scale)
    gear = draw(112 * scale)
    img.paste(gear, ((w - gear.width) // 2, 48 * scale), gear)
    title = font("seguisb.ttf", 27 * scale)
    d.text((w / 2, 192 * scale), "OpenFO3", font=title, fill=(255, 255, 255), anchor="mm")
    small = font("segoeui.ttf", 11 * scale)
    d.text((w / 2, 218 * scale), "An open-source engine", font=small, fill=(0xD0, 0xC8, 0xBE), anchor="mm")
    d.text((w / 2, 233 * scale), "for Fallout 3", font=small, fill=(0xD0, 0xC8, 0xBE), anchor="mm")
    credit = font("seguisb.ttf", 11 * scale)
    d.text((w / 2, 280 * scale), "by RPGKING", font=credit, fill=(0xFF, 0x8A, 0x3D), anchor="mm")
    d.text((w / 2, 295 * scale), "The Mad Modder", font=credit, fill=(0xFF, 0x8A, 0x3D), anchor="mm")
    return img


def wizard_small(scale: int) -> Image.Image:
    """The icon in the corner of the installer's other pages (a white header)."""
    size = 55 * scale
    img = Image.new("RGB", (size, size), (255, 255, 255))
    icon = draw(size - 4 * scale)
    img.paste(icon, (2 * scale, 2 * scale), icon)
    return img


def wizard_images(dest: str) -> None:
    os.makedirs(dest, exist_ok=True)
    for scale, suffix in [(1, ""), (2, "-2x")]:
        wizard(scale).save(os.path.join(dest, f"wizard{suffix}.bmp"))
        wizard_small(scale).save(os.path.join(dest, f"wizard-small{suffix}.bmp"))


def main() -> None:
    if sys.argv[1:2] == ["--wizard"]:
        wizard_images(sys.argv[2])
        print("wrote the wizard pictures in", sys.argv[2])
        return
    os.makedirs(OUT, exist_ok=True)
    big = draw(256)
    big.save(os.path.join(OUT, "openfo3.png"))
    big.save(
        os.path.join(OUT, "openfo3.ico"),
        sizes=[(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    wizard_images(INSTALLER)
    print("wrote", OUT, "and", INSTALLER)


if __name__ == "__main__":
    main()
