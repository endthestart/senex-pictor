"""Moment composites for dynamic skies (aurora, moving cloud).

Aurora and cloud are camera-fixed and change within minutes, so a long stack smears them.
A moment composite keeps them as they were around one frame: the temporal median of a short
window of frames in camera coordinates (stars move between those frames and drop out of the
median), plus the stars of the deep registered sky stack (the stack minus its own smooth
background). Foreground comes from the full-sequence ground stack.
"""
import numpy as np
from scipy import ndimage

STAR_SCALE_PX = 15


def window_median(frames, centre, half):
    """Per-pixel median of frames centre-half..centre+half (clipped to the sequence)."""
    lo, hi = max(0, centre - half), min(len(frames), centre + half + 1)
    return np.median(np.asarray(frames[lo:hi], np.float32), axis=0), hi - lo


def star_layer(image, size=STAR_SCALE_PX, k=0.0):
    """Point sources: the image minus its per-channel median background (median filter of
    about `size` px, computed at half resolution), kept where it exceeds k robust σ."""
    import cv2
    filled = np.where(np.isfinite(image), image, np.nanmedian(image, axis=(0, 1))).astype(np.float32)
    h, w, _ = filled.shape
    half = cv2.resize(filled, (w // 2, h // 2), interpolation=cv2.INTER_AREA)
    smooth = np.stack([ndimage.median_filter(half[..., c], size=size // 2 + 1) for c in range(3)], axis=-1)
    residual = filled - cv2.resize(smooth, (w, h), interpolation=cv2.INTER_LINEAR)
    sigma = 1.4826 * np.median(np.abs(residual - np.median(residual, axis=(0, 1))), axis=(0, 1))
    return np.where(residual > np.maximum(k * sigma, 0), residual, 0)


def compose(frames, sky, ground, alpha, centre, half, flat):
    """alpha·(starless window median + deep stars) + (1 − alpha)·ground. Stars that linger
    in the window median (near the pole) are removed from it so they are not counted twice."""
    still, count = window_median(frames, centre, half)
    still = still / flat[..., None]
    still = still - star_layer(still, k=3.0)
    a = alpha[..., None]
    return a * (still + star_layer(sky)) + (1 - a) * ground, count
