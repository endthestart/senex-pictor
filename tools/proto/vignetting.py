"""Lens vignetting from the lensfun database (CC-BY-SA data, as bundled with darktable here):
an oracle and starting point until vignetting is self-calibrated from star motion.

lensfun's "pa" model: V(r) = 1 + k1 r² + k2 r⁴ + k3 r⁶, r from the image centre normalized so
that the corner is 1 for the lens's calibration crop factor.
"""
import xml.etree.ElementTree as ElementTree
from pathlib import Path

import numpy as np

LENSFUN = Path("/Applications/darktable.app/Contents/Resources/share/lensfun/version_1")


def profile(lens, focal, aperture, database=LENSFUN):
    """(k1, k2, k3) of the named lens at the calibrated aperture nearest to `aperture`
    (same focal length); raises when the lens or focal length has no vignetting data."""
    rows = []
    for path in sorted(database.glob("*.xml")):
        for entry in ElementTree.parse(path).getroot().iter("lens"):
            if _name(lens) not in [_name(m.text) for m in entry.findall("model")]:
                continue
            for v in entry.iter("vignetting"):
                if v.get("model") == "pa" and abs(float(v.get("focal")) - focal) < 0.5:
                    rows.append((abs(np.log(float(v.get("aperture")) / aperture)), float(v.get("aperture")),
                                 tuple(float(v.get(k)) for k in ("k1", "k2", "k3"))))
    if not rows:
        raise ValueError(f"lensfun has no vignetting for {lens} at {focal} mm")
    _, calibrated, k = min(rows)
    return k, calibrated


def _name(model):
    return model.lower().removeprefix("nikon ").strip()


def field(shape, k):
    """V(x) over an image of `shape` (H, W): 1 at the centre, k as in `profile`."""
    h, w = shape
    yy, xx = np.mgrid[0:h, 0:w]
    r2 = ((xx - (w - 1) / 2) ** 2 + (yy - (h - 1) / 2) ** 2) / (((w - 1) / 2) ** 2 + ((h - 1) / 2) ** 2)
    return (1 + k[0] * r2 + k[1] * r2 ** 2 + k[2] * r2 ** 3).astype(np.float32)


def recorded(background, shape):
    """The flat field an iteration 2 run used, from its background stage's record."""
    vignette = background.get("vignetting", {"vignetting": "none"})
    return field(shape, vignette["k"]) if vignette["vignetting"] == "lensfun" else np.ones(shape, np.float32)


def flat_for(source, raw_path, shape):
    """Flat field for a sequence: "none" (all ones) or "lensfun" (the raw's lens, focal length
    and aperture looked up as in `profile`). Returns the field and a description."""
    if source == "none":
        return np.ones(shape, np.float32), {"vignetting": "none"}
    if source != "lensfun":
        raise ValueError(f"unknown vignetting source {source}")
    import json
    import subprocess
    tags = json.loads(subprocess.run(["exiftool", "-j", "-n", "-LensID", "-FocalLength", "-FNumber", str(raw_path)],
                                     check=True, capture_output=True, text=True).stdout)[0]
    lens = subprocess.run(["exiftool", "-s3", "-LensID", str(raw_path)], check=True, capture_output=True, text=True).stdout.strip()
    k, calibrated = profile(lens, tags["FocalLength"], tags["FNumber"])
    return field(shape, k), {"vignetting": "lensfun", "lens": lens, "k": list(k), "calibrated_aperture": calibrated,
                             "aperture": tags["FNumber"]}
