"""Sky warping, robust stacking with coverage/rejection maps, ground stack and composite."""
from concurrent.futures import ThreadPoolExecutor

import cv2
import numpy as np

CLEAR_SKY = 0.98
# Lanczos-4 reaches 4 px; samples nearer than this to non-clear pixels mix in tree pixels
# (measured: 15–66× more dark rejections within 4 px of tree edges, none beyond).
CLEARANCE_PX = 5.0


def clearance(alpha, min_obstruction=64):
    """Distance in pixels to the nearest obstruction: a non-clear region of at least
    `min_obstruction` pixels (isolated noise pixels in clear sky are not obstructions) or the
    frame border, beyond which a resampling kernel has no data."""
    from scipy import ndimage
    from skimage import morphology
    obstruction = morphology.remove_small_objects(alpha < CLEAR_SKY, max_size=min_obstruction - 1)
    h, w = alpha.shape
    yy, xx = np.ogrid[:h, :w]
    border = np.minimum(np.minimum(yy, h - 1 - yy), np.minimum(xx, w - 1 - xx)) + 1
    return np.minimum(ndimage.distance_transform_edt(~obstruction), border).astype(np.float32)


def warp_all(frames, saturated, model, alpha, workers=16):
    """Every frame warped once into the reference sky; samples whose source pixel is not
    clear sky are NaN. Returns aligned (N, H, W, 3) and saturated-source flags."""
    n, h, w, _ = frames.shape
    rays = model.reference_rays((h, w))
    aligned = np.empty((n, h, w, 3), np.float32)
    clipped = np.zeros((n, h, w), bool)

    def one(i):
        mx, my = model.source_map(i, rays)
        inside = np.isfinite(mx) & np.isfinite(my) & (mx >= 0) & (my >= 0) & (mx <= w - 1) & (my <= h - 1)
        mx, my = np.where(inside, mx, -1).astype(np.float32), np.where(inside, my, -1).astype(np.float32)
        image = cv2.remap(np.asarray(frames[i]), mx, my, cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=0)
        source_alpha = cv2.remap(alpha, mx, my, cv2.INTER_LINEAR, borderMode=cv2.BORDER_CONSTANT, borderValue=0)
        image[~inside | (source_alpha < CLEAR_SKY)] = np.nan
        aligned[i] = image
        clipped[i] = cv2.remap(np.asarray(saturated[i]), mx, my, cv2.INTER_NEAREST,
                               borderMode=cv2.BORDER_CONSTANT, borderValue=0).astype(bool)

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(one, range(n)))
    return aligned, clipped


