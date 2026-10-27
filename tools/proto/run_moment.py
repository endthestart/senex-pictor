"""Moment composites for dynamic skies: short-window sky around chosen frames plus deep stars.

Usage: .venv/bin/python tools/proto/run_moment.py RAW_DIR ITERATION2_WORK OUT_DIR --centres 62 99 --halves 0 4 12
       [--sky ITERATION3_WORK]  (deep stars from the CFA stack instead of the demosaiced one)
"""
import argparse
import json
import pickle
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from proto import fullres, moment, preview, render, vignetting  # noqa: E402


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", type=Path)
    parser.add_argument("iteration2", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--centres", type=int, nargs="+", required=True)
    parser.add_argument("--halves", type=int, nargs="+", default=[0, 4, 12])
    parser.add_argument("--sky", type=Path, help="iteration 3 work directory")
    parser.add_argument("--black-percentile", type=float, default=75, help="sky luminance percentile rendered black")
    parser.add_argument("--stretch", type=float, default=3.0)
    parser.add_argument("--shadows", type=float, default=2.0, help="shadow curve power (higher: deeper blacks)")
    parser.add_argument("--foreground", type=float, default=0.3, help="gain on non-sky light (lit trees)")
    args = parser.parse_args()
    manifest = json.loads((args.iteration2 / "cache" / "manifest.json").read_text())
    frames, meta = fullres.load_existing(args.iteration2 / "cache", manifest["names"])
    load = lambda work, name: pickle.loads((work / "stages" / f"{name}.pkl").read_bytes())
    two, sep = load(args.iteration2, "stack"), load(args.iteration2, "separation")
    sky = load(args.sky, "stack")["sky"] if args.sky else two["sky"]
    args.out.mkdir(parents=True, exist_ok=True)
    flat = vignetting.recorded(load(args.iteration2, "background"), frames.shape[1:3])
    zero = np.zeros(flat.shape + (3,), np.float32)
    report = {"sky": "iteration 3 (CFA)" if args.sky else "iteration 2", "variants": []}
    for centre in args.centres:
        wb = meta["wb"][centre]
        for half in args.halves:
            image, count = moment.compose(frames, sky, two["ground"], sep["alpha"], centre, half, flat)
            name = f"moment-{manifest['names'][centre].split('.')[0]}-{count}frames"
            matrix = render.camera_to_srgb(args.raw / manifest["names"][centre])
            view, = render.compare([image], sep["alpha"], zero, wb, matrix, flat, black_percentile=args.black_percentile,
                                   white_percentile=99.95, stretch=args.stretch, shadows=args.shadows, saturation=1.4,
                                   foreground=args.foreground)
            preview.save(args.out / f"{name}.jpg", view, 2400)
            report["variants"].append({"name": name, "centre": centre, "frames": count,
                                       "seconds": float(meta["times_s"][min(len(frames) - 1, centre + half)]
                                                        - meta["times_s"][max(0, centre - half)])})
    (args.out / "report.json").write_text(json.dumps(report, indent=2))
    print(json.dumps(report))


if __name__ == "__main__":
    main()
