# ADR 0008: Sky registration as a rotation of the celestial sphere

Status: proposed, 2026-10-25. Research evidence only; no product implementation.

## Context

A fixed tripod sees the sky rotate rigidly about the celestial pole at the sidereal
rate. Wide-angle lenses then move stars along curved, position-dependent paths, so a
single planar transform cannot align the whole field (processing specification:
corners and spatial residuals matter).

## Evidence

`tools/star_motion.py` (research tool; rawpy/LibRaw, sep, astroalign) on all 262 Z6 II
frames (NIKKOR Z 24-70 mm f/4 S at 24 mm, 20 s, 23.0 s spacing, 100 min): about 5,000
stars per frame on 2×2 superpixels, tracked frame to frame. Models were fitted between
frame 0 and later frames on 70% of tracked unsaturated stars and scored on the held-
out 30%, by normalized radius from the center (center < 0.35, mid, outer > 0.7).

| Span | Similarity center/mid/outer px | Homography | Sphere, pinhole | Sphere, camera warp | Sphere, fitted radial |
|---|---|---|---|---|---|
| 5 min | 0.85 / 1.30 / 2.59 | 0.47 / 0.34 / 0.57 | 0.49 / 0.34 / 0.64 | 0.35 / 0.28 / 0.60 | 0.35 / 0.25 / 0.20 |
| 20 min | 3.21 / 4.45 / 9.66 | 1.23 / 0.87 / 2.14 | 1.30 / 0.96 / 2.60 | 0.40 / 0.45 / 2.61 | 0.36 / 0.31 / 0.34 |
| 50 min | 7.86 / 10.4 / 18.4 | 2.75 / 2.21 / 4.98 | 2.87 / 2.38 / 6.47 | 0.51 / 0.66 / 3.39 | 0.43 / 0.42 / 0.54 |
| 100 min | 14.6 / 17.0 / 34.9 | 4.40 / 3.85 / 11.0 | 5.25 / 4.27 / 13.9 | 0.64 / 0.91 / 6.77 | 0.47 / 0.47 / 0.98 |

- Sphere models map pixels to rays (focal length, image-center principal point),
  rotate them, and reproject. "Camera warp" uses the decoder's DNG WarpRectilinear
  coefficients for the NEF; "fitted radial" self-calibrates kr1, kr2 (kr0 = 1) and the
  focal length from the star motion itself.
- Fitted rotation angles equal the sidereal rate times elapsed time (1.249°, 4.998°,
  12.494°, 25.079° versus 1.250°, 5.00°, 12.49°, 25.08°): the model is physical.
- Self-calibrated distortion is stable across spans (kr1 −0.115 to −0.117, kr2 +0.042
  to +0.046; focal 4,149–4,162 px versus 4,043 px nominal for 24 mm at 5.936 µm).
- The decoder's camera warp leaves 2.6–6.8 px in the outer field. Its conversion from
  Nikon maker-note coefficients is unverified, and the research tool's inversion
  overflowed for a few far-out points.

Phase 1 iteration 1 (`tools/proto/`, half resolution) fitted one focal length
(4,164 px), radial terms (kr1 −0.116, kr2 +0.044) and rotation axis jointly on every
tenth frame, then each frame's angle: angles match sidereal time within 0.003° RMS
(0.011° maximum, the 1 s timestamp resolution). Held-out residuals are 0.2–0.4 px near
the reference and up to 1.1 px at the sequence ends (one frame's mid-field RMS of
3.1 px is probably a few mismatched tracks, not yet verified; robust statistics are needed).

At full resolution (iteration 2, ~9,000 stars per frame) the same fit gives focal
4,164.7 px and angles within 0.0033° RMS; held-out median residuals are 0.15–0.21 px
near the reference and 0.29–1.16 px at the ends (start worse than end), consistent with
uncorrected atmospheric refraction. A refraction term needs the zenith direction.

On a second sequence (Maplewood, 20 mm, 280 frames) the unchanged fit gives focal
3,419.9 px (kr1 −0.048, kr2 +0.014), angles within 0.0018° RMS of sidereal and held-out
median residuals of 0.14–0.41 px. A few mismatched tracks inflate the first frame's RMS
to 88 px while its median stays at 0.38 px, so held-out reporting must use robust statistics.

Northern Lights (same 20 mm lens, 200 frames, bright aurora sky, ~3,100 stars per frame
mostly high in the frame) fitted its own distortion poorly: kr1 +0.020, kr2 −0.101, angles
0.014° RMS from sidereal, outer-field held-out medians of 4.5 px at the sequence ends. A
free 3-axis rotation per frame did not reduce residuals (the camera did not move); star
centroids are simply noisier (0.4–0.5 px between adjacent frames). With Maplewood's radial
terms held fixed and only focal length, axis and angles fitted, angles fall to 0.0044° RMS
and outer-field medians at the ends to 0.34–0.48 px.

## Decision (proposed)

1. Register sky samples by a rotation of rays, not by planar transforms.
2. A fixed-tripod sequence shares one lens model and one rotation axis; each frame's
   angle follows from its capture time and the sidereal rate. Fit these jointly over
   the sequence. Per-frame angles are a diagnostic, and timestamp errors must surface.
3. Lens intrinsics are self-calibrated from star motion. Maker-note or database models
   (lensfun) are initial values and oracles, never silent substitutes. A sequence with
   sparse or poorly distributed stars reuses the radial terms of a better-constrained
   sequence taken with the same lens (a per-lens profile), and says so in its report.
4. Warp each frame once, composing undistort, unproject, rotate, project and distort,
   to avoid repeated resampling.
5. Report held-out residuals by field zone, sidereal-rate consistency and star counts
   with every registration.

## Open questions

Atmospheric refraction near the horizon (outer residual grows with span), principal
point and tangential terms, per-color lateral chromatic aberration, trailing within
each 20 s exposure (up to about 6 px at the celestial equator for this lens, an
estimate rather than a measurement), foreground occlusion and sky masks, sequences
with clouds or few stars, and other lenses and cameras. Acceptance needs predeclared
residual thresholds, held-out scenes and synthetic truth (plan M2 gate).
