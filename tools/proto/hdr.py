"""Prototype: merge a handheld RAW exposure bracket into one natural-looking photo.

usage: .venv/bin/python -I tools/proto/hdr.py OUT_DIR REFERENCE.NEF OTHER.NEF...

Decodes each frame linear (LibRaw through rawpy, no white balance yet, so every channel clips
at 1.0), aligns the others to the reference with tiled phase correlation and an affine fit,
scales each by its exposure measured against the reference, and merges in proportion to each
frame's light (shutter time) where it is neither clipped nor disagreeing with the reference
(something moved). The merged linear image is white-balanced, converted to sRGB and
tone-mapped with an edge-aware base/detail split that compresses only the large-scale range.
"""
import json
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
import rawpy
import tifffile
from PIL import Image
from scipy import ndimage

CLIP = 0.98
TILE = 128


START = time.time()


def log(msg):
    print(f"{time.time() - START:6.1f}s {msg}", flush=True)


def exposure_settings(paths):
    out = subprocess.run(["exiftool", "-j", "-n", "-ExposureTime", "-ISO", "-FNumber", *map(str, paths)],
                         check=True, capture_output=True, text=True).stdout
    return {Path(r["SourceFile"]).name: (r["ExposureTime"], r["ISO"], r["FNumber"]) for r in json.loads(out)}


def decode(path):
    with rawpy.imread(str(path)) as r:
        s = r.sizes
        cam = r.postprocess(user_flip=0, gamma=(1, 1), no_auto_bright=True, output_bps=16,
                            user_wb=[1.0, 1.0, 1.0, 1.0], output_color=rawpy.ColorSpace.raw)
        wb = np.array(r.camera_whitebalance[:3], dtype=np.float32)
        cam_xyz = np.array(r.rgb_xyz_matrix[:3], dtype=np.float64)
    y0, x0 = s.crop_top_margin, s.crop_left_margin
    cam = cam[y0:y0 + s.crop_height, x0:x0 + s.crop_width]
    img = cam.astype(np.float32) / 65535.0
    return img, wb / wb[1], camera_to_srgb(cam_xyz)


XYZ_FROM_SRGB = np.array([[0.412453, 0.357580, 0.180423],
                          [0.212671, 0.715160, 0.072169],
                          [0.019334, 0.119193, 0.950227]])


def camera_to_srgb(cam_xyz):
    """dcraw's rgb_cam: white-balanced camera RGB to linear sRGB, from the camera's XYZ matrix."""
    cam_rgb = cam_xyz @ XYZ_FROM_SRGB
    cam_rgb /= cam_rgb.sum(axis=1, keepdims=True)
    return np.linalg.inv(cam_rgb).astype(np.float32)


def luminance(img):
    return img.mean(axis=2)


