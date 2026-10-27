"""Camera drift: a sagging or creeping tripod head turns (and can shift) the camera slowly.

Stars give each frame's full sky rotation F_i relative to the reference precisely, so the sky
stays exact under drift when every frame keeps its own rotation (SkyModel.rotations). The
foreground does not follow one rigid motion when the camera also shifts or when silhouettes
change against a moving sky, so foreground handling uses each frame's measured foreground
displacement as a safety margin instead of a warp.
"""
import cv2
import numpy as np
from scipy.optimize import least_squares

from proto.sky_model import rotation
from proto.stars import correspondences, identity


def _rotvec(v):
    angle = np.linalg.norm(v)
    return np.eye(3) if angle < 1e-15 else rotation(v / angle, angle)


def free_rotations(detections, tracks, camera, reference, start):
    """Per frame the rotation F_i with rays_i = rays_ref @ F_i.T, fitted to tracked stars."""
    out = []
    for j in range(len(detections)):
        if j == reference:
            out.append(np.eye(3))
            continue
        a, b = correspondences(detections, tracks, reference, j, identity)
        if len(a) < 10:
            raise ValueError(f"frame {j} shares only {len(a)} tracked stars with the reference")
        rays = camera.rays(a)
        fit = least_squares(lambda v: (camera.pixels(rays @ _rotvec(v).T) - b).ravel(), start[j],
                            loss="soft_l1", f_scale=1.0)
        out.append(_rotvec(fit.x))
    return np.stack(out)


def _features(frame):
    """Half-resolution 8-bit log luminance with stars median-filtered away, so that tracked
    features sit on foreground silhouettes rather than on stars that move with the sky."""
    lum = np.asarray(frame, np.float32).sum(axis=2)
    lum = cv2.resize(lum, (lum.shape[1] // 2, lum.shape[0] // 2), interpolation=cv2.INTER_AREA)
    g = np.log1p(np.clip(lum, 0, None) / 20.0)
    g = cv2.normalize(g, None, 0, 255, cv2.NORM_MINMAX).astype(np.uint8)
    return cv2.medianBlur(cv2.medianBlur(g, 5), 5)


def _terms(x, y, scale):
    u, v = x / scale - 1.0, y / scale - 1.0
    return np.stack([np.ones_like(u), u, v, u * u, u * v, v * v], axis=-1)


def foreground_displacement(frames, ground, reference, indices, count=3000):
    """Per frame in `indices`, a smooth field (quadratic in x, y) fitted robustly to the
    displacement of foreground corners tracked from the reference (Lucas–Kanade with a
    forward–backward check, each frame started from the nearest frame already tracked).
    Returns {frame: (coefficients (2, 6), 90th-percentile fit residual px, corners tracked)}
    in full-resolution pixels; evaluate with `displacement_field`."""
    target = _features(frames[reference])
    scale = frames.shape[2] / 2.0
    mask = cv2.resize(ground.astype(np.uint8), target.shape[::-1], interpolation=cv2.INTER_NEAREST)
    mask = cv2.dilate(mask, np.ones((9, 9), np.uint8)) & ~cv2.erode(mask, np.ones((25, 25), np.uint8))
    corners = cv2.goodFeaturesToTrack(target, count, 0.01, 12, mask=mask * 255).reshape(-1, 2).astype(np.float32)
    full = 2.0 * corners + 0.5
    shifts = {reference: np.zeros(2, np.float32)}
    out = {reference: (np.zeros((2, 6)), 0.0, len(corners))}
    for i in sorted(indices, key=lambda i: (abs(i - reference), i)):
        if i == reference:
            continue
        nearest = min(shifts, key=lambda j: abs(j - i))
        image = _features(frames[i])
        start = corners + shifts[nearest]
        forward, ok, _ = cv2.calcOpticalFlowPyrLK(target, image, corners, start.copy(), winSize=(31, 31), maxLevel=3,
                                                  flags=cv2.OPTFLOW_USE_INITIAL_FLOW)
        back, ok_back, _ = cv2.calcOpticalFlowPyrLK(image, target, forward, corners.copy(), winSize=(31, 31), maxLevel=3,
                                                    flags=cv2.OPTFLOW_USE_INITIAL_FLOW)
        good = (ok.ravel() == 1) & (ok_back.ravel() == 1) & (np.linalg.norm(back - corners, axis=1) < 0.5)
        if good.sum() < 20:
            raise ValueError(f"frame {i}: only {int(good.sum())} foreground corners tracked")
        moved = 2.0 * (forward[good] - corners[good])
        design = _terms(full[good, 0], full[good, 1], scale)
        keep = np.ones(len(moved), bool)
        for _ in range(3):
            coefficients = np.linalg.lstsq(design[keep], moved[keep], rcond=None)[0].T
            residual = np.linalg.norm(design @ coefficients.T - moved, axis=1)
            keep = residual < max(3.0 * 1.4826 * np.median(residual[keep]), 1.0)
        shifts[i] = np.median(forward[good] - corners[good], axis=0)
        out[i] = (coefficients, float(np.percentile(residual[keep], 90)), int(good.sum()))
    return out


def displacement_field(coefficients, shape, step=16):
    """Dense (H, W, 2) displacement from `foreground_displacement` coefficients."""
    h, w = shape
    gy, gx = np.mgrid[0:h + step:step, 0:w + step:step].astype(np.float64)
    coarse = _terms(gx, gy, w / 2.0) @ coefficients.T
    return cv2.resize(coarse.astype(np.float32), (gx.shape[1] * step, gx.shape[0] * step), interpolation=cv2.INTER_LINEAR)[:h, :w]
