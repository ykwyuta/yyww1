#!/usr/bin/env python3
"""第一次大戦版の素材（兵・建物・英雄・天使）を描いて assets/ に書き出す。

  python3 tools/gen_art.py

4倍の大きさで描いて縮め、なめらかにする。書き出すのは PNG（確認用）と、
ゲームが埋め込む前乗算アルファの BGRA（.bgra）。PIL（Pillow）だけを使う。
"""
import math
import os
import random

from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets")
SS = 4  # 描画の倍率


def rgba(c, a=255):
    return (int(c[0]), int(c[1]), int(c[2]), a)


def shade(c, k):
    return tuple(max(0, min(255, int(v * k))) for v in c[:3])


def mix(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


class Pen:
    """4倍の画布に描く道具"""

    def __init__(self, size):
        self.size = size
        self.im = Image.new("RGBA", (size * SS, size * SS), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.im, "RGBA")

    def P(self, pts):
        return [(x * SS, y * SS) for x, y in pts]

    def poly(self, pts, c, outline=None, w=1.0):
        self.d.polygon(self.P(pts), fill=rgba(c) if len(c) == 3 else c)
        if outline is not None:
            self.d.line(self.P(pts + [pts[0]]), fill=rgba(outline), width=max(1, int(w * SS)), joint="curve")

    def ell(self, cx, cy, rx, ry, c, outline=None, w=1.0):
        box = [(cx - rx) * SS, (cy - ry) * SS, (cx + rx) * SS, (cy + ry) * SS]
        fill = None if c is None else (rgba(c) if len(c) == 3 else c)
        self.d.ellipse(box, fill=fill, outline=rgba(outline) if outline else None, width=max(1, int(w * SS)) if outline else 0)

    def line(self, pts, c, w):
        self.d.line(self.P(pts), fill=rgba(c) if len(c) == 3 else c, width=max(1, int(w * SS)), joint="curve")
        r = w / 2
        for (x, y) in (pts[0], pts[-1]):
            self.ell(x, y, r, r, c)

    def rect(self, x0, y0, x1, y1, c, outline=None, w=1.0):
        self.poly([(x0, y0), (x1, y0), (x1, y1), (x0, y1)], c, outline, w)

    def done(self):
        return self.im.resize((self.size, self.size), Image.LANCZOS)


def save(im, rel):
    path = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    im.save(path + ".png")
    px = im.load()
    w, h = im.size
    out = bytearray(w * h * 4)
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            i = (y * w + x) * 4
            out[i] = b * a // 255
            out[i + 1] = g * a // 255
            out[i + 2] = r * a // 255
            out[i + 3] = a
    with open(path + ".bgra", "wb") as f:
        f.write(bytes(out))


def shadow(p, cx, cy, rx, ry=None):
    p.ell(cx, cy, rx, ry or rx * 0.3, (0, 0, 0, 70))


# ---------------------------------------------------------------- 人物
SKIN = (226, 184, 150)
KHAKI = (150, 128, 82)
FELDGRAU = (112, 118, 104)
HORIZON = (120, 140, 170)
OLIVE = (104, 110, 70)
LEATHER = (96, 64, 36)
STEEL = (120, 124, 120)
WOOD = (120, 78, 40)
DARK = (34, 30, 26)


def head(p, x, y, r, skin=SKIN, mustache=False, beard=False):
    p.ell(x, y, r, r * 1.08, skin, outline=shade(skin, 0.6), w=0.8)
    p.ell(x + r * 0.45, y - r * 0.1, r * 0.13, r * 0.13, DARK)
    p.ell(x + r * 0.95, y + r * 0.05, r * 0.18, r * 0.25, shade(skin, 0.92))
    if mustache:
        p.poly([(x + r * 0.2, y + r * 0.45), (x + r * 1.0, y + r * 0.35), (x + r * 0.95, y + r * 0.55), (x + r * 0.3, y + r * 0.62)], (210, 210, 210) if beard else (70, 50, 30))
    if beard:
        p.poly([(x - r * 0.4, y + r * 0.3), (x + r * 0.9, y + r * 0.5), (x + r * 0.5, y + r * 1.35), (x - r * 0.1, y + r * 1.1)], (215, 215, 210))


def headgear(p, kind, x, y, r, col):
    if kind == "brodie":  # 英国の皿型ヘルメット
        p.ell(x, y - r * 0.55, r * 1.7, r * 0.42, shade(col, 0.85), outline=shade(col, 0.5), w=0.8)
        p.ell(x, y - r * 0.75, r * 1.0, r * 0.65, col, outline=shade(col, 0.5), w=0.8)
    elif kind == "stahl":  # ドイツの鉄兜
        p.poly([(x - r * 1.15, y - r * 0.2), (x - r * 1.05, y - r * 1.0), (x, y - r * 1.45), (x + r * 1.0, y - r * 1.0), (x + r * 1.2, y - r * 0.35), (x + r * 0.9, y - r * 0.3), (x - r * 0.6, y + r * 0.35)], col, outline=shade(col, 0.5), w=0.8)
    elif kind == "pickel":  # ピッケルハウベ
        p.ell(x, y - r * 0.7, r * 1.08, r * 0.7, (40, 36, 30), outline=(20, 18, 15), w=0.8)
        p.rect(x - r * 0.15, y - r * 1.75, x + r * 0.15, y - r * 1.2, (200, 180, 90))
        p.ell(x, y - r * 1.8, r * 0.2, r * 0.2, (220, 200, 110))
        p.ell(x + r * 0.15, y - r * 0.85, r * 0.3, r * 0.3, (210, 190, 100))
    elif kind == "adrian":  # フランスのアドリアン兜
        p.ell(x, y - r * 0.5, r * 1.45, r * 0.35, shade(col, 0.85), outline=shade(col, 0.5), w=0.8)
        p.ell(x, y - r * 0.85, r * 1.0, r * 0.7, col, outline=shade(col, 0.5), w=0.8)
        p.rect(x - r * 0.12, y - r * 1.75, x + r * 0.12, y - r * 1.2, shade(col, 0.8))
    elif kind == "kepi":
        p.poly([(x - r * 0.95, y - r * 0.3), (x - r * 0.85, y - r * 1.4), (x + r * 0.85, y - r * 1.3), (x + r * 0.95, y - r * 0.3)], col, outline=shade(col, 0.5), w=0.8)
        p.poly([(x + r * 0.3, y - r * 0.35), (x + r * 1.5, y - r * 0.15), (x + r * 0.9, y - r * 0.5)], DARK)
    elif kind == "cap":  # 庇つき軍帽
        p.ell(x, y - r * 0.85, r * 1.15, r * 0.5, col, outline=shade(col, 0.5), w=0.8)
        p.rect(x - r * 0.95, y - r * 0.85, x + r * 0.95, y - r * 0.45, shade(col, 0.85))
        p.poly([(x + r * 0.4, y - r * 0.5), (x + r * 1.55, y - r * 0.35), (x + r * 0.6, y - r * 0.3)], DARK)
        p.ell(x + r * 0.3, y - r * 0.75, r * 0.18, r * 0.18, (220, 190, 90))
    elif kind == "fez":
        p.poly([(x - r * 0.8, y - r * 0.4), (x - r * 0.65, y - r * 1.55), (x + r * 0.65, y - r * 1.55), (x + r * 0.8, y - r * 0.4)], (170, 40, 36), outline=(90, 20, 18), w=0.8)
    elif kind == "kalpak":
        p.ell(x, y - r * 0.9, r * 1.0, r * 0.75, (90, 70, 50), outline=(50, 40, 28), w=0.8)
    elif kind == "alpine":  # 羽根つき山岳帽
        p.ell(x, y - r * 0.45, r * 1.5, r * 0.3, shade(col, 0.8))
        p.ell(x, y - r * 0.85, r * 0.95, r * 0.6, col, outline=shade(col, 0.5), w=0.8)
        p.line([(x - r * 0.5, y - r * 1.0), (x - r * 1.6, y - r * 2.4)], (40, 40, 40), r * 0.28)
    elif kind == "flatcap":
        p.ell(x, y - r * 0.8, r * 1.1, r * 0.55, col, outline=shade(col, 0.5), w=0.8)
        p.poly([(x + r * 0.3, y - r * 0.6), (x + r * 1.5, y - r * 0.4), (x + r * 0.6, y - r * 0.25)], shade(col, 0.7))
    elif kind == "bicorne":
        p.poly([(x - r * 1.4, y - r * 0.5), (x, y - r * 1.8), (x + r * 1.4, y - r * 0.5)], (30, 30, 34), outline=(10, 10, 10), w=0.8)
        p.ell(x, y - r * 1.0, r * 0.25, r * 0.25, (220, 200, 120))
    elif kind == "crown":
        p.poly([(x - r * 0.9, y - r * 0.6), (x - r * 0.9, y - r * 1.5), (x - r * 0.45, y - r * 1.1), (x, y - r * 1.7), (x + r * 0.45, y - r * 1.1), (x + r * 0.9, y - r * 1.5), (x + r * 0.9, y - r * 0.6)], (230, 196, 70), outline=(140, 100, 30), w=0.8)
    elif kind == "keffiyeh":
        p.poly([(x - r * 1.1, y + r * 0.9), (x - r * 1.15, y - r * 0.8), (x, y - r * 1.25), (x + r * 1.05, y - r * 0.75), (x + r * 0.7, y - r * 0.3), (x - r * 0.4, y + r * 1.1)], (236, 230, 214), outline=(150, 140, 120), w=0.8)
        p.ell(x, y - r * 0.85, r * 1.05, r * 0.25, (40, 34, 30))
    elif kind == "pilot":
        p.ell(x, y - r * 0.45, r * 1.12, r * 1.0, (96, 66, 40), outline=(50, 34, 20), w=0.8)
        p.ell(x + r * 0.35, y - r * 0.55, r * 0.55, r * 0.3, (170, 200, 210), outline=(60, 60, 60), w=0.8)


def soldier(p, x, base, s, uni, gear, gear_col, weapon="rifle", pack=True, skin=SKIN, mustache=False, beard=False, coat=False, sash=None, pose=0):
    """横向き（右向き）の兵士。x=中心, base=足元の y, s=倍率"""
    dk = shade(uni, 0.62)
    # 足
    lx = x - 7 * s
    rx = x + 6 * s
    p.poly([(x - 9 * s, base - 46 * s), (x - 1 * s, base - 46 * s), (lx + 2 * s, base - 8 * s), (lx - 6 * s, base - 8 * s)], dk, outline=shade(uni, 0.4), w=0.6)
    p.poly([(x - 2 * s, base - 46 * s), (x + 7 * s, base - 46 * s), (rx + 5 * s, base - 8 * s), (rx - 3 * s, base - 8 * s)], uni, outline=shade(uni, 0.4), w=0.6)
    # ゲートル・靴
    p.rect(lx - 6 * s, base - 22 * s, lx + 2.5 * s, base - 7 * s, shade(uni, 0.75))
    p.rect(rx - 3 * s, base - 22 * s, rx + 5.5 * s, base - 7 * s, shade(uni, 0.8))
    p.poly([(lx - 7 * s, base - 8 * s), (lx + 5 * s, base - 8 * s), (lx + 6 * s, base), (lx - 7 * s, base)], DARK)
    p.poly([(rx - 4 * s, base - 8 * s), (rx + 8 * s, base - 8 * s), (rx + 10 * s, base), (rx - 4 * s, base)], DARK)
    # 背嚢
    if pack:
        p.poly([(x - 18 * s, base - 82 * s), (x - 8 * s, base - 86 * s), (x - 6 * s, base - 58 * s), (x - 17 * s, base - 56 * s)], shade(uni, 0.7), outline=shade(uni, 0.4), w=0.6)
        p.ell(x - 13 * s, base - 88 * s, 7 * s, 3.5 * s, shade(uni, 0.55))
    # 胴
    tl = 54 if coat else 48
    p.poly([(x - 11 * s, base - 88 * s), (x + 10 * s, base - 88 * s), (x + 13 * s, base - (88 - tl) * s), (x - 13 * s, base - (88 - tl) * s)], uni, outline=shade(uni, 0.4), w=0.7)
    p.poly([(x + 2 * s, base - 88 * s), (x + 10 * s, base - 88 * s), (x + 13 * s, base - (88 - tl) * s), (x + 4 * s, base - (88 - tl) * s)], shade(uni, 1.12))
    p.rect(x - 12 * s, base - 54 * s, x + 12 * s, base - 50 * s, LEATHER)
    p.rect(x + 3 * s, base - 55 * s, x + 7 * s, base - 49 * s, (190, 170, 100))
    if sash:
        p.poly([(x - 10 * s, base - 88 * s), (x - 4 * s, base - 88 * s), (x + 12 * s, base - 58 * s), (x + 6 * s, base - 56 * s)], sash)
    # 首・頭
    p.rect(x - 3 * s, base - 94 * s, x + 4 * s, base - 86 * s, skin)
    hr = 9.5 * s
    hx, hy = x + 1 * s, base - 102 * s
    head(p, hx, hy, hr, skin, mustache, beard)
    headgear(p, gear, hx, hy, hr, gear_col)
    # 腕と武器
    if weapon == "rifle":
        p.line([(x - 20 * s, base - 60 * s), (x + 34 * s, base - 108 * s)], WOOD, 3.6 * s)
        p.line([(x + 20 * s, base - 96 * s), (x + 34 * s, base - 108 * s)], STEEL, 2.4 * s)
        p.line([(x + 34 * s, base - 108 * s), (x + 46 * s, base - 119 * s)], (215, 215, 220), 1.4 * s)
        p.line([(x + 6 * s, base - 84 * s), (x + 4 * s, base - 70 * s), (x + 14 * s, base - 80 * s)], uni, 6 * s)
        p.ell(x + 15 * s, base - 81 * s, 3.2 * s, 3.2 * s, skin)
        p.line([(x - 4 * s, base - 84 * s), (x - 8 * s, base - 66 * s), (x - 12 * s, base - 64 * s)], dk, 6 * s)
        p.ell(x - 13 * s, base - 63 * s, 3.2 * s, 3.2 * s, skin)
    elif weapon == "grenade":
        p.line([(x - 4 * s, base - 84 * s), (x - 16 * s, base - 108 * s)], dk, 6 * s)
        p.line([(x - 16 * s, base - 108 * s), (x - 22 * s, base - 124 * s)], WOOD, 2.6 * s)
        p.ell(x - 23 * s, base - 128 * s, 4 * s, 5.5 * s, (90, 96, 86), outline=(40, 40, 36), w=0.6)
        p.line([(x + 6 * s, base - 84 * s), (x + 18 * s, base - 70 * s)], uni, 6 * s)
        p.line([(x + 4 * s, base - 72 * s), (x + 34 * s, base - 76 * s)], DARK, 3.4 * s)
        for k in range(3):
            p.line([(x - 9 * s + k * 5 * s, base - 52 * s), (x - 9 * s + k * 5 * s, base - 40 * s)], WOOD, 2 * s)
    elif weapon == "shovel":
        p.line([(x - 6 * s, base - 30 * s), (x + 22 * s, base - 120 * s)], WOOD, 3.2 * s)
        p.poly([(x - 12 * s, base - 30 * s), (x, base - 34 * s), (x - 2 * s, base - 14 * s), (x - 10 * s, base - 12 * s)], STEEL, outline=(60, 60, 60), w=0.6)
        p.line([(x + 6 * s, base - 84 * s), (x + 10 * s, base - 76 * s)], uni, 6 * s)
        p.ell(x + 11 * s, base - 76 * s, 3.2 * s, 3.2 * s, skin)
        p.line([(x - 4 * s, base - 84 * s), (x + 2 * s, base - 70 * s)], dk, 6 * s)
    elif weapon == "sword":
        p.line([(x + 6 * s, base - 84 * s), (x + 18 * s, base - 96 * s)], uni, 6 * s)
        p.ell(x + 19 * s, base - 97 * s, 3.2 * s, 3.2 * s, skin)
        p.line([(x + 19 * s, base - 97 * s), (x + 40 * s, base - 132 * s)], (225, 225, 232), 2.2 * s)
        p.line([(x - 4 * s, base - 84 * s), (x - 8 * s, base - 64 * s)], dk, 6 * s)
    elif weapon == "baton":
        p.line([(x + 6 * s, base - 84 * s), (x + 16 * s, base - 70 * s)], uni, 6 * s)
        p.ell(x + 17 * s, base - 69 * s, 3.2 * s, 3.2 * s, skin)
        p.line([(x + 14 * s, base - 64 * s), (x + 22 * s, base - 76 * s)], (60, 40, 90), 3 * s)
        p.line([(x - 4 * s, base - 84 * s), (x - 8 * s, base - 64 * s)], dk, 6 * s)
    elif weapon == "binocular":
        p.line([(x + 6 * s, base - 84 * s), (x + 12 * s, base - 100 * s)], uni, 6 * s)
        p.rect(x + 10 * s, base - 106 * s, x + 18 * s, base - 98 * s, DARK)
        p.line([(x - 4 * s, base - 84 * s), (x - 8 * s, base - 64 * s)], dk, 6 * s)
    elif weapon == "alpen":
        p.line([(x - 2 * s, base), (x + 18 * s, base - 120 * s)], WOOD, 2.6 * s)
        p.line([(x + 6 * s, base - 84 * s), (x + 12 * s, base - 80 * s)], uni, 6 * s)
        p.ell(x + 13 * s, base - 80 * s, 3.2 * s, 3.2 * s, skin)
        p.line([(x - 20 * s, base - 60 * s), (x - 4 * s, base - 110 * s)], WOOD, 3 * s)
        p.ell(x - 14 * s, base - 70 * s, 7 * s, 4 * s, (170, 150, 110), outline=(110, 90, 60), w=0.6)
        p.line([(x - 4 * s, base - 84 * s), (x - 8 * s, base - 66 * s)], dk, 6 * s)


def horse(p, x, base, s, coat=(110, 70, 40)):
    dk = shade(coat, 0.7)
    p.poly([(x - 34 * s, base - 70 * s), (x + 20 * s, base - 72 * s), (x + 26 * s, base - 48 * s), (x - 30 * s, base - 44 * s)], coat, outline=shade(coat, 0.5), w=0.8)
    for lx, ph in ((-28, 0.2), (-18, -0.3), (14, 0.35), (22, -0.2)):
        p.line([(x + lx * s, base - 48 * s), (x + (lx + ph * 18) * s, base - 22 * s), (x + (lx + ph * 6) * s, base - 3 * s)], dk if lx in (-18, 22) else coat, 5 * s)
        p.ell(x + (lx + ph * 6) * s, base - 2 * s, 3.5 * s, 2.2 * s, DARK)
    p.poly([(x + 14 * s, base - 72 * s), (x + 30 * s, base - 104 * s), (x + 40 * s, base - 104 * s), (x + 30 * s, base - 64 * s)], coat, outline=shade(coat, 0.5), w=0.8)
    p.poly([(x + 30 * s, base - 108 * s), (x + 52 * s, base - 96 * s), (x + 50 * s, base - 88 * s), (x + 34 * s, base - 92 * s)], coat, outline=shade(coat, 0.5), w=0.8)
    p.poly([(x + 28 * s, base - 106 * s), (x + 12 * s, base - 76 * s), (x + 18 * s, base - 72 * s), (x + 32 * s, base - 100 * s)], DARK)
    p.line([(x - 34 * s, base - 68 * s), (x - 46 * s, base - 40 * s)], DARK, 4 * s)
    p.ell(x + 44 * s, base - 99 * s, 1.6 * s, 1.6 * s, DARK)
    p.poly([(x - 10 * s, base - 76 * s), (x + 10 * s, base - 76 * s), (x + 8 * s, base - 64 * s), (x - 8 * s, base - 64 * s)], (130, 30, 30))


def gen_units():
    # 民（作業着・ハンチング帽・スコップ）
    p = Pen(160)
    shadow(p, 80, 152, 26)
    soldier(p, 76, 150, 1.12, (92, 82, 70), "flatcap", (80, 70, 60), weapon="shovel", pack=False)
    p.ell(102, 122, 10, 12, (176, 150, 100), outline=(110, 90, 60), w=0.8)
    save(p.done(), "units/worker")
    # 歩兵
    p = Pen(160)
    shadow(p, 78, 152, 26)
    soldier(p, 72, 150, 1.12, KHAKI, "brodie", OLIVE)
    save(p.done(), "units/infantry")
    # 突撃兵
    p = Pen(160)
    shadow(p, 78, 152, 26)
    soldier(p, 80, 150, 1.12, FELDGRAU, "stahl", (96, 104, 92), weapon="grenade", pack=False)
    save(p.done(), "units/storm")
    # 山岳兵
    p = Pen(160)
    shadow(p, 78, 152, 26)
    soldier(p, 76, 150, 1.12, (100, 104, 92), "alpine", (80, 86, 70), weapon="alpen")
    save(p.done(), "units/alpine")
    # 砲兵（野砲と砲手）
    p = Pen(160)
    shadow(p, 80, 150, 58, 10)
    soldier(p, 34, 148, 0.82, KHAKI, "cap", OLIVE, weapon="binocular", pack=False)
    gun = (96, 104, 78)
    p.line([(70, 112), (150, 82)], shade(gun, 0.8), 9)
    p.line([(120, 92), (154, 80)], shade(gun, 0.7), 6)
    p.poly([(78, 82), (106, 72), (112, 120), (82, 128)], gun, outline=shade(gun, 0.5), w=1)
    p.line([(84, 122), (40, 146)], shade(gun, 0.7), 6)
    for cx in (96,):
        p.ell(cx, 124, 24, 24, None, outline=(60, 44, 30), w=4)
        for k in range(10):
            a = k * math.pi / 5
            p.line([(cx, 124), (cx + 20 * math.cos(a), 124 + 20 * math.sin(a))], (110, 80, 50), 2.2)
        p.ell(cx, 124, 5, 5, (70, 60, 50))
    save(p.done(), "units/artillery")
    # 騎兵
    p = Pen(160)
    shadow(p, 80, 152, 46, 9)
    horse(p, 74, 150, 1.0)
    soldier(p, 70, 120, 0.72, KHAKI, "cap", OLIVE, weapon="sword", pack=False)
    save(p.done(), "units/cavalry")
    # 戦車（菱形戦車）
    p = Pen(160)
    shadow(p, 80, 138, 72, 12)
    hull = (122, 116, 82)
    pts = [(8, 108), (26, 70), (126, 70), (154, 112), (138, 136), (22, 136)]
    p.poly(pts, shade(hull, 0.7), outline=(40, 38, 30), w=1.5)
    p.poly([(16, 108), (30, 78), (120, 78), (144, 112), (132, 128), (28, 128)], hull, outline=shade(hull, 0.5), w=1)
    for k in range(16):
        t = k / 15
        x = 22 + t * 116
        p.line([(x, 130), (x + 3, 136)], (50, 46, 36), 2)
    p.rect(54, 86, 98, 118, shade(hull, 0.85), outline=shade(hull, 0.5), w=1)
    p.ell(76, 100, 12, 10, shade(hull, 0.75), outline=(40, 38, 30), w=1)
    p.line([(84, 98), (110, 92)], (60, 58, 50), 4)
    for (x, y) in [(40, 96), (110, 96), (60, 120), (100, 120)]:
        p.ell(x, y, 2, 2, (60, 56, 44))
    p.poly([(60, 70), (92, 70), (88, 60), (64, 60)], shade(hull, 0.8), outline=shade(hull, 0.5), w=1)
    save(p.done(), "units/tank")
    # Uボート
    p = Pen(160)
    sea = (40, 80, 120, 150)
    p.ell(80, 112, 74, 14, sea)
    hull = (84, 90, 96)
    p.poly([(6, 108), (24, 98), (140, 98), (156, 106), (140, 116), (20, 116)], hull, outline=(30, 32, 36), w=1.2)
    p.poly([(6, 108), (24, 98), (140, 98), (156, 106)], shade(hull, 1.25))
    p.poly([(62, 98), (68, 76), (98, 76), (104, 98)], shade(hull, 0.9), outline=(30, 32, 36), w=1)
    p.line([(84, 76), (84, 56)], (60, 62, 66), 2.4)
    p.line([(78, 62), (84, 56)], (60, 62, 66), 1.6)
    p.line([(40, 96), (40, 88)], (70, 70, 70), 2)
    p.line([(36, 88), (48, 86)], (70, 70, 70), 2.5)
    p.rect(70, 82, 74, 86, (230, 230, 230))
    p.line([(10, 113), (150, 113)], (180, 210, 230), 1.5)
    save(p.done(), "units/submarine")
    # 駆逐艦
    p = Pen(160)
    p.ell(80, 118, 76, 12, sea)
    hull = (110, 116, 124)
    p.poly([(2, 104), (156, 100), (146, 124), (14, 124)], hull, outline=(40, 42, 46), w=1.2)
    p.poly([(2, 104), (156, 100), (153, 108), (6, 110)], shade(hull, 1.2))
    p.rect(52, 84, 104, 102, shade(hull, 1.05), outline=(40, 42, 46), w=1)
    p.rect(108, 80, 124, 100, shade(hull, 0.95), outline=(40, 42, 46), w=1)
    for fx in (60, 74, 88):
        p.rect(fx, 62, fx + 9, 86, (60, 60, 64), outline=(30, 30, 32), w=1)
        p.ell(fx + 2, 56, 6, 4, (120, 120, 120, 120))
    p.line([(116, 80), (116, 48)], (60, 60, 60), 2)
    p.line([(108, 58), (124, 58)], (60, 60, 60), 1.6)
    for gx in (24, 140):
        p.ell(gx, 98, 6, 4, shade(hull, 0.8))
        p.line([(gx, 97), (gx + (12 if gx > 80 else -12), 95)], (50, 50, 50), 2.4)
    p.line([(10, 120), (150, 118)], (180, 210, 230), 1.5)
    save(p.done(), "units/destroyer")
    # 輸送船
    p = Pen(160)
    p.ell(80, 120, 76, 12, sea)
    hull = (60, 54, 50)
    p.poly([(4, 98), (156, 96), (146, 126), (16, 126)], hull, outline=(24, 22, 20), w=1.2)
    p.poly([(4, 98), (156, 96), (154, 104), (6, 106)], (150, 40, 34))
    p.rect(62, 78, 108, 98, (230, 226, 214), outline=(90, 86, 80), w=1)
    for k in range(4):
        p.rect(66 + k * 10, 84, 72 + k * 10, 90, (60, 80, 110))
    p.rect(80, 50, 92, 80, (40, 40, 40), outline=(20, 20, 20), w=1)
    p.rect(80, 50, 92, 58, (170, 40, 34))
    p.ell(86, 44, 8, 5, (110, 110, 110, 140))
    for mx in (26, 132):
        p.line([(mx, 98), (mx, 52)], (90, 70, 50), 2.4)
        p.line([(mx, 60), (mx + 18 if mx < 80 else mx - 18, 92)], (90, 70, 50), 1.4)
    p.rect(14, 86, 40, 98, (150, 120, 80), outline=(90, 70, 40), w=1)
    p.rect(118, 86, 146, 98, (120, 130, 90), outline=(70, 80, 50), w=1)
    p.line([(10, 122), (150, 120)], (180, 210, 230), 1.5)
    save(p.done(), "units/transport")


# ---------------------------------------------------------------- 建物（斜め上から見た図）
def iso(cx, cy, w, d):
    """菱形の床：中心・幅・奥行き"""
    return [(cx, cy - d), (cx + w, cy), (cx, cy + d), (cx - w, cy)]


def box(p, cx, cy, w, d, h, wall, roof=None, roof_h=0, outline=(40, 34, 28)):
    """斜めの箱（左右の壁と屋根）"""
    top = [(cx, cy - d - h), (cx + w, cy - h), (cx, cy + d - h), (cx - w, cy - h)]
    p.poly([(cx - w, cy - h), (cx, cy + d - h), (cx, cy + d), (cx - w, cy)], shade(wall, 0.78), outline=outline, w=0.8)
    p.poly([(cx, cy + d - h), (cx + w, cy - h), (cx + w, cy), (cx, cy + d)], wall, outline=outline, w=0.8)
    if roof is None:
        p.poly(top, shade(wall, 1.15), outline=outline, w=0.8)
    else:
        rh = roof_h
        # 切妻屋根
        p.poly([(cx - w, cy - h), (cx, cy + d - h), (cx + w * 0.0, cy + d * 0.0 - h - rh), (cx - w, cy - h - rh * 0.0)], shade(roof, 0.8), outline=outline, w=0.8)
        p.poly([(cx - w, cy - h), (cx, cy - d - h), (cx, cy - d - h - rh * 0.0), (cx - w * 0.5, cy - d * 0.5 - h - rh)], roof, outline=outline, w=0.8)
        p.poly([(cx - w * 0.5, cy - d * 0.5 - h - rh), (cx + w * 0.5, cy + d * 0.5 - h - rh), (cx, cy + d - h), (cx - w, cy - h)], shade(roof, 0.75), outline=outline, w=0.8)
        p.poly([(cx - w * 0.5, cy - d * 0.5 - h - rh), (cx + w * 0.5, cy + d * 0.5 - h - rh), (cx + w, cy - h), (cx, cy - d - h)], roof, outline=outline, w=0.8)
        p.poly([(cx + w * 0.5, cy + d * 0.5 - h - rh), (cx + w, cy - h), (cx, cy + d - h)], shade(roof, 0.9), outline=outline, w=0.8)
    return top


def windows(p, cx, cy, w, d, h, rows, cols, col=(60, 70, 90)):
    for r in range(rows):
        for c in range(cols):
            t = (c + 0.5) / cols
            x = cx + w * t
            y = cy + d - d * t - h * (r + 0.6) / (rows + 0.4)
            p.poly([(x - w * 0.06, y - h * 0.18 / rows), (x + w * 0.04, y - h * 0.18 / rows - d * 0.1), (x + w * 0.04, y + h * 0.12 / rows - d * 0.1), (x - w * 0.06, y + h * 0.12 / rows)], col)
            x2 = cx - w * t
            y2 = cy + d - d * t - h * (r + 0.6) / (rows + 0.4)
            p.poly([(x2 + w * 0.06, y2 - h * 0.18 / rows), (x2 - w * 0.04, y2 - h * 0.18 / rows - d * 0.1), (x2 - w * 0.04, y2 + h * 0.12 / rows - d * 0.1), (x2 + w * 0.06, y2 + h * 0.12 / rows)], shade(col, 0.8))


def ground(p, cx, cy, w, d, c):
    p.poly(iso(cx, cy, w, d), c, outline=shade(c, 0.6), w=1)


def gen_buildings(angel=False):
    stone = (214, 204, 182) if not angel else (250, 246, 230)
    roofc = (90, 110, 120) if not angel else (240, 210, 110)
    plaza = (170, 160, 140) if not angel else (250, 240, 200)
    # 首都：ドームを戴く議事堂
    p = Pen(192)
    if angel:
        p.ell(96, 96, 92, 70, (255, 240, 170, 70))
    ground(p, 96, 132, 88, 46, plaza)
    for k in range(6):
        t = k / 5
        p.ell(30 + t * 132, 132 + (t - 0.5) * 0 + 30 * abs(t - 0.5) - 10, 7, 4, (60, 110, 50) if not angel else (255, 250, 220))
    box(p, 96, 122, 66, 34, 34, stone)
    windows(p, 96, 122, 66, 34, 34, 2, 5)
    box(p, 96, 112, 30, 16, 52, shade(stone, 1.02))
    for k in range(5):
        x = 84 + k * 6
        p.rect(x, 104 - 8 + k * 3 - 10, x + 2.4, 128 - 10 + k * 3 - 6, (240, 236, 224))
    p.ell(96, 62, 22, 9, shade(roofc, 0.8))
    p.poly([(74, 62), (76, 46), (84, 36), (96, 32), (108, 36), (116, 46), (118, 62)], roofc, outline=(40, 50, 54), w=1)
    p.poly([(96, 32), (108, 36), (116, 46), (118, 62), (100, 62)], shade(roofc, 1.2))
    p.rect(94, 20, 98, 33, (200, 180, 90))
    p.ell(96, 18, 3, 3, (230, 200, 90))
    if not angel:
        p.line([(140, 92), (140, 60)], (70, 60, 50), 1.6)
        p.poly([(140, 60), (158, 64), (140, 70)], (200, 40, 40))
    else:
        p.ell(96, 22, 18, 5, None, outline=(255, 220, 90), w=2)
    save(p.done(), "divine/divine_capital" if angel else "map/capital")
    # 都市：赤い屋根の家並みと教会の尖塔
    p = Pen(192)
    if angel:
        p.ell(96, 100, 88, 64, (255, 240, 170, 70))
    ground(p, 96, 134, 84, 44, (150, 140, 118) if not angel else (250, 240, 200))
    red = (170, 70, 50) if not angel else (240, 214, 120)
    wall = (220, 206, 176) if not angel else (252, 248, 236)
    for (x, y, w, d, h) in [(56, 120, 22, 12, 22), (136, 122, 22, 12, 20), (96, 146, 24, 13, 18), (70, 148, 16, 9, 16), (124, 146, 18, 10, 18)]:
        box(p, x, y, w, d, h, wall, roof=red, roof_h=14)
        windows(p, x, y, w, d, h, 1, 2)
    box(p, 98, 108, 18, 10, 46, shade(wall, 0.95))
    p.poly([(80, 62), (98, 52), (116, 62), (98, 72)], shade(red, 0.9))
    p.poly([(80, 62), (98, 72), (98, 18)], shade(red, 0.75), outline=(60, 30, 20), w=1)
    p.poly([(98, 72), (116, 62), (98, 18)], red, outline=(60, 30, 20), w=1)
    p.line([(98, 18), (98, 6)], (60, 50, 40), 1.6)
    p.line([(94, 10), (102, 10)], (60, 50, 40), 1.6)
    p.ell(104, 86, 4, 5, (90, 80, 70))
    save(p.done(), "divine/divine_town" if angel else "map/town")
    if angel:
        return
    # 兵営：煉瓦の長屋・練兵場・旗
    p = Pen(192)
    ground(p, 96, 132, 86, 44, (176, 160, 120))
    brick = (160, 84, 60)
    box(p, 70, 112, 46, 22, 26, brick, roof=(80, 80, 86), roof_h=14)
    windows(p, 70, 112, 46, 22, 26, 1, 4, (230, 220, 190))
    box(p, 136, 130, 26, 14, 20, brick, roof=(80, 80, 86), roof_h=12)
    for k in range(3):
        for j in range(4):
            x = 104 + k * 9 - j * 7
            y = 148 + k * 5 + j * 4
            p.ell(x, y, 2.4, 1.6, (0, 0, 0, 60))
            p.rect(x - 1.5, y - 9, x + 1.5, y, KHAKI)
            p.ell(x, y - 10, 2, 2, (110, 100, 70))
    p.line([(40, 140), (40, 84)], (70, 60, 50), 2)
    p.poly([(40, 84), (66, 90), (40, 98)], (200, 40, 40), outline=(100, 20, 20), w=0.8)
    save(p.done(), "map/barracks")
    # 農場：麦畑と農家
    p = Pen(192)
    for (cx, cy, c) in [(66, 120, (200, 176, 70)), (126, 120, (120, 150, 60)), (96, 150, (190, 160, 60)), (96, 96, (110, 140, 60))]:
        ground(p, cx, cy, 34, 18, c)
        for k in range(6):
            t = (k + 0.5) / 6
            a = (cx - 34 + 34 * t, cy - 18 * t)
            b = (cx + 34 * t, cy + 18 - 18 * t)
            p.line([(a[0] + 1, a[1] + 9), (b[0] - 1, b[1] - 9)], shade(c, 0.8), 1.2)
    box(p, 132, 98, 18, 10, 16, (226, 214, 190), roof=(150, 70, 50), roof_h=12)
    p.ell(60, 92, 9, 11, (60, 110, 50))
    p.rect(59, 98, 61, 106, (90, 60, 40))
    save(p.done(), "map/farm")
    # 塹壕陣地：ジグザグの塹壕・土嚢・鉄条網・機関銃座
    p = Pen(192)
    ground(p, 96, 128, 88, 46, (120, 104, 76))
    pts = [(14, 128), (40, 116), (60, 132), (90, 114), (112, 132), (140, 114), (160, 128), (180, 122)]
    p.line(pts, (60, 46, 32), 12)
    p.line([(x, y - 3) for x, y in pts], (90, 74, 52), 4)
    for (x, y) in pts[1:-1]:
        for k in range(3):
            p.ell(x - 6 + k * 6, y - 9, 4, 2.6, (186, 168, 120), outline=(110, 96, 66), w=0.6)
    # 鉄条網
    for k in range(12):
        x = 20 + k * 13
        y = 98 - (k % 2) * 2 + (k * 1.6)
        p.line([(x, y), (x, y - 10)], (70, 60, 50), 1.4)
        p.line([(x, y - 8), (x + 13, y - 6)], (90, 90, 90), 0.8)
        p.line([(x, y - 3), (x + 13, y - 2)], (90, 90, 90), 0.8)
    # 機関銃座
    p.ell(100, 146, 22, 11, (150, 136, 100), outline=(90, 80, 60), w=1)
    p.ell(100, 142, 14, 7, (70, 60, 44))
    p.line([(100, 140), (126, 132)], (40, 40, 40), 3)
    p.ell(102, 140, 4, 3, (50, 50, 50))
    for k in range(3):
        x = 50 + k * 14
        p.rect(x - 1.5, 124 - k * 2, x + 1.5, 132 - k * 2, FELDGRAU)
        p.ell(x, 122 - k * 2, 2.4, 2, (96, 104, 92))
    save(p.done(), "map/fort")
    # 軍港：桟橋・起重機・倉庫と、海に浮かぶ軍艦
    p = Pen(192)
    p.poly(iso(96, 136, 90, 46), (50, 90, 130), outline=(30, 60, 90), w=1)
    p.poly([(6, 136), (96, 90), (110, 98), (20, 144)], (150, 140, 120), outline=(90, 80, 60), w=1)
    box(p, 62, 104, 26, 13, 20, (180, 170, 150), roof=(110, 110, 116), roof_h=10)
    p.poly([(96, 112), (150, 140), (138, 146), (86, 120)], (130, 110, 80), outline=(80, 66, 46), w=1)
    p.line([(120, 128), (120, 66)], (60, 60, 64), 3)
    p.line([(120, 70), (152, 86)], (60, 60, 64), 2.4)
    p.line([(152, 86), (152, 106)], (40, 40, 40), 1)
    hull = (110, 116, 124)
    p.poly([(98, 160), (166, 128), (176, 134), (112, 168)], hull, outline=(40, 42, 46), w=1)
    p.rect(134, 132, 146, 142, (70, 70, 74))
    p.rect(138, 120, 142, 134, (60, 60, 64))
    save(p.done(), "map/dock")


# ---------------------------------------------------------------- 天使
def wings(p, x, y, s):
    for side in (-1, 1):
        pts = [(x, y)]
        for k in range(7):
            a = math.radians(-70 + k * 22)
            r = (30 + 10 * math.sin(k)) * s
            pts.append((x + side * (8 + abs(math.cos(a)) * r * 1.1) * s / s, y - math.sin(a) * r * 0.9 - 6 * s))
        pts.append((x + side * 6 * s, y + 20 * s))
        p.poly(pts, (250, 248, 238), outline=(200, 180, 120), w=0.8)
        for k in range(4):
            p.line([(x + side * 8 * s, y + k * 4 * s), (x + side * (30 - k * 4) * s, y - (16 - k * 7) * s)], (215, 205, 170), 0.8)


def gen_angels():
    gold = (240, 210, 110)
    white = (250, 248, 240)
    # 歩兵の天使（槍と盾）
    p = Pen(160)
    p.ell(80, 80, 72, 72, (255, 236, 150, 60))
    wings(p, 74, 72, 1.2)
    soldier(p, 74, 150, 1.12, white, "crown", gold, weapon="sword", pack=False, skin=(250, 226, 200))
    p.ell(75, 26, 14, 4, (0, 0, 0, 0), outline=(255, 214, 80), w=2.2)
    save(p.done(), "divine/divine_infantry")
    # 騎兵の天使（白馬）
    p = Pen(160)
    p.ell(80, 80, 74, 74, (255, 236, 150, 60))
    wings(p, 66, 84, 1.0)
    horse(p, 74, 150, 1.0, coat=(246, 244, 236))
    soldier(p, 70, 120, 0.72, white, "crown", gold, weapon="sword", pack=False, skin=(250, 226, 200))
    p.ell(71, 40, 10, 3, (0, 0, 0, 0), outline=(255, 214, 80), w=2)
    save(p.done(), "divine/divine_cavalry")


# ---------------------------------------------------------------- 英雄
# (名前, 国, 兵種, 制服, 帽子, 帽子の色, ひげ)
UNI = {
    "GER": (112, 118, 104), "AUH": (128, 150, 160), "OTT": (150, 132, 90), "BUL": (110, 116, 90),
    "GBR": (150, 128, 82), "FRA": (120, 140, 176), "RUS": (130, 128, 92), "SRB": (110, 112, 96),
    "BEL": (100, 96, 70), "ITA": (120, 124, 112), "ROU": (110, 120, 100), "GRE": (130, 120, 90),
    "POR": (110, 120, 120), "USA": (126, 110, 70), "NED": (110, 116, 104), "SUI": (90, 104, 110),
    "ESP": (90, 100, 110), "SWE": (60, 70, 100), "NOR": (70, 80, 100), "DEN": (70, 70, 80),
}
HEROES = [
    ("GER", "Infantry", "pickel", True, True), ("GER", "Artillery", "cap", True, False), ("GER", "Cavalry", "kalpak", True, False),
    ("GER", "Cavalry", "pilot", False, False), ("GER", "Submarine", "cap", False, False),
    ("AUH", "Infantry", "kepi", True, False), ("AUH", "Alpine", "cap", True, False), ("AUH", "Submarine", "cap", True, False),
    ("OTT", "Infantry", "kalpak", True, False), ("OTT", "Infantry", "fez", True, False), ("OTT", "Cavalry", "kalpak", True, True),
    ("BUL", "Infantry", "cap", True, False), ("BUL", "Artillery", "cap", True, False),
    ("GBR", "Infantry", "cap", True, False), ("GBR", "Infantry", "cap", True, False), ("GBR", "Cavalry", "cap", True, False),
    ("GBR", "Cavalry", "keffiyeh", False, False), ("GBR", "Destroyer", "bicorne", False, False),
    ("FRA", "Infantry", "kepi", True, False), ("FRA", "Infantry", "kepi", True, False), ("FRA", "Artillery", "kepi", True, False),
    ("FRA", "Cavalry", "pilot", False, False),
    ("RUS", "Infantry", "cap", True, True), ("RUS", "Cavalry", "kalpak", True, True), ("RUS", "Infantry", "cap", True, False),
    ("RUS", "Destroyer", "cap", False, False),
    ("SRB", "Infantry", "cap", True, False), ("SRB", "Infantry", "cap", True, False),
    ("BEL", "Infantry", "cap", True, False),
    ("ITA", "Infantry", "kepi", True, False), ("ITA", "Infantry", "alpine", True, False), ("ITA", "Cavalry", "pilot", False, False),
    ("ITA", "Destroyer", "cap", False, False),
    ("ROU", "Infantry", "cap", True, False), ("ROU", "Infantry", "cap", False, False),
    ("GRE", "Infantry", "cap", True, False),
    ("POR", "Infantry", "cap", True, False),
    ("USA", "Infantry", "cap", True, False), ("USA", "Infantry", "brodie", False, False), ("USA", "Cavalry", "pilot", False, False),
    ("SUI", "Alpine", "kepi", True, False), ("NED", "Infantry", "kepi", True, False), ("ESP", "Infantry", "crown", True, False),
    ("SWE", "Infantry", "crown", True, False), ("NOR", "Infantry", "flatcap", False, True), ("DEN", "Infantry", "crown", True, False),
]


def hero_glow(p, col):
    p.ell(64, 64, 60, 60, (255, 230, 140, 46))
    p.ell(64, 64, 58, 58, None, outline=(240, 200, 90), w=2.2)


def gen_heroes():
    for i, (nat, kind, gear, mus, beard) in enumerate(HEROES):
        p = Pen(128)
        uni = UNI[nat]
        hero_glow(p, uni)
        gold = (230, 196, 80)
        if kind == "Submarine" or kind == "Destroyer":
            p.ell(64, 98, 56, 9, (40, 80, 120, 160))
            hull = (84, 90, 96) if kind == "Submarine" else (110, 116, 124)
            if kind == "Submarine":
                p.poly([(8, 94), (22, 86), (106, 86), (122, 92), (106, 100), (20, 100)], hull, outline=(30, 32, 36), w=1)
                p.poly([(52, 86), (56, 70), (78, 70), (82, 86)], shade(hull, 0.9), outline=(30, 32, 36), w=1)
            else:
                p.poly([(6, 88), (122, 84), (114, 104), (14, 104)], hull, outline=(40, 42, 46), w=1)
                p.rect(46, 72, 84, 86, shade(hull, 1.05), outline=(40, 42, 46), w=0.8)
                for fx in (52, 64):
                    p.rect(fx, 56, fx + 7, 74, (60, 60, 64))
            # 艦橋に立つ艦長
            soldier(p, 66, 72, 0.42, (40, 44, 70), gear, (30, 34, 60), weapon="binocular", pack=False, mustache=mus, beard=beard)
            p.poly([(96, 60), (112, 64), (96, 70)], gold)
            p.line([(96, 60), (96, 86)], (60, 60, 60), 1.2)
        elif kind == "Cavalry":
            horse(p, 60, 116, 0.78, coat=(80, 50, 30) if i % 2 else (150, 110, 70))
            soldier(p, 56, 92, 0.58, uni, gear, shade(uni, 0.85), weapon="sword", pack=False, mustache=mus, beard=beard, sash=gold)
        elif kind == "Artillery":
            p.line([(60, 96), (118, 74)], (90, 96, 76), 6)
            p.ell(84, 100, 16, 16, None, outline=(60, 44, 30), w=3)
            soldier(p, 40, 120, 0.82, uni, gear, shade(uni, 0.85), weapon="binocular", pack=False, mustache=mus, beard=beard, coat=True, sash=gold)
        else:
            weapon = "alpen" if kind == "Alpine" else ("baton" if mus else "rifle")
            soldier(p, 62, 122, 0.92, uni, gear, shade(uni, 0.85), weapon=weapon, pack=not mus, mustache=mus, beard=beard, coat=mus, sash=gold if mus else None)
        save(p.done(), "heroes/hero_%02d" % i)


def gen_icon():
    """アプリのアイコン（鉄兜と月桂冠）"""
    p = Pen(512)
    p.ell(256, 256, 250, 250, (60, 44, 30), outline=(200, 160, 70), w=12)
    p.ell(256, 256, 226, 226, (110, 30, 26))
    for side in (-1, 1):
        for k in range(9):
            a = math.radians(200 + k * 16) if side < 0 else math.radians(-20 - k * 16)
            x = 256 + math.cos(a) * 170 * side * side
            y = 290 - math.sin(a) * 150
            p.ell(x, y, 22, 11, (210, 180, 80), outline=(140, 110, 40), w=2)
    p.ell(256, 290, 150, 36, (90, 96, 86), outline=(40, 40, 36), w=4)
    p.poly([(140, 290), (150, 200), (200, 150), (256, 136), (312, 150), (362, 200), (372, 290)], (110, 118, 104), outline=(40, 40, 36), w=4)
    p.poly([(256, 136), (312, 150), (362, 200), (372, 290), (290, 290)], (130, 140, 124))
    im = p.done()
    return im


def main():
    gen_units()
    gen_buildings()
    gen_buildings(angel=True)
    gen_angels()
    gen_heroes()
    icon = gen_icon()
    web = os.path.join(ROOT, "..", "web")
    icon.save(os.path.join(web, "icon-512.png"))
    icon.resize((192, 192), Image.LANCZOS).save(os.path.join(web, "icon-192.png"))
    bg = Image.new("RGBA", (180, 180), (18, 15, 12, 255))
    bg.alpha_composite(icon.resize((180, 180), Image.LANCZOS))
    bg.convert("RGB").save(os.path.join(web, "apple-touch-icon.png"))
    icon.save(os.path.join(ROOT, "app_icon.ico"), sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
    print("heroes", len(HEROES))


if __name__ == "__main__":
    main()
