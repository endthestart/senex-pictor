# Implementation status

Updated: 2026-10-29 (renamed Pictor; star cores, colour fringes, lit foliage, matte corrections,
HDR output, settings, menu bar, side-by-side results).

## Milestone gates

| Milestone | State | Evidence / remaining gate |
|---|---|---|
| M0 Foundations | Partial; upstream refresh done | Vendored subset at Classic HEAD dbe90dc; source lock; corpus inventory. Registration/UI candidate audits pending. |
| M1 RAW pipeline | Diagnostic slice plus calibration core | Camera-RGB export; calibration library with synthetic-truth tests; no end-to-end calibration, characterization or real calibration validation. |
| M2 Registration/integration | Rust port R1–R7 done; lateral chromatic aberration measured from the stars | Fixed-axis and drift models, CFA stack (CPU and GPU) on the five owner sequences; PSF, photometry and colour gates not run. |
| M3 Foreground/compositing | Partial | Matte from star transits, brightness and colour; owner corrections; long exposure, hero frame and moment composites. Lamp-lit swaying foliage looking straight up (Maplewood) still needs corrections. |
| M4 Guided desktop app | In progress | Six steps, corrections, Settings, menu bar, side-by-side results, HDR preview and export; verified in headless renders only, not yet in a live window or by independent users. |
| M5 Acceleration | Started | GPU sky stack (wgpu) with CPU fallback; no profiling-led work yet. |
| M6–M7 | Planned | No automation work or public release yet. |

## Completed

- Saved requirements, plan, processing/validation specifications, research agenda,
  reuse audit and six ADRs (0006 proposed); index: [README](README.md).
- Refreshed vendored Classic to upstream dbe90dc (166 files locked); identical local
  outputs; updated notices vendored verbatim.
- Calibration library (`src/calibration/`): per-frame black subtraction, capture
  matching with disclosed gaps, mean/Winsorized σ-clip masters, per-color normalized
  flats with validity masks, light calibration in ADU with saturation flags and a
  provenance report. 18 synthetic-truth tests; deliberate faults caught. Not in the CLI.
- Two-phase plan: Phase 1 core sky/foreground logic first, Phase 2 decoder and
  calibration hardening ([plan](plan.md)); installs logged in [install-log](install-log.md).
