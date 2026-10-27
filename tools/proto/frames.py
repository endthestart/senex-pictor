"""Half-resolution RGB superpixel frames from RAW mosaics, via rawpy/LibRaw (research oracle)."""
import json
import subprocess
from concurrent.futures import ProcessPoolExecutor
from pathlib import Path

import numpy as np
import rawpy

CHANNELS = 3


def superpixel(path):
    """Black-subtracted (R, mean G, B) per 2x2 block, and whether any site is clipped."""
    with rawpy.imread(str(path)) as r:
        raw = r.raw_image_visible.astype(np.float32)
        colors = r.raw_colors_visible
        black = np.asarray(r.black_level_per_channel, np.float32)[colors]
        clipped = raw >= r.white_level
        wb = np.asarray(r.camera_whitebalance[:3], np.float64)
        pattern = colors[:2, :2]
    h, w = (raw.shape[0] // 2) * 2, (raw.shape[1] // 2) * 2
    blocks = (raw - black)[:h, :w].reshape(h // 2, 2, w // 2, 2)
    rgb = np.zeros((h // 2, w // 2, CHANNELS), np.float32)
    for dy in range(2):
        for dx in range(2):
            c = int(pattern[dy, dx])
            channel = 1 if c == 3 else c
            rgb[..., channel] += blocks[:, dy, :, dx] * (0.5 if channel == 1 else 1.0)
    saturated = clipped[:h, :w].reshape(h // 2, 2, w // 2, 2).any(axis=(1, 3))
    return rgb, saturated, wb / wb[1]


def _fill(job):
    index, path, cache, shape = job
    rgb, saturated, wb = superpixel(path)
    frames = np.memmap(cache / "frames.f32", np.float32, "r+", shape=shape + (CHANNELS,))
    frames[index] = rgb
    frames.flush()
    clip = np.memmap(cache / "saturated.u8", np.uint8, "r+", shape=shape)
    clip[index] = saturated
    clip.flush()
    return wb.tolist()


def capture_times(paths):
    rows = json.loads(subprocess.check_output(["exiftool", "-j", "-n", "-DateTimeOriginal", "-SubSecTimeOriginal", *map(str, paths)]))
    by_name = {Path(r["SourceFile"]).name: r for r in rows}
    times = []
    for p in paths:
        stamp = by_name[p.name]["DateTimeOriginal"]
        date, clock = stamp.split(" ")
        hh, mm, ss = (int(v) for v in clock.split(":"))
        times.append(hh * 3600 + mm * 60 + ss + float("0." + str(by_name[p.name].get("SubSecTimeOriginal", 0))))
    return times


def load(paths, cache, workers=24):
    """Frames (N, H, W, 3) float32 and saturation (N, H, W) as memmaps, built once per cache."""
    cache.mkdir(parents=True, exist_ok=True)
    manifest = cache / "manifest.json"
    names = [p.name for p in paths]
    if manifest.exists():
        meta = json.loads(manifest.read_text())
        if meta["names"] != names:
            raise SystemExit(f"cache {cache} holds a different frame list")
    else:
        rgb, _, _ = superpixel(paths[0])
        shape = (len(paths),) + rgb.shape[:2]
        np.memmap(cache / "frames.f32", np.float32, "w+", shape=shape + (CHANNELS,)).flush()
        np.memmap(cache / "saturated.u8", np.uint8, "w+", shape=shape).flush()
        with ProcessPoolExecutor(workers) as pool:
            wbs = list(pool.map(_fill, [(i, p, cache, shape) for i, p in enumerate(paths)]))
        meta = {"names": names, "shape": list(shape), "wb": wbs, "times_s": capture_times(paths)}
        manifest.write_text(json.dumps(meta))
    return load_existing(cache, names)


def load_existing(cache, names):
    meta = json.loads((cache / "manifest.json").read_text())
    if meta["names"] != list(names):
        raise SystemExit(f"cache {cache} holds a different frame list")
    shape = tuple(meta["shape"])
    frames = np.memmap(cache / "frames.f32", np.float32, "r", shape=shape + (CHANNELS,))
    saturated = np.memmap(cache / "saturated.u8", np.uint8, "r", shape=shape)
    return frames, saturated, meta
