"""Sequence check before processing: one run of frames from one fixed camera, fit to stack.

From EXIF alone, frames split into runs at capture-time gaps and setting changes. From the
RAW data, each frame's saturated share flags overexposure (dawn, headlights), a jump in the
lower half's brightness or redness against its darker neighbours (lower quartile within ±7
frames, so clusters of lit frames and slow twilight both stay honest) flags a lit frame (headlamp, car,
light painting: usable for the sky, not for an averaged foreground), and the
foreground is tracked from frame to frame on star-free half-resolution luminance. A camera
move needs two witnesses: a foreground step that stands out from the run's own step scatter
(or cannot be tracked), and a break in the otherwise smooth frame-to-frame star motion (wind
in foliage moves the foreground, never the stars). A steady accumulation is drift.
"""
import json
import subprocess
from concurrent.futures import ProcessPoolExecutor

import astroalign
import cv2
import numpy as np
import rawpy
import sep

GAP_FACTOR = 3.0
MIN_GAP_S = 60.0
OVEREXPOSED_SHARE = 0.02
LIT_FACTOR = 1.3
LIT_WINDOW = 7
MOVE_FLOOR_PX = 1.5
MOVE_SIGMA = 4.0


def capture(paths):
    tags = ["-DateTimeOriginal", "-SubSecTimeOriginal", "-ExposureTime", "-ISO", "-FNumber", "-FocalLength", "-LensID", "-Model"]
    rows = json.loads(subprocess.run(["exiftool", "-j", "-n", *tags, *map(str, paths)], check=True,
                                     capture_output=True, text=True).stdout)
    from datetime import datetime
    out = []
    for r in rows:
        t = datetime.strptime(r["DateTimeOriginal"], "%Y:%m:%d %H:%M:%S").timestamp()
        sub = r.get("SubSecTimeOriginal")
        out.append({"time": t + (float(f"0.{sub}") if sub not in (None, "") else 0.0),
                    "settings": [r.get(k) for k in ("Model", "LensID", "ExposureTime", "ISO", "FNumber", "FocalLength")]})
    return out


def runs(frames):
    """Index ranges [start, end) split at long gaps or setting changes."""
    gaps = np.diff([f["time"] for f in frames])
    typical = float(np.median(gaps)) if len(gaps) else 0.0
    cuts = [i + 1 for i, g in enumerate(gaps)
            if g > max(GAP_FACTOR * typical, MIN_GAP_S) or frames[i + 1]["settings"] != frames[i]["settings"]]
    edges = [0, *cuts, len(frames)]
    return [(a, b) for a, b in zip(edges, edges[1:])], typical


