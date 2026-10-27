"""Sky/ground separation from the sequence itself, refined by SAM segments."""
import subprocess
from pathlib import Path

import numpy as np
from scipy import ndimage
from skimage import morphology

ROOT = Path(__file__).resolve().parents[2]
MAX_GAP_PX = 32.0


def sam_labels(image8, work):
    """SAM automatic segments of an 8-bit RGB image, run in the separate SAM venv."""
    from PIL import Image
    png, out = work / "sam_input.png", work / "sam_segments.npz"
    Image.fromarray(image8).save(png)
    subprocess.run([str(ROOT / ".venv-sam/bin/python"), str(ROOT / "tools/proto/sam_segments.py"),
                    str(png), str(ROOT / ".models/sam/sam_vit_b_01ec64.pth"), str(out)], check=True)
    return np.load(out)["labels"]


def sky_background(median, sky, cell=16, smooth=2.0):
    """Smooth sky brightness from confirmed sky pixels: masked block medians, cells without
    sky take the nearest sky cell's value, then light smoothing. Near the celestial pole the
    transit pixels are biased bright by slow-moving stars (a known limitation)."""
    import cv2
    h, w = median.shape
    gh, gw = h // cell, w // cell
    blocks = median[:gh * cell, :gw * cell].reshape(gh, cell, gw, cell).transpose(0, 2, 1, 3).reshape(gh, gw, -1)
    keep = sky[:gh * cell, :gw * cell].reshape(gh, cell, gw, cell).transpose(0, 2, 1, 3).reshape(gh, gw, -1)
    values = np.where(keep, blocks, np.nan)
    enough = keep.sum(axis=2) >= cell * cell // 4
    grid = np.where(enough, np.nanmedian(np.where(enough[..., None], values, 0.0), axis=2), 0.0)
    _, (iy, ix) = ndimage.distance_transform_edt(~enough, return_indices=True)
    filled = ndimage.gaussian_filter(grid[iy, ix], smooth)
    return cv2.resize(filled.astype(np.float32), (w, h), interpolation=cv2.INTER_LINEAR)


def brightness(level, sigma, frames_count, sky):
    """Sky background fitted to `sky` pixels, ground level, brightness alpha and its noise."""
    background = sky_background(level, sky)
    dark = ~sky & (level < 0.25 * background)
    ground_level = float(np.median(level[ndimage.binary_erosion(dark, iterations=3)]))
    span = np.maximum(background - ground_level, 1e-3)
    alpha = np.clip((level - ground_level) / span, 0, 1)
    return background, ground_level, alpha, 1.2533 * sigma / np.sqrt(frames_count) / span


