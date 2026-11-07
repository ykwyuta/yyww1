"""Export browser and Windows icons from the dedicated app icon artwork.

Run: python tools/update_app_icons.py
Requires Pillow. Only size/format conversion is performed here.
"""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ICO_SIZES = [(n, n) for n in (16, 24, 32, 48, 64, 128, 256)]


def update_icons():
    with Image.open(ROOT / "assets" / "app_icon.png") as source:
        if source.width != source.height:
            raise ValueError("App icon artwork must be square")
        image = source.convert("RGB")
    for filename, size in [
        ("icon-512.png", 512),
        ("icon-192.png", 192),
        ("apple-touch-icon.png", 180),
    ]:
        image.resize((size, size), Image.Resampling.LANCZOS).save(
            ROOT / "web" / filename, optimize=True
        )
    image.convert("RGBA").save(ROOT / "assets" / "app_icon.ico", sizes=ICO_SIZES)


if __name__ == "__main__":
    update_icons()
