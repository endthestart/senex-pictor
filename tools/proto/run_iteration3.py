"""Iteration 3 (CFA stacking): reuses iteration 2's registration, matte, glow and ground stack,
replaces the demosaiced sky stack with a Bayer-drizzle stack of raw CFA samples, and compares.

Usage: .venv/bin/python tools/proto/run_iteration3.py RAW_DIR ITERATION2_WORK WORK_DIR [--stages ...]
"""
import argparse
import json
import pickle
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import cfa, fullres, occlusion, preview, quality, stack, stages, vignetting  # noqa: E402
from proto.stages import log  # noqa: E402

COMPARE = 512


def compare_boxes(alpha):
    """Up to four 512 px boxes of clear sky spread over the frame (centre first)."""
    h, w = alpha.shape
    candidates = [(w // 2 - 256, h // 2 - 256), (0, 0), (w - COMPARE, 0), (w // 4 - 256, h // 3 - 256),
                  (3 * w // 4 - 256, h // 3 - 256)]
    clear = [(x, y) for x, y in candidates if (alpha[y:y + COMPARE, x:x + COMPARE] >= stack.CLEAR_SKY).mean() > 0.9]
    return clear[:4]


def stage_cfa(ctx):
    _, meta = cfa.load(ctx["paths"], ctx["work"] / "cache", ctx["crop"])
    log(ctx["report"], "cfa", {"colors": meta["colors"], "black_range": [float(np.min(meta["black"])), float(np.max(meta["black"]))],
                               "white": sorted(set(meta["white"]))})
    return meta


def stage_stack(ctx):
    raw, meta = cfa.load(ctx["paths"], ctx["work"] / "cache", ctx["crop"])
    two, sep, model = ctx["iter2"]["stack"], ctx["iter2"]["separation"], ctx["iter2"]["model"]
    glow, ref, alpha = ctx["iter2"]["background"]["glow"], ctx["reference"], sep["alpha"]
    occ = ctx["iter2"]["occlusion"]
    occluded = occlusion.load(occ["path"], occ["shape"])
    sky, kept, halves, clipped = cfa.stack_tiles(raw, meta, model, alpha, glow, ref, occluded, ctx["flat"], ctx["kernel"])
    white = float(np.min(np.asarray(meta["white"])[:, None] - np.asarray(meta["black"]).reshape(len(meta["white"]), -1)))
    saturated = ~np.isfinite(sky) & clipped[..., None]
    sky[saturated] = white
    flat = ctx["flat"][..., None]
    window = np.mean([np.asarray(ctx["frames"][i], np.float32) for i in range(max(0, ref - 4), min(len(ctx["frames"]), ref + 5))],
                     axis=0) / flat
    # one red site per 2×2 cell: a frame gives a red sample to about πr²/4 of pixels (kernel radius r)
    per_frame = np.pi * ctx["kernel"][1] ** 2 / 4
    final, missing = stack.composite(sky, two["ground"], alpha, window, kept[..., 0] / (per_frame * len(ctx["frames"])))
    wb = ctx["meta"]["wb"][ref]
    clear = alpha >= stack.CLEAR_SKY
    boxes = {}
    for x, y in compare_boxes(alpha):
        box = (slice(y, y + COMPARE), slice(x, x + COMPARE))
        boxes[f"{x},{y}"] = {name: quality.stars_in(image[box]) | quality.ringing_and_colour(image[box], wb)
                             for name, image in (("single", np.asarray(ctx["frames"][ref], np.float32) / flat),
                                                 ("iteration2", two["sky"]), ("iteration3", sky))}
        boxes[f"{x},{y}"]["iteration2"]["noise"] = quality.half_noise(two["sky_halves"][(slice(None),) + box])
        boxes[f"{x},{y}"]["iteration3"]["noise"] = quality.half_noise(halves[(slice(None),) + box])
    log(ctx["report"], "stack", {
        "samples_per_colour_median": [float(np.median(kept[..., c][clear])) for c in range(3)],
        "samples_per_colour_p1": [float(np.percentile(kept[..., c][clear], 1)) for c in range(3)],
        "kernel_sigma_radius_px": list(ctx["kernel"]), "saturated_filled_pixels": int(saturated.any(axis=2).sum()), "sky_missing_pixels": int(missing.sum()),
        "compare": boxes})
    return {"final": final, "sky": sky, "kept": kept, "halves": halves, "missing": missing, "saturated": saturated}


def stage_previews(ctx):
    out = ctx["work"] / "review"
    out.mkdir(exist_ok=True)
    ref, three, two = ctx["reference"], ctx["stack"], ctx["iter2"]["stack"]
    wb = ctx["meta"]["wb"][ref]
    show = lambda image: preview.stretch(np.nan_to_num(image), wb, 20, 600)
    views = {"single": show(np.asarray(ctx["frames"][ref], np.float32) / ctx["flat"][..., None]), "iteration2": show(two["final"]),
             "iteration3": show(three["final"])}
    preview.save(out / "stack.jpg", views["iteration3"], 2000)
    for x, y in [tuple(int(v) for v in k.split(",")) for k in ctx["report"]["stack"]["compare"]]:
        for name, image in views.items():
            preview.save(out / f"crop-{x}-{y}-{name}.jpg", preview.crop(image, x + 156, y + 156, 200, 200))
    return {"views": list(views)}


STAGES = {"cfa": stage_cfa, "stack": stage_stack, "previews": stage_previews}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", type=Path)
    parser.add_argument("iteration2", type=Path)
    parser.add_argument("work", type=Path)
    parser.add_argument("--stages", nargs="+", choices=list(STAGES), default=list(STAGES))
    parser.add_argument("--kernel", type=float, nargs=2, default=cfa.KERNEL, metavar=("SIGMA", "RADIUS"),
                        help="Gaussian σ and cut-off radius in sensor pixels")
    parser.add_argument("--first")
    parser.add_argument("--last")
    args = parser.parse_args()
    paths = sorted(p for p in args.raw.iterdir() if p.suffix.lower() == ".nef" and not p.name.startswith("."))
    names = [p.name for p in paths]
    paths = paths[names.index(args.first) if args.first else 0:(names.index(args.last) + 1) if args.last else None]
    frames, meta = fullres.load_existing(args.iteration2 / "cache", [p.name for p in paths])
    probe = json.loads(subprocess.run([str(fullres.BINARY), "inspect", str(paths[0])], check=True,
                                      capture_output=True, text=True).stdout)
    iter2 = {}
    for name in ("model", "separation", "background", "occlusion", "stack"):
        with (args.iteration2 / "stages" / f"{name}.pkl").open("rb") as stream:
            iter2[name] = pickle.load(stream)
    args.work.mkdir(parents=True, exist_ok=True)
    (args.work / "cache").mkdir(exist_ok=True)
    area, crop = probe["active_area"], probe["default_crop"]
    crop = {**crop, "x": area["x"] + crop["x"], "y": area["y"] + crop["y"]}
    ctx = {"paths": paths, "frames": frames, "meta": meta, "crop": crop, "iter2": iter2,
           "work": args.work, "reference": iter2["model"].reference,
           "kernel": tuple(args.kernel)}
    ctx["flat"] = vignetting.recorded(iter2["background"], frames.shape[1:3])
    stages.run(STAGES, ctx, args.work / "stages", args.stages)


if __name__ == "__main__":
    main()
