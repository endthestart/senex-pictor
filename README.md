# SENEX Pictor

An open-source standalone nightscape stacker in development, written in Rust. The app and
its command are `pictor` (Pictor.app on macOS); no public release yet.

Taking over development? Read the [agent handoff](docs/HANDOFF.md) first.

Start with [the planning index](docs/README.md), [requirements](docs/requirements.md),
[milestone plan](docs/plan.md), and [current status](docs/status.md).
[STATUS](STATUS.md) is the one-page closing note: what is done, what the data is, what waits on whom.

## What it does

`inspect` and `develop` give headless RAW inspection and full-resolution camera-RGB
development. Their outputs are diagnostics, not calibrated/color-managed public masters.
No white balance, standard-space conversion, dark/flat/bias correction, tone mapping,
denoising, sharpening or lens resampling is performed by this path.
Sensor-stage opcodes and non-square sensor resampling are explicitly rejected;
post-demosaic opcodes are retained in the report but deferred.
A tested calibration library (`src/calibration/`, proposed ADR 0006) is not yet wired
into this command.

The Rust port of the stacker (plan R1–R6) is complete. One command takes a folder of RAW
files to a finished image (`pictor COMMAND …` runs any of these without the window):

- `process RAW_DIR NEW_OUTPUT_DIR [--run N] [--first NAME] [--last NAME] [--hero NAME]
  [--lens NAME] [--gpu yes|NAME]`: checks the folder, ingests the recommended run (or the
  frames from `--first` to `--last`; overexposed frames left out) after checking that the
  output volume has room (about 200 MB per 24 MP frame), registers,
  separates, stacks (or, with `--hero`, builds a hero composite around that frame) and renders
  `final.jpg`/`.tif` (or `hero-NAME.jpg`/`.tif`). The sky is stretched; the foreground keeps
  the exposure it was shot with. The working store and `process.json` (every stage's report
  and time) stay in the output folder.

The desktop app (`app/`, egui on the shared wgpu device, [ADR 0011](docs/adr/0011-desktop-toolkit.md))
runs the same steps with six screens in a step list. **Import** checks a folder of RAW files
in the background (runs, lit and overexposed frames, thumbnails), lets you choose the frames,
checks lens, colour, memory and disk before anything is written, then processes with live
progress, Cancel and Resume (a cancelled or failed run continues from its last finished
stage). **Frames** shows why each frame was used (lit, lost, drift per frame, blink against
the reference); **Sky & ground** tints the sky and the boundary over the result (O) and
takes corrections painted as sky or ground (S, G), applied in the background;
**Scene** makes a hero frame or a short-window moment and opens it; **Compare** keeps two
renderings on the GPU (Space swaps in place, S splits, chosen regions as 200% crops with
A / same / B verdicts saved to `compare.json`); **Look & export** tunes the look live and
exports the linear 32-bit master, a 16-bit TIFF, a JPEG or an HDR JPEG, with an HDR
preview on screens that show light above paper white. Drop a folder on the window or
run `cargo run --release -p pictor -- [FOLDER] [--step NAME]`.
`tools/package-macos.sh` builds `dist/Pictor.app` (universal, ad hoc signed) and a zip of it,
after `tools/notices.sh` refreshes the Third-Party Notices with `cargo about`.
`.venv/bin/python tools/icon.py app/assets/icon` redraws the icon from the template in
[DESIGN-TOKENS](docs/DESIGN-TOKENS.md), the look shared with SENEX Corvus.
`cargo test -p pictor` renders the window headlessly (`PICTOR_UI_SHOTS=DIR`
saves the frames; `PICTOR_UI_FOLDER`, `PICTOR_UI_RAW`, `PICTOR_UI_STOPPED`, `PICTOR_UI_CORRECT`
and `PICTOR_UI_STACKED` point the screen tests at a processed folder, a RAW folder, a stopped
run, a run the tests change and a folder with a whole-sequence stack; each is skipped without).
`tools/proto/hdr.py` is a parked prototype that merges a RAW exposure bracket.

