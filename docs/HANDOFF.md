# Agent handoff: Star Photos

Prepared 2026-10-25. Project root: `/Users/andermic/Development/star-photos`.
This file is an entry point, not a replacement for the requirements or milestone gates.

## Start here

1. Read `/Users/andermic/Development/AGENTS.md` and any newly added project instructions.
2. Read [requirements](requirements.md), [status](status.md), [plan](plan.md), and
   [processing specification](processing.md).
3. Read [validation](validation.md), [reuse audit](reuse-audit.md),
   [research agenda](research.md), and [ADRs](adr/README.md).
4. Inspect the current files before assuming this handoff is still current.

The user explicitly requested planning/specification/requirement/ADR files in this
project, followed by implementation of the first milestone. Scope and priorities
were settled through eight questions; do not repeat that interview.
Respect the parent rules for Git actions, package installs, test creation and scope.
This handoff does not override those rules or authorize publishing anything.

## Agreed product

- Public open-source, native Rust standalone application. Editor integrations later.
- First release: fixed-tripod nightscapes, hundreds of full-resolution exposures.
- Accurate, realistic sky approximating a perfectly aligned long exposure; no
  invented detail. Preserve star position, shape, color and brightness relationships.
- Editable master plus optional natural finished rendering.
- Guided checkpoints first; build reliable automation from evaluated corrections.
- Reference-moment preservation and explicit transient-object cleanup are both wanted.
- Separate foreground reference must support a flash-lit exposure.
- Moving foliage may require reference fallback; do not promise reconstruction of
  every moving leaf or continuously obscured background.
- Broad RAW decoder coverage; camera/compression-specific validation starts with Nikon.
- Optional calibration, capture/matching guidance, and researched RAW processing order.
  The owner currently edits one RAW and synchronizes settings across a stack; evaluate
  that practice rather than assuming it is scientifically appropriate.
- macOS first, Linux second, Windows third. Intel Mac with AMD GPUs is available here;
  the user also has an M2 laptop and a Linux AMD-GPU desktop for subsequent validation.
- Complete CPU baseline → portable GPU → platform-specific GPU capabilities.
  Use hardware fully where justified, with explicit memory/scratch/concurrency budgets.
- No scientific shortcuts, hidden substitutions, silent fallbacks or premature
  release-quality claims. Incomplete methods stay explicit.

Deferred: plugins, tracked-sky capture, star trails, deep sky, planetary/lunar/solar,
mosaics. These can become separate workflows sharing the engine. Do not build a
general catalog editor or speculative empty modules.

## Actual implementation state

M0 is partial: vendored Classic dbe90dc (upstream has since moved); candidate audits remain.
M1 has a RAW diagnostic path and a tested calibration library, not an end-to-end
calibrated pipeline.
M2 has research evidence (ADR 0008) but no product code; M3–M7 are planned. There is
no native stacker, GUI or GPU backend yet.

| File | Responsibility |
|---|---|
| `Cargo.toml`, `Cargo.lock` | Standalone Rust package; locked dependency graph; local vendor path dependency |
| `src/main.rs` | `inspect` / `levels` / `develop` CLI, explicit errors and new-output-path requirement |
| `src/input.rs` | File-byte SHA-256 identity, metadata probe and full RAW decode |
| `src/pipeline.rs` | Shared sensor checks, normalization, AHD demosaic, crop/orientation and diagnostic report |
| `src/calibration/` | Proposed ADR 0006 core: mosaic, matching, masters, flats, light calibration |
| `src/output.rs` | Float32 TIFF plus JSON report; explicit buffer flush/sync; incomplete-output marker |
| `src/lib.rs` | Modules, shared error type and pinned decoder revision constant |
| `tests/pipeline.rs` | Five diagnostic-pipeline tests |
| `tests/calibration_*.rs`, `tests/support/` | 18 synthetic-truth calibration tests and the simulated sensor |
| `tools/inventory.py` | Private NEF metadata inventory using ExifTool and full byte hashing |
| `tools/validate_corpus.py` | Compare native headers/hashes; develop first/middle/last captures |
| `tools/verify_sources.py` | Verify exact vendor file set and SHA-256 hashes |
| `tools/oracle_levels.py`, `docs/oracle-lock.json` | Compare decoded levels with pinned rawspeed/RawTherapee oracles |
| `tools/star_motion.py` | Phase 1 research: star tracking and registration-model comparison (needs `.venv`) |
| `src/sequence/`, `src/store/`, `src/lens.rs` | Rust port R1: sequence check, frame/CFA stores, lensfun flat fields (CLI `check`, `ingest`, `flat`) |
| `tools/proto/` | Phase 1 prototype: `run_iteration1.py` (half res), `run_iteration2.py` (full res via `star-photos develop`: sky-coherent transits in `transits.py`, matte in `separation.py`, glow in `background.py`, per-frame occlusion in `occlusion.py`, tiled Lanczos stack; `--lens-from` reuses a lens profile), `run_iteration3.py` (CFA/Bayer-drizzle sky stack in `cfa.py`), `run_moment.py` (dynamic-sky moment composites), `run_hero.py` (one chosen frame with the stacked sky behind it, drift-proof via `drift.py`), `run_foreground.py` (moving-foreground variants), `run_check.py` (sequence check in `sequence.py`, run first); `vignetting.py` lensfun flat fields, `quality.py` metrics, `render.py` review rendering; SAM via `.venv-sam` |

