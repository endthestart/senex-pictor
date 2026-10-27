"""Per-pixel temporal statistics and star transits that move with the sky (ADR 0008 model).

A > 6σ excess at a camera pixel is a star transit only when the same sky point, looked up
in a frame where the sky has moved it at least MIN_SHIFT px, shows a ≥ 3σ peak there while
the original pixel has dropped below half that peak. Sky glimpses through swaying leaves,
lights and aircraft do not follow the sky model and fail; faint stars below the detection
threshold pass. Near the celestial pole the sky barely moves and nothing can be confirmed.
"""
from concurrent.futures import ThreadPoolExecutor

import numpy as np
from scipy import ndimage

from proto.sky_model import rotation

TRANSIT_SIGMA = 6.0
TREND_BLOCK = 64
PARTNER_SIGMA = 3.0
MIN_SHIFT = 2.5
STEPS = (1, 2, 4, 8, 16, 32)
CELL = 16
HALO = 8


def sky_velocity(model, shape, cell=CELL, delta=1e-4):
    """Image-coordinate displacement per radian of sky rotation, on a coarse grid of cells."""
    h, w = shape
    gy, gx = np.mgrid[cell // 2:h + cell // 2:cell, cell // 2:w + cell // 2:cell]
    p = np.stack([gx, gy], axis=-1).astype(np.float64)
    q = model.to_image(model.camera.pixels(model.camera.rays(model.to_model(p)) @ rotation(model.axis, -delta)))
    return ((q - p) / delta).astype(np.float32)


def frame_steps(velocity, angles):
    """Per cell, the smallest frame step that moves the sky at least MIN_SHIFT px (0: none does)."""
    per_frame = np.linalg.norm(velocity, axis=-1) * float(np.median(np.abs(np.diff(angles))))
    steps = np.zeros(per_frame.shape, np.int32)
    for k in reversed(STEPS):
        steps[k * per_frame >= MIN_SHIFT] = k
    return steps


def _coherent(score, peak, angles, velocity, steps, y0, y1, ya):
    """Events in rows y0:y1 whose sky point peaks where the model moved it, in either direction."""
    n, _, w = score.shape
    rows = np.arange(y0, y1)
    cells = velocity[rows // CELL][:, np.arange(w) // CELL]
    step = steps[rows // CELL][:, np.arange(w) // CELL]
    yy, xx = np.meshgrid(rows, np.arange(w), indexing="ij")
    frame = np.arange(n, dtype=np.int32)[:, None, None]
    coherent = np.zeros((n, y1 - y0, w), bool)
    for sign in (1, -1):
        other = frame + sign * step[None]
        valid = (step[None] > 0) & (other >= 0) & (other < n)
        other = np.clip(other, 0, n - 1)
        turn = (angles[other] - angles[frame]).astype(np.float32)
        ty = np.rint(yy + turn * cells[..., 1]).astype(np.int32) - ya
        tx = np.rint(xx + turn * cells[..., 0]).astype(np.int32)
        valid &= (ty >= 0) & (ty < score.shape[1]) & (tx >= 0) & (tx < w)
        there = peak[other, np.clip(ty, 0, score.shape[1] - 1), np.clip(tx, 0, w - 1)]
        here = score[other, yy - ya, xx]
        coherent |= valid & (there >= PARTNER_SIGMA) & (here < 0.5 * there)
    return coherent


def frame_trends(frames, workers=16):
    """Per frame, block medians of luminance (TREND_BLOCK px): the large-scale brightness that
    twilight, aurora or passing cloud change from frame to frame."""
    n, h, w, _ = frames.shape
    gh, gw = h // TREND_BLOCK, w // TREND_BLOCK

    def one(i):
        lum = np.asarray(frames[i, :gh * TREND_BLOCK, :gw * TREND_BLOCK], np.float32).sum(axis=2)
        return np.median(lum.reshape(gh, TREND_BLOCK, gw, TREND_BLOCK).swapaxes(1, 2).reshape(gh, gw, -1), axis=2)

    with ThreadPoolExecutor(workers) as pool:
        return np.stack(list(pool.map(one, range(n))))


def _trend_rows(trends, y0, y1, w):
    """Bilinear interpolation of block trends (N, gh, gw) onto rows y0:y1 of width w."""
    _, gh, gw = trends.shape
    gy = np.clip((np.arange(y0, y1) + 0.5) / TREND_BLOCK - 0.5, 0, gh - 1)
    gx = np.clip((np.arange(w) + 0.5) / TREND_BLOCK - 0.5, 0, gw - 1)
    y_lo, x_lo = np.floor(gy).astype(int), np.floor(gx).astype(int)
    y_hi, x_hi = np.minimum(y_lo + 1, gh - 1), np.minimum(x_lo + 1, gw - 1)
    fy, fx = (gy - y_lo)[None, :, None], (gx - x_lo)[None, None, :]
    rows_lo = trends[:, y_lo][:, :, x_lo] * (1 - fx) + trends[:, y_lo][:, :, x_hi] * fx
    rows_hi = trends[:, y_hi][:, :, x_lo] * (1 - fx) + trends[:, y_hi][:, :, x_hi] * fx
    return (rows_lo * (1 - fy) + rows_hi * fy).astype(np.float32)


def temporal_stats(frames, model, rows=16, workers=16):
    """Per camera pixel: median and 20th-percentile luminance; then, after removing each
    frame's large-scale brightness change (frame_trends), robust temporal noise, temporal
    scatter, the fraction of frames with a > 6σ excess (`excess`) and the transit fraction
    (`transit`): coherent events, kept only where at least half of the pixel's events cohere."""
    n, h, w, _ = frames.shape
    trends = frame_trends(frames)
    angles = np.asarray(model.angles, np.float64)
    velocity = sky_velocity(model, (h, w))
    steps = frame_steps(velocity, angles)
    out = {k: np.zeros((h, w), np.float32) for k in ("median", "low", "sigma", "excess", "transit", "scatter")}

    def strip(y0):
        y1 = min(h, y0 + rows)
        ya, yb = max(0, y0 - HALO), min(h, y1 + HALO)
        lum = np.asarray(frames[:, ya:yb], np.float32).sum(axis=3)
        trend = _trend_rows(trends, ya, yb, w)
        steady = lum - trend + np.median(trend, axis=0)
        m = np.median(steady, axis=0)
        s = 1.4826 * np.median(np.abs(steady - m), axis=0)
        score = (steady - m) / np.maximum(s, 1e-3)
        peak = ndimage.maximum_filter(score, size=(1, 3, 3))
        core = slice(y0 - ya, y1 - ya)
        events = score[:, core] > TRANSIT_SIGMA
        coherent = events & _coherent(score, peak, angles, velocity, steps, y0, y1, ya)
        hits, total = coherent.sum(axis=0), events.sum(axis=0)
        out["median"][y0:y1] = np.median(lum[:, core], axis=0)
        out["low"][y0:y1] = np.percentile(lum[:, core], 20, axis=0)
        out["sigma"][y0:y1] = s[core]
        out["excess"][y0:y1] = total / n
        out["transit"][y0:y1] = np.where(2 * hits >= total, hits / n, 0.0)
        out["scatter"][y0:y1] = steady[:, core].std(axis=0)

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(strip, range(0, h, rows)))
    full = np.arange(h) // CELL, np.arange(w) // CELL
    out["resolvable"] = steps[full[0]][:, full[1]] > 0
    return out
