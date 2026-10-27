"""Iteration 3: stack raw CFA samples directly (Bayer drizzle).

For each output pixel and frame, the sky model gives the sensor position that saw it. Each
CFA site (R, G1, G2, B) contributes its nearest sample of that site's colour, weighted by a
Gaussian of the distance. Sky rotation dithers the sub-pixel phase from frame to frame, so
every colour is measured at every output pixel and nothing is demosaiced or resampled.
"""
import json
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor

import numpy as np
import rawpy

from proto import occlusion
from proto.background import sample as sample_glow
from proto.frames import capture_times
from proto.stack import CLEAR_SKY, clearance, nan_median

KERNEL = (0.5, 1.0)  # Gaussian σ and cut-off radius in sensor pixels
CLEARANCE_PX = 1.5
CENTRE_WEIGHT = 0.3


def _read(job):
    index, path, cache, shape, crop = job
    with rawpy.imread(str(path)) as raw:
        window = (slice(crop["y"], crop["y"] + crop["height"]), slice(crop["x"], crop["x"] + crop["width"]))
        data = raw.raw_image[window]
        colors = raw.raw_colors[window][:2, :2]
        site_black = np.asarray(raw.black_level_per_channel)[colors].tolist()
        out = np.memmap(cache / "cfa.u16", np.uint16, "r+", shape=shape)
        out[index] = data
        out.flush()
        return {"colors": np.where(colors == 3, 1, colors).tolist(), "black": site_black, "white": int(raw.white_level)}


def load(paths, cache, crop, workers=12):
    """uint16 CFA cache of the default crop with per-frame site blacks and the 2×2 colour layout.
    `crop` is in full-raw coordinates (active-area origin plus default crop), not LibRaw's
    visible area, which some cameras offset (D700: 2 columns)."""
    manifest = cache / "cfa.json"
    names = [p.name for p in paths]
    if not manifest.exists():
        shape = (len(paths), crop["height"], crop["width"])
        np.memmap(cache / "cfa.u16", np.uint16, "w+", shape=shape).flush()
        with ProcessPoolExecutor(workers) as pool:
            sites = list(pool.map(_read, [(i, p, cache, shape, crop) for i, p in enumerate(paths)]))
        if any(s["colors"] != sites[0]["colors"] for s in sites):
            raise SystemExit("CFA layout changes within the sequence")
        meta = {"names": names, "shape": list(shape), "colors": sites[0]["colors"], "times_s": capture_times(paths),
                "black": [s["black"] for s in sites], "white": [s["white"] for s in sites]}
        manifest.write_text(json.dumps(meta))
    meta = json.loads(manifest.read_text())
    if meta["names"] != names:
        raise SystemExit(f"CFA cache {cache} holds a different frame list")
    return np.memmap(cache / "cfa.u16", np.uint16, "r", shape=tuple(meta["shape"])), meta


def _frame_samples(cfa, meta, model, rays, alpha, clear_px, glow, glow_reference, occluded, i, kernel, flat):
    """Per site: value (ADU, glow-corrected) and weight of the nearest same-site sample, or
    weight 0 where the sample is outside the frame, too far, not clear sky, occluded in this
    frame or saturated."""
    h, w = alpha.shape
    sigma, radius = kernel
    p = model.to_image(model.camera.pixels(rays @ model.rotation(i).T))
    px, py = p[..., 0], p[..., 1]
    finite = np.isfinite(px) & np.isfinite(py)
    if not finite.any():
        return None
    glow_here = sample_glow(glow[i], np.where(finite[..., None], p, -1e9)) - glow_reference
    x0, x1 = max(0, int(np.nanmin(px)) - 2), min(w, int(np.nanmax(px)) + 3)
    y0, y1 = max(0, int(np.nanmin(py)) - 2), min(h, int(np.nanmax(py)) + 3)
    if x0 >= x1 or y0 >= y1:
        return None
    covered = occlusion.patch(occluded, i, y0, y1, x0, x1)
    values, weights, clipped = [], [], []
    for dy in range(2):
        for dx in range(2):
            sx = dx + 2 * np.rint((np.nan_to_num(px, nan=-9) - dx) / 2)
            sy = dy + 2 * np.rint((np.nan_to_num(py, nan=-9) - dy) / 2)
            d2 = (sx - px) ** 2 + (sy - py) ** 2
            ok = finite & (sx >= 0) & (sy >= 0) & (sx < w) & (sy < h) & (d2 <= radius ** 2)
            ix, iy = np.where(ok, sx, 0).astype(np.intp), np.where(ok, sy, 0).astype(np.intp)
            ok &= (alpha[iy, ix] >= CLEAR_SKY) & (clear_px[iy, ix] >= CLEARANCE_PX)
            ok &= ~covered[np.clip(iy - y0, 0, y1 - y0 - 1), np.clip(ix - x0, 0, x1 - x0 - 1)]
            raw = cfa[i][iy, ix]
            colour = meta["colors"][dy][dx]
            value = (raw.astype(np.float32) - meta["black"][i][dy][dx]) / flat[iy, ix] - glow_here[..., colour]
            saturated = ok & (raw >= meta["white"][i])
            usable = ok & ~saturated & np.isfinite(value)
            values.append(np.where(usable, value, np.nan))
            weights.append(np.where(usable, np.exp(-d2 / (2 * sigma ** 2)), 0.0).astype(np.float32))
            clipped.append(saturated)
    return np.stack(values), np.stack(weights), np.stack(clipped)


