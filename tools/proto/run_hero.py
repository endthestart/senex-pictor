"""Hero composite: one chosen frame as the photograph, the stacked sky behind it.

The hero frame keeps its own foreground, lighting and transient light (light painting,
headlights); stacked sky replaces it only where the matte is confidently sky (alpha above
0.5, fully at clear sky), so lit foreground that reads as half-bright never receives stars. The sky comes from a CFA stack of the whole sequence aligned to the hero's
moment, every frame with its own star-fitted rotation so camera drift cannot blur it. The
matte is built from the frames around the hero, where drift is small; for every other frame
it is moved by that frame's foreground displacement (a smooth field fitted to tracked
silhouette corners), with 5 px plus the fit's residual as the safety margin.

Usage: .venv/bin/python tools/proto/run_hero.py RAW_DIR ITERATION2_WORK CFA_WORK OUT_DIR
           --hero DSC_1259.NEF --first DSC_1256.NEF [--window 12]
ITERATION2_WORK holds the developed frames, stars and fixed-axis model of the sequence;
CFA_WORK holds its CFA cache (run_iteration3.py --stages cfa).
"""
import argparse
import json
import pickle
import sys
from pathlib import Path

import cv2
import numpy as np
from scipy import ndimage
from scipy.spatial.transform import Rotation
from skimage import morphology

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import background, cfa, drift, fullres, preview, render, stack, vignetting  # noqa: E402
from proto.background import sample as sample_glow  # noqa: E402
from proto.sky_model import SkyModel, rotation  # noqa: E402
from proto.stages2 import stage_separation  # noqa: E402

TRANSIENT_SIGMA = 20.0
TRANSIENT_MIN_PX = 500


