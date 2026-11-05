"""Slice approved ImageGen atlases and export the game's premultiplied BGRA.

Usage: python tools/import_generated_art.py ATLAS infantry|equipment|buildings|heroes|late
The original atlas is retained in assets/sources for reproducibility.
"""
import argparse
import shutil
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1] / 'assets'
COUNTRIES = 'ger auh ott bul gbr fra rus srb bel ita rou gre por usa ned sui esp swe nor den'.split()
LAYOUTS = {
    'infantry': (5, 4, 160, [f'nations/{c}_infantry' for c in COUNTRIES]),
    'equipment': (5, 2, 160, ['units/worker', 'units/artillery', 'units/cavalry', 'units/tank', 'units/storm', 'units/alpine', 'units/submarine', 'units/destroyer', 'units/transport', 'units/tank_french']),
    'buildings': (3, 2, 192, [f'map/{n}' for n in 'capital town barracks farm fort dock'.split()]),
    'heroes': (8, 6, 128, [f'heroes/hero_{i:02}' for i in range(46)]),
    'late': (2, 2, 160, [f'nations/{c}_infantry_late' for c in ['ger', 'gbr', 'fra', 'bel']]),
}

def export(im, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    im.save(path.with_suffix('.png'))
    out = bytearray()
    for r, g, b, a in im.getdata():
        out.extend((b * a // 255, g * a // 255, r * a // 255, a))
    path.with_suffix('.bgra').write_bytes(out)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('atlas', type=Path)
    parser.add_argument('kind', choices=LAYOUTS)
    args = parser.parse_args()
    im = Image.open(args.atlas).convert('RGBA')
    if im.getchannel('A').getextrema()[0] > 0:
        raise ValueError('Atlas has no fully transparent pixels; correct with ImageGen first')
    cols, rows, size, names = LAYOUTS[args.kind]
    source = ROOT / 'sources' / f'{args.kind}-atlas.png'
    source.parent.mkdir(parents=True, exist_ok=True)
    if args.atlas.resolve() != source.resolve():
        shutil.copyfile(args.atlas, source)
    for i, name in enumerate(names):
        x, y = i % cols, i // cols
        cell = im.crop((round(x * im.width / cols), round(y * im.height / rows), round((x + 1) * im.width / cols), round((y + 1) * im.height / rows)))
        # Preserve atlas framing and alpha; normalize into the renderer's square canvas.
        cell.thumbnail((size, size), Image.Resampling.LANCZOS)
        sprite = Image.new('RGBA', (size, size))
        sprite.paste(cell, ((size - cell.width) // 2, (size - cell.height) // 2))
        export(sprite, ROOT / name)
    if args.kind == 'infantry':
        for suffix in ('.png', '.bgra'):
            shutil.copyfile(ROOT / f'nations/ger_infantry{suffix}', ROOT / f'units/infantry{suffix}')
    print(f'Imported {len(names)} {args.kind} sprites')

if __name__ == '__main__':
    main()