def shrink(a, f):
    h, w = (a.shape[0] // f) * f, (a.shape[1] // f) * f
    return a[:h, :w].reshape(h // f, f, w // f, f).mean(axis=(1, 3))


def phase_shift(a, b):
    """Shift (dy, dx) that moves b onto a, with a sub-pixel peak, and the peak's strength."""
    win = np.outer(np.hanning(a.shape[0]), np.hanning(a.shape[1]))
    fa, fb = np.fft.fft2((a - a.mean()) * win), np.fft.fft2((b - b.mean()) * win)
    cross = fa * np.conj(fb)
    corr = np.fft.ifft2(cross / (np.abs(cross) + 1e-12)).real
    py, px = np.unravel_index(np.argmax(corr), corr.shape)
    def sub(c, p, n):
        m, z, q = c[(p - 1) % n], c[p], c[(p + 1) % n]
        d = m - 2 * z + q
        return p + (0.5 * (m - q) / d if d != 0 else 0.0)
    dy = sub(corr[:, px], py, corr.shape[0])
    dx = sub(corr[py, :], px, corr.shape[1])
    dy = dy - corr.shape[0] if dy > corr.shape[0] / 2 else dy
    dx = dx - corr.shape[1] if dx > corr.shape[1] / 2 else dx
    return dy, dx, corr[py, px]


def tile_shifts(ref, mov, valid, tile):
    pts, shifts = [], []
    for y in range(0, ref.shape[0] - tile + 1, tile // 2):
        for x in range(0, ref.shape[1] - tile + 1, tile // 2):
            if valid[y:y + tile, x:x + tile].mean() < 0.9:
                continue
            a, b = ref[y:y + tile, x:x + tile], mov[y:y + tile, x:x + tile]
            if a.std() < 0.02:
                continue
            dy, dx, peak = phase_shift(a, b)
            if peak > 0.1:
                pts.append((y + tile / 2, x + tile / 2))
                shifts.append((dy, dx))
    return np.array(pts), np.array(shifts)


def fit_affine(pts, shifts):
    """Affine map from reference coordinates to moving-frame coordinates, fitted robustly."""
    src = pts
    dst = pts - shifts
    keep = np.ones(len(src), bool)
    for _ in range(5):
        A = np.hstack([src[keep], np.ones((keep.sum(), 1))])
        coef, *_ = np.linalg.lstsq(A, dst[keep], rcond=None)
        resid = np.linalg.norm(np.hstack([src, np.ones((len(src), 1))]) @ coef - dst, axis=1)
        limit = max(3 * np.median(resid[keep]), 0.3)
        keep = resid < limit
    return coef, keep.sum(), len(src), float(np.median(resid[keep]))


def warp(img, coef):
    """Resample img (moving frame) onto the reference grid through the affine coef."""
    matrix = coef[:2].T
    offset = coef[2]
    out = np.empty_like(img)
    for c in range(img.shape[2]):
        out[..., c] = ndimage.affine_transform(img[..., c], matrix, offset=offset, order=1, mode="nearest")
    return out


def warp_mask(mask, coef):
    return warp(mask[..., None].astype(np.float32), coef)[..., 0] > 0.999


def align(ref_l, mov_l, ref_ok, mov_ok):
    """Affine (rows, cols) coefficients mapping reference pixels to moving-frame pixels."""
    f = 4
    a, b = np.log2(shrink(ref_l, f) + 1e-4), np.log2(shrink(mov_l, f) + 1e-4)
    a, b = a - np.median(a), b - np.median(b)
    ok = shrink((ref_ok & mov_ok).astype(np.float32), f) > 0.99
    pts, shifts = tile_shifts(a, b, ok, TILE)
    coef, kept, total, resid = fit_affine(pts, shifts)
    coef[2] *= f
    log(f"  coarse: {kept}/{total} tiles, residual {resid * f:.2f} px")
    return coef


def refine(ref_l, warped_l, ok, coef):
    a, b = np.log2(ref_l + 1e-4), np.log2(warped_l + 1e-4)
    a, b = a - np.median(a), b - np.median(b)
    pts, shifts = tile_shifts(a, b, ok, 256)
    fix, kept, total, resid = fit_affine(pts, shifts)
    log(f"  fine: {kept}/{total} tiles, residual {resid:.2f} px")
    # compose: reference -> (fix) -> warped grid -> (coef) -> moving frame
    M1, t1 = coef[:2].T, coef[2]
    M2, t2 = fix[:2].T, fix[2]
    M, t = M1 @ M2, M1 @ t2 + t1
    return np.vstack([M.T, t])


def guided_filter(guide, src, radius, eps):
    box = lambda x: ndimage.uniform_filter(x, size=2 * radius + 1, mode="reflect")
    mi, ms = box(guide), box(src)
    cov = box(guide * src) - mi * ms
    var = box(guide * guide) - mi * mi
    a = cov / (var + eps)
    b = ms - a * mi
    return box(a) * guide + box(b)


def tone_map(rgb, compress):
    """Natural rendering: compress only the large-scale log-luminance range, keep local detail."""
    y = np.maximum(rgb @ np.array([0.2126, 0.7152, 0.0722], np.float32), 1e-6)
    ly = np.log2(y)
    small = shrink(ly, 4)
    r = max(4, int(min(small.shape) * 0.02))
    base_small = guided_filter(small, small, r, 0.25)
    base = ndimage.zoom(base_small, (ly.shape[0] / small.shape[0], ly.shape[1] / small.shape[1]), order=1)
    base = base[:ly.shape[0], :ly.shape[1]]
    detail = ly - base
    anchor = np.percentile(base, 60)
    base2 = anchor + (base - anchor) * compress
    ly2 = base2 + detail
    key = 0.20 / 2 ** np.percentile(ly2, 50)
    y2 = 2 ** ly2 * key
    out = rgb * (y2 / y)[..., None]
    return out


def display(lin):
    """Soft shoulder above 0.8, then the sRGB curve, to 8 or 16 bits."""
    x = np.maximum(lin, 0)
    knee = 0.8
    over = x > knee
    x = np.where(over, knee + (1 - knee) * (1 - np.exp(-(x - knee) / (1 - knee))), x)
    x = np.clip(x, 0, 1)
    return np.where(x <= 0.0031308, 12.92 * x, 1.055 * x ** (1 / 2.4) - 0.055)


def save(path, srgb, jpeg_width=None):
    path = Path(path)
    tifffile.imwrite(path.with_suffix(".tif"), (srgb * 65535 + 0.5).astype(np.uint16), compression="zlib")
    im = Image.fromarray((srgb * 255 + 0.5).astype(np.uint8))
    im.save(path.with_suffix(".jpg"), quality=92)
    if jpeg_width:
        im.thumbnail((jpeg_width, jpeg_width))
        im.save(path.with_name(path.stem + "-small.jpg"), quality=88)


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    paths = [Path(p) for p in sys.argv[2:]]
    settings = exposure_settings(paths)
    ref_path = paths[0]
    t_ref, iso_ref, _ = settings[ref_path.name]
    ref, wb, matrix = decode(ref_path)
    ref_ok = ~(ref >= CLIP).any(axis=2)
    ref_l = luminance(ref)
    log(f"reference {ref_path.name}: {ref.shape[1]}x{ref.shape[0]}, clipped {100 * (1 - ref_ok.mean()):.2f}%")
    frames = [(ref_path.name, ref, ref_ok, settings[ref_path.name][0], 1.0)]
    for p in paths[1:]:
        raw, _, _ = decode(p)
        t, iso, _ = settings[p.name]
        nominal = (t * iso) / (t_ref * iso_ref)
        raw_ok = ~(raw >= CLIP).any(axis=2)
        log(f"{p.name}: 1/{1 / t:.0f} s ISO {iso}, nominal x{nominal:.3f}, clipped {100 * (1 - raw_ok.mean()):.2f}%")
        coef = align(ref_l, luminance(raw) / nominal, ref_ok, raw_ok)
        img, ok = warp(raw, coef), warp_mask(raw_ok, coef)
        both = ref_ok & ok & (ref_l > 0.02) & (luminance(img) > 0.02)
        coef = refine(ref_l, luminance(img) / nominal, both, coef)
        img, ok = warp(raw, coef), warp_mask(raw_ok, coef)
        del raw
        both = ref_ok & ok & (ref_l > 0.02) & (luminance(img) > 0.02)
        gain = float(np.median(luminance(img)[both] / ref_l[both]))
        log(f"  measured x{gain:.3f} against the reference")
        frames.append((p.name, img, ok, t, gain))
    # merge: per-frame radiance estimate img/gain, weighted by light gathered (shutter time)
    num = np.zeros_like(ref)
    den = np.zeros(ref.shape[:2], np.float32)
    ghost_report = []
    for name, img, ok, t, gain in frames:
        est = img / gain
        w = ok & (luminance(img) > 0.002)
        if name != ref_path.name:
            both = w & ref_ok & (ref_l > 0.01)
            diff = np.abs(np.log2((luminance(est) + 1e-4) / (ref_l + 1e-4)))
            diff = ndimage.uniform_filter(np.where(both, diff, 0), 15) / np.maximum(ndimage.uniform_filter(both.astype(np.float32), 15), 1e-3)
            moved = both & (diff > 0.25)
            moved = ndimage.binary_dilation(moved, iterations=8)
            ghost_report.append((name, float(moved.mean())))
            w = w & ~(moved & ref_ok)
        wt = w.astype(np.float32) * t
        num += est * wt[..., None]
        den += wt
    lost = den == 0
    merged = num / np.maximum(den, 1e-12)[..., None]
    darkest = min(frames, key=lambda f: f[4])
    merged[lost] = (darkest[1] / darkest[4])[lost]
    log(f"merged; ghosts excluded {ghost_report}; pixels with no unclipped frame {100 * lost.mean():.3f}%")
    np.save(out / "merged-cam.npy", merged.astype(np.float32))
    rgb = (merged * wb) @ matrix.T
    single = (ref * wb) @ matrix.T
    for name, compress in (("hdr", 0.55), ("hdr-gentle", 0.75)):
        save(out / name, display(tone_map(rgb, compress)), 1600)
    key = 0.20 / np.percentile(single @ np.array([0.2126, 0.7152, 0.0722]), 50)
    save(out / "single-0ev", display(single * key), 1600)
    log("written")


if __name__ == "__main__":
    main()
