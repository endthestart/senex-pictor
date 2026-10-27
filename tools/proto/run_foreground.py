"""Moving-foreground rendering variants for review (swaying foliage is a product choice).

  mean     the sequence's robust temporal mean: smooth, motion-blurred like a long exposure
  moment   the median of a short window around the reference frame: leaves frozen, noisier
  hybrid   the moment where foreground covers the pixel at that moment, the deep sky stack
           where the moment shows sky (alpha from the window's own brightness)

Static foreground and clear sky are identical in all variants. Moving foreground is where
the matte is not clear sky and temporal scatter exceeds the robust noise by 1.5×.

Usage: .venv/bin/python tools/proto/run_foreground.py ITERATION2_WORK OUT_DIR [--half 4] [--sky ITERATION3_WORK]
"""
import argparse
import json
import pickle
import sys
from pathlib import Path

import numpy as np
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import fullres, moment, preview, stack, vignetting  # noqa: E402


def variants(frames, sep, two, sky, reference, half, flat):
    alpha, stats = sep["alpha"], sep["stats"]
    moving = (alpha < stack.CLEAR_SKY) & (stats["scatter"] > 1.5 * np.maximum(stats["sigma"], 1e-3))
    moving = ndimage.binary_opening(moving, iterations=2)
    weight = ndimage.gaussian_filter(moving.astype(np.float32), 3)[..., None]
    still, count = moment.window_median(frames, reference, half)
    still /= flat[..., None]
    span = np.maximum(sep["background"] - sep["ground_level"], 1e-3)
    moment_alpha = np.clip((still.sum(axis=2) - sep["ground_level"]) / span, 0, 1)[..., None]
    a = alpha[..., None]
    deep = np.nan_to_num(sky)
    mean = two["final"]
    frozen = a * deep + (1 - a) * still
    hybrid = moment_alpha * deep + (1 - moment_alpha) * still
    blend = lambda image: (1 - weight) * mean + weight * image
    return {"mean": mean, "moment": blend(frozen), "hybrid": blend(hybrid)}, moving, count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("iteration2", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--half", type=int, default=4)
    parser.add_argument("--sky", type=Path, help="iteration 3 work directory")
    parser.add_argument("--crop", help="x,y of the 800×600 review crop (default: densest moving foreground)")
    args = parser.parse_args()
    manifest = json.loads((args.iteration2 / "cache" / "manifest.json").read_text())
    frames, meta = fullres.load_existing(args.iteration2 / "cache", manifest["names"])
    load = lambda work, name: pickle.loads((work / "stages" / f"{name}.pkl").read_bytes())
    two, sep, model = load(args.iteration2, "stack"), load(args.iteration2, "separation"), load(args.iteration2, "model")
    sky = load(args.sky, "stack")["sky"] if args.sky else two["sky"]
    flat = vignetting.recorded(load(args.iteration2, "background"), frames.shape[1:3])
    images, moving, count = variants(frames, sep, two, sky, model.reference, args.half, flat)
    args.out.mkdir(parents=True, exist_ok=True)
    wb = meta["wb"][model.reference]
    if args.crop:
        x0, y0 = (int(v) for v in args.crop.split(","))
    else:
        density = ndimage.uniform_filter(moving.astype(np.float32), 401)
        cy, cx = np.unravel_index(np.argmax(density), density.shape)
        y0 = int(np.clip(cy - 300, 0, frames.shape[1] - 600))
        x0 = int(np.clip(cx - 400, 0, frames.shape[2] - 800))
    for name, image in images.items():
        view = preview.stretch(image, wb, 20, 600)
        preview.save(args.out / f"foreground-{name}.jpg", view, 2000)
        preview.save(args.out / f"foreground-{name}-crop.jpg", preview.crop(view, x0, y0, 800, 600))
    report = {"moving_pixels": int(moving.sum()), "window_frames": count, "crop": [x0, y0, 800, 600],
              "sky": "iteration 3 (CFA)" if args.sky else "iteration 2"}
    (args.out / "report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report))


if __name__ == "__main__":
    main()
