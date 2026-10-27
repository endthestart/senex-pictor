"""Review renderings (not the product renderer): as-shot white balance and the camera's
Adobe colour matrix to linear sRGB; optionally the camera-fixed glow (light pollution,
airglow) removed by subtracting the reference frame's glow layer, which the stack separated
from sky light by its motion; part of the lens falloff put back; then levels shared across a
comparison, a black point at the darker sky, an asinh stretch and a shadow curve."""
import numpy as np
import rawpy

SRGB_TO_XYZ = np.array([[0.4124564, 0.3575761, 0.1804375],
                        [0.2126729, 0.7151522, 0.0721750],
                        [0.0193339, 0.1191920, 0.9503041]])


def camera_to_srgb(raw_path):
    """3×3 camera RGB (white-balanced) → linear sRGB with rows summing to 1, from the raw's
    XYZ→camera matrix (LibRaw rgb_xyz_matrix, Adobe ColorMatrix convention)."""
    with rawpy.imread(str(raw_path)) as raw:
        xyz_to_camera = np.asarray(raw.rgb_xyz_matrix[:3], np.float64)
    if not np.abs(xyz_to_camera).sum():
        raise ValueError(f"{raw_path} carries no colour matrix")
    srgb_to_camera = xyz_to_camera @ SRGB_TO_XYZ
    srgb_to_camera /= srgb_to_camera.sum(axis=1, keepdims=True)
    return np.linalg.inv(srgb_to_camera)


def encode(linear):
    a = np.clip(linear, 0, 1)
    return np.where(a <= 0.0031308, 12.92 * a, 1.055 * np.power(a, 1 / 2.4) - 0.055)


WEIGHTS = np.array([0.2126, 0.7152, 0.0722], np.float32)


def linear(image, alpha, glow_reference, wb, matrix, vignette=None):
    """Linear sRGB of a flat-fielded image with the camera-fixed glow removed from sky pixels
    (weighted by alpha); `vignette` (a field, 1 at the centre) puts back some natural falloff,
    so corrected corners do not show their amplified noise."""
    a = alpha[..., None]
    rgb = np.nan_to_num(image) - a * glow_reference
    if vignette is not None:
        rgb = rgb * vignette[..., None]
    return (rgb * np.asarray(wb, np.float32)) @ matrix.T.astype(np.float32)


def levels(rgb, alpha, black_percentile=30, white_percentile=99.9):
    """Black colour (median colour of the sky up to `black_percentile` in luminance, so the
    background goes black without a cast) and white scale. Compute once and pass the same
    levels to every image in a comparison."""
    sky = alpha >= 0.98
    level = rgb[sky] @ WEIGHTS
    black = np.median(rgb[sky][level <= np.percentile(level, black_percentile)], axis=0)
    return black, float(np.percentile(level - black @ WEIGHTS, white_percentile))


def finish(rgb, black, scale, stretch=12.0, saturation=1.2, shadows=1.4, depth=8):
    """sRGB at `depth` bits (8 or 16): subtract `black`, moderate saturation, asinh stretch to
    `scale`, then a shadow curve (power `shadows`) for deep, contrasty night skies."""
    rgb = rgb - black
    luminance = rgb @ WEIGHTS
    rgb = luminance[..., None] + saturation * (rgb - luminance[..., None])
    y = np.arcsinh(stretch * np.clip(rgb / scale, 0, None)) / np.arcsinh(stretch)
    top = {8: (255, np.uint8), 16: (65535, np.uint16)}[depth]
    return (encode(np.power(y, shadows)) * top[0] + 0.5).astype(top[1])


VIGNETTE_KEPT = 0.4


def compare(images, alpha, glow_reference, wb, matrix, flat, black_percentile=30, white_percentile=99.9,
            foreground=1.0, **options):
    """8-bit renderings of flat-fielded `images` with levels taken from the first, so a stack
    and a single frame are judged on the same black and white points. A fraction
    (VIGNETTE_KEPT) of the lens falloff described by `flat` is put back; `foreground` scales
    non-sky light (alpha 0) to calm trees lit by town light."""
    keep = np.power(flat, VIGNETTE_KEPT)
    gain = (alpha + (1 - alpha) * foreground)[..., None]
    rgbs = [linear(image, alpha, glow_reference, wb, matrix, keep) * gain for image in images]
    black, scale = levels(rgbs[0], alpha, black_percentile, white_percentile)
    return [finish(rgb, black, scale, **options) for rgb in rgbs]
