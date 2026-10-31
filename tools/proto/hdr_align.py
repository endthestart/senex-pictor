"""Prototype, alignment half of tools/proto/hdr.py: decode a RAW bracket linear and align it.

Each frame is decoded with LibRaw (rawpy) without white balance, so every channel clips at 1.0.
The others are aligned to the reference with tiled phase correlation and a robust affine fit,
first at quarter size, then refined at full size; each is scaled by its exposure measured
against the reference. Handheld parallax (near things shifting against far ones) is not
corrected: the merge's motion test leaves the reference alone where frames disagree. The aligned frames, their unclipped masks and the exposures are cached
under OUT/aligned so the merge can be tuned without aligning again.
"""
import json
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
import rawpy
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
    """Where the warped frame has unclipped data; outside the frame counts as missing."""
    matrix, offset = coef[:2].T, coef[2]
    return ndimage.affine_transform(mask.astype(np.float32), matrix, offset=offset, order=1, cval=0.0) > 0.999


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


def align_bracket(out, paths):
    """Aligns `paths` (reference first) and caches them under out/aligned."""
    cache = out / "aligned"
    cache.mkdir(parents=True, exist_ok=True)
    settings = exposure_settings(paths)
    ref_path = paths[0]
    t_ref, iso_ref, _ = settings[ref_path.name]
    ref, wb, matrix = decode(ref_path)
    ref_ok = ~(ref >= CLIP).any(axis=2)
    ref_l = luminance(ref)
    log(f"reference {ref_path.name}: {ref.shape[1]}x{ref.shape[0]}, clipped {100 * (1 - ref_ok.mean()):.2f}%")
    np.save(cache / f"{ref_path.stem}.npy", ref.astype(np.float16))
    np.save(cache / f"{ref_path.stem}-ok.npy", ref_ok)
    frames = [{"name": ref_path.stem, "shutter": t_ref, "iso": iso_ref, "gain": 1.0}]
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
        np.save(cache / f"{p.stem}.npy", img.astype(np.float16))
        np.save(cache / f"{p.stem}-ok.npy", ok)
        frames.append({"name": p.stem, "shutter": t, "iso": iso, "gain": gain})
    meta = {"reference": ref_path.stem, "frames": frames, "wb": wb.tolist(), "matrix": matrix.tolist()}
    (cache / "meta.json").write_text(json.dumps(meta, indent=1))