Product code is Rust. Python tools are development-only, not product dependencies.
Old `app.py`, `stacker.py`, `templates/`, `requirements.txt`, and
`tests/test_stacker.py` are the disposable Python POC. They remain temporarily with
prior output evidence; do not extend them or preserve their API for compatibility.
The user permits discarding the POC. Preserve originals and useful evidence first.

## RAW boundary: read before changing processing

Current output is **linear camera RGB diagnostic data**, not sRGB/Rec.2020, not
calibrated, and not a public color-managed master. TIFF intentionally has no
misleading standard-space ICC profile. It may look dark or oddly colored in viewers.

The wrapper does not apply WB, camera characterization, profile looks, baseline
exposure, tone mapping, denoising, sharpening, lens warps or hot-pixel repair.
It preserves unapplied WB/color information and deferred post-demosaic opcodes.
Sensor-stage opcodes and non-square-pixel resampling are explicitly rejected.
Negative samples/headroom and crop/orientation behavior have targeted tests;
stellar photometry, PSFs and scientific fidelity have not been validated.

Important upstream behaviors found during the audit:

- `RawImage::develop` can apply opcodes and resample; the wrapper deliberately uses
  separate normalization/demosaic/crop stages instead.
- `normalized` can replace non-finite values and substitute scales for invalid
  levels. The wrapper rejects non-finite samples and invalid ranges beforehand.
- Color APIs can use generic camera fallback matrices. Do not silently adopt these.
- AHD availability is not evidence of best stellar reconstruction.
- Decoder defaults versus values explicitly present in the file need further audit.

Memory handling is not the final bounded-memory design: files and full-frame arrays
are allocated, with several intermediate buffers. Do not hold hundreds of these
frames in RAM or imply out-of-core integration already exists.

## Source reuse and upstream refresh

Fresh upstream clones are in ignored `.sources/upstream-lightcraft` (387bed1) and
`.sources/upstream-lightcraft-classic` (dbe90dc); first-session snapshots remain in
`.sources/lightcraft` and `.sources/lightcraft-classic`. Vendored Classic is dbe90dc,
adopted with owner approval; see [reuse audit](reuse-audit.md) for manifest changes,
identical-output evidence and decoder risks. The LightCraft session keeps the sibling
checkouts at `/Users/andermic/Development/lightcraft` and `lightcraft-classic`
current; never reset, clean or pull them from here. For a future refresh, fetch into
`.sources/upstream-*` and review differences before replacing vendored code.

`vendor/lightcraft-classic/` contains the required Classic subset: raw, brand, tiff,
meta, raster, color, geom and testkit. It does not depend on absolute sibling paths.
The subset workspace manifest is adapted; other inventoried files retain upstream
bytes. [source-lock.json](source-lock.json) records 166 files and source identities.
If the decoder revision changes, update provenance consistently, including
`DECODER_REVISION`, source-lock, audit, tests and validation evidence.

**Never use `cargo fmt --all` here:** it also reformats local path dependencies.
Use `cargo fmt --package star-photos`. Previously reformatted vendor files were
restored and their original hashes verified. Do not casually edit vendor files or
regenerate their hashes to conceal drift.

Retain upstream licenses/notices and camera-data attribution. Spectral data involve
rawtoaces-data Apache-2.0 and cited CIE CC-BY-4.0 provenance. Full distribution audit
and the new project's public license are unresolved. UI toolkit, GPU framework and
public master format/profile choices are also unresolved; do not present candidates
as accepted decisions. No font, icon or photo assets were adopted.

## Verification and private evidence