`correct OUTPUT_DIR [--gpu yes|NAME]` runs a finished run again from its matte with the
corrections in its store (`store/corrections.json`, the strokes the app paints), reusing the
saved statistics. Moving foreground (swaying branches, water) keeps the long exposure's blur
unless `process … --moving sharp` takes it from the reference minutes instead;
`recompose OUTPUT_DIR --moving blur|sharp` (or Scene's switch) changes a finished run.

`process` itself resumes too: run the same command into the same output folder and it keeps
the finished stages, removes what the interrupted stage left in the store, and goes on.

Every command takes `--memory GB` (before the command): the memory the run may use. Without
it the run takes 75% of the memory free when it starts. Each parallel step runs as many
workers as fit (at most one per core) and fails with what it needs when even one does not.

The stages also run one at a time:

- `check RAW_DIR`: runs split at time gaps, setting changes and camera moves (a foreground
  jump confirmed by a break in the star motion), overexposed and lit frames, twilight ramps,
  foreground drift, and the recommended frames (JSON).
- `ingest RAW_DIR NEW_STORE [FIRST [LAST]]`: per frame the developed camera RGB in ADU (16-bit
  float), the CFA samples and a saturation mask, plus a manifest.
- `register STORE [--lens-from MODEL_JSON]`: stars detected and tracked in every frame, the sky model
  (fixed axis; free per-frame rotations when held-out stars show drift; another sequence's
  lens profile with a `model.json` from the same lens), written into the store with a report.
  Lateral chromatic aberration is measured from star centroids on the raw colour sites (no
  lens profile needed); the sky stack then samples red and blue where the lens put them.
- `separate STORE [--labels PLANES] [--lens NAME]`: temporal statistics, sky-coherent star
  transits, the sky matte, camera-fixed glow and per-frame occlusion (`--lens` names a lens
  the files do not record, for its lensfun vignetting).
- `stack STORE [--sigma PX] [--radius PX] [--gpu yes|NAME] [--lit NAME,…]`: the CFA sky
  stack (on a GPU with `--gpu`: `yes` for the most capable one or part of its name), the
  long-exposure ground stack (`--lit` frames left out) and their composite (`final.planes`).
- `hero STORE --hero NAME --first NAME [--foreground frame|long] [--lit NAME,…] [--gpu yes|NAME]`:
  a composite around one frame, with the drift-proof stacked sky behind it (`hero.planes`).
  The foreground is that frame as shot, or (`long`) the long exposure of the unlit frames,
  each moved back by its camera turn; `process` uses the latter around the reference frame
  when the foreground drifted more than 4 px.
- `moment STORE --centre NAME [--half N]`: a short-window sky (aurora) with the deep stars.
- `render STORE PLANES OUT_PREFIX [--matrix JSON] [--look natural|dark]`: JPEG and 16-bit
  TIFF (`--single yes` adds the reference frame at the same levels, `--linear yes` a linear
  float TIFF, `--hdr 4` an HDR JPEG: the SDR picture with an ISO 21496-1 / Ultra HDR gain map
  that lifts the brightest stars up to four times paper white on HDR displays). The camera's colour matrix comes from `data/colour-matrices.json` unless a file
  is given.
- `flat INPUT [LENS]`: the lensfun vignetting profile for the file's lens, focal length and
  aperture (lensfun's database is read in place; set `PICTOR_LENSFUN` to choose it).

```sh
cargo build --release --locked
target/release/pictor inspect /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw/DSC_0226.NEF
target/release/pictor levels /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw/DSC_0226.NEF
target/release/pictor develop /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw/DSC_0226.NEF /Volumes/Work/star-photos-data/validation/native-first
target/release/pictor check /Volumes/Work/star-photos-data/bwca-2024-05-19/raw
target/release/pictor flat /Volumes/Work/star-photos-data/maplewood-2025-09-26/raw/DSC_3910.NEF
```

The output directory must not already exist; its parent must exist. Outputs:

- `camera-linear.tif`: float32 RGB in camera primaries, with no misleading ICC profile.
  It may look dark or have unusual color in ordinary viewers.
- `report.json`: source SHA-256, levels, geometry, unapplied color/WB data and limitations.
- `INCOMPLETE`: remains if export fails after starting. Successful export removes it.

Original bytes and sidecars are never modified. Missing metadata stays missing.
The native path does not depend on the Python POC or Python packages.

## Verification

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo fmt -p pictor-engine -p pictor -- --check
python3 tools/verify_sources.py
```

Use `--offline` with Cargo when required dependencies are already cached.
Format only this package; `cargo fmt --all` also reformats local path dependencies,
which would break the verbatim vendor source inventory.

Private corpus tooling requires Python 3 and ExifTool, solely for development:

```sh
python3 tools/inventory.py /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw /Volumes/Work/star-photos-data/validation/corpus-new.json
python3 tools/validate_corpus.py /Volumes/Work/star-photos-data/wild-river-2024-04-14/raw /Volumes/Work/star-photos-data/validation/corpus.json /Volumes/Work/star-photos-data/validation/run-1
```

These tools reject existing destinations. Inventory hashes every capture and uses
ExifTool as an independent metadata oracle. Native validation compares every header
and source hash, then develops the first/middle/last files at full resolution.
Neither a successful header check nor three successful developments validates all
frames' pixels, scientific fidelity, camera variants or other camera models.

## Source reuse and licensing

Selected LightCraft Classic crates are pinned and vendored with original licenses
and notices. See [reuse audit](docs/reuse-audit.md) and
[source inventory](docs/source-lock.json), pinned to upstream Classic dbe90dc
(checked 2026-10-25).
Pictor is licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. A complete dependency and data attribution audit for distribution remains open.

The prior Python POC and generated baselines remain temporarily for comparison.
They impose no compatibility requirement and can be removed after useful evidence
is retained. Private photographs, outputs, source clones and build products are ignored.
