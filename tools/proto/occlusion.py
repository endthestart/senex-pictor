"""Per-frame occlusion of sky pixels.

The matte says where sky is visible over the whole sequence; foreground that moves (swaying
leaves, a branch in the wind) or dark cloud can still cover a sky pixel in single frames. In
such a frame the pixel is far darker than its own glow-corrected sky history: the 5×5-smoothed
luminance falls more than K robust σ below that pixel's temporal median. Local light that is
not sky (a lamp lighting mist or leaves near the trees) is the mirror case: smoothed luminance
far above the history over a region larger than any star (opening by BRIGHT_SIZE px removes
star-sized blobs). Flagged pixels are dilated by DILATE px (the resampling kernel's reach)
and kept as one bit per frame and pixel.
"""
from concurrent.futures import ThreadPoolExecutor

import numpy as np
from scipy import ndimage

from proto.background import sample as sample_glow

SMOOTH = 5
K = 5.0
BRIGHT_K = 8.0
BRIGHT_SIZE = 15
DILATE = 4
HALO = SMOOTH // 2 + DILATE + BRIGHT_SIZE


def build(frames, glow, alpha, path, flat, rows=32, workers=16, min_alpha=0.5):
    """Writes packed occlusion bits (N, H, ceil(W/8)) to `path`; returns per-frame occluded
    shares of sky pixels (alpha ≥ min_alpha)."""
    n, h, w, _ = frames.shape
    packed = np.memmap(path, np.uint8, "w+", shape=(n, h, -(-w // 8)))
    sky_pixels = np.zeros(n)
    sky = alpha >= min_alpha

    def strip(y0):
        y1 = min(h, y0 + rows)
        ya, yb = max(0, y0 - HALO), min(h, y1 + HALO)
        yy, xx = np.mgrid[ya:yb, 0:w]
        points = np.stack([xx, yy], axis=-1).astype(np.float64)
        lum = np.asarray(frames[:, ya:yb], np.float32).sum(axis=3)
        values = ndimage.uniform_filter(lum / flat[ya:yb][None], size=(1, SMOOTH, SMOOTH), mode="nearest")
        values -= np.stack([sample_glow(glow[i], points).sum(axis=2) for i in range(n)])
        centre = np.median(values, axis=0)
        sigma = 1.4826 * np.median(np.abs(values - centre), axis=0)
        dark = (values < centre - K * np.maximum(sigma, 1e-3)) & sky[ya:yb][None]
        bright = values > centre + BRIGHT_K * np.maximum(sigma, 1e-3)
        bright = ndimage.binary_opening(bright, structure=np.ones((1, BRIGHT_SIZE, BRIGHT_SIZE), bool)) & sky[ya:yb][None]
        dark = ndimage.binary_dilation(dark | bright, structure=np.ones((1, 2 * DILATE + 1, 2 * DILATE + 1), bool))
        core = slice(y0 - ya, y1 - ya)
        packed[:, y0:y1] = np.packbits(dark[:, core], axis=2)
        return dark[:, core].sum(axis=(1, 2))

    with ThreadPoolExecutor(workers) as pool:
        for counts in pool.map(strip, range(0, h, rows)):
            sky_pixels += counts
    packed.flush()
    return sky_pixels / max(int(sky.sum()), 1)


def load(path, shape):
    n, h, w = shape
    return np.memmap(path, np.uint8, "r", shape=(n, h, -(-w // 8)))


def patch(packed, i, y0, y1, x0, x1):
    """Occlusion bits of frame i for rows y0:y1 and columns x0:x1 as a boolean array."""
    b0, b1 = x0 // 8, -(-x1 // 8)
    bits = np.unpackbits(np.asarray(packed[i, y0:y1, b0:b1]), axis=1)
    return bits[:, x0 - 8 * b0:x1 - 8 * b0].astype(bool)