- Renamed Pictor (SENEX Pictor), engine and app work (2026-10-29): MIT OR Apache-2.0 licence;
  one `pictor` binary (window, or a tool given a command). Clipped star cores fill with sensor
  white and the renderer clips white-balanced colour at the lowest sensor level and bounds the
  saturation boost (no more cyan cores). Lateral chromatic aberration is measured from star
  centroids on the raw colour sites at registration and corrected in the CPU and GPU stacks
  (Wild River: stacked stars' red and blue offsets from green 0.69 px → under 0.03 px at the
  edge). The matte makes part-sky pixels with no transit within the local transit gap ground,
  and light of its own colour where no star crosses (lamp-lit leaves); Yellowstone's orange
  canopy patches are gone, Maplewood's red-lit leaves only partly. Owner corrections (brush
  strokes in `store/corrections.json`) re-run a finished run from its matte on saved statistics
  (`process::correct`, `pictor correct`). HDR JPEGs (gain map with Ultra HDR and ISO 21496-1
  metadata) decode to HDR in macOS ImageIO. App: Settings (GPU, memory, output folder), macOS
  menu bar, Compare B from another result of the run, HDR preview. GPU stack bands now stay
  within the device's buffer limit. Validated on all five owner sequences; UI by headless renders.
  Later the same night: moving foliage keeps the long exposure's blur by default, as the owner
  decided (it had always taken the reference minutes); sharp is a per-photo choice recorded in
  the run (`--moving sharp`, `recompose`, Scene's switch). Sky & ground lists the regions the
  colour check made ground. The sequence check is a quarter faster with identical output (BWCA
  591 files: 86 s → 65 s); ingest and store reads use less CPU with byte-identical stores, but
  ingest here is bound by the data volume's writes (133 MB/s, 92% full). Recent folders, a
  packaging script (`tools/package-macos.sh`) and Linux/Windows compile checks.
- Desktop app, all six steps (2026-10-27): Import checks a RAW folder in the background
  (thumbnails made during the sequence check), chooses runs and frames, checks lens profile,
  colour matrix, memory and disk before writing, and runs `process` with live progress, time
  left, Cancel and Resume; Frames, Sky & ground (GPU overlay of the sky weight), Scene (hero
  frame and moment made in the background and opened), Compare and Look & export (live look
  controls, realism check, linear master / 16-bit TIFF / JPEG export) share one GPU viewer.
  Engine: `src/progress.rs` (stage plan, step counts, cancel at the next item, time left),
  `process` resumes from its journal (`process.json` written around every stage; the
  interrupted stage's store files rolled back; killed twice and resumed, the final TIFF is
  byte-identical to an uninterrupted run), named hero results, per-frame drift, lens
  candidates and colour-matrix checks. Screens verified headlessly with kittest renders on
  Yellowstone (a RAW folder, a finished run, a stopped run); no live window yet.
- Desktop app start (2026-10-27): app research report, screen designs, ADR 0011 (egui on a
  shared wgpu device, LightCraft Classic's HDR-patched egui-wgpu vendored); `app/` opens a
  processed folder, store or scene file and compares two GPU-resident looks (instant swap,
  split, auto-chosen region crops with saved verdicts, live stretch); headless UI test with
  kittest (41 tests).
- Rust port R7 (2026-10-27): `process RAW_DIR OUT` (`src/process.rs`) runs every stage and
  renders the finished image; memory budget (`src/budget.rs`, `--memory GB`, default 75% of
  free memory) sizes every parallel step and keeps outputs byte-identical; foreground as
  shot; lit frames left out of the long exposure; drifting sequences (foreground drift over
  4 px) composed around the reference frame with the long exposure moved back by each
  frame's camera turn (`src/flow/rotation.rs`, `src/integrate/foreground.rs`,
  `src/integrate/ground.rs`); GPU chosen by name; built-in colour matrices; 1 new test (40).
  Idle-GPU timings and the review page are in [plan](plan.md).
- Rust port R1 (2026-10-26): `check`, `ingest` and `flat` commands (`src/sequence/`,
  `src/store/`, `src/lens.rs`); 10 new tests (33 total); stores bit-identical to the Python
  caches; the check recommends Maplewood's 280 frames, Yellowstone's 70-frame run, BWCA's dark
  start (DSC_0932–0967, 30.7× twilight ramp) and catches the headlamp-lit DSC_4007.
- Rust port R6 (2026-10-26): GPU sky stack (`src/gpu/`, wgpu/Metal) matching the CPU to
  0.0002–0.0004 of the noise (median); review page with owner decisions:
  https://claude.ai/artifact/Es68QoYvbyvT6DHDMoyVsN
- Rust port R3–R5 (2026-10-26): `separate`, `stack`, `hero`, `moment`, `render` (`src/separate/`,
  `src/integrate/`, `src/render.rs`, `src/review.rs`, `src/export.rs`); 3 new tests (39 total).
  Matte parity 99.89–100% on all five sets; CFA stack parity 1e-5 of noise with the same
  sky model; Yellowstone hero and Northern Lights aurora moment reproduced. Run times
  (8 threads): Maplewood separate 696 s (22 GB peak; occlusion dominated before its filters
  were made O(1)), stack 276 s; prototype about 10× slower.
- Rust port R2 (2026-10-26): `register` command and camera moves/drift in `check`
  (`src/stars/`, `src/sky/`, `src/flow/`, `src/register.rs`); 3 new tests (36 total). Stars
  match sep on 99.5% of its detections (median position difference 4e-5 px); registration
  matches or beats the prototype on its own star pairs for all five sets; Yellowstone's
  drift is detected and free rotations bring held-out medians to 0.12–0.42 px. Maplewood
  registers in 97 s (prototype 193 s), 6 GB peak with 8 frames at once.
- Phase 1 iteration 3 work (2026-10-26, prototype; [ADR 0009](adr/0009-sky-separation.md),
  [ADR 0010](adr/0010-sequence-check-and-drift.md); private review page with the owner's
  product decisions: https://claude.ai/artifact/Mwbg9P3PL5Tjn3z77KsCva):
  - Separation: transits must follow the sky model's motion (pixel-level coherence after
    removing each frame's large-scale brightness change); absence of motion means ground
    only where nearby sky shows dense transits; per-frame occlusion bits; frame border is an
    obstruction. Maplewood canopy now ground; Wild River unchanged except lit trunks.
  - CFA (Bayer-drizzle) sky stack: no demosaic; ringing around bright stars 6% → 1% of
    peak and no green/black star rings; σ 0.5 px kernel trades a slightly wider star core
    for lower noise (0.35 px matches iteration 2 sharpness at similar noise).
  - Registration: per-pair motion for tracking (frame gaps); lens profile reuse for sparse
    skies (Northern Lights angles 0.014° → 0.0044° RMS); free per-frame rotations for a
    drifting camera (Yellowstone sag, 1.2–1.5° over 37 min).
  - Sequence check (gaps, settings, overexposure, camera moves, drift) runs before
    processing; BWCA's last 84 frames are dawn-overexposed; Yellowstone's first eight are
    setup shots.
  - Hero composite (`run_hero.py`): one chosen frame (Yellowstone DSC_1259, light painting)
    as the photograph with the stacked sky behind it.
  - Vignetting: lensfun profiles exist for all three owner lenses (corners at 14–35% of
    centre uncorrected); flat fields are needed for finished renderings; self-calibration
    from star motion is open. Saturation is stored as packed bits (a JSON list reached 19 GB).
- Iteration 2 unchanged on Maplewood (280 frames, 20 mm): registration generalizes
  (angles within 0.0018° RMS of sidereal, held-out medians 0.14–0.41 px). Separation
  fails under canopy: swaying leaves let sky flicker through, which passes the transit
  test, so 86% of pixels look like sky and foliage smears into the stack. Transits must
  follow the sky model's motion, not only exceed the median. Haze or cloud late in the
  sequence (stars per frame down to 1,760 from a median of 11,767).
- Phase 1 iteration 2 (full resolution from `pictor develop`, private review page
  https://claude.ai/artifact/2JBHr3TKrsGhCCtzNrGE2B): camera-fixed glow separated from
  sky light per frame (glow changed 34 ADU over 100 min); treeline bands traced to tree
  pixels inside the resampling kernel (15–66× more dark rejections within 4 px) and fixed
  with 5 px clearance; Lanczos-4 stars 11% smaller than bilinear, noise at √N (17.2×
  center); motion outranks brightness for pale trunks; 20th-percentile matte fixes the
  pole; 3-minute reference window for fallbacks. Open: AHD false color and −4,936 ADU
  ringing, residuals growing toward sequence ends (refraction suspected), in-frame trailing.
- Phase 1 iteration 1 (`tools/proto/`, private review page
  https://claude.ai/artifact/WbMPhCKx7NjP77EMJCYW9T): one lens model and rotation axis
  for all 262 frames (angles within 0.003° RMS of sidereal), matte from star transits,
  brightness and SAM, robust stack (median 252 frames per sky pixel, 0.14% rejected),
  26× center noise reduction (bilinear smoothing included). Found: horizon glow is
  camera-fixed and must be removed per frame before star alignment.
- Registration research (`tools/star_motion.py`): ~5,000 stars per frame tracked
  through 262 frames; sphere rotation with self-calibrated distortion holds held-out
  residuals to 0.47–0.98 px over 100 min, matching the sidereal rate within 0.01°.
- Open-source-first policy accepted ([ADR 0007](adr/0007-open-source-first.md)); decoder
  findings relayed to the LightCraft session; `levels` command and pinned rawspeed/
  RawTherapee oracle comparison over 528 frames ([reuse audit](reuse-audit.md)).
- Real-data evaluation on two private Canon EOS 40D sets (ADR 0006): matching
  caught misfiled and mismatched frames; temperature-mismatched darks over- or
  under-subtracted hot pixels by 13–35%; decoded white levels unreliable.
- Isolated committed local snapshots of LightCraft and Classic without sibling edits.
- Vendored selected Classic components, adapted only the subset workspace manifest,
  retained upstream licenses/notices and saved per-file hashes.
- Native CLI: SHA-256 source identity, RAW metadata inspection, decode, sensor level
  validation, normalization, AHD demosaic, crop/orientation, float TIFF/report export.
- Negative values/headroom preserved. No generic camera-color transform, white
  balance, aesthetic processing or post-demosaic lens opcodes are applied.
- Invalid sensor ranges/non-finite samples, unreviewed sensor opcodes and non-square
  sensor resampling are rejected. Post-demosaic opcodes are recorded as deferred.
- New output directory required; failed exports keep an INCOMPLETE marker.
- Independent corpus inventory via ExifTool plus SHA-256 for all 262 originals.
- Five pipeline tests: normalization/range, bad sensor values, Bayer channel
  preservation, crop/rotation coordinates, float TIFF round-trip/existing-output.

## Verification evidence

Local commands:

```sh
cargo fmt -p pictor-engine -p pictor -- --check
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo build --release --offline --locked
python3 tools/verify_sources.py
```

The redundant encoder drop that Clippy identified is removed; the buffered writer
is explicitly flushed and errors propagated before file synchronization.
Vendored source integrity: 166 files. Format only the first-party package, because
`cargo fmt --all` also formats path dependencies. Vendor originals were restored
and their hashes verified after discovering that behavior.

Private run: `/Volumes/Work/star-photos-data/validation/native-validation/validation.json`, complete=true.
Header metadata and source hash checks: 262/262 passed against independent inventory.
Full pixel decoding/development/export: DSC_0226, DSC_0357, DSC_0487 only.
All three outputs: 6048 × 4024 pixels, RGB float32; ExifTool verified TIFF tags.
262 successful header checks do not establish successful pixel decoding of every file.

| Capture | Native develop/export seconds | Validation process peak RSS bytes* |
|---|---:|---:|
| DSC_0226.NEF | 1.445 | 1,098,817,536 |
| DSC_0357.NEF | 1.498 | 1,101,287,424 |
| DSC_0487.NEF | 1.488 | 1,110,302,720 |

*Python resource.RUSAGE_CHILDREN records the child high-water mark across the
sequential validation process, not a separately instrumented stage allocation.
Intel macOS 15.7.4, Rust 1.99.1, release build. Cached filesystem conditions were
not controlled. These are initial measurements, not performance promises.
All captures: Nikon Z6 II, ISO 1600, 20 seconds, f/4, 24 mm, NEF compression tag 34713.
Other Nikon bodies/compressions and other cameras have no local validation evidence yet.

Second session (same machine/toolchain): all five commands pass with 23 tests.
`/Volumes/Work/star-photos-data/validation/dbe90dc-validation/validation.json` (decoder dbe90dc) repeated
262/262 header and source-hash checks and the three developments; their TIFF pixel
data hash identically to `session2-validation` (snapshot decoder) and to
`calibration-refactor-validation` (after the shared sensor-check refactor). Sensor
measurements and the absence of local calibration frames are in
[ADR 0006](adr/0006-calibration.md).

External datasets (private, `/Volumes/Work/star-photos-data/validation/datasets/`): `HeartNebula.zip`
1,085,014,451 bytes, SHA-256 4a0c4142d8d0e7b056ec9b46046eec78fe6fc149aabf3c028923f81d6851f0a0;
`Pleiades.zip` 2,181,389,750 bytes, SHA-256
8fa8d9fbfb1a04fb53ade058516ca715bdbf8dc704e5d0d7b072e614a0b27f69.

## Blockers and limits

Vendored Classic was refreshed to upstream dbe90dc with owner approval; see
[reuse audit](reuse-audit.md). Sibling checkouts are left to the LightCraft session.
Calibration: only private Canon 40D calibration sets; the decoder exposes no shutter
mode, compression, NR or temperature, which ADR 0006 needs for dark matching.

No complete dark/flat/bias calibration, shared development recipe, camera
characterization, color-managed master export, TIFF input, PSF/photometry study or
distribution audit. Licence: MIT OR Apache-2.0 (owner decision, 2026-10-29).
Decoder-provided defaults versus explicitly recorded camera values need deeper audit.
Current diagnostics are not release-quality photographic outputs.

## Next bounded work

1. Re-audit the RAW decoder choice under ADR 0007 (LibRaw, rawler, LightCraft;
   Classic is now 08f7526c8); guard truncated packed NEFs, which Classic zero-fills.
   Independent per-ISO white levels, checked against the oracles.
2. End-to-end calibration path: vendor maker-note fields (NR, shutter, temperature),
   bounded-memory master loading, calibration before demosaic.
3. Cross-check calibration against Siril or ccdproc on the Canon sets.
3. Define shared white balance and characterization provenance, with explicit missing
   profile behavior; write the color/master-format ADR before public master output.
4. Measure stellar PSFs and color/flux behavior of demosaic alternatives on held-out
   scenes; investigate memory peaks before scaling simultaneous frame work.

The Python POC and prior outputs remain temporarily as private comparison evidence;
the native application does not depend on them. No original image was modified and
nothing was published.