def nan_median(values):
    """Median along axis 0 ignoring NaN; NaN where nothing is valid."""
    ordered = np.sort(values, axis=0)
    count = np.isfinite(values).sum(axis=0)
    lo = np.take_along_axis(ordered, np.maximum((count - 1) // 2, 0)[None], axis=0)[0]
    hi = np.take_along_axis(ordered, np.maximum(count // 2, 0)[None], axis=0)[0]
    return np.where(count > 0, (lo + hi) / 2, np.nan), count


def combine(samples, k=4.0, iterations=2, rows=32, workers=16):
    """Luminance sigma clipping about the median (σ = 1.4826 MAD); a rejected sample is
    dropped in all channels. Returns mean RGB, kept count, valid count, and means of the kept
    even- and odd-indexed frames (half their difference is pure noise)."""
    n, h, w, _ = samples.shape
    mean = np.full((h, w, 3), np.nan, np.float32)
    kept = np.zeros((h, w), np.int32)
    valid = np.zeros((h, w), np.int32)
    halves = np.full((2, h, w, 3), np.nan, np.float32)

    def strip(y0):
        y1 = min(h, y0 + rows)
        block = np.asarray(samples[:, y0:y1], np.float32)
        lum = block.sum(axis=3)
        keep = np.isfinite(lum)
        valid[y0:y1] = keep.sum(axis=0)
        for _ in range(iterations):
            current = np.where(keep, lum, np.nan)
            centre, count = nan_median(current)
            spread, _ = nan_median(np.abs(current - centre))
            sigma = 1.4826 * spread
            clip = (count >= 3)[None] & (np.abs(lum - centre) > k * np.maximum(sigma, 1e-3))
            keep &= ~clip
        kept[y0:y1] = keep.sum(axis=0)
        with np.errstate(invalid="ignore"):
            mean[y0:y1] = np.nansum(np.where(keep[..., None], block, 0), axis=0) / keep.sum(axis=0)[..., None]
            for parity in range(2):
                part = keep & (np.arange(n) % 2 == parity)[:, None, None]
                halves[parity, y0:y1] = np.nansum(np.where(part[..., None], block, 0), axis=0) / part.sum(axis=0)[..., None]

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(strip, range(0, h, rows)))
    return mean, kept, valid, halves


FULL_COVERAGE = 0.3


def composite(sky, ground, alpha, reference_frame, coverage):
    """alpha·sky + (1 − alpha)·ground. Sky seen in only a short run of frames (coverage, the
    share of frames that contributed, below FULL_COVERAGE) blends toward the frames around
    the reference: such a run carries its own glow and lighting, which shows as arcs along
    the star paths, while the reference window matches the foreground's moment. Sky without
    coverage is the reference window."""
    a = alpha[..., None]
    missing = ~np.isfinite(sky).all(axis=2) & (alpha > 0)
    weight = np.clip(coverage / FULL_COVERAGE, 0, 1)[..., None]
    sky = np.where(missing[..., None], reference_frame, weight * np.nan_to_num(sky) + (1 - weight) * reference_frame)
    return a * sky + (1 - a) * ground, missing


def warp_tile(frames, model, clear_px, glow, glow_reference, rays, occluded, flat, kernel=cv2.INTER_LANCZOS4, workers=16,
              min_clearance=CLEARANCE_PX):
    """Samples (N, th, tw, 3) for one output tile: each frame resampled once from the window
    it needs, glow-corrected (− G_i at the source + G_ref at the target), NaN where the source
    is outside the frame, closer than CLEARANCE_PX to anything that is not clear sky, or
    occluded in that frame (packed bits from `occlusion.build`). Frames are divided by `flat`
    (vignetting) before resampling."""
    from proto import occlusion
    from proto.background import sample as sample_glow
    n, (h, w) = frames.shape[0], clear_px.shape
    th, tw = rays.shape[:2]
    samples = np.full((n, th, tw, 3), np.nan, np.float32)

    def one(i):
        p = model.camera.pixels(rays @ model.rotation(i).T)
        mx, my = p[..., 0], p[..., 1]
        inside = np.isfinite(mx) & np.isfinite(my) & (mx >= 0) & (my >= 0) & (mx <= w - 1) & (my <= h - 1)
        if not inside.any():
            return
        x0, x1 = max(0, int(mx[inside].min()) - 4), min(w, int(mx[inside].max()) + 6)
        y0, y1 = max(0, int(my[inside].min()) - 4), min(h, int(my[inside].max()) + 6)
        lx = np.where(inside, mx - x0, -1).astype(np.float32)
        ly = np.where(inside, my - y0, -1).astype(np.float32)
        patch = np.asarray(frames[i, y0:y1, x0:x1], np.float32) / flat[y0:y1, x0:x1, None]
        image = cv2.remap(patch, lx, ly, kernel, borderMode=cv2.BORDER_REPLICATE)
        source_clear = cv2.remap(np.ascontiguousarray(clear_px[y0:y1, x0:x1]), lx, ly, cv2.INTER_LINEAR,
                                 borderMode=cv2.BORDER_CONSTANT, borderValue=0)
        covered = cv2.remap(occlusion.patch(occluded, i, y0, y1, x0, x1).astype(np.uint8), lx, ly, cv2.INTER_NEAREST,
                            borderMode=cv2.BORDER_CONSTANT, borderValue=0).astype(bool)
        image = image - sample_glow(glow[i], p) + glow_reference
        image[~inside | (source_clear < min_clearance) | covered | ~np.isfinite(image).all(axis=2)] = np.nan
        samples[i] = image

    with ThreadPoolExecutor(workers) as pool:
        list(pool.map(one, range(n)))
    return samples


def stack_tiles(frames, model, alpha, glow, reference, occluded, flat, tile=512, kernel=cv2.INTER_LANCZOS4, relaxed=1.5):
    """Streams the sky stack tile by tile; returns mean, kept, valid and even/odd halves.
    Sky pixels with no sample at the full clearance get a second pass at `relaxed` px."""
    from proto.background import sample as sample_glow
    h, w = alpha.shape
    clear_px = clearance(alpha)
    mean = np.full((h, w, 3), np.nan, np.float32)
    kept = np.zeros((h, w), np.int32)
    valid = np.zeros((h, w), np.int32)
    halves = np.full((2, h, w, 3), np.nan, np.float32)
    for y0 in range(0, h, tile):
        for x0 in range(0, w, tile):
            y1, x1 = min(h, y0 + tile), min(w, x0 + tile)
            yy, xx = np.mgrid[y0:y1, x0:x1]
            points = np.stack([xx, yy], axis=-1).astype(np.float64)
            rays = model.camera.rays(points)
            glow_reference = sample_glow(glow[reference], points)
            samples = warp_tile(frames, model, clear_px, glow, glow_reference, rays, occluded, flat, kernel)
            m, k, v, hv = combine(samples)
            empty = (k == 0) & (alpha[y0:y1, x0:x1] > 0)
            if empty.any():
                retry = warp_tile(frames, model, clear_px, glow, glow_reference, rays, occluded, flat, kernel,
                                  min_clearance=relaxed)
                rm, rk, rv, rh = combine(retry)
                m[empty], k[empty], v[empty] = rm[empty], rk[empty], rv[empty]
                hv[:, empty] = rh[:, empty]
            mean[y0:y1, x0:x1], kept[y0:y1, x0:x1], valid[y0:y1, x0:x1] = m, k, v
            halves[:, y0:y1, x0:x1] = hv
    return mean, kept, valid, halves
