"""Star detection on superpixel images and frame-to-frame tracking."""
from concurrent.futures import ProcessPoolExecutor

import astroalign
import numpy as np
import sep
from scipy.spatial import cKDTree
from skimage.measure import ransac
from skimage.transform import SimilarityTransform


def detect_image(image, saturated, threshold=8.0):
    """Stars in a 2-D superpixel image: positions (superpixel coords), flux, saturation."""
    image = np.ascontiguousarray(image, dtype=np.float32)
    background = sep.Background(image, bw=64, bh=64)
    signal = image - background
    objects = sep.extract(signal, threshold, err=background.globalrms, minarea=4)
    good = (objects["flag"] == 0) & (objects["a"] < 3 * objects["b"]) & (objects["npix"] < 400)
    objects = objects[good]
    x, y, _ = sep.winpos(signal, objects["x"], objects["y"], np.full(len(objects), 1.0))
    near = lambda v, n: np.clip(np.round(v).astype(int), 0, n - 1)
    sat = saturated[near(y, saturated.shape[0]), near(x, saturated.shape[1])].astype(bool)
    return {"xy": np.column_stack([x, y]), "flux": objects["flux"], "saturated": sat}


def to_sensor(xy):
    """Superpixel (x, y) centers to sensor-pixel coordinates of the visible area."""
    return 2.0 * xy + 0.5


def to_superpixel(xy):
    return (xy - 0.5) / 2.0


def identity(xy):
    """Full-resolution images: image and model coordinates coincide."""
    return xy


def _detect_cached(job):
    from proto import frames as cache
    index, directory, names = job
    frames, saturated, _ = cache.load_existing(directory, names)
    return detect_image(frames[index].sum(axis=2), saturated[index])


def detect_all(directory, names, workers=24):
    with ProcessPoolExecutor(workers) as pool:
        return list(pool.map(_detect_cached, [(i, directory, names) for i in range(len(names))]))


def track(frames, radius=2.0, threshold=0.75):
    """Link detections frame to frame; returns tracks[track] = {frame: detection index}. Each
    pair's motion is found afresh by triangle matching of the brightest stars (frame gaps and
    uneven intervals change it); the previous pair's motion is the fallback when that fails."""
    bright = lambda f: f["xy"][np.argsort(f["flux"])[::-1][:300]]
    step, _ = astroalign.find_transform(bright(frames[0]), bright(frames[1]))
    tracks, last = [], {}
    for j in range(len(frames[0]["xy"])):
        tracks.append({0: j})
        last[j] = len(tracks) - 1
    for i in range(1, len(frames)):
        prev, cur = frames[i - 1]["xy"], frames[i]["xy"]
        try:
            step, _ = astroalign.find_transform(bright(frames[i - 1]), bright(frames[i]))
        except (astroalign.MaxIterError, ValueError):
            pass
        predicted = step(prev)
        dist, idx = cKDTree(cur).query(predicted, distance_upper_bound=radius)
        back = cKDTree(predicted).query(cur[np.minimum(idx, len(cur) - 1)])[1]
        pairs = [(a, b) for a, (d, b) in enumerate(zip(dist, idx)) if np.isfinite(d) and back[a] == a]
        src = np.array([prev[a] for a, _ in pairs])
        dst = np.array([cur[b] for _, b in pairs])
        step, inliers = ransac((src, dst), SimilarityTransform, min_samples=3, residual_threshold=threshold, max_trials=200)
        current = {}
        for (a, b), ok in zip(pairs, inliers):
            if ok and a in last:
                tracks[last[a]][i] = b
                current[b] = last[a]
        for b in range(len(cur)):
            if b not in current:
                tracks.append({i: b})
                current[b] = len(tracks) - 1
        last = current
    return tracks


def correspondences(frames, tracks, reference, frame, to_model=to_sensor):
    """Unsaturated (reference, frame) model-coordinate positions of stars tracked through both."""
    both = [t for t in tracks if reference in t and frame in t
            and not frames[reference]["saturated"][t[reference]] and not frames[frame]["saturated"][t[frame]]]
    a = np.array([frames[reference]["xy"][t[reference]] for t in both]).reshape(-1, 2)
    b = np.array([frames[frame]["xy"][t[frame]] for t in both]).reshape(-1, 2)
    return to_model(a), to_model(b)
