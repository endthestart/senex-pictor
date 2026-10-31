"""Prototype: merge a handheld RAW exposure bracket into one natural-looking photo.

usage: .venv/bin/python -I tools/proto/hdr.py align OUT_DIR REFERENCE.NEF OTHER.NEF...
       .venv/bin/python -I tools/proto/hdr.py merge OUT_DIR

`align` decodes and aligns the bracket and caches it (hdr_align.py). `merge` keeps the reference
frame wherever it is well exposed and brings the others in only where it needs them: darker
frames as it nears clipping, and in its deep shadows the frames that gathered more light (a
longer shutter; a higher ISO at the same shutter adds no light). There each pixel is the
light-weighted mean (shutter time) of the usable frames, leaving out a frame where it differs
from the reference (or, where the reference clips, from the darkest frame) by more than that
frame's own noise at that brightness allows, so things that moved do not double.
The result keeps the reference frame's midtones and brings its highlights (and a little of its
shadows) into range by compressing only the large-scale brightness, so local detail stays.
"""
import json
import sys
from pathlib import Path

import numpy as np
import tifffile
from PIL import Image
from scipy import ndimage

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hdr_align import CLIP, align_bracket, log, luminance, shrink  # noqa: E402

LUMA = np.array([0.2126, 0.7152, 0.0722], np.float32)
WINDOW = 15
NOISE_SIGMAS = 5.0
SHADOW = 0.01


def load(out):
    cache = out / "aligned"
    meta = json.loads((cache / "meta.json").read_text())
    frames = []
    for f in meta["frames"]:
        img = np.load(cache / f"{f['name']}.npy").astype(np.float32)
        ok = np.load(cache / f"{f['name']}-ok.npy")
        frames.append((f["name"], img, ok, f["shutter"], f["gain"]))
    return meta, frames


def clip_weight(img, ok):
    """1 well below clipping, falling smoothly to 0 at it, so frames hand over without seams."""
    top = img.max(axis=2)
    x = np.clip((CLIP - top) / (CLIP - 0.85), 0, 1)
    return (x * x * (3 - 2 * x)) * ok


def moved(est, anchor, valid):
    """Soft mask (0..1) of where est disagrees with anchor beyond its noise at that level."""
    level = luminance(anchor)
    rel = (luminance(est) - level) / (level + 1e-3)
    box = lambda a: ndimage.uniform_filter(a, WINDOW)
    v = valid.astype(np.float32)
    smooth = box(np.where(valid, rel, 0)) / np.maximum(box(v), 1e-3)
    bins = np.clip(np.floor(np.log2(np.maximum(level, 1e-4)) * 2), -28, 2).astype(np.int16)
    limit = np.zeros(level.shape, np.float32)
    centre = np.zeros(level.shape, np.float32)
    sample = valid[::5, ::5]
    for b in np.unique(bins[::5, ::5][sample]):
        values = smooth[::5, ::5][sample & (bins[::5, ::5] == b)]
        mid = np.median(values)
        spread = 1.4826 * np.median(np.abs(values - mid))
        here = bins == b
        centre[here] = mid
        limit[here] = NOISE_SIGMAS * spread + 0.03
    hard = valid & (np.abs(smooth - centre) > limit)
    hard = ndimage.binary_dilation(hard, iterations=4)
    return np.clip(ndimage.gaussian_filter(hard.astype(np.float32), 6) * 2, 0, 1)


def merge(frames, reference):
    ref = next(f for f in frames if f[0] == reference)
    darkest = min(frames, key=lambda f: f[4])
    ref_est, ref_ok = ref[1] / ref[4], ref[2]
    dark_est, dark_ok = darkest[1] / darkest[4], darkest[2]
    ref_weight = clip_weight(ref[1], ref_ok)
    ref_clean = ref_weight > 0.999
    feather = lambda a: np.clip(ndimage.gaussian_filter(a.astype(np.float32), 8), 0, 1)
    need_darker = feather(1 - ref_weight)
    need_brighter = feather(np.clip((SHADOW - luminance(ref_est)) / (0.8 * SHADOW), 0, 1))
    anchor = np.where(ref_clean[..., None], ref_est, dark_est)
    anchor_ok = ref_clean | dark_ok
    num = np.zeros_like(ref_est)
    den = np.zeros(ref_est.shape[:2], np.float32)
    report, masks = [], {}
    for name, img, ok, t, gain in frames:
        est = img / gain
        w = clip_weight(img, ok)
        if name != reference:
            w = w * (luminance(img) > 0.002) * (need_darker if gain < 1 else need_brighter * (t > ref[3]))
        is_anchor = (name == reference) | ((name == darkest[0]) & ~ref_clean)
        if name != reference:
            valid = ok & anchor_ok & (luminance(anchor) > 0.002) & ~is_anchor
            m = moved(est, anchor, valid)
            masks[name] = m
            report.append(f"{name} {100 * (m > 0.5).mean():.2f}%")
            w = w * np.where(is_anchor, 1.0, 1 - m)
        num += est * (w * t)[..., None]
        den += w * t
    lost = den < 1e-9
    merged = num / np.maximum(den, 1e-12)[..., None]
    merged[lost] = dark_est[lost]
    log(f"merged; moved: {', '.join(report)}; no usable frame {100 * lost.mean():.3f}%")
    return merged, lost, masks


