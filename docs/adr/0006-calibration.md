# ADR 0006: Sensor-stage calibration domain, order and matching

Status: proposed, 2026-10-25. Library core (`src/calibration/`) and synthetic-truth
tests exist; no CLI/end-to-end path. Acceptance needs real dark, bias and flat captures.

## Evidence

Primary: EMVA 1288 Release 4.0 Linear (2021) §4 Eq. 5, §3.1–3.2 Eq. 29–30;
ESO-MIDAS User Guide Vol. B, CCD reductions Eq. 3.2–3.6; DNG 1.6.0.0 ch. 5;
Nikon Z 7II/Z 6II online manual (NEF recording, Long exposure NR).
Tool documentation: Siril calibration/stacking/cosmetic pages, PixInsight "Master
Calibration Frames" tutorial, DeepSkyStacker technical details, Astro Pixel
Processor calibration principles, ccdproc and the Astropy CCD reduction guide.
Measurement blogs (unique evidence, lower weight): M. Shelley's Nikon Z6 review,
R. Clark on dark-current suppression. Not accessed: PixInsight ImageCalibration
reference, Howell's handbook, AAVSO DSLR manual, full texts of Dunlap 2010/2011.

- Linear sensor model: offset (DSNU) and slope (PRNU) per pixel (EMVA); calibrated
  value `(L − D)/(F − Dᶠ)` with both darks containing bias (ESO 3.6; Siril).
  Calibration precedes demosaic in every documented tool; Siril requires ADU.
- DNG: stored → LinearizationTable → subtract BlackLevel + DeltaH + DeltaV → scale
  1/(WhiteLevel − maximum plane black) → clip. OpcodeList2 follows that mapping,
  OpcodeList3 follows demosaic. Nikon Lossless NEF is reversible; maker note 0x96
  version 0x46 samples need no curve. Lossy NEF values pass through a curve.
- Dark scaling: Siril, PixInsight and DSS offer it; APP does not. DSLR dark current
  depends on temperature, exposure, ISO and frame sequence (Dunlap 2010 abstract);
  CMOS sense-node dark current does not scale with time (Dunlap 2011 abstract);
  EMVA notes many CMOS sensors depart from simple temperature models. Nikon EXIF
  carries no sensor temperature (ExifTool tables).
- Masters: clipped means beat medians for noise (Astropy guide); PixInsight uses
  Winsorized 3σ for bias/darks; flats are normalized multiplicatively per frame.
  Per-channel flat normalization avoids tinting (Siril "Equalize CFA").
- Local Z6 II (DSC_0226/0487; electronic shutter, NR off, ISO 1600): black 1008
  at all sites; 4.6–8.3% of samples below black (minimum 923), so no black clamp;
  14-bit codes used; no masked area decoded. Red and blue sites show near-empty
  codes about every 6 ADU (digital gain ≈1.2; Shelley reports ≈1.18 on the Z6),
  the second green site a weaker irregular pattern, the first green none.
  All 262 local frames are lights (green median 1060–1069 ADU).

## Decision

1. Calibrate each CFA site on the mosaic before demosaic, in linear ADU with the
   pedestal retained: after any linearization table or maker curve, before black
   subtraction, rescale or clipping. Never calibrate demosaiced, white-balanced,
   opcode-modified, rendered or clipped data.
2. `C = (L − b_L − D) / F̂`. Every frame first loses its own decoded black `b`
   (metadata or masked-pixel measurement), as overscan correction does for CCDs, so
   per-frame pedestal drift cancels. `D`: matched master of `dark − b`, holding residual
   pedestal structure and dark signal; without it dark signal is reported uncorrected.
   `F̂`: each `flat − b − Dᶠ` (flat-dark or bias master; none is allowed and disclosed)
   is divided by its per-color median, combined, then renormalized so each color's
   median is 1. Per-color (not per-site) normalization lets the flat correct
   green-site and digital-gain differences while preserving the light's color ratios.
3. The pedestal is removed exactly once. The API has no bias input for lights, so
   bias cannot be subtracted twice; bias substitutes only for a flat-dark.
4. No dark scaling or optimization. Darks match the lights' ISO and exposure exactly
   (no ISO interpolation: the Z6 changes conversion gain at ISO 800).
5. Saturation is flagged from stored values (≥ white level) before calibration;
   flat sites saturated in any flat are invalid. Values stay unclipped float in ADU
   above black; negatives are preserved. No per-frame rescale: decoders may infer
   white per frame, which would give frames of one sequence different gains.