def _look(path):
    """Saturated share, lower-half median per colour (R, G, B above black) and a star-free
    half-resolution 8-bit luminance of one RAW."""
    with rawpy.imread(str(path)) as raw:
        data = raw.raw_image_visible.astype(np.float32)
        share = float((raw.raw_image_visible >= raw.white_level).mean())
        black = float(np.mean(raw.black_level_per_channel))
        colors = raw.raw_colors_visible
        lower = slice(data.shape[0] // 2, None)
        ground = [float(np.median(data[lower][np.isin(colors[lower], c)]) - black) for c in ((0,), (1, 3), (2,))]
    h, w = data.shape[0] // 2 * 2, data.shape[1] // 2 * 2
    lum = data[:h, :w].reshape(h // 2, 2, w // 2, 2).sum(axis=(1, 3)) - 4 * black
    stars = _stars(lum)
    g = np.log1p(np.clip(lum, 0, None) / 40.0)
    g = cv2.normalize(g, None, 0, 255, cv2.NORM_MINMAX).astype(np.uint8)
    return share, cv2.medianBlur(cv2.medianBlur(g, 5), 5), ground, stars


def _stars(lum, count=300):
    """Positions of the brightest compact sources in the upper half (half-resolution px)."""
    upper = np.ascontiguousarray(lum[: lum.shape[0] // 2], np.float32)
    background = sep.Background(upper, bw=64, bh=64)
    objects = sep.extract(upper - background, 8.0, err=background.globalrms, minarea=3)
    objects = objects[(objects["flag"] == 0) & (objects["npix"] < 100)]
    order = np.argsort(objects["flux"])[::-1][:count]
    return np.column_stack([objects["x"][order], objects["y"][order]])


def star_step(previous, current):
    """Translation and rotation (degrees) of the stars from one frame to the next, or None."""
    if len(previous) < 10 or len(current) < 10:
        return None
    try:
        transform, _ = astroalign.find_transform(previous, current)
    except (astroalign.MaxIterError, ValueError):
        return None
    return [*(2.0 * transform.translation), float(np.degrees(transform.rotation))]


def motion(previous, current):
    """Median foreground shift (full-res px) of corners in the lower half from one frame to the next."""
    h = previous.shape[0]
    mask = np.zeros_like(previous)
    mask[h // 2:] = 255
    corners = cv2.goodFeaturesToTrack(previous, 1500, 0.01, 10, mask=mask)
    if corners is None:
        return None
    forward, ok, _ = cv2.calcOpticalFlowPyrLK(previous, current, corners, None, winSize=(31, 31), maxLevel=3)
    back, ok_back, _ = cv2.calcOpticalFlowPyrLK(current, previous, forward, None, winSize=(31, 31), maxLevel=3)
    good = (ok.ravel() == 1) & (ok_back.ravel() == 1) & (np.linalg.norm((back - corners).reshape(-1, 2), axis=1) < 0.5)
    if good.sum() < 20:
        return None
    return (2.0 * np.median((forward - corners).reshape(-1, 2)[good], axis=0)).tolist()


def _sky_break(sky, i, a, b, window=4):
    """Whether star motion from frame i−1 to i departs from its neighbours' (median of up to
    `window` steps either side) by more than 1.5 px or 0.02°, or cannot be measured."""
    if sky[i] is None:
        return True
    near = [sky[j] for j in range(max(a + 1, i - window), min(b, i + window + 1)) if j != i and sky[j] is not None]
    if len(near) < 2:
        return True
    typical = np.median(near, axis=0)
    return bool(np.hypot(*(np.subtract(sky[i][:2], typical[:2]))) > 1.5 or abs(sky[i][2] - typical[2]) > 0.02)


def check(paths, workers=12):
    frames = capture(paths)
    pieces, interval = runs(frames)
    with ProcessPoolExecutor(workers) as pool:
        looks = list(pool.map(_look, paths))
    shares = [s for s, _, _, _ in looks]
    ground = np.maximum(np.array([g for _, _, g, _ in looks]), 1.0)
    sky = [None] + [star_step(looks[i - 1][3], looks[i][3]) for i in range(1, len(paths))]
    steps = [None] + [motion(looks[i - 1][1], looks[i][1]) for i in range(1, len(paths))]
    report = {"interval_s": interval, "runs": [], "frames": []}
    for i, p in enumerate(paths):
        step = steps[i]
        report["frames"].append({"name": p.name, "saturated_share": round(shares[i], 5),
                                 "step_px": None if step is None else [round(v, 2) for v in step]})
    segments = []
    for a, b in pieces:
        tracked = np.array([steps[i] for i in range(a + 1, b) if steps[i] is not None]).reshape(-1, 2)
        centre = np.median(tracked, axis=0) if len(tracked) else np.zeros(2)
        spread = 1.4826 * np.median(np.linalg.norm(tracked - centre, axis=1)) if len(tracked) else 0.0
        limit = max(MOVE_FLOOR_PX, MOVE_SIGMA * spread)
        cuts = [i for i in range(a + 1, b) if (steps[i] is None or np.linalg.norm(np.subtract(steps[i], centre)) > limit)
                and _sky_break(sky, i, a, b)]
        edges = [a, *cuts, b]
        segments += [(x, y, "camera move" if x in cuts else "gap or setting change", round(float(spread), 2))
                     for x, y in zip(edges, edges[1:])]
    for a, b, starts_after, spread in segments:
        usable = [i for i in range(a, b) if shares[i] <= OVEREXPOSED_SHARE]
        lit = []
        for k in range(a, b):
            near = [j for j in range(max(a, k - LIT_WINDOW), min(b, k + LIT_WINDOW + 1)) if j != k]
            if not near:
                continue
            typical = np.percentile(ground[near], 25, axis=0)
            if (ground[k].sum() / typical.sum() > LIT_FACTOR
                    or (ground[k, 0] / ground[k, 1]) / (typical[0] / typical[1]) > LIT_FACTOR):
                lit.append(paths[k].name)
        drift = [steps[i] for i in range(a + 1, b) if steps[i] is not None]
        report["runs"].append({"first": paths[a].name, "last": paths[b - 1].name, "frames": b - a, "starts_after": starts_after,
                               "step_scatter_px": spread,
                               "overexposed": [paths[i].name for i in range(a, b) if shares[i] > OVEREXPOSED_SHARE],
                               "lit": lit,
                               "drift_px": np.round(np.sum(drift, axis=0), 1).tolist() if drift else [0.0, 0.0],
                               "usable_first": paths[usable[0]].name if usable else None,
                               "usable_last": paths[usable[-1]].name if usable else None, "usable_frames": len(usable)})
    report["recommended"] = max(report["runs"], key=lambda r: r["usable_frames"])
    return report
