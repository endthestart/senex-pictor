"""Iteration 2 (full resolution): Rust develop frames, sky model, matte, glow separation,
tiled Lanczos stacking, previews.

Usage: .venv/bin/python tools/proto/run_iteration2.py RAW_DIR WORK_DIR [--stages ...]
"""
import argparse
import json
import pickle
import subprocess
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import fullres, sky_model, stages, stars, vignetting  # noqa: E402
from proto.stages import log  # noqa: E402
from proto.stages2 import (stage_background, stage_occlusion, stage_previews, stage_separation,  # noqa: E402
                           stage_stack)


def stage_stars(ctx):
    detections = fullres.detect_all(ctx["cache"], ctx["names"])
    tracks = stars.track(detections, radius=4.0, threshold=1.5)
    counts = [len(d["xy"]) for d in detections]
    log(ctx["report"], "stars", {"per_frame_min": min(counts), "per_frame_median": int(np.median(counts)),
                                 "per_frame_max": max(counts), "tracks": len(tracks)})
    return {"detections": detections, "tracks": tracks}


def stage_model(ctx):
    s, times, ref = ctx["stars"], ctx["times"], ctx["reference"]
    subset = sorted(set(range(0, len(times), 10)) - {ref})
    height, width = ctx["frames"].shape[1:3]
    # starting focal length assumes a 36 mm wide full-frame sensor; the fit refines it
    f0, k0, lens = ctx["focal_mm"] * width / 36.0, (-0.116, 0.044), ctx["lens"]
    if lens is not None:
        f0, k0 = lens.camera.f, (lens.camera.k1, lens.camera.k2)
    model = sky_model.fit(s["detections"], s["tracks"], times, ref, subset, width, height, f0, k0,
                          to_model=stars.identity, to_image=stars.identity, fix_distortion=lens is not None)
    error = np.degrees(model.angles - sky_model.SIDEREAL_RAD_PER_S * (times - times[ref]))
    held = {int(i): sky_model.held_out(model, s["detections"], s["tracks"], int(i))
            for i in [0, len(times) // 4, ref - 1, ref + 1, 3 * len(times) // 4, len(times) - 1]}
    log(ctx["report"], "model", {"distortion": "fixed from " + str(ctx["lens_path"]) if lens is not None else "fitted",
                                 "focal_px": model.camera.f, "k1": model.camera.k1, "k2": model.camera.k2,
                                 "axis": model.axis.tolist(), "reference": ref,
                                 "angle_minus_sidereal_deg": {"max_abs": float(np.abs(error).max()),
                                                              "rms": float(np.sqrt((error ** 2).mean()))},
                                 "held_out": held})
    return model


STAGES = {"stars": stage_stars, "model": stage_model, "separation": stage_separation,
          "background": stage_background, "occlusion": stage_occlusion, "stack": stage_stack, "previews": stage_previews}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", type=Path)
    parser.add_argument("work", type=Path)
    parser.add_argument("--stages", nargs="+", choices=list(STAGES), default=list(STAGES))
    parser.add_argument("--reference", type=int, help="reference frame index (default: middle)")
    parser.add_argument("--vignetting", choices=["none", "lensfun"], default="none",
                        help="flat field applied to glow and sky samples")
    parser.add_argument("--lens-from", type=Path, help="model.pkl whose radial distortion is reused (same lens)")
    parser.add_argument("--first", help="first RAW file name of the sequence (default: first file)")
    parser.add_argument("--last", help="last RAW file name of the sequence (default: last file)")
    args = parser.parse_args()
    paths = sorted(p for p in args.raw.iterdir() if p.suffix.lower() == ".nef" and not p.name.startswith("."))
    names = [p.name for p in paths]
    paths = paths[names.index(args.first) if args.first else 0:(names.index(args.last) + 1) if args.last else None]
    directory = args.work / "cache"
    frames, meta = fullres.load(paths, directory)
    focal = json.loads(subprocess.run(["exiftool", "-j", "-n", "-FocalLength", str(paths[len(paths) // 2])],
                                      check=True, capture_output=True, text=True).stdout)[0]["FocalLength"]
    ctx = {"cache": directory, "names": [p.name for p in paths], "frames": frames, "meta": meta, "focal_mm": focal,
           "times": np.array(meta["times_s"]) - meta["times_s"][0], "work": args.work,
           "lens": pickle.loads(args.lens_from.read_bytes()) if args.lens_from else None, "lens_path": args.lens_from,
           "reference": args.reference if args.reference is not None else (len(paths) - 1) // 2}
    ctx["flat"], ctx["vignetting"] = vignetting.flat_for(args.vignetting, paths[0], frames.shape[1:3])
    stages.run(STAGES, ctx, args.work / "stages", args.stages)


if __name__ == "__main__":
    main()