def guided_filter(guide, radius, eps):
    box = lambda x: ndimage.uniform_filter(x, size=2 * radius + 1, mode="reflect")
    m = box(guide)
    var = box(guide * guide) - m * m
    a = var / (var + eps)
    b = m - a * m
    return box(a) * guide + box(b)


def soft_compress(x, knee, factor, width=0.5):
    """Log-domain curve: slope 1 below knee, `factor` above, blended over `width` stops."""
    d = (x - knee) / width
    return knee + width * (d + (factor - 1) * np.logaddexp(0, d))


def tone_map(rgb, key):
    """Keeps the reference's midtones (key); brings large-scale highlights and shadows in."""
    y = np.maximum(rgb @ LUMA, 1e-7) * key
    ly = np.log2(y)
    small = shrink(ly, 4)
    base_small = guided_filter(small, max(4, int(min(small.shape) * 0.02)), 0.25)
    base = ndimage.zoom(base_small, (ly.shape[0] / small.shape[0], ly.shape[1] / small.shape[1]), order=1)
    base = base[:ly.shape[0], :ly.shape[1]]
    high, white = np.log2(0.45), np.log2(0.92)
    top = np.percentile(base, 99.5)
    factor = float(np.clip((white - high) / max(top - high, 1e-3), 0.45, 1.0))
    lifted = -soft_compress(-base, -np.log2(0.03), 0.85)
    base2 = soft_compress(lifted, high, factor)
    log(f"tone map: highlights above {2 ** high:.2f} compressed x{factor:.2f}")
    y2 = 2 ** (base2 + ly - base)
    return rgb * key * (y2 / y)[..., None]


def display(lin):
    """Soft shoulder above 0.8, then the sRGB curve."""
    x = np.maximum(lin, 0)
    knee = 0.8
    x = np.where(x > knee, knee + (1 - knee) * (1 - np.exp(-(x - knee) / (1 - knee))), x)
    x = np.clip(x, 0, 1)
    return np.where(x <= 0.0031308, 12.92 * x, 1.055 * x ** (1 / 2.4) - 0.055)


def save(path, srgb, small=1600):
    path = Path(path)
    tifffile.imwrite(path.with_suffix(".tif"), (srgb * 65535 + 0.5).astype(np.uint16), compression="zlib")
    im = Image.fromarray((srgb * 255 + 0.5).astype(np.uint8))
    im.save(path.with_suffix(".jpg"), quality=92)
    im.thumbnail((small, small))
    im.save(path.with_name(path.stem + "-small.jpg"), quality=88)


def render(out):
    meta, frames = load(out)
    wb = np.array(meta["wb"], np.float32)
    matrix = np.array(meta["matrix"], np.float32)
    merged, lost, masks = merge(frames, meta["reference"])
    for name, m in masks.items():
        Image.fromarray((shrink(m, 8) * 255).astype(np.uint8)).save(out / f"moved-{name}.png")
    rgb = (merged * wb) @ matrix.T
    rgb[lost] = rgb[lost].max(axis=1, keepdims=True)
    single = (frames[0][1] * wb) @ matrix.T
    key = 0.20 / float(np.percentile(single @ LUMA, 50))
    save(out / "single", display(single * key))
    save(out / "hdr", display(tone_map(rgb, key)))
    log("written")


def main():
    command, out = sys.argv[1], Path(sys.argv[2])
    if command == "align":
        align_bracket(out, [Path(p) for p in sys.argv[3:]])
    elif command == "merge":
        render(out)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
