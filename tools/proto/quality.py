"""Stack quality measures shared by iteration comparisons: star size, ringing, per-channel
star width and pure-noise level from even/odd half stacks."""
import numpy as np
import sep


def _objects(lum, threshold=10.0):
    lum = np.ascontiguousarray(lum, np.float32)
    bkg = sep.Background(np.nan_to_num(lum), bw=64, bh=64)
    data = np.nan_to_num(lum) - bkg
    objects = sep.extract(data, threshold, err=bkg.globalrms, minarea=6)
    return data, bkg, objects[(objects["flag"] == 0) & (objects["peak"] < 0.5 * np.nanmax(data))]


def stars_in(image):
    """Median half-flux radius and elongation (a/b) of unsaturated stars in an RGB box."""
    data, _, objects = _objects(image.sum(axis=2))
    radius, _ = sep.flux_radius(data, objects["x"], objects["y"], np.full(len(objects), 6.0), 0.5)
    return {"stars": int(len(objects)), "half_flux_radius_px": float(np.median(radius)),
            "elongation": float(np.median(objects["a"] / objects["b"]))}


def ringing_and_colour(image, wb, brightest=30):
    """For the brightest unsaturated stars: the median undershoot below the local background
    within 2–6 px (ringing, as a fraction of the star's peak) and the median half-flux radius of
    each colour channel (a channel wider or narrower than the others shows as a coloured halo)."""
    rgb = np.nan_to_num(image) * np.asarray(wb, np.float32)
    data, _, objects = _objects(rgb.sum(axis=2))
    objects = objects[np.argsort(objects["flux"])[::-1][:brightest]]
    h, w = data.shape
    inside = (objects["x"] >= 6) & (objects["x"] < w - 7) & (objects["y"] >= 6) & (objects["y"] < h - 7)
    objects = objects[inside]
    if not len(objects):
        return {"stars": 0, "ringing_of_peak": None, "channel_hfr_px": None}
    yy, xx = np.mgrid[-6:7, -6:7]
    ring = (np.hypot(yy, xx) >= 2) & (np.hypot(yy, xx) <= 6)
    undershoot = [-data[int(round(o["y"])) - 6:int(round(o["y"])) + 7, int(round(o["x"])) - 6:int(round(o["x"])) + 7][ring].min()
                  / max(o["peak"], 1e-6) for o in objects]
    channel = []
    for c in range(3):
        plane = np.ascontiguousarray(rgb[..., c], np.float32)
        plane = plane - sep.Background(plane, bw=64, bh=64)
        radius, _ = sep.flux_radius(plane, objects["x"], objects["y"], np.full(len(objects), 6.0), 0.5)
        channel.append(round(float(np.median(radius)), 3))
    return {"stars": int(len(objects)), "ringing_of_peak": float(np.median(undershoot)), "channel_hfr_px": channel}


def half_noise(halves):
    """Robust σ of half the difference of the even/odd half stacks (luminance), in ADU."""
    diff = ((halves[0] - halves[1]) / 2).sum(axis=-1)
    diff = diff[np.isfinite(diff)]
    return float(1.4826 * np.median(np.abs(diff - np.median(diff)))) if diff.size else None
