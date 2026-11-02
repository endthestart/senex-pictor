# Validation specification

## Evidence classes

Recognized container ≠ supported compression ≠ successful decode ≠ validated camera.
Record exact model, encoding, dimensions and evidence. Never generalize one Z6 II
file to Z8 HE/HE* or another Nikon variant.

## Corpus

Private data lives outside the repository under `$STAR_DATA/` (the owner's Work disk); the first
sequence's originals and sidecars are in `wild-river-2024-04-14/raw/`, excluded from distribution. Inventory
RAW files independently of ON1/XMP sidecars and movies. Record SHA-256, byte count,
metadata, sequence grouping, frame count and gaps. Preserve originals read-only.
Fast metadata inventory may precede full byte hashing; mark it incomplete until hashes
exist. Generated outputs and machine-specific results belong in `$STAR_DATA/validation/`.

Private external sets in `$STAR_DATA/validation/datasets/` (not redistributable): Humboldt
University Canon EOS 40D Heart Nebula and Pleiades lights/darks/flats/bias, archive
SHA-256 recorded in [status](status.md). The Z6 II is no longer available for new
captures; its 262 lights remain the nightscape reference.

Owner datasets, copied 2026-10-26 to `$STAR_DATA/` (private, never
redistributed; RAWs with the owner's ON1 sidecars, edited JPEG exports and final videos):

| Dataset | Frames | Capture | Role |
|---|---|---|---|
| wild-river-2024-04-14 | 262 | Z6 II, 24 mm f/4, ISO 1600, 20 s; heavy light pollution | Regression; owner-edited JPEGs for comparison |
| maplewood-2025-09-26 | 280 (+13) | Z6 II, 20 mm f/1.8, ISO 3200, 8 s; dark site, canopy framing | Primary: sky islands, Milky Way, red-lit frames |
| northern-lights-2025-11-12 | 1,036 | Z6 II, 20 mm f/1.8, ISO 1600, 6 s; aurora, clouds, aircraft | Dynamic sky; owner-edited JPEGs |
| bwca-2024-05-19 | 196 | Z6 II, 20 mm f/1.8, ISO 800, 20 s; dark sky into dawn | Dark segment; twilight glow stress test |
| yellowstone-2013-08-27 | 76 | D700, 24 mm f/2.8, ISO 800, 30 s; dark sky | Second camera |
| mott-2024-11-01 | 13 | Z6 II, 20 mm f/1.8, ISO 800, 25 s; light-painted bank | Lit-foreground reference |

Owner notes and findings (2026-10-26):
- Maplewood: windy, a cold front came through; the red-lit frames are probably the
  owner's red headlamp catching the trees.
- Yellowstone: the owner re-aimed the camera at the start (setup shots DSC_1245–1255,
  frame DSC_1259 is the light-painting favourite); the head then sagged about 1.2–1.5°
  over 37 minutes (D700, AF Nikkor 24mm f/2.8D, ISO 400 in EXIF).
- BWCA: dawn overexposes the last 84 frames (DSC_1044 on); the dark segment is DSC_0932–1043.
- Mott: purpose unknown; probably lit by the nearby road or the owner's lights before the
  time-lapse.
- Northern Lights: no favourite moment.
- Wild River: the owner's edits use one copied setting across the whole set.

Not copied yet: 2024-11-07 (1,801 frames, clouds, 55 GB) and a GoPro HERO8 DNG lapse.

Required cases: Z6 II, D750 and Z8 including available encodings; additional camera
families; dark/flat/bias sets; wide-angle corners; sky islands; moving leaves, water,
people, vehicles, clouds; flash foreground; clipped stars; low star counts; mismatch,
corruption, full disk, cancellation, missing files and long sequences.
Synthetic fixtures provide known star positions, PSFs, flux, noise and motion.
Public fixtures require explicit redistribution permission; never publish private captures.

## Measurements

Full-resolution output dimensions, crop and orientation; black/white arithmetic;
finite samples, negative preservation, saturation; color-transform provenance;
stellar centroid/PSF/FWHM/ellipticity across field; flux and color consistency;
background variance and pattern noise; rejection/coverage; horizon seams and ghosting.
Decoded levels are compared with pinned oracle databases (`tools/oracle_levels.py`,
[oracle-lock.json](oracle-lock.json)); disagreements are findings to resolve by
measurement or primary source, never copied values.
Set numerical thresholds after baseline measurement and before evaluating new methods.
Use held-out scenes. Compare competing tools with matching inputs and display settings.
Visual review complements numerical tests and cannot replace ground truth.

## Hardware

| Machine | Required evidence |
|---|---|
| Intel Mac, AMD GPUs | Primary CPU baseline, GPU enumeration/selection, memory, full-resolution processing. |
| M2 Mac laptop | CPU/GPU consistency, unified-memory pressure, sustained workload and responsiveness. |
| Linux, AMD GPU | Driver/device behavior, portable backend, CPU fallback and packaging. |
| Windows hardware | Installation and real processing before runtime support claims. |

Build checks begin across platforms early. This session's machine is not proof of
runtime behavior elsewhere. Record compiler, OS, source revisions, command, duration,
peak resident memory and settings. Separate cold/warm caches and build/run timings.

## Release gates

Meaningful tests cover normalization, transforms, masks, photometry and corruption;
end-to-end tests use real binaries and small synthetic fixtures. Real RAW runs are
explicit integration checks. No silent test skips: unavailable data/hardware is listed.
CPU/GPU tolerances are stage-specific; default output must remain faithful on CPU.
All known scientific shortcuts/fallbacks must be removed, rejected, or explicitly
marked experimental before a supported public workflow ships.
