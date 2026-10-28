# Execution plan

2026-10-25. Gates determine completion; no speculative calendar estimates.

## Delivery phases (owner direction, 2026-10-25)

Phase 1 (current): reach the core nightscape logic first — M2 sky registration and
integration, then M3 foreground — on the Z6 II sequence, using the vendored decoder,
the existing calibration library and open-source research tools or oracles wherever
they unblock the work. M1 gates are deferred, not dropped; Phase 1 outputs remain
diagnostics, not release masters.

Phase 2: harden or rebuild decoding and calibration (decoder re-audit under ADR 0007,
independent per-ISO level data, end-to-end calibration), then complete the M1 gates.
Development-only installs are recorded in [install-log](install-log.md) and removed
when no longer needed.

Phase 1 iterations (owner decisions, 2026-10-25), each building on the last:

1. Half resolution: 2×2 RGGB superpixels as RGB, prototyped in Python (research venv).
   Sky/ground separation is fully algorithmic: per-pixel static-versus-sky-motion
   tests, star-occultation visibility, and an open-source segmentation model where
   motion evidence is weak. Sky aligned per [ADR 0008](adr/0008-sky-registration.md)
   to the middle frame (131 of 262); stacked with outlier rejection and a map of
   rejected samples (aircraft and satellite trails); coverage map; foreground averaged
   where stable with reference fallback. Each iteration is reviewed on a private
   artifact page. No external stack exists; judge by measurements and review.
2. Full resolution after demosaic (done 2026-10-26: per-frame glow separation, Lanczos-4,
   edge clearance, motion-first mask; see [status](status.md)).
3. Full resolution by stacking CFA samples directly (Bayer-drizzle style).

Order after iteration 2 (decided 2026-10-26, datasets in [validation](validation.md)):
run the iteration 2 pipeline unchanged on Maplewood to expose what does not generalize
(canopy on all sides, red-lit frames, a faster lens); then iteration 3 on Maplewood and
Wild River; then the dynamic sky of Northern Lights (aurora and cloud transients,
reference moment versus cleanup); then the D700 and BWCA twilight sets. Owner-edited
JPEGs serve later as rendering comparisons, not as ground truth.

Proven algorithms are then ported to Rust on CPU; the portable GPU layer follows the
Rust port, and platform-specific GPU work follows that.

Owner decisions after iteration 3 (2026-10-26, review page answers): moving foliage is a
per-photo choice with long-exposure blur as the default; aurora and other fast skies use a
9-frame moment (about 1 minute) with deep blacks; the hero frame (one chosen exposure as the
photograph, stacked sky behind it) is a main workflow; the smoother CFA kernel (σ 0.5 px) is
the default; next comes the Rust port. Rendering note from the owner (Yellowstone export): the
result reads as over-processed, the foreground pushed too far to match the sky; the renderer
should keep foreground exposure closer to the frame as shot.

## Rust port (decided 2026-10-26)

The Python prototype (`tools/proto/`) is the oracle: each milestone reproduces its outputs
on the owner sequences within stated tolerances before the next starts. Frame data is read
with positioned reads from tiled files (the crate forbids `unsafe`, so no memory maps), so
memory stays bounded on ordinary machines. Crates are chosen by ADR 0007 (open source,
licence recorded in the reuse audit); sep, astroalign and SAM stay oracles until ported.

- R1 Sequence and frames (done 2026-10-26): sequence check (capture gaps, settings,
  overexposure, lit frames, twilight ramps), frame and CFA stores, lensfun flat fields.
  Parity: stores bit-identical to the prototype caches (Z6 II and D700); runs, overexposed
  and lit frames match the prototype check; lensfun profiles identical for all three lenses.
- R2 Registration (done 2026-10-26): star detection (clean-room after Source Extractor),
  triangle matching (astroalign port), tracking, the sky model with fixed axis, free
  per-frame rotations under drift and lens-profile reuse; held-out reports; camera moves and
  drift in the sequence check. Parity: on the prototype's own stars the Rust solver
  reproduces its angles within 3e-7°; end to end, the Rust model fits the prototype's star
  pairs as well or better (median 0.200/0.199 px BWCA, 0.185/0.185 Maplewood, 0.220/0.220
  Wild River, 0.256/0.335 Northern Lights). The prototype's own angles move by up to 3.4e-4°
  when 10% of its stars are dropped, so 1e-4° end to end is below its noise. New: pairs more
  than 5× the median distance (2 px floor) off the fit are rejected (static lights tracked
  as stars when the sky moves under the pairing tolerance), drift is declared when held-out
  centre medians exceed 1.5 px, and lit frames are not camera-move witnesses.
