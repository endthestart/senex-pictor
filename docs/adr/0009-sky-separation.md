# ADR 0009: Sky/foreground separation from sky-coherent motion

Status: proposed, 2026-10-26. Prototype evidence on three owner sequences (`tools/proto/`).

## Context

A stack needs to know, per camera pixel and per frame, whether it saw sky. The owner
cannot draw masks; separation must be automatic (CPU first, GPU later). Iteration 2 used
star transits (a > 6σ excess over the pixel's temporal median), 20th-percentile brightness
against a fitted sky background, and SAM v1 segments. It worked on Wild River (still pines,
light pollution) and failed on Maplewood (canopy on all sides, wind): sky glimpses through
swaying leaves also exceed 6σ, so 86% of pixels counted as sky and foliage smeared into
the stack.

## Evidence

| Sequence | Transit test | Seen share | Sky clear / ground / edge | Notes |
|---|---|---|---|---|
| Wild River | any 6σ excess | 55.4% | 69.7 / 22.8 / 7.5% | reference matte |
| Wild River | sky-coherent | 55.2% | 69.6 / 23.1 / 7.3% | lit trunks and pine specks move to ground |
| Maplewood | any 6σ excess | 85.6% | 75.8 / 8.8 / 15.4% | canopy read as sky |
| Maplewood | sky-coherent | 50.1% | 58.3 / 21.3 / 20.4% | canopy ground; swaying canopy partial |
| Northern Lights | sky-coherent | 11.6% | 84.0 / 2.3 / 13.7% | bright aurora sky, ~3,100 stars per frame |

- Detection-gated transits (an excess counts only near a star detection that recurs in
  sky coordinates) rejected flicker but lost faint stars behind clutter: the "no transit
  within 6 px is ground" rule then cut bars through sky between birch branches.
- A pixel-level test keeps faint stars: an excess at x in frame i is a transit only if the
  sky model moves that sky point ≥ 2.5 px in a nearby frame (step 1–32 frames by local
  sky speed), a ≥ 3σ peak is found there and the original pixel has dropped below half of
  it. A pixel's transits count only when at least half its excess events cohere.
- With sparse transits (Northern Lights), absence of motion is not evidence: a fixed 6 px
  rule turned tree crowns into SAM-shaped blobs. The radius now follows the local transit
  gap (99th percentile over nearby brightness-clear sky, 128 px blocks, smoothed) and the
  rule is off beyond 32 px.
- The sky background fitted only to transit pixels was extrapolated across sparse-transit
  sky, leaving mottled partial alpha; refitting with featureless pixels that the first fit
  already calls nearly clear removes it.
- Swaying foliage can still cover pixels the matte calls clear in single frames. A
  per-frame occlusion bit (5×5-smoothed, glow-corrected luminance > 5σ below the pixel's own
  history, dilated 4 px) removes those samples: 0.07% of sky pixels per frame (median),
  1.6% in the windiest Maplewood frame.
- Behind the swaying canopy each output pixel draws its sky from a short run of frames, so
  anything that differs between runs shows as arcs along the star paths: uncorrected
  vignetting (Z 20mm f/1.8 corners at 30%; now a flat field on every sample) and local light
  near the trees in a few frames (DSC_3915–3920, DSC_4007: probably the owner's headlamp
  lighting mist or leaves). The occlusion test now also flags regions much brighter than
  their own history, larger than any star (opening by 15 px). The remaining arcs were glow
  left over in a short run of frames (26 frames, the first 10 minutes); sky seen by fewer
  than 30% of the frames now blends toward the frames around the reference, which match the
  foreground's moment, and the arcs are gone.
- Lanczos samples within 4 px of the frame border read zeros; each frame's border maps to
  a different arc in sky coordinates, visible as black arcs where coverage is thin. The
  border is now an obstruction for clearance.

## Decision (proposed)

1. Sky evidence is motion that follows the registration model, not brightness change.
2. Absence of motion means ground only where nearby clear sky shows dense transits.
3. Brightness gives fractional coverage; SAM segments refine where motion is silent.
4. Every sample also passes a per-frame occlusion test against its own pixel history.

## Open questions

Rendering of moving foreground (temporal mean blurs swaying leaves; a reference-window
frame is sharper but noisier) is a product choice. The pole disk (no measurable motion),
lit or pale foreground in sparse-transit skies, transparent canopy and water remain open.
