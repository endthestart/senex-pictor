"""Separate camera-fixed sky glow (atmosphere, light pollution) from sky-fixed light.

Frame i at camera pixel x sees G_i(x) + S(sky position). Block medians of clear sky (stars
suppressed) are fitted by alternation: S is the robust mean of frames with their glow removed
and warped to the reference; each G_i is the smoothed remainder in camera coordinates.
Stacking then uses F_i(x_i(q)) − G_i(x_i(q)) + G_ref(q). Components that look the same
at every rotation angle cannot be told apart, and they cancel in that correction.
"""
from concurrent.futures import ThreadPoolExecutor

import cv2
import numpy as np
from scipy import ndimage

from proto.stack import nan_median

BLOCK = 32


def block_medians(frames, alpha, flat, workers=16, clear=0.98, fill=0.9):
    """Per-frame median RGB of clear-sky pixels in BLOCK×BLOCK cells, each frame divided by
    `flat` (vignetting); invalid cells NaN."""
    n, h, w, _ = frames.shape
    gh, gw = h // BLOCK, w // BLOCK
    cells = lambda a: a[:gh * BLOCK, :gw * BLOCK].reshape(gh, BLOCK, gw, BLOCK, *a.shape[2:]).swapaxes(1, 2)
    sky = cells(alpha >= clear).reshape(gh, gw, -1)
    usable = sky.mean(axis=2) >= fill
    out = np.full((n, gh, gw, 3), np.nan, np.float32)

    def one(i):
        values = cells(np.asarray(frames[i], np.float32) / flat[..., None]).reshape(gh, gw, -1, 3)
        values = np.where(sky[..., None], values, np.nan)
        centre, _ = nan_median(np.moveaxis(values, 2, 0))
        out[i] = np.where(usable[..., None], centre, np.nan)

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(one, range(n)))
    return out, usable


def centers(gh, gw):
    yy, xx = np.mgrid[0:gh, 0:gw]
    return np.stack([xx * BLOCK + (BLOCK - 1) / 2, yy * BLOCK + (BLOCK - 1) / 2], axis=-1).astype(np.float64)


def map_between(model, points, turn):
    """Full-resolution positions of `points` after the sky rotation `turn` (3×3)."""
    return model.camera.pixels(model.camera.rays(points) @ turn.T)


def sample(grid, points):
    """Bilinear sample of a (gh, gw, 3) cell grid at full-resolution positions; positions within
    one cell of the grid edge take the edge value, farther ones are NaN."""
    gx = (points[..., 0] - (BLOCK - 1) / 2) / BLOCK
    gy = (points[..., 1] - (BLOCK - 1) / 2) / BLOCK
    valid = np.isfinite(gx) & np.isfinite(gy) & (gx >= -1) & (gy >= -1) & (gx <= grid.shape[1]) & (gy <= grid.shape[0])
    gx, gy = np.clip(gx, 0, grid.shape[1] - 1), np.clip(gy, 0, grid.shape[0] - 1)
    filled = np.nan_to_num(grid)
    weight = np.isfinite(grid[..., 0]).astype(np.float32)
    mx, my = np.where(valid, gx, -1).astype(np.float32), np.where(valid, gy, -1).astype(np.float32)
    value = cv2.remap(filled * weight[..., None], mx, my, cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=0)
    norm = cv2.remap(weight, mx, my, cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=0)
    with np.errstate(invalid="ignore", divide="ignore"):
        return np.where((norm > 0.5)[..., None], value / norm[..., None], np.nan)


def smooth(grid, sigma=2.0):
    """Normalized Gaussian smoothing; cells far from data take the nearest smoothed value."""
    valid = np.isfinite(grid[..., 0])
    weight = ndimage.gaussian_filter(valid.astype(np.float64), sigma)
    out = np.stack([ndimage.gaussian_filter(np.where(valid, grid[..., c], 0.0), sigma) for c in range(3)], -1)
    out = out / np.maximum(weight, 1e-9)[..., None]
    known = weight > 0.05
    _, (iy, ix) = ndimage.distance_transform_edt(~known, return_indices=True)
    return out[iy, ix].astype(np.float32)


def decompose(medians, model, iterations=4):
    """Returns per-frame camera glow G (N, gh, gw, 3), sky light S on the reference grid and
    the RMS change of G at each iteration."""
    n, gh, gw, _ = medians.shape
    grid = centers(gh, gw)
    to_frame = [map_between(model, grid, model.rotation(i)) for i in range(n)]
    to_reference = [map_between(model, grid, model.rotation(i).T) for i in range(n)]
    glow = np.zeros_like(medians)
    changes = []
    for _ in range(iterations):
        warped = np.stack([sample(medians[i] - glow[i], to_frame[i]) for i in range(n)])
        sky, _ = nan_median(warped)
        previous = glow.copy()
        for i in range(n):
            glow[i] = smooth(medians[i] - sample(sky, to_reference[i]))
        changes.append(float(np.sqrt(np.nanmean((glow - previous) ** 2))))
    return glow, sky, changes