6. Non-finite or sub-threshold `F̂` sites become invalid; they are never defaulted.
7. Masters use a recorded estimator: mean, or Winsorized σ-clipping (σ from the
   sample winsorized at 1.5σ about the median, ×1.134; reject beyond kσ; at least two
   samples kept). Bias/dark frames are not normalized. Each master keeps per-site
   accepted counts and standard errors. MAD-based σ was rejected: on integer ADU with
   12 frames it removed samples at 22% of sites (3% expected for Gaussian noise).
   Simulated clean darks: k = 3 costs 3–5% RMS versus truth with −0.04 to −0.07 ADU
   bias (skewed dark tail); k = 4 costs 1–2%. Default k awaits real dark measurements.
8. Hot/cold sites found from masters form a defect map. Same-color neighbor repair is
   a separate, opt-in, reported stage; non-defect sites are never filtered.
9. Reject masters differing in camera model, sensor/active-area size, CFA pattern,
   bit depth or black pattern, or whose mean black differs by more than 1% of the
   range (guards decoder fallbacks such as black 0), or for darks in ISO, exposure or
   in-camera NR. White level is not a matching key. Missing values needed for a
   rejection rule reject. Warn on differing or missing shutter mode, compression and,
   for darks, recorded temperature. The decoder does not expose these vendor fields;
   an end-to-end path must read them (for example Nikon maker-note tags 0x0034,
   0x0093, 0x0095; Canon CameraTemperature), or darks stay rejected. Flats need the
   same lens, focal length and aperture: absent on both captures (manual lenses,
   telescopes) is disclosed, absent on one side rejects. Focus is unrecorded and
   disclosed; flat ISO and orientation may differ (with warning). Camera serials are
   personal metadata and stay out of shared reports.
10. Report master standard errors per color. For static foreground pixels the master
    dark's noise is fully correlated across the stack: stack variance is about
    σ_L²/N_L + σ_D²/N_D, so N_D bounds foreground quality. Warning thresholds are set
    after baseline measurement.

Deferred until reviewed: LinearizationTable/BlackLevelDelta DNGs, sensor-stage
opcodes (for example GainMap), packed NEFs without a truncation guard, other cameras,
dark scaling, defect repair.

## Real calibration sets (Canon EOS 40D, private)

Humboldt University datasets (Santiago Rodriguez; no reuse terms, so private only):
Heart Nebula (ISO 800: 39 × 60 s lights, 17 darks, 10 × 1 s bias plus one 60 s frame
misfiled as bias, 17 ISO 100 flats) and Pleiades (ISO 1600: 73 × 45 s lights, 18 × 45 s
darks, 17 × 0.5 s bias; also unmatched ISO 640/75 s lights and 30 s darks).

- Matching worked as intended: the misfiled bias, mixed 30/45 s darks and ISO 640
  lights were rejected. The first implementation rejected every set because masked-
  column black differs per frame by up to 1 ADU, which led to decision 2.
- Bias frames sit 1.17 ADU (ISO 800) and 2.74 ADU (ISO 1600) above the masked-column
  black; masters carry that residual. Dark signal median: −0.16 and +0.19 ADU.
- Hot sites (dark − bias > 1000 ADU, about 300): darks 1 °C warmer and 3 h earlier
  over-subtracted them by a median 270–470 ADU (≈13%); darks 4–10 °C colder left
  +1,380–1,980 ADU (about one third uncorrected). Exact ISO/exposure matching is not
  sufficient; temperature governs hot-pixel residuals, supporting decision 8.
- Decoded white is a 16383 fallback for darks and ISO 800 lights although stars clip
  near 16003; ISO 1600 and 640 lights infer 15940–15942 and 12522–12523 per frame.
  RawTherapee camconst.json records the same per-ISO behavior (ISO 100 typical 13825).
- Loading all frames peaked at 3.6–5.2 GB; masters need bounded-memory loading.

## Open measurements

Dark behavior versus exposure and temperature over a session; in-camera hot-pixel
capping and its effect on undersampled stars; dark count needed for hundreds of
lights; flat dependence on aperture/focus; digital-gain factors by ISO; electronic
shutter effects. The Z6 II is no longer available; use the owner's other cameras.

## Synthetic-truth tests

`tests/calibration_*.rs` simulate pedestal, per-site offset, dark signal with hot
sites, per-color digital gain, per-color vignetting, PRNU, Poisson/read noise and
stars. They check master levels and standard errors, transient rejection, flat
background (corner/center within 4%), star flux (within 3%), hot-site removal,
flat-tint independence, unbiased negative noise, per-frame black drift, implausible
black rejection, white-level independence, saturation flags, invalid and saturated
flat sites, optics disclosure, temperature warnings and matching rules. Deliberate
faults (for example global flat normalization, clipped negatives, ignored dark or
flat, dark-only pedestal, unchecked ISO or black tolerance) each failed a test.

## Consequences

Calibration is optional and inspectable; master provenance and masks are retained.
Uncorrected dark signal is disclosed rather than estimated. Saturation flags are only
as reliable as decoded white levels; per-camera/ISO level data is an open decision
(see reuse audit). Acceptance requires the open measurements above.
