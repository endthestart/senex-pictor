"""Sky registration model (ADR 0008): rays rotated about one axis, self-calibrated lens."""
import numpy as np
from scipy.optimize import least_squares

from proto.stars import correspondences, to_sensor, to_superpixel

SIDEREAL_RAD_PER_S = 2 * np.pi / 86164.0905


class Camera:
    """Pinhole camera with a DNG-style radial term (kr0 = 1) about the image center."""

    def __init__(self, width, height, f, k1, k2):
        self.size = (width, height)
        self.center = np.array([(width - 1) / 2.0, (height - 1) / 2.0])
        cx, cy = self.center
        self.m = np.hypot(max(cx, width - 1 - cx), max(cy, height - 1 - cy))
        self.f, self.k1, self.k2 = f, k1, k2

    def distort(self, u):
        d = (u - self.center) / self.m
        r2 = (d * d).sum(-1, keepdims=True)
        return self.center + (u - self.center) * (1 + self.k1 * r2 + self.k2 * r2 * r2)

    def undistort(self, p):
        u = p.copy()
        for _ in range(30):
            u = u + (p - self.distort(u))
        return u

    def rays(self, p):
        u = self.undistort(p)
        v = np.concatenate([(u - self.center) / self.f, np.ones(u.shape[:-1] + (1,))], axis=-1)
        return v / np.linalg.norm(v, axis=-1, keepdims=True)

    def pixels(self, v):
        with np.errstate(divide="ignore", invalid="ignore"):
            u = self.center + self.f * v[..., :2] / v[..., 2:3]
        u[v[..., 2] <= 0] = np.nan
        return self.distort(u)


def rotation(axis, angle):
    k = np.array([[0, -axis[2], axis[1]], [axis[2], 0, -axis[0]], [-axis[1], axis[0], 0]])
    return np.eye(3) + np.sin(angle) * k + (1 - np.cos(angle)) * k @ k


def axis_from(theta, phi):
    return np.array([np.sin(theta) * np.cos(phi), np.sin(theta) * np.sin(phi), np.cos(theta)])


def kabsch(a, b):
    u, _, vt = np.linalg.svd(b.T @ a)
    return u @ np.diag([1, 1, np.sign(np.linalg.det(u @ vt))]) @ vt


class SkyModel:
    """Camera, rotation axis and per-frame angles relative to the reference frame. With
    `rotations` (one 3×3 per frame, fitted freely to stars) the sky follows those instead:
    a drifting camera adds its own turn to the sky's (see drift.py)."""

    rotations = None

    def __init__(self, camera, axis, angles, reference, to_model=to_sensor, to_image=to_superpixel, rotations=None):
        self.camera, self.axis, self.angles, self.reference = camera, axis, np.asarray(angles), reference
        self.to_model, self.to_image, self.rotations = to_model, to_image, rotations

    def rotation(self, frame):
        """R with rays_frame = rays_reference @ R.T."""
        return self.rotations[frame] if self.rotations is not None else rotation(self.axis, self.angles[frame])

    def predict(self, frame, reference_points):
        rays = self.camera.rays(reference_points) @ self.rotation(frame).T
        return self.camera.pixels(rays)

    def reference_rays(self, shape):
        """Rays of every reference superpixel center, computed once for all warps."""
        yy, xx = np.mgrid[0:shape[0], 0:shape[1]]
        return self.camera.rays(self.to_model(np.stack([xx, yy], axis=-1).astype(np.float64)))

    def source_map(self, frame, rays):
        """Superpixel coordinates in `frame` that land on each reference superpixel."""
        sp = self.to_image(self.camera.pixels(rays @ self.rotation(frame).T))
        return sp[..., 0].astype(np.float32), sp[..., 1].astype(np.float32)


def fit(stars, tracks, times, reference, subset, width, height, f0, k0, to_model=to_sensor, to_image=to_superpixel,
        fix_distortion=False):
    """Joint fit of focal length, radial terms, axis and angles on `subset` frames, then
    per-frame angles for every frame with the shared parameters fixed. With `fix_distortion`
    the radial terms stay at `k0` (a lens profile from a better-constrained sequence)."""
    pairs = {i: correspondences(stars, tracks, reference, i, to_model) for i in subset}
    far = max(subset, key=lambda i: abs(times[i] - times[reference]))
    cam = Camera(width, height, f0, *k0)
    r = kabsch(cam.rays(pairs[far][0]), cam.rays(pairs[far][1]))
    rv = np.array([r[2, 1] - r[1, 2], r[0, 2] - r[2, 0], r[1, 0] - r[0, 1]])
    axis = rv / np.linalg.norm(rv) * np.sign(times[far] - times[reference])
    theta, phi = np.arccos(axis[2]), np.arctan2(axis[1], axis[0])
    initial = [SIDEREAL_RAD_PER_S * (times[i] - times[reference]) for i in subset]

    def unpack(x):
        k, rest = (k0, x[1:]) if fix_distortion else (x[1:3], x[3:])
        return Camera(width, height, x[0], *k), axis_from(rest[0], rest[1]), rest[2:]

    def residual(x):
        camera, axis, angles = unpack(x)
        model = SkyModel(camera, axis, np.zeros(len(times)), reference, to_model, to_image)
        out = []
        for j, i in enumerate(subset):
            model.angles[i] = angles[j]
            out.append((model.predict(i, pairs[i][0]) - pairs[i][1]).ravel())
        return np.concatenate(out)

    x0 = np.array([f0, *([] if fix_distortion else k0), theta, phi, *initial])
    camera, axis, _ = unpack(least_squares(residual, x0, loss="soft_l1", f_scale=1.0, x_scale="jac").x)
    model = SkyModel(camera, axis, np.zeros(len(times)), reference, to_model, to_image)
    for i in range(len(times)):
        if i == reference:
            continue
        a, b = correspondences(stars, tracks, reference, i, to_model)

        def one(x, a=a, b=b, i=i):
            model.angles[i] = x[0]
            return (model.predict(i, a) - b).ravel()

        model.angles[i] = least_squares(one, [SIDEREAL_RAD_PER_S * (times[i] - times[reference])],
                                        loss="soft_l1", f_scale=1.0).x[0]
    return model


def held_out(model, stars, tracks, frame, seed=1):
    """Held-out residuals by field zone (RMS, median, 95th percentile) for 30% of stars when
    the frame's angle is refit on the other 70%."""
    a, b = correspondences(stars, tracks, model.reference, frame, model.to_model)
    test = np.random.default_rng(seed).random(len(a)) < 0.3
    saved = model.angles[frame]
    model.angles[frame] = least_squares(lambda x: (_with(model, frame, x[0]).predict(frame, a[~test]) - b[~test]).ravel(),
                                        [saved], loss="soft_l1", f_scale=1.0).x[0]
    errors = model.predict(frame, a[test]) - b[test]
    model.angles[frame] = saved
    rho = np.linalg.norm(b[test] - model.camera.center, axis=1) / model.camera.m
    zones = {}
    for name, lo, hi in [("center", 0, 0.35), ("mid", 0.35, 0.7), ("outer", 0.7, 1.01)]:
        sel = (rho >= lo) & (rho < hi)
        size = np.sqrt((errors[sel] ** 2).sum(1))
        zones[name] = {"stars": int(sel.sum()), "rms_px": float(np.sqrt((size ** 2).mean())) if sel.any() else None,
                       "median_px": float(np.median(size)) if sel.any() else None,
                       "p95_px": float(np.percentile(size, 95)) if sel.any() else None}
    return zones


def _with(model, frame, angle):
    model.angles[frame] = angle
    return model