def transient_light(hero, still, sky_noise):
    """Light present in the hero frame only (light painting): 5×5-smoothed luminance far above
    the window's median, as connected regions of at least TRANSIENT_MIN_PX, feathered 3 px."""
    excess = ndimage.uniform_filter(hero.sum(axis=2) - still.sum(axis=2), 5)
    bright = excess > TRANSIENT_SIGMA * sky_noise
    bright = morphology.remove_small_objects(bright, max_size=TRANSIENT_MIN_PX - 1)
    return ndimage.gaussian_filter(ndimage.binary_dilation(bright, iterations=3).astype(np.float32), 3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("raw", "work", "cfa_work", "out"):
        parser.add_argument(name, type=Path)
    parser.add_argument("--hero", required=True)
    parser.add_argument("--first", required=True)
    parser.add_argument("--window", type=int, default=12, help="frames from --first used for the matte")
    args = parser.parse_args()
    manifest = json.loads((args.work / "cache" / "manifest.json").read_text())
    names = manifest["names"]
    frames, meta = fullres.load_existing(args.work / "cache", names)
    load = lambda name: pickle.loads((args.work / "stages" / f"{name}.pkl").read_bytes())
    stars, fixed = load("stars"), load("model")
    hero, first = names.index(args.hero), names.index(args.first)
    use = list(range(first, len(names)))
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    report = {"hero": args.hero, "first": args.first, "frames_used": len(use)}

    start = [Rotation.from_matrix(rotation(fixed.axis, fixed.angles[j] - fixed.angles[hero])).as_rotvec() for j in range(len(names))]
    rotations = drift.free_rotations(stars["detections"], stars["tracks"], fixed.camera, hero, start)
    model = SkyModel(fixed.camera, fixed.axis, fixed.angles - fixed.angles[hero], hero, fixed.to_model, fixed.to_image,
                     rotations=rotations)

    w0, w1 = first, min(len(names), first + args.window)
    window_model = SkyModel(fixed.camera, fixed.axis, fixed.angles[w0:w1], hero - w0, fixed.to_model, fixed.to_image)
    window = {"frames": frames[w0:w1], "model": window_model, "stage_dir": out / "window", "report": report}
    window["stage_dir"].mkdir(exist_ok=True)
    sep = stage_separation(window)
    alpha = sep["alpha"]

    fields = drift.foreground_displacement(frames, alpha <= 0.5, hero, use)
    report["foreground_fit_residual_p90_px"] = {names[i]: round(fields[i][1], 2) for i in use}
    distance = stack.clearance(alpha)
    h, w = alpha.shape
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    bits = np.memmap(out / "occlusion.bits", np.uint8, "w+", shape=(len(names), h, -(-w // 8)))
    widest = 0.0
    for i in range(len(names)):
        if i not in fields:
            bits[i] = np.packbits(np.ones((h, w), bool), axis=1)
            continue
        shift = drift.displacement_field(fields[i][0], (h, w))
        widest = max(widest, float(np.abs(shift).max()))
        moved = cv2.remap(distance, xx - shift[..., 0], yy - shift[..., 1], cv2.INTER_LINEAR,
                          borderMode=cv2.BORDER_CONSTANT, borderValue=0)
        bits[i] = np.packbits(moved < stack.CLEARANCE_PX + fields[i][1] + 1.0, axis=1)
    bits.flush()

    flat, report["vignetting"] = vignetting.flat_for("lensfun", args.raw / args.hero, (h, w))
    glow_alpha = np.where(distance >= stack.CLEARANCE_PX + widest, alpha, 0.0).astype(np.float32)
    medians, usable = background.block_medians(frames, glow_alpha, flat)
    medians[[i for i in range(len(names)) if i not in fields]] = np.nan
    glow, _, changes = background.decompose(medians, model)
    report["glow_iteration_rms_change_adu"] = changes

    raw, cmeta = cfa.load([args.raw / n for n in names], args.cfa_work / "cache", None)
    sky, kept, halves, clipped = cfa.stack_tiles(raw, cmeta, model, alpha, glow, hero, bits, flat)
    report["samples_per_colour_median_clear_sky"] = [float(np.median(kept[..., c][alpha >= 0.98])) for c in range(3)]

    hero_frame = np.asarray(frames[hero], np.float32) / flat[..., None]
    still = np.median(np.asarray(frames[w0:w1], np.float32), axis=0) / flat[..., None]
    noise = float(np.median(sep["stats"]["sigma"][alpha >= 0.98])) / 5
    transient = transient_light(hero_frame, still, noise)[..., None]
    confident = np.clip((alpha - 0.5) / (stack.CLEAR_SKY - 0.5), 0, 1)
    a = np.where(np.isfinite(sky).all(axis=2), confident, 0.0)[..., None]
    base = a * np.nan_to_num(sky) + (1 - a) * hero_frame
    final = (1 - transient) * base + transient * hero_frame
    report["transient_pixels"] = int((transient[..., 0] > 0.5).sum())

    wb = meta["wb"][hero]
    yy, xx = np.mgrid[0:h, 0:w]
    glow_hero = np.nan_to_num(sample_glow(glow[hero], np.stack([xx, yy], axis=-1).astype(np.float64)))
    matrix = render.camera_to_srgb(args.raw / args.hero)
    show = lambda image: preview.stretch(np.nan_to_num(image), wb, 20, 600)
    preview.save(out / "hero-frame.jpg", show(hero_frame), 2400)
    preview.save(out / "hero-composite.jpg", show(final), 2400)
    composite_view, frame_view = render.compare([final, hero_frame], alpha, glow_hero, wb, matrix, flat)
    preview.save(out / "hero-composite-finished.jpg", composite_view, 2400)
    preview.save(out / "hero-frame-finished.jpg", frame_view, 2400)
    preview.save(out / "hero-transient.jpg", preview.gray(transient[..., 0], 0, 1), 1600)
    preview.save(out / "hero-alpha.jpg", preview.gray(alpha, 0, 1), 1600)
    with (out / "hero.pkl").open("wb") as stream:
        pickle.dump({"final": final, "sky": sky, "alpha": alpha, "transient": transient[..., 0], "glow_hero": glow_hero,
                     "halves": halves}, stream, protocol=pickle.HIGHEST_PROTOCOL)
    (out / "report.json").write_text(json.dumps(report, indent=1, default=float))
    print(json.dumps({k: v for k, v in report.items() if k != "foreground_displacement_p90_px"}, default=float)[:1500])


if __name__ == "__main__":
    main()
