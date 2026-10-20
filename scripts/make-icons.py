#!/usr/bin/env python3
"""Builds the app icons in src-tauri/icons from Icon/TaskCraft.png.

The source artwork is a full-bleed rounded square. macOS app icons place that
shape at 824×824 inside a transparent 1024×1024 canvas (Apple's icon grid), so
the artwork is trimmed to its visible pixels, fitted into 824×824 and centred.
Then every PNG size and icon.icns (via the system's iconutil) are written.

Usage: python3 scripts/make-icons.py   (needs Pillow)
"""
from pathlib import Path
import shutil
import subprocess
import tempfile

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "Icon" / "TaskCraft.png"
OUT = ROOT / "src-tauri" / "icons"
CANVAS = 1024
SHAPE = 824  # Apple's rounded-square size on a 1024 canvas


def master() -> Image.Image:
    art = Image.open(SRC).convert("RGBA")
    art = art.crop(art.getbbox())  # drop fully transparent margins
    scale = SHAPE / max(art.size)
    art = art.resize((round(art.width * scale), round(art.height * scale)), Image.LANCZOS)
    canvas = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    canvas.paste(art, ((CANVAS - art.width) // 2, (CANVAS - art.height) // 2), art)
    return canvas


def save(img: Image.Image, size: int, path: Path) -> None:
    img.resize((size, size), Image.LANCZOS).save(path, optimize=True)


def main() -> None:
    img = master()
    for name, size in [("32x32.png", 32), ("64x64.png", 64), ("128x128.png", 128),
                       ("128x128@2x.png", 256), ("icon.png", 512)]:
        save(img, size, OUT / name)

    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "icon.iconset"
        iconset.mkdir()
        for base in (16, 32, 128, 256, 512):
            save(img, base, iconset / f"icon_{base}x{base}.png")
            save(img, base * 2, iconset / f"icon_{base}x{base}@2x.png")
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "icon.icns")], check=True)

    # Keep the fitted 1024 master next to the source for reference.
    img.save(ROOT / "Icon" / "TaskCraft-1024.png", optimize=True)
    print("icons written to", OUT)


if __name__ == "__main__":
    if shutil.which("iconutil") is None:
        raise SystemExit("iconutil not found (macOS only)")
    main()
