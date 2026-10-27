"""Phase 1 research: star detection, tracking and sky-registration model comparison.

Reads RAW mosaics through rawpy/LibRaw, a research oracle and not a product dependency.
Coordinates are sensor pixels of the visible area; detection runs on 2x2 superpixels.
"""
import argparse
import json
from pathlib import Path

import astroalign
import numpy as np
import rawpy
import sep
from scipy.optimize import least_squares
from scipy.spatial import cKDTree
from skimage.measure import ransac
from skimage.transform import AffineTransform, ProjectiveTransform, SimilarityTransform

SIDEREAL_DEG_PER_S = 360.0 / 86164.0905


def detect(path):
    with rawpy.imread(str(path)) as r:
        raw = r.raw_image_visible.astype(np.float32)
        black = np.asarray(r.black_level_per_channel, np.float32)[r.raw_colors_visible]
        clipped = raw >= r.white_level
    h, w = (raw.shape[0] // 2) * 2, (raw.shape[1] // 2) * 2
    image = np.ascontiguousarray((raw - black)[:h, :w].reshape(h // 2, 2, w // 2, 2).sum(axis=(1, 3)))
    saturated = clipped[:h, :w].reshape(h // 2, 2, w // 2, 2).any(axis=(1, 3))
    background = sep.Background(image, bw=64, bh=64)
    signal = image - background
    objects = sep.extract(signal, 8.0, err=background.globalrms, minarea=4)
    good = (objects["flag"] == 0) & (objects["a"] < 3 * objects["b"]) & (objects["npix"] < 400)
    objects = objects[good]
    x, y, _ = sep.winpos(signal, objects["x"], objects["y"], np.full(len(objects), 1.0))
    near = lambda v, n: np.clip(np.round(v).astype(int), 0, n - 1)
    sat = saturated[near(y, saturated.shape[0]), near(x, saturated.shape[1])]
    return {"xy": np.column_stack([2 * x + 0.5, 2 * y + 0.5]), "flux": objects["flux"],
            "saturated": sat, "shape": raw.shape}


def track(frames):
    """Link detections frame to frame; returns tracks[track] = {frame: detection index}."""
    first, second = frames[0], frames[1]
    bright = lambda f: f["xy"][np.argsort(f["flux"])[::-1][:300]]
    step, _ = astroalign.find_transform(bright(first), bright(second))
    tracks, last = [], {}
    for j in range(len(first["xy"])):
        tracks.append({0: j})
        last[j] = len(tracks) - 1
    for i in range(1, len(frames)):
        prev, cur = frames[i - 1]["xy"], frames[i]["xy"]
        predicted = step(prev)
        dist, idx = cKDTree(cur).query(predicted, distance_upper_bound=4.0)
        back = cKDTree(predicted).query(cur[np.minimum(idx, len(cur) - 1)])[1]
        pairs = [(a, b) for a, (d, b) in enumerate(zip(dist, idx)) if np.isfinite(d) and back[a] == a]
        src = np.array([prev[a] for a, _ in pairs])
        dst = np.array([cur[b] for _, b in pairs])
        model, inliers = ransac((src, dst), SimilarityTransform, min_samples=3, residual_threshold=1.5, max_trials=200)
        step = model
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


def rays(points, f, center):
    v = np.column_stack([(points - center) / f, np.ones(len(points))])
    return v / np.linalg.norm(v, axis=1, keepdims=True)


def project(v, f, center):
    return center + f * v[:, :2] / v[:, 2:3]


def kabsch(a, b):
    u, _, vt = np.linalg.svd(b.T @ a)
    d = np.sign(np.linalg.det(u @ vt))
    return u @ np.diag([1, 1, d]) @ vt


def rotvec(v):
    theta = np.linalg.norm(v)
    if theta == 0:
        return np.eye(3)
    k = np.array([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]]) / theta
    return np.eye(3) + np.sin(theta) * k + (1 - np.cos(theta)) * k @ k


def radial(points, center, m, coeffs):
    """DNG WarpRectilinear radial term: corrected -> distorted positions."""
    d = (points - center) / m
    r2 = (d ** 2).sum(axis=1, keepdims=True)
    scale = coeffs[0] + coeffs[1] * r2 + coeffs[2] * r2 ** 2 + coeffs[3] * r2 ** 3
    return center + m * d * scale


def unradial(points, center, m, coeffs):
    u = points.copy()
    for _ in range(50):
        u = u + (points - radial(u, center, m, coeffs))
    return u


def sphere_model(train, center, m, f0, distortion):
    """distortion: None, ('fixed', coeffs) or 'fit' (kr0 = 1, kr1, kr2 fitted)."""
    (p0, pk) = train

    def unpack(x):
        coeffs = (1.0, x[4], x[5], 0.0) if distortion == "fit" else (distortion[1] if distortion else None)
        return rotvec(x[:3]), x[3], coeffs

    def predict(x, a):
        rot, f, coeffs = unpack(x)
        u = a if coeffs is None else unradial(a, center, m, coeffs)
        q = project(rays(u, f, center) @ rot.T, f, center)
        return q if coeffs is None else radial(q, center, m, coeffs)

    start_rot = kabsch(rays(p0, f0, center), rays(pk, f0, center))
    angle = np.arccos(np.clip((np.trace(start_rot) - 1) / 2, -1, 1))
    axis = np.array([start_rot[2, 1] - start_rot[1, 2], start_rot[0, 2] - start_rot[2, 0], start_rot[1, 0] - start_rot[0, 1]])
    x0 = np.concatenate([axis / (np.linalg.norm(axis) or 1) * angle, [f0], [0.0, 0.0] if distortion == "fit" else []])
    fit = least_squares(lambda x: (predict(x, p0) - pk).ravel(), x0, loss="soft_l1", f_scale=1.0)
    return (lambda a: predict(fit.x, a)), {"angle_deg": float(np.degrees(np.linalg.norm(fit.x[:3]))),
                                            "focal_px": float(fit.x[3]),
                                            "distortion": list(map(float, fit.x[4:])) if distortion == "fit" else None}


def zones(points, errors, center, m):
    rho = np.linalg.norm(points - center, axis=1) / m
    out = {}
    for name, lo, hi in [("center", 0, 0.35), ("mid", 0.35, 0.7), ("outer", 0.7, 1.01)]:
        sel = (rho >= lo) & (rho < hi)
        out[name] = {"stars": int(sel.sum()),
                     "rms_px": float(np.sqrt((errors[sel] ** 2).sum(axis=1).mean())) if sel.any() else None}
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("frames", type=Path, nargs="+")
    parser.add_argument("--focal-mm", type=float, required=True)
    parser.add_argument("--pitch-um", type=float, required=True)
    parser.add_argument("--interval-s", type=float, required=True, help="capture spacing between frames")
    parser.add_argument("--warp", type=float, nargs=6, metavar=("KR0", "KR1", "KR2", "KR3", "CX", "CY"))
    parser.add_argument("--pairs", type=int, nargs="+", required=True, help="frame offsets from frame 0")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    frames = [detect(p) for p in args.frames]
    tracks = track(frames)
    height, width = frames[0]["shape"]
    hat = np.array(args.warp[4:]) if args.warp else np.array([0.5, 0.5])
    center = hat * np.array([width - 1, height - 1])
    m = np.hypot(max(center[0], width - 1 - center[0]), max(center[1], height - 1 - center[1]))
    f0 = args.focal_mm / (args.pitch_um / 1000.0)
    rng = np.random.default_rng(1)
    results = {"frames": len(frames), "stars_per_frame": [len(f["xy"]) for f in frames],
               "tracks": len(tracks), "focal_px_nominal": f0, "pairs": []}
    for k in args.pairs:
        both = [t for t in tracks if 0 in t and k in t
                and not frames[0]["saturated"][t[0]] and not frames[k]["saturated"][t[k]]]
        p0 = np.array([frames[0]["xy"][t[0]] for t in both])
        pk = np.array([frames[k]["xy"][t[k]] for t in both])
        test = rng.random(len(both)) < 0.3
        train = (p0[~test], pk[~test])
        models = {}
        for name, cls in [("similarity", SimilarityTransform), ("affine", AffineTransform), ("homography", ProjectiveTransform)]:
            t = cls()
            t.estimate(*train)
            models[name] = (t, {})
        models["sphere_pinhole"] = sphere_model(train, center, m, f0, None)
        if args.warp:
            models["sphere_camera_warp"] = sphere_model(train, center, m, f0, ("fixed", tuple(args.warp[:4])))
        models["sphere_fitted_radial"] = sphere_model(train, center, m, f0, "fit")
        pair = {"offset": k, "seconds": k * args.interval_s,
                "sidereal_deg": k * args.interval_s * SIDEREAL_DEG_PER_S, "stars": len(both),
                "held_out": int(test.sum()), "models": {}}
        for name, (predict, info) in models.items():
            errors = predict(p0[test]) - pk[test]
            pair["models"][name] = info | zones(pk[test], errors, center, m)
        results["pairs"].append(pair)
        print(f"offset {k} ({pair['seconds'] / 60:.1f} min, sidereal {pair['sidereal_deg']:.2f} deg): {len(both)} stars, {test.sum()} held out")
        for name, r in pair["models"].items():
            extra = f" angle {r['angle_deg']:.3f} deg f {r['focal_px']:.1f}" if "angle_deg" in r else ""
            if r.get("distortion"):
                extra += f" k {r['distortion']}"
            print(f"  {name:22s} " + "  ".join(f"{z} {r[z]['rms_px']:.3f}px/{r[z]['stars']}" if r[z]["rms_px"] is not None else f"{z} -"
                                               for z in ("center", "mid", "outer")) + extra)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as stream:
        json.dump(results, stream, indent=2)


if __name__ == "__main__":
    main()