def matte(stats, labels, frames_count, transit_min=0.004, near_radius=None, max_ground_island=0):
    """Sky alpha per camera pixel (1 = clear sky) for silhouette foregrounds. Confirmed star
    transits mark sky; brightness against the sky background gives fractional coverage; values
    within 3σ of 0 or 1 snap, as do near-clear pixels inside clear neighborhoods. Inside dark SAM
    segments, pixels without transits are ground unless they are clearly bright; with
    `near_radius`, motion outranks brightness there: a pixel with no transit within that many
    pixels is ground however bright (pale trunks, lit bark). Locally the radius grows to the gap
    that 99% of nearby brightness-clear sky stays within; beyond MAX_GAP_PX transits are too sparse
    (bright skies, low near the horizon) for their absence to mean anything and the rule is off.
    Ground islands up to `max_ground_island` pixels (hot-pixel demosaic artifacts) become sky.
    The sky background is fitted to transit pixels, then refitted with featureless pixels that
    the first fit already calls nearly clear (bright skies where few transits are seen)."""
    seen = ndimage.binary_opening(stats["transit"] >= transit_min, iterations=1)
    level = stats.get("low", stats["median"])
    background, ground_level, alpha, noise = brightness(level, stats["sigma"], frames_count, seen)
    mean = ndimage.uniform_filter(level, 5)
    texture = np.sqrt(np.maximum(ndimage.uniform_filter(level * level, 5) - mean * mean, 0))
    featureless = (alpha >= 0.85) & (texture < 3 * 1.2533 * stats["sigma"] / np.sqrt(frames_count))
    background, ground_level, alpha, noise = brightness(level, stats["sigma"], frames_count, seen | featureless)
    clear = ((alpha >= 1 - 3 * noise) & (noise < 0.05)) | (seen & (alpha >= 0.85))
    dark_segments = np.zeros_like(seen)
    segments = {}
    for label in np.unique(labels):
        if label == 0:
            continue
        region = labels == label
        segments[int(label)] = {"pixels": int(region.sum()), "transit_share": float(seen[region].mean()),
                                "mean_alpha": float(alpha[region].mean())}
        info = segments[int(label)]
        if info["transit_share"] < 0.25 and info["mean_alpha"] < (0.6 if near_radius else 0.3):
            dark_segments |= region
    radius = None
    if near_radius:
        gap = ndimage.distance_transform_edt(~seen)
        radius = transit_gaps(gap, alpha >= 0.98, near_radius)
        near = (gap <= radius) | (radius > MAX_GAP_PX)
        ground = (alpha <= 3 * noise) | (dark_segments & ~near)
    else:
        ground = (alpha <= 3 * noise) | (dark_segments & ~seen & (alpha < 0.5))
    snapped = np.where(ground & ~seen, 0.0, np.where(clear, 1.0, np.where(ground, 0.0, alpha)))
    local = ndimage.median_filter(snapped, size=5)
    snapped = np.where((snapped > 0.85) & (local >= 0.98), 1.0, snapped)
    islands = np.zeros_like(seen)
    if max_ground_island:
        solid = snapped <= 0.02
        islands = solid & ~morphology.remove_small_objects(solid, max_size=max_ground_island)
        snapped[islands] = 1.0
    conflicts = seen & (snapped < 0.2)
    return {"alpha": snapped.astype(np.float32), "raw_alpha": alpha.astype(np.float32), "seen": seen,
            "background": background, "ground_level": ground_level, "noise": noise.astype(np.float32),
            "sam_ground": ground & ~seen & (alpha > 3 * noise),
            "conflicts": conflicts, "segments": segments, "islands": islands,
            "near_radius": None if radius is None else {q: float(np.percentile(radius, q)) for q in (10, 50, 90)}}


def transit_gaps(gap, clear, minimum, block=128, share=0.05):
    """Per pixel, the 99th-percentile distance to the nearest transit over brightness-clear
    pixels of its block (blocks with too little clear sky take the nearest block's value),
    never below `minimum`."""
    h, w = gap.shape
    gh, gw = -(-h // block), -(-w // block)
    grid = np.full((gh, gw), np.nan)
    for by in range(gh):
        for bx in range(gw):
            box = (slice(by * block, (by + 1) * block), slice(bx * block, (bx + 1) * block))
            values = gap[box][clear[box]]
            if values.size >= share * block * block:
                grid[by, bx] = np.percentile(values, 99)
    if np.isnan(grid).all():
        raise ValueError("no brightness-clear sky to measure transit density")
    import cv2
    _, (iy, ix) = ndimage.distance_transform_edt(np.isnan(grid), return_indices=True)
    grid = ndimage.gaussian_filter(np.log(np.maximum(grid[iy, ix], minimum)), 1.0, mode="nearest")
    return np.exp(cv2.resize(grid.astype(np.float32), (gw * block, gh * block), interpolation=cv2.INTER_LINEAR)[:h, :w])


def unstable(stats, alpha, seen, factor=1.5):
    """Mostly-ground pixels (alpha ≤ 0.5) with no star transit within 3 px whose temporal
    scatter exceeds the robust noise by `factor`: moving foliage or changing lights."""
    ratio = stats["scatter"] / np.maximum(stats["sigma"], 1e-3)
    return (alpha <= 0.5) & ~ndimage.binary_dilation(seen, iterations=3) & (ratio > factor)
