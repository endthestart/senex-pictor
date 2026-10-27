"""Iteration 2 stages after registration: matte, glow, tiled stack, previews."""
import cv2
import numpy as np

from proto import background, occlusion, preview, separation, stack, transits
from proto.quality import stars_in
from proto.stages import log

DOWN = 4
COMPARE_TILES = {"center": (2768, 1756), "top-left corner": (0, 0)}


def stage_separation(ctx):
    frames, out = ctx["frames"], ctx["stage_dir"]
    stats = transits.temporal_stats(frames, ctx["model"])
    gray = preview.gray(stats["median"], 0, float(np.percentile(stats["median"], 99.5)))
    small = cv2.resize(np.dstack([gray] * 3), (gray.shape[1] // DOWN, gray.shape[0] // DOWN), interpolation=cv2.INTER_AREA)
    coarse = separation.sam_labels(small, out)
    ys = np.minimum(np.arange(gray.shape[0]) // DOWN, coarse.shape[0] - 1)
    xs = np.minimum(np.arange(gray.shape[1]) // DOWN, coarse.shape[1] - 1)
    labels = coarse[ys[:, None], xs[None, :]]
    result = separation.matte(stats, labels, len(frames), transit_min=0.003, near_radius=6, max_ground_island=16)
    alpha = result["alpha"]
    unstable = separation.unstable(stats, alpha, result["seen"])
    share = lambda m: round(float(m.mean()), 4)
    log(ctx["report"], "separation", {
        "unresolvable_pole_pixels": int((~stats["resolvable"]).sum()),
        "transit_percentiles": {q: float(np.percentile(stats["transit"], q)) for q in (30, 50, 70, 90)},
        "incoherent_excess_share": share((stats["excess"] >= 0.003) & ~result["seen"]),
        "sky_clear_share": share(alpha >= 0.98), "ground_share": share(alpha <= 0.02),
        "edge_share": share((alpha > 0.02) & (alpha < 0.98)), "transit_seen_share": share(result["seen"]),
        "sam_segments": int(len(result["segments"])), "near_radius_px": result["near_radius"], "motion_ground_pixels": int(result["sam_ground"].sum()),
        "ground_islands_to_sky_pixels": int(result["islands"].sum()),
        "conflict_pixels": int(result["conflicts"].sum()), "unstable_pixels": int(unstable.sum())})
    preview.save(out / "alpha.jpg", preview.gray(alpha, 0, 1), 1800)
    preview.save(out / "transit.jpg", preview.gray(stats["transit"], 0, 0.03), 1800)
    preview.save(out / "flicker.jpg", preview.gray(stats["excess"] - stats["transit"], 0, 0.03), 1800)
    return {"stats": stats, "labels": labels, "unstable": unstable, **result}


def stage_background(ctx):
    medians, usable = background.block_medians(ctx["frames"], ctx["separation"]["alpha"], ctx["flat"])
    glow, sky, changes = background.decompose(medians, ctx["model"])
    ref = ctx["reference"]
    lum = glow.sum(axis=3)
    usable_cells = usable & np.isfinite(lum[ref])
    rows = np.nonzero(usable_cells.any(axis=1))[0]
    top, bottom = rows.min(), rows.max()
    over_time = lum[:, usable_cells]
    log(ctx["report"], "background", {"vignetting": ctx["vignetting"],
        "iteration_rms_change_adu": changes,
        "reference_glow_adu": {"top_row": float(np.nanmedian(lum[ref, top][usable_cells[top]])),
                               "lowest_sky_row": float(np.nanmedian(lum[ref, bottom][usable_cells[bottom]]))},
        "glow_change_over_sequence_adu": float(np.nanmedian(over_time[-1] - over_time[0])),
        "glow_range_over_time_adu": float(np.nanmedian(over_time.max(axis=0) - over_time.min(axis=0)))})
    out = ctx["stage_dir"]
    scale = float(np.nanpercentile(lum[ref], 99))
    preview.save(out / "glow-reference.jpg", preview.gray(np.where(usable, lum[ref], 0), 0, scale), 1200)
    preview.save(out / "glow-change.jpg", preview.gray(lum[-1] - lum[0], -0.1 * scale, 0.1 * scale), 1200)
    return {"glow": glow, "sky": sky, "usable": usable, "vignetting": ctx["vignetting"]}


def stage_occlusion(ctx):
    frames, path = ctx["frames"], ctx["stage_dir"] / "occlusion.bits"
    share = occlusion.build(frames, ctx["background"]["glow"], ctx["separation"]["alpha"], path, ctx["flat"])
    log(ctx["report"], "occlusion", {"sky_share_occluded": {q: float(np.percentile(share, q)) for q in (10, 50, 90, 100)},
                                     "worst_frames": [int(i) for i in np.argsort(share)[::-1][:5]]})
    return {"path": path, "shape": frames.shape[:3], "share": share}


def stage_stack(ctx):
    frames, sep_, model = ctx["frames"], ctx["separation"], ctx["model"]
    glow, ref, alpha = ctx["background"]["glow"], ctx["reference"], sep_["alpha"]
    occluded = occlusion.load(ctx["occlusion"]["path"], ctx["occlusion"]["shape"])
    flat = ctx["flat"]
    sky, kept, valid, halves = stack.stack_tiles(frames, model, alpha, glow, ref, occluded, flat)
    ground, ground_kept, ground_valid, _ = stack.combine(frames)
    ground /= flat[..., None]
    reference = np.asarray(frames[ref], np.float32) / flat[..., None]
    window = np.mean([np.asarray(frames[i], np.float32) for i in range(max(0, ref - 4), min(len(frames), ref + 5))],
                     axis=0) / flat[..., None]
    ground[sep_["unstable"]] = window[sep_["unstable"]]
    final, missing = stack.composite(sky, ground, alpha, window, kept / len(frames))
    from proto.background import sample as sample_glow
    kernels = {}
    for name, (x0, y0) in COMPARE_TILES.items():
        yy, xx = np.mgrid[y0:y0 + 512, x0:x0 + 512]
        points = np.stack([xx, yy], axis=-1).astype(np.float64)
        samples = stack.warp_tile(frames, model, stack.clearance(alpha), glow, sample_glow(glow[ref], points),
                                  model.camera.rays(points), occluded, flat, kernel=cv2.INTER_LINEAR)
        bilinear, _, _, bil_halves = stack.combine(samples)
        box = (slice(y0, y0 + 512), slice(x0, x0 + 512))
        noise = lambda h: float(1.4826 * np.nanmedian(np.abs(((h[0] - h[1]) / 2).sum(axis=2) - np.nanmedian(((h[0] - h[1]) / 2).sum(axis=2)))))
        kernels[name] = {"single": stars_in(reference[box]), "lanczos": stars_in(sky[box]) | {"noise": noise(halves[(slice(None),) + box])},
                         "bilinear": stars_in(bilinear) | {"noise": noise(bil_halves)}}
    clear = alpha >= stack.CLEAR_SKY
    log(ctx["report"], "stack", {
        "sky_coverage_frames": {q: float(np.percentile(kept[clear], q)) for q in (1, 10, 50, 90)},
        "sky_rejected_share": float((valid - kept)[clear].sum() / max(valid[clear].sum(), 1)),
        "sky_missing_pixels": int(missing.sum()), "unstable_reference_pixels": int(sep_["unstable"].sum()),
        "ground_rejected_share": float(1 - ground_kept.sum() / max(ground_valid.sum(), 1)), "kernels": kernels})
    return {"final": final, "sky": sky, "ground": ground, "sky_kept": kept, "sky_valid": valid,
            "sky_halves": halves, "missing": missing}


def stage_previews(ctx):
    from proto.stages3 import previews
    return previews(ctx)
