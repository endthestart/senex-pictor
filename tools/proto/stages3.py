"""Iteration 2 previews: full views, 100% crops, iteration 1 versus 2, maps and noise."""
import pickle

import cv2
import numpy as np

from proto import preview
from proto.stages import log

CROP = (600, 450)
# Wild River crop positions (6048 × 4024); other sequences get relative generic crops.
WILD_RIVER = {"center": (2724, 1787), "top-left corner": (0, 0), "top-right corner": (5448, 0),
              "celestial pole": (1886, 1499), "bare branches": (4860, 1915), "birches": (2400, 3195),
              "pale trunks": (3400, 2950)}
VERSUS = {"center": (2632, 1620), "bare branches": (4760, 1840)}


def crops_for(ctx, alpha):
    if ctx["names"][0] == "DSC_0226.NEF":
        return WILD_RIVER
    h, w = alpha.shape
    edges = (alpha > 0.02) & (alpha < 0.98)
    cells = edges[:h // 450 * 450, :w // 600 * 600].reshape(h // 450, 450, w // 600, 600).mean(axis=(1, 3))
    ey, ex = np.unravel_index(np.argmax(cells), cells.shape)
    return {"center": (w // 2 - 300, h // 2 - 225), "top-left corner": (0, 0), "top-right corner": (w - 600, 0),
            "bottom-left corner": (0, h - 450), "bottom-right corner": (w - 600, h - 450),
            "busiest sky edge": (int(ex) * 600, int(ey) * 450)}


def previews(ctx):
    out = ctx["work"] / "review"
    out.mkdir(exist_ok=True)
    st, sep, ref = ctx["stack"], ctx["separation"], ctx["reference"]
    wb = ctx["meta"]["wb"][ref]
    single = np.asarray(ctx["frames"][ref], np.float32) / ctx["flat"][..., None]
    show = lambda image: preview.stretch(image, wb, 20, 600)
    views = {"single": show(single), "stack": show(st["final"])}
    for name, image in views.items():
        preview.save(out / f"{name}.jpg", image, 2000)
    w, h = CROP
    crops = crops_for(ctx, sep["alpha"])
    for name, (x, y) in crops.items():
        for kind, image in views.items():
            preview.save(out / f"crop-{name.replace(' ', '-')}-{kind}.jpg", preview.crop(image, x, y, w, h))
    first = ctx["work"].parent / "iter1" / "stages" / "stack.pkl"
    if first.exists() and ctx["names"][0] == "DSC_0226.NEF":
        with first.open("rb") as stream:
            half = show(pickle.load(stream)["final"])
        for name, (x, y) in VERSUS.items():
            slug = name.replace(" ", "-")
            preview.save(out / f"versus-{slug}-single.jpg", preview.crop(views["single"], x, y, 800, 600))
            preview.save(out / f"versus-{slug}-iteration1.jpg",
                         cv2.resize(preview.crop(half, x // 2, y // 2, 400, 300), (800, 600), interpolation=cv2.INTER_CUBIC))
            preview.save(out / f"versus-{slug}-iteration2.jpg", preview.crop(views["stack"], x, y, 800, 600))
    clear = sep["alpha"] >= 0.98
    noise = {}
    for name, (x, y) in crops.items():
        box = (slice(y, y + h), slice(x, x + w))
        if clear[box].mean() > 0.9:
            diff = ((st["sky_halves"][0][box] - st["sky_halves"][1][box]) / 2).sum(axis=2)
            diff = diff[np.isfinite(diff)]
            frames_used = float(np.median(st["sky_kept"][box]))
            noise[name] = {"single": float(np.median(sep["stats"]["sigma"][box])),
                           "stack": float(1.4826 * np.median(np.abs(diff - np.median(diff)))),
                           "median_frames": frames_used, "sqrt_frames": float(np.sqrt(frames_used))}
            noise[name]["improvement"] = noise[name]["single"] / noise[name]["stack"]
    preview.save(out / "alpha.jpg", preview.gray(sep["alpha"], 0, 1), 2000)
    preview.save(out / "transit.jpg", preview.gray(sep["stats"]["transit"], 0, 0.03), 2000)
    preview.save(out / "flicker.jpg", preview.gray(sep["stats"]["excess"] - sep["stats"]["transit"], 0, 0.03), 2000)
    preview.save(out / "coverage.jpg", preview.gray(np.where(sep["alpha"] > 0, st["sky_kept"], 0), 0, len(ctx["frames"])), 2000)
    preview.save(out / "rejections.jpg", preview.gray(st["sky_valid"] - st["sky_kept"], 0, 6), 2000)
    preview.save(out / "fallbacks.jpg", preview.overlay(views["stack"], sep["unstable"] | st["missing"]), 2000)
    glow = ctx["background"]["glow"].sum(axis=3)
    usable = ctx["background"]["usable"]
    panels = {"first": glow[0], "reference": glow[ref], "last": glow[-1]}
    lo, hi = (float(np.percentile(np.concatenate([v[usable] for v in panels.values()]), q)) for q in (1, 99))
    for name, grid in panels.items():
        image = preview.gray(np.where(usable, grid, lo), lo, hi)
        preview.save(out / f"glow-{name}.jpg", cv2.resize(image, (image.shape[1] * 6, image.shape[0] * 6), interpolation=cv2.INTER_CUBIC))
    noise["glow_scale_adu"] = {"black": lo, "white": hi}
    log(ctx["report"], "noise", noise)
    return {"noise": noise, "crops": crops}
