# ADR 0010: Check the sequence and the camera's stability before processing

Status: proposed, 2026-10-26. Prototype evidence (`tools/proto/sequence.py`, `drift.py`,
`run_check.py`, `run_hero.py`); the owner asked for alignment to be ensured first.

## Context

A folder of night frames is rarely one clean sequence. Yellowstone (D700, 78 frames) holds
setup shots with other framings and a 404 s gap before the time-lapse; its tripod head then
sagged steadily. BWCA (196 frames) runs into dawn, and its last 84 frames are overexposed.
The fixed-tripod model (ADR 0008) silently degrades on both.

## Evidence

- Yellowstone, all 78 frames: tracking found no track longer than 3 frames (motion was
  predicted from the previous frame pair across gaps); after re-estimating each pair's
  motion, the joint fit still missed sidereal by up to 3.1° because setup frames with other
  framings entered it.
- Yellowstone from DSC_1253: fixed-axis angles 0.07° RMS from sidereal, held-out medians 2.5–
  11 px; a free rotation per frame fits the same stars to 0.24–0.37 px. The extra rotation
  grows smoothly to 0.26° (stars) while foreground silhouettes slide about 70 px (1.4°):
  a steady drift is mostly absorbed into a tilted fitted axis and cannot be measured from
  stars alone.
- Foreground motion is not one rigid camera rotation here (median residual up to 16 px for
  a rotation fit to tracked silhouette corners; homography and similarity also fail), so
  stabilizing frames by one warp is not possible for this scene. Silhouettes against a
  moving Milky Way also change their apparent edges.
- BWCA: from frame 111 the share of saturated RAW pixels rises to 98%. Saturation stored as
  JSON coordinate lists grew the frame manifest to 19 GB; it is now packed bits.

## Decision (proposed)

1. Every run starts with a sequence check: split at capture gaps (over 3× the typical
   interval and 60 s) and setting changes, flag overexposed frames (over 2% saturated),
   track the foreground frame to frame, flag steps that stand out from the run's own step
   scatter (or cannot be tracked) as camera moves, and report accumulated drift. The owner
   confirms or picks the run; the report says what was left out and why.
2. Sky alignment allows a free rotation per frame, fitted to stars, whenever the fixed-axis
   model's residuals or sidereal consistency show drift. The sky stays exact under drift.
3. Foreground under drift is not warped. Sky samples from each frame keep a margin from the
   reference matte of 5 px plus that frame's measured foreground displacement; the matte
   comes from frames near the reference.
4. A hero frame (one chosen exposure as the photograph, sky stacked behind it) is a
   first-class output: its foreground, lighting and transient light are kept as shot.

## Rust port notes (2026-10-26)

Registration declares drift when the held-out median in the centre zone of any reported
frame exceeds 1.5 px (owner sequences without drift stay below 0.7 px; Yellowstone reaches
2.5–11 px) and then fits a free rotation per frame. A steady drift alone is absorbed by a
tilted fitted axis; an accelerating one is not. Lit and overexposed frames (and the frames
after them) are not camera-move witnesses: the prototype split Maplewood at its headlamp-lit
DSC_4007 because neither the foreground nor the stars could be measured there.

## R7 notes (2026-10-27)

Decision 3 is revised for drifting cameras. With the foreground rendered as shot, a matte
from the whole sequence no longer fits: where the trees drifted it is part sky, part tree,
and blends stacked sky with an averaged foreground into dark halos and ghost trunks
(Yellowstone). When the foreground drifted more than 4 px (a quadratic in time fitted to every
frame's median corner motion, at its largest), the whole-sequence composite is built like the
hero composite around the reference frame: matte
from the frames around it, per-frame sky-sample margins, and as foreground the long exposure
of the unlit frames, each resampled where its camera turn moved it. The turn is three
rotation angles fitted robustly to the corners through the sky model's lens; the quadratic
field fitted to the same corners extrapolates by hundreds of pixels away from them (1290 px
at Yellowstone's sequence ends), so margins now use the turn as well. Frames whose corners
moved under 0.5 px are not resampled. Below 4 px of drift the whole-sequence stack is used
unchanged: there, per-frame motion is mostly branches and reflections swaying (BWCA: medians
of 1–6 px without a trend, turns scattering 1–15 px) and moving frames by it would only blur
the ground. Frames the
sequence check marks as lit by local light are left out of every long exposure; the hero
composite keeps light painting.

The hero's matte window is now centred on the hero frame (it started at the first frame
used), and transient light counts only where it lies mostly outside clear sky: the window
median is no reference where the sky moves, and a Milky Way patch in DSC_1259 had been taken
for transient light. Rendering blends the stretched sky and the as-shot foreground with the
same weight the composite gave the stacked sky.

## Open questions

Small deliberate camera adjustments are indistinguishable from drift at the step level
(Yellowstone DSC_1253–1255, about 2 px each); star residuals should confirm suspected moves.
Foreground tracking is noisy over water and dark forest (BWCA step scatter 2.2 px).
