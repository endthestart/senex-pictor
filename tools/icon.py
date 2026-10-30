"""Draw Pictor's icon from the icon template in docs/DESIGN-TOKENS.md and build the .icns.

usage: .venv/bin/python tools/icon.py app/assets/icon
"""
import argparse
import math
import random
import subprocess
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

S = 1024
PAD = 100
ACCENT = (143, 180, 255)
# Pictor, the Painter's Easel: J2000 RA and Dec in degrees, V magnitude.
STARS = {
    "alpha": (102.048, -61.941, 3.27),
    "beta": (86.821, -51.066, 3.86),
    "gamma": (87.457, -56.166, 4.50),
}
FIGURE = ["alpha", "gamma", "beta"]
SEEDS = (7, 3)


def layer():
    return Image.new("RGBA", (S, S), (0, 0, 0, 0))


def gradient():
    image = Image.new("RGBA", (S, S))
    for y in range(S):
        t = y / S
        colour = (int(10 + 14 * t), int(16 + 22 * t), int(36 + 42 * t), 255)
        image.paste(colour, (0, y, S, y + 1))
    return image


def milky_way(image):
    band = layer()
    draw = ImageDraw.Draw(band)
    for k in range(60):
        t = k / 60
        cx = PAD + 80 + t * (S - 2 * PAD - 160)
        cy = 0.62 * S - t * 0.38 * S
        draw.ellipse([cx - 120, cy - 60, cx + 120, cy + 60], fill=(150, 165, 230, 10))
    return Image.alpha_composite(image, band.filter(ImageFilter.GaussianBlur(40)))


def dots(image, seed):
    draw = ImageDraw.Draw(image)
    rng = random.Random(seed)
    for _ in range(140):
        x = rng.uniform(PAD, S - PAD)
        y = rng.uniform(PAD, 0.72 * S)
        r = rng.choice([1.2, 1.6, 2.2, 2.8])
        a = rng.randint(120, 255)
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(220, 228, 255, a))


def placed():
    ras = [v[0] for v in STARS.values()]
    decs = [v[1] for v in STARS.values()]
    ra0, dec0 = (min(ras) + max(ras)) / 2, (min(decs) + max(decs)) / 2
    c = math.cos(math.radians(dec0))
    sky = {k: (-(v[0] - ra0) * c, -(v[1] - dec0)) for k, v in STARS.items()}
    xs, ys = [p[0] for p in sky.values()], [p[1] for p in sky.values()]
    scale = 0.34 * S / max(max(xs) - min(xs), max(ys) - min(ys))
    return {k: (0.5 * S + p[0] * scale, 0.36 * S + p[1] * scale) for k, p in sky.items()}


def lines(image, at):
    figure = layer()
    draw = ImageDraw.Draw(figure)
    colour = ACCENT + (150,)
    for a, b in zip(FIGURE, FIGURE[1:]):
        draw.line([at[a], at[b]], fill=colour, width=5)
    for x, y in (at[k] for k in FIGURE):
        draw.ellipse([x - 2.5, y - 2.5, x + 2.5, y + 2.5], fill=colour)
    return Image.alpha_composite(image, figure)


def sparkles(image, at):
    for name, (x, y) in at.items():
        r = min(64, max(28, 64 - 16 * (STARS[name][2] - 2.6)))
        halo = layer()
        ImageDraw.Draw(halo).ellipse([x - r, y - r, x + r, y + r], fill=ACCENT + (120,))
        image = Image.alpha_composite(image, halo.filter(ImageFilter.GaussianBlur(0.5 * r)))
        points = []
        for k in range(8):
            radius = r if k % 2 == 0 else 0.22 * r
            angle = math.radians(k * 45 - 90)
            points.append((x + radius * math.cos(angle), y + radius * math.sin(angle)))
        ImageDraw.Draw(image).polygon(points, fill=(232, 240, 255, 255))
    return image


def treeline(image, seed):
    draw = ImageDraw.Draw(image)
    rng = random.Random(seed)
    base = 0.78 * S
    x = PAD
    while x < S - PAD:
        w = rng.uniform(26, 52)
        h = rng.uniform(60, 190)
        draw.polygon([(x, base), (x + w / 2, base - h), (x + w, base)], fill=(6, 9, 14, 255))
        x += 0.8 * w
    draw.rectangle([PAD, base, S - PAD, S - PAD], fill=(6, 9, 14, 255))


def icon():
    image = milky_way(gradient())
    dots(image, SEEDS[0])
    at = placed()
    image = sparkles(lines(image, at), at)
    treeline(image, SEEDS[1])
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([PAD, PAD, S - PAD, S - PAD], radius=185, fill=255)
    out = layer()
    out.paste(image, (0, 0), mask)
    return out


def icns(png, target):
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "pictor.iconset"
        iconset.mkdir()
        for size in (16, 32, 128, 256, 512):
            for scale, suffix in ((1, ""), (2, "@2x")):
                px = str(size * scale)
                out = iconset / f"icon_{size}x{size}{suffix}.png"
                subprocess.run(["sips", "-z", px, px, str(png), "--out", str(out)], check=True, capture_output=True)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(target)], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folder", type=Path)
    folder = parser.parse_args().folder
    png = folder / "pictor.png"
    icon().save(png)
    icns(png, folder / "pictor.icns")
    print(f"wrote {png}")


if __name__ == "__main__":
    main()
