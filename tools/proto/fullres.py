"""Full-resolution frames from the Pictor `develop` command (AHD, camera RGB, negatives
kept), cached as float16 ADU above black; saturated RAW pixels as packed bits per frame."""
import json
import shutil
import subprocess
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
import rawpy
import tifffile

from proto.frames import capture_times

ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / "target/release/pictor"


def _develop(job):
    index, path, cache, shape = job
    scratch = cache / "develop" / f"{index:04d}"
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([str(BINARY), "develop", str(path), str(scratch)], check=True, capture_output=True)
    report = json.loads((scratch / "report.json").read_text())["development"]
    image = tifffile.imread(scratch / "camera-linear.tif")
    scale = report["white"][0] - sum(report["black"]["values"]) / len(report["black"]["values"])
    frames = np.memmap(cache / "frames.f16", np.float16, "r+", shape=shape + (3,))
    frames[index] = (image * scale).astype(np.float16)
    frames.flush()
    shutil.rmtree(scratch)
    crop = report["default_crop"]
    origin = report["active_area"]
    with rawpy.imread(str(path)) as r:
        clipped = r.raw_image >= r.white_level
        wb = np.asarray(r.camera_whitebalance[:3], np.float64)
    y0, x0 = origin["y"] + crop["y"], origin["x"] + crop["x"]
    clipped = clipped[y0:y0 + crop["height"], x0:x0 + crop["width"]]
    bits = np.memmap(cache / "saturated.bits", np.uint8, "r+", shape=_bits_shape(shape))
    bits[index] = np.packbits(clipped, axis=1)
    bits.flush()
    return {"scale_adu": scale, "wb": (wb / wb[1]).tolist(), "saturated_pixels": int(clipped.sum())}


def _bits_shape(shape):
    n, h, w = shape
    return n, h, -(-w // 8)


def load(paths, cache, workers=12):
    cache.mkdir(parents=True, exist_ok=True)
    manifest = cache / "manifest.json"
    names = [p.name for p in paths]
    if not manifest.exists():
        if not BINARY.exists():
            raise SystemExit("build the release binary first: cargo build --release")
        probe = json.loads(subprocess.run([str(BINARY), "inspect", str(paths[0])], check=True, capture_output=True, text=True).stdout)
        crop = probe["default_crop"]
        if probe["orientation"] not in ("Normal", "Rotate180"):
            raise SystemExit("portrait-oriented sequences are not supported by this prototype")
        shape = (len(paths), crop["height"], crop["width"])
        np.memmap(cache / "frames.f16", np.float16, "w+", shape=shape + (3,)).flush()
        np.memmap(cache / "saturated.bits", np.uint8, "w+", shape=_bits_shape(shape)).flush()
        with ProcessPoolExecutor(workers) as pool:
            results = list(pool.map(_develop, [(i, p, cache, shape) for i, p in enumerate(paths)]))
        meta = {"names": names, "shape": list(shape), "times_s": capture_times(paths),
                "scale_adu": [r["scale_adu"] for r in results], "wb": [r["wb"] for r in results],
                "saturated_pixels": [r["saturated_pixels"] for r in results], "source": "pictor develop (AHD)"}
        manifest.write_text(json.dumps(meta))
    return load_existing(cache, names)


def saturation_mask(cache, meta, index):
    n, h, w = meta["shape"]
    bits = np.memmap(cache / "saturated.bits", np.uint8, "r", shape=_bits_shape((n, h, w)))
    return np.unpackbits(np.asarray(bits[index]), axis=1)[:, :w].astype(bool)


def _detect(job):
    from proto import stars
    index, cache, names = job
    frames, meta = load_existing(cache, names)
    return stars.detect_image(np.asarray(frames[index], np.float32).sum(axis=2), saturation_mask(cache, meta, index))


def detect_all(cache, names, workers=24):
    with ProcessPoolExecutor(workers) as pool:
        return list(pool.map(_detect, [(i, cache, names) for i in range(len(names))]))


def load_existing(cache, names):
    meta = json.loads((cache / "manifest.json").read_text())
    if meta["names"] != list(names):
        raise SystemExit(f"cache {cache} holds a different frame list")
    return np.memmap(cache / "frames.f16", np.float16, "r", shape=tuple(meta["shape"]) + (3,)), meta
