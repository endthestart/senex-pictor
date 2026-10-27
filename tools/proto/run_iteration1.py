"""Iteration 1 (half resolution): registration, sky/ground separation, stacking, previews.

Usage: .venv/bin/python tools/proto/run_iteration1.py RAW_DIR WORK_DIR [--stages ...]
Stages cache their results in WORK_DIR/stages so later stages can be re-run alone.
"""
import argparse
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import frames as cache  # noqa: E402
from proto import preview, separation, sky_model, stack, stages, stars, transits  # noqa: E402
from proto.stages import log  # noqa: E402

STAGES = ["stars", "model", "separation", "stack", "previews"]


def stage_stars(ctx):
    detections = stars.detect_all(ctx["cache"], ctx["names"])
    tracks = stars.track(detections)
    counts = [len(d["xy"]) for d in detections]
    log(ctx["report"], "stars", {"per_frame_min": min(counts), "per_frame_median": int(np.median(counts)),
                                 "per_frame_max": max(counts), "tracks": len(tracks)})
    return {"detections": detections, "tracks": tracks}


def stage_model(ctx):
    s = ctx["stars"]
    times = ctx["times"]
    ref = ctx["reference"]
    subset = sorted(set(range(0, len(times), 10)) - {ref})
    model = sky_model.fit(s["detections"], s["tracks"], times, ref, subset, *ctx["sensor"], 4150.0, (-0.115, 0.044))
    sidereal = sky_model.SIDEREAL_RAD_PER_S * (times - times[ref])
    error = np.degrees(model.angles - sidereal)
    held = {int(i): sky_model.held_out(model, s["detections"], s["tracks"], int(i))
            for i in [0, len(times) // 4, ref - 1, ref + 1, 3 * len(times) // 4, len(times) - 1]}
    log(ctx["report"], "model", {"focal_px": model.camera.f, "k1": model.camera.k1, "k2": model.camera.k2,
                                 "axis": model.axis.tolist(), "reference": ref,
                                 "angle_minus_sidereal_deg": {"max_abs": float(np.abs(error).max()),
                                                              "rms": float(np.sqrt((error ** 2).mean()))},
                                 "held_out": held})
    return model


def stage_separation(ctx):
    stats = transits.temporal_stats(ctx["frames"], ctx["model"])
    gray = preview.gray(stats["median"], 0, float(np.percentile(stats["median"], 99.5)))
    labels = separation.sam_labels(np.dstack([gray] * 3), ctx["stage_dir"])
    result = separation.matte(stats, labels, len(ctx["frames"]))
    alpha = result["alpha"]
    unstable = separation.unstable(stats, alpha, result["seen"])
    shares = lambda m: round(float(m.mean()), 4)
    log(ctx["report"], "separation", {
        "sky_clear_share": shares(alpha >= 0.98), "ground_share": shares(alpha <= 0.02),
        "edge_share": shares((alpha > 0.02) & (alpha < 0.98)), "transit_seen_share": shares(result["seen"]),
        "ground_level": result["ground_level"], "sam_segments": len(result["segments"]),
        "sam_ground_pixels": int(result["sam_ground"].sum()),
        "conflict_pixels": int(result["conflicts"].sum()), "unstable_pixels": int(unstable.sum())})
    out = ctx["stage_dir"]
    preview.save(out / "alpha.jpg", preview.gray(alpha, 0, 1), 1600)
    preview.save(out / "transit.jpg", preview.gray(stats["transit"], 0, 0.05), 1600)
    rng = np.random.default_rng(3)
    palette = rng.integers(40, 255, size=(labels.max() + 1, 3), dtype=np.uint8)
    palette[0] = 0
    preview.save(out / "sam.jpg", palette[labels], 1600)
    preview.save(out / "conflicts.jpg", preview.overlay(np.dstack([gray] * 3), result["conflicts"] | unstable), 1600)
    return {"stats": stats, "labels": labels, "unstable": unstable, **result}


def stage_stack(ctx):
    frames, sep, ref = ctx["frames"], ctx["separation"], ctx["reference"]
    aligned, clipped = stack.warp_all(frames, ctx["saturated"], ctx["model"], sep["alpha"])
    sky, sky_kept, sky_valid, sky_halves = stack.combine(aligned)
    del aligned
    ground, ground_kept, ground_valid, _ = stack.combine(frames)
    reference = np.asarray(frames[ref])
    ground[sep["unstable"]] = reference[sep["unstable"]]
    final, missing = stack.composite(sky, ground, sep["alpha"], reference)
    clear = sep["alpha"] >= stack.CLEAR_SKY
    rejected = sky_valid - sky_kept
    log(ctx["report"], "stack", {
        "sky_coverage_frames": {q: float(np.percentile(sky_kept[clear], q)) for q in (0, 1, 10, 50, 90, 100)},
        "sky_rejected_share": float(rejected[clear].sum() / max(sky_valid[clear].sum(), 1)),
        "pixels_with_trail_rejections": int((rejected >= 1).sum()),
        "sky_missing_pixels": int(missing.sum()),
        "saturated_source_pixels": int(clipped.any(axis=0).sum()),
        "ground_rejected_share": float(1 - ground_kept.sum() / max(ground_valid.sum(), 1)),
        "unstable_reference_pixels": int(sep["unstable"].sum())})
    return {"final": final, "sky": sky, "ground": ground, "sky_kept": sky_kept, "sky_valid": sky_valid,
            "sky_halves": sky_halves, "missing": missing}


CROPS = {"center": (1316, 810, 400, 300), "top-left corner": (0, 0, 400, 300), "top-right corner": (2632, 0, 400, 300),
         "celestial pole": (893, 712, 400, 300), "bare branches": (2380, 920, 400, 300), "birches": (1150, 1560, 400, 300)}


def robust_sd(values):
    values = values[np.isfinite(values)]
    return float(1.4826 * np.median(np.abs(values - np.median(values))))


def stage_previews(ctx):
    out = ctx["work"] / "review"
    out.mkdir(exist_ok=True)
    st, sep = ctx["stack"], ctx["separation"]
    wb = ctx["meta"]["wb"][ctx["reference"]]
    single = np.asarray(ctx["frames"][ctx["reference"]])
    show = lambda image: preview.stretch(image, wb, 20, 600)
    views = {"single": show(single), "stack": show(st["final"])}
    for name, image in views.items():
        preview.save(out / f"{name}.jpg", image, 1800)
    clear = sep["alpha"] >= 0.98
    measurements = {}
    for name, (x, y, w, h) in CROPS.items():
        slug = name.replace(" ", "-")
        for kind, image in views.items():
            preview.save(out / f"crop-{slug}-{kind}.jpg", preview.crop(image, x, y, w, h))
        box_clear = clear[y:y + h, x:x + w]
        if box_clear.mean() > 0.9:
            box = (slice(y, y + h), slice(x, x + w))
            even, odd = (st["sky_halves"][p][box].sum(axis=2) for p in range(2))
            measurements[name] = {"single": float(np.median(sep["stats"]["sigma"][box])),
                                  "stack": robust_sd((even - odd) / 2),
                                  "median_frames": float(np.median(st["sky_kept"][box]))}
    preview.save(out / "alpha.jpg", preview.gray(sep["alpha"], 0, 1), 1800)
    preview.save(out / "transit.jpg", preview.gray(sep["stats"]["transit"], 0, 0.05), 1800)
    preview.save(out / "coverage.jpg", preview.gray(np.where(sep["alpha"] > 0, st["sky_kept"], 0), 0, len(ctx["frames"])), 1800)
    preview.save(out / "rejections.jpg", preview.gray(st["sky_valid"] - st["sky_kept"], 0, 6), 1800)
    rng = np.random.default_rng(3)
    palette = rng.integers(40, 255, size=(sep["labels"].max() + 1, 3), dtype=np.uint8)
    palette[0] = 0
    preview.save(out / "sam.jpg", palette[sep["labels"]], 1800)
    preview.save(out / "fallbacks.jpg", preview.overlay(views["stack"], sep["unstable"] | st["missing"]), 1800)
    log(ctx["report"], "noise", {k: v | {"improvement": v["single"] / max(v["stack"], 1e-6),
                                         "sqrt_frames": float(np.sqrt(v["median_frames"]))} for k, v in measurements.items()})
    return {"crops": CROPS, "noise": measurements}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", type=Path)
    parser.add_argument("work", type=Path)
    parser.add_argument("--stages", nargs="+", choices=STAGES, default=STAGES)
    parser.add_argument("--reference", type=int, help="reference frame index (default: middle)")
    args = parser.parse_args()
    paths = sorted(p for p in args.raw.iterdir() if p.suffix.lower() == ".nef")
    directory = args.work / "cache"
    frames, saturated, meta = cache.load(paths, directory)
    ctx = {"cache": directory, "names": [p.name for p in paths], "frames": frames, "saturated": saturated,
           "meta": meta, "times": np.array(meta["times_s"]) - meta["times_s"][0], "sensor": (6064, 4040),
           "work": args.work, "reference": args.reference if args.reference is not None else (len(paths) - 1) // 2}
    stages.run({name: globals()[f"stage_{name}"] for name in STAGES}, ctx, args.work / "stages", args.stages)


if __name__ == "__main__":
    main()