Last implementation checks passed: formatting, 23 Rust tests, Clippy with warnings
denied, release build and integrity of all 166 vendor files; see [status](status.md).

```sh
cargo fmt --package star-photos -- --check
cargo test --offline --locked
cargo clippy --offline --locked --all-targets -- -D warnings
cargo build --release --offline --locked
python3 tools/verify_sources.py
```

Rust 1.99.1, Intel macOS 15.7.4 were used. `--offline` assumes cached Cargo
dependencies; do not silently install tooling or change dependencies to bypass errors.

- Private data root: `/Volumes/Work/star-photos-data/` on the Work disk (datasets with SHA-256 manifests,
  plus `validation/` for results and caches). First sequence: `wild-river-2024-04-14/raw/`
  (262 NEFs, ON1/XMP sidecars, one movie). Nothing private lives in the repository.
- Inventory: `/Volumes/Work/star-photos-data/validation/corpus.json`; all RAW bytes hashed.
- Completed run: `/Volumes/Work/star-photos-data/validation/native-validation/validation.json`.
- Header metadata/source identities: 262/262 passed against independent inventory.
- Full development/export: **three files only**—DSC_0226, DSC_0357, DSC_0487.
- All inputs inventoried as Nikon Z6 II, ISO 1600, 20 seconds, f/4, 24 mm.
- Sensor dimensions 6064 × 4040; default cropped output 6048 × 4024, float32 RGB.
- Native develop/export roughly 1.4–1.5 seconds; process high-water memory roughly
  1 GiB. Cache conditions were uncontrolled; these are not release performance claims.
- No local validation evidence for Z8, D750, other encodings or other camera families.

Recognized format ≠ supported compression ≠ successful header probe ≠ successful
full decode ≠ scientifically validated camera. Keep these distinctions in reports.
Do not distribute private captures, sidecars, reports containing personal metadata,
old outputs, source clones or build artifacts. These paths are ignored.

## Reproduce a diagnostic

Run from the project root; choose an output directory that does not exist and whose
parent exists. Keep previous validation evidence intact.

```sh
target/release/star-photos inspect /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw/DSC_0226.NEF
target/release/star-photos develop /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw/DSC_0226.NEF /Volumes/Work/star-photos-data/validation/handoff-check
```

Successful export contains `camera-linear.tif` and `report.json`. An `INCOMPLETE`
marker remains when a write fails after starting. These protections are not a
complete project-resume or crash-durability implementation.

For a new complete corpus check, use a fresh destination:

```sh
python3 tools/validate_corpus.py /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw /Volumes/Work/star-photos-data/validation/corpus.json /Volumes/Work/star-photos-data/validation/handoff-validation
```

## Next work, in order

0. Phase 1 first (see [plan](plan.md)): registration per proposed
   [ADR 0008](adr/0008-sky-registration.md), integration, then foreground. Policy:
   open-source first, oracles where licensing is complicated, fixes upstream via the
   LightCraft session ([ADR 0007](adr/0007-open-source-first.md)). Log installs in
   [install-log](install-log.md).
1. Finish M0 candidate audits (registration, UI) when those milestones approach;
   re-audit the RAW decoder choice under ADR 0007 (LibRaw, rawler, LightCraft).
2. End-to-end calibration per [ADR 0006](adr/0006-calibration.md): read Nikon
   shutter/compression/NR maker-note tags, load calibration sets within a memory
   budget, calibrate before demosaic. Real validation needs new dark/bias/flat
   captures; the owner has star-photo sets ready when development is testable.
   Do not subtract darks from rendered data or apply incompatible masters.
3. Specify shared sequence WB and camera characterization provenance; define explicit
   missing-profile behavior. Decide scene-linear master/export semantics in an ADR.
4. Evaluate demosaic alternatives using known stellar positions, flux and PSFs plus
   held-out real scenes. Measure corners, false color, noise and negative handling.
5. Investigate allocation peaks before introducing multi-frame concurrency.

Do not jump to UI polish, plugins or GPU kernels while describing M1 as complete.
Update status, affected requirements/specs, ADRs and evidence with each bounded change.
The milestone gates are defined in [plan.md](plan.md).

## Workspace and handoff boundaries

At handoff inspection, the project has **no `.git` directory**. No init, commit,
push or publication has occurred. `.gitignore` does not provide history or backups.
Do not assume the POC or current changes can be recovered through Git.
No processing jobs are intentionally left running by this handoff.
This task changes documentation only; implementation and data are unchanged.
