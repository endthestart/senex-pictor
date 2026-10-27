"""Display-only previews: as-shot white balance and a fixed asinh stretch (not a rendering)."""
import numpy as np
from PIL import Image


def stretch(rgb, wb, black, scale, strength=40.0):
    """Camera RGB × as-shot WB, minus `black`, asinh-stretched so `scale` maps near white."""
    x = (rgb * np.asarray(wb, np.float32)[None, None, :] - black) / scale
    y = np.arcsinh(strength * np.clip(x, 0, None)) / np.arcsinh(strength)
    return (np.clip(y, 0, 1) * 255 + 0.5).astype(np.uint8)


def gray(values, lo, hi):
    y = np.clip((np.asarray(values, np.float32) - lo) / (hi - lo), 0, 1)
    return (y * 255 + 0.5).astype(np.uint8)


def overlay(image8, mask, color=(255, 64, 32), alpha=0.45):
    out = image8.astype(np.float32).copy()
    out[mask] = (1 - alpha) * out[mask] + alpha * np.asarray(color, np.float32)
    return out.astype(np.uint8)


def save(path, array8, max_side=None, quality=90):
    image = Image.fromarray(array8)
    if max_side and max(image.size) > max_side:
        image.thumbnail((max_side, max_side), Image.LANCZOS)
    image.save(path, quality=quality)


def crop(array8, x0, y0, w, h):
    return array8[y0:y0 + h, x0:x0 + w]