- R3 Separation (done 2026-10-26): detrended temporal statistics, sky-coherent transits,
  matte (optional segments), glow decomposition, per-frame occlusion, foreground
  displacement for drift margins. SAM changes the matte on 0.07–1.1% of pixels (>0.5) and is
  not ported yet; segments can be supplied as a labels plane. Parity with the prototype's
  current algorithm and the same segments: matte within 0.5 on 99.89–100% of pixels (BWCA
  100%, Yellowstone 99.89%, Northern Lights 99.995%, Maplewood 99.97%, Wild River 100%),
  temporal medians bit-identical, transits equal on 99.8–100% of pixels, occlusion bits on
  99.999% (BWCA), glow within 0.0004 ADU.
- R4 Integration (done 2026-10-26): CFA (Bayer-drizzle) sky stack, ground stack, composite
  with coverage blending, hero and moment composites. Parity: with the prototype's sky model
  the BWCA stack matches to 1e-5 of the stack noise (99.3% identical sample sets); with
  Rust's own registration 0.02 of the noise, no bias. Northern Lights differs by 0.3 of the
  noise because Rust's registration rejects static lights: its stars are as sharp or
  sharper (top-left corner half-flux radius 2.14 px against 2.72). Hero: same sample counts
  and transient light; its glow now leaves out each frame's own moved-foreground margin
  (the prototype's frame-wide worst-case margin left almost no glow cells), so its sky
  differs by 0.4–1.0 of the noise.
- R5 Output (done 2026-10-26): review rendering (natural and dark looks), JPEG, 16-bit TIFF
  and linear float TIFF; commands `separate`, `stack`, `hero`, `moment`, `render` with JSON
  reports. Colour matrices are calibration files (generated with the LibRaw oracle for the
  Z6 II and D700; NEFs carry none).
- R6 GPU (done 2026-10-26, sky stack only): the CFA sky stack as a wgpu kernel (`--gpu yes`).
  Against the CPU: 0.0002–0.0004 of the stack noise at the median, 0.08–0.16 at the 99th
  percentile (32-bit geometry flips a few boundary samples). Speed with the cards idle
  (2026-10-27; the first figures were taken while other models ran on both cards), sky
  stack in seconds, CPU on 32 threads: Maplewood 91 CPU / 73 W5700X / 135 580X, BWCA 19 /
  11 / 15, Yellowstone 6.6 / 8.7 / 11. Band reads and uploads to the card limit it. Transit
  tests and warping stay on the CPU. `--gpu` takes `yes` or part of a card's name.
- R7 One command (2026-10-27): `process RAW_DIR OUT_DIR` runs check, ingest of the
  recommended run, register, separate, stack (or hero) and render. Memory budget
  (`src/budget.rs`, `--memory GB`, default 75% of the free memory): every parallel step
  runs as many workers as fit, at most one per core, and fails with what it needs when one
  does not; occlusion and hero masks stream to disk, sky-stack bands shrink to fit, outputs
  are byte-identical whatever the budget (Yellowstone, 32 workers in 6 GB against 8 before).
  Foreground as shot (owner decision): the sky is stretched, the foreground keeps its
  exposure. Lit frames are left out of the long-exposure ground. When the foreground drifted
  more than 4 px (Yellowstone), the composite is built around the reference frame with the
  long exposure moved back frame by frame by its camera turn (fitted to the tracked
  foreground through the lens model). Colour matrices for the Z6 II and D700 are built in.
  The working store takes about 200 MB per 24 MP frame; `process` checks the output volume
  before ingesting (Northern Lights' full 1,039-frame run would need 218 GB) and takes
  `--first`/`--last` to process part of a run.

Desktop app (M4, started 2026-10-27): research report (`reports/Star Photos app research.md`),
six screen designs (private design canvas), toolkit decided in ADR 0011 from the canvas spike
(45 MP on an M2 Max: under 1 ms of GPU per frame, 120 Hz in a window, 5 ms look recompute,
EDR confirmed). The app crate (`app/`) has all six steps, runs `process` with progress,
Cancel and Resume, and makes hero frames and moments; next is refining the look and feel in
a live window with the owner, then mask corrections (brush, candidate masks), comparing two
results side by side, and settings for the memory budget and GPU.

Gate per milestone: prototype parity on Wild River, Maplewood, Northern Lights, BWCA and
Yellowstone (registration angles within 1e-4°, matte agreement on 99.5% of pixels, stack
differences below a tenth of the stack noise), with run time and memory recorded.

## M0: Foundations and source audit

- Refresh both LightCraft repositories in isolated source copies; preserve local edits.
- Pin revisions, retain licenses/notices and inventory reused files with hashes.
- Audit open-source candidates, LightCraft and others (ADR 0007), for RAW, camera data,
  color, metadata, TIFF, calibration, demosaic, geometry, registration and UI.
- Establish Rust build, capability boundaries and private corpus inventory.
- Record upstream currency failures rather than claiming freshness.

Gate: reproducible build and reuse map; identified datasets and scientific evaluation
procedure. If upstream refresh is blocked, M0 remains partial.

## M1: Trustworthy RAW pipeline

- Headless inspect/develop path at original decoded resolution.
- Explicit sensor normalization, calibration ordering, demosaic, crop, orientation,
  white balance, camera characterization and linear working-space conversion.
- Preserve source identity, missing metadata and all applied operations.
- Reject or expose generic camera-color fallbacks and hidden processing.
- Research and validate dark/flat/bias matching, master construction and uncertainty.
- Compare demosaic behavior on stellar PSFs; no quality claims from one decoder run.
- Add TIFF input with explicit profile/transfer-function interpretation.

Gate: numerical and real-file tests, supported/validated camera distinctions,
consistent sequence development and verified calibration path. A diagnostic
camera-RGB TIFF alone does not complete M1 or public master export.

## M2: Sky registration and integration

Star detection; reference selection; frame quality; distortion-aware transforms;
spatial residuals and withheld validation stars; weighting; robust rejection;
coverage and clipping masks; bounded-memory integration; no repeated resampling.

Gate: center/corner PSFs, position, photometry, color, noise and coverage meet
predeclared thresholds on held-out scenes and synthetic truth.

## M3: Foreground and compositing

Editable masks with sky islands; per-source occlusion; ground alignment; static
foreground averaging; explicit flash reference; transient removal; reference
fallback for unstable foliage/clouds/water; honest insufficient-information reporting.

Gate: branch boundaries and scene content pass reference/cleanup-mode review.

## M4: Guided desktop app

Complete six-step workflow, confidence/correction review, project persistence,
dependency-aware invalidation, full-size inspection, resource estimates, scratch
selection, editable master and optional finished rendering.

Gate: independent users complete and resume sessions without developer assistance.

## M5: Acceleration

Profile before optimizing. Prototype portable GPU early to check architecture;
promote validated kernels in this milestone. CPU fallback remains complete.
Then investigate Metal/Apple-specific and other platform-specific improvements.

Gate: measured speed/resource benefits, numerical equivalence within declared
tolerances, device-loss/error recovery, tested multi-device selection.

## M6: Automation

Evaluate guided corrections, uncertainty calibration and escalation policies.
Keep held-out datasets separate. Offer automatic mode only for validated conditions.

Gate: routine cases complete automatically; difficult cases reliably request review.

## M7: Public release

macOS Intel/Apple Silicon first, Linux second, Windows third. Start cross-platform
build checks early. Require actual-device evidence before claiming runtime support.
Deliver packaging, source/dependency notices, reproducible builds, onboarding,
capture/calibration guides, redistributable examples, support table and recovery docs.

Gate: clean-machine installs, validated end-to-end behavior and documented limitations.

## After first release

Editor adapters and return/import workflow, then independent star-trail/deep-sky/
planetary/mosaic modules as separately scoped projects. Audit existing LightCraft
HDR, panorama and focus-stack work before duplicating it.