def _combine(values, weights, frame_index, k=4.0, iterations=2):
    """Weighted mean of one colour's samples (M, th, tw) after σ-clipping about the median of
    the near samples; also even/odd-frame halves and kept counts."""
    near = np.where(weights >= CENTRE_WEIGHT, values, np.nan)
    keep = np.isfinite(values)
    for _ in range(iterations):
        centre, count = nan_median(np.where(keep, near, np.nan))
        spread, _ = nan_median(np.abs(np.where(keep, near, np.nan) - centre))
        clip = (count >= 3)[None] & (np.abs(values - centre) > k * np.maximum(1.4826 * spread, 1e-3))
        keep &= ~clip
    w = np.where(keep, weights, 0.0)
    v = np.nan_to_num(values)
    with np.errstate(invalid="ignore", divide="ignore"):
        mean = (w * v).sum(axis=0) / w.sum(axis=0)
        halves = [(np.where(frame_index % 2 == parity, w, 0) * v).sum(axis=0)
                  / np.where(frame_index % 2 == parity, w, 0).sum(axis=0) for parity in range(2)]
    return mean, keep.sum(axis=0), np.stack(halves)


def _combine_rows(values, weights, frame_index, workers, rows=16):
    """_combine over row strips in parallel (the sorts release the GIL)."""
    h = values.shape[1]
    with ThreadPoolExecutor(workers) as pool:
        parts = list(pool.map(lambda y: _combine(values[:, y:y + rows], weights[:, y:y + rows], frame_index),
                              range(0, h, rows)))
    return (np.concatenate([p[0] for p in parts]), np.concatenate([p[1] for p in parts]),
            np.concatenate([p[2] for p in parts], axis=1))


def stack_tile(cfa, meta, model, alpha, clear_px, glow, reference, occluded, x0, y0, x1, y1, flat, kernel=KERNEL,
               workers=16):
    n = cfa.shape[0]
    yy, xx = np.mgrid[y0:y1, x0:x1]
    points = np.stack([xx, yy], axis=-1).astype(np.float64)
    rays = model.camera.rays(model.to_model(points))
    glow_reference = sample_glow(glow[reference], points)
    values = np.full((n, 4, y1 - y0, x1 - x0), np.nan, np.float32)
    weights = np.zeros((n, 4, y1 - y0, x1 - x0), np.float32)
    clipped = np.zeros((y1 - y0, x1 - x0), bool)

    def one(i):
        result = _frame_samples(cfa, meta, model, rays, alpha, clear_px, glow, glow_reference, occluded, i, kernel, flat)
        if result is not None:
            values[i], weights[i], c = result
            return c.any(axis=0)
        return np.zeros_like(clipped)

    with ThreadPoolExecutor(workers) as pool:
        for c in pool.map(one, range(n)):
            clipped |= c
    sites = np.asarray(meta["colors"]).ravel()
    out, kept, halves = [], [], []
    for colour in range(3):
        chosen = np.nonzero(sites == colour)[0]
        v = values[:, chosen].reshape(-1, y1 - y0, x1 - x0)
        w = weights[:, chosen].reshape(-1, y1 - y0, x1 - x0)
        frame_index = np.repeat(np.arange(n), len(chosen))[:, None, None]
        m, k, hv = _combine_rows(v, w, frame_index, workers)
        out.append(m), kept.append(k), halves.append(hv)
    return np.stack(out, -1), np.stack(kept, -1), np.stack(halves, -1), clipped


def stack_tiles(cfa, meta, model, alpha, glow, reference, occluded, flat, kernel=KERNEL, tile=256):
    """Sky stack (H, W, 3) in camera RGB ADU, kept samples per colour, even/odd halves and the
    pixels where some frame's sample was saturated. `flat` (H, W) divides each sample at its
    sensor position (vignetting); the glow must then come from frames divided the same way."""
    h, w = alpha.shape
    clear_px = clearance(alpha)
    mean = np.full((h, w, 3), np.nan, np.float32)
    kept = np.zeros((h, w, 3), np.int32)
    halves = np.full((2, h, w, 3), np.nan, np.float32)
    clipped = np.zeros((h, w), bool)
    for y0 in range(0, h, tile):
        for x0 in range(0, w, tile):
            y1, x1 = min(h, y0 + tile), min(w, x0 + tile)
            if (alpha[y0:y1, x0:x1] <= 0).all():
                continue
            m, k, hv, c = stack_tile(cfa, meta, model, alpha, clear_px, glow, reference, occluded, x0, y0, x1, y1, flat, kernel)
            mean[y0:y1, x0:x1], kept[y0:y1, x0:x1], clipped[y0:y1, x0:x1] = m, k, c
            halves[:, y0:y1, x0:x1] = hv
    return mean, kept, halves, clipped
