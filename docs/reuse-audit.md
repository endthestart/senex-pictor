# Initial source reuse audit

2026-10-25. Development audit, not a completed security/scientific validation.

## Source identities

| Source | Upstream HEAD checked 2026-10-25 | Use |
|---|---|---|
| storytold/lightcraft | 387bed199fca44d76af5aa1c24dd7cc3de4c8db2 | Reference clone only; not vendored |
| emmertex/lightcraft-classic | dbe90dc1b5959708cc6424c7fcc18d55d52d2593 | Vendored subset (adopted 2026-10-25) |

Earlier snapshots (lightcraft 701f78e, Classic 6f48dbd) came from the sibling
checkouts while GitHub was unreachable; both are ancestors of the HEADs above. Fresh
clones are in `.sources/upstream-*`; the old snapshots remain in `.sources/` for
comparison. The sibling checkouts are maintained by the LightCraft session (informed
2026-10-25); Pictor never pulls, resets or edits them.

## Adopted subset

Vendor location: `vendor/lightcraft-classic/`. Complete per-file SHA-256 and
unchanged-source status: [source-lock.json](source-lock.json).

| Crates | Initial use | Risk/remaining review |
|---|---|---|
| raw | Probe/decode, normalization, AHD demosaic | Scientific correctness and model/encoding coverage unvalidated |
| tiff | RAW container parsing | Malformed data and allocation behavior |
| meta | Capture metadata | Missing fields and maker-note interpretation |
| raster / geom | Float buffers, crop and orientation | Peak allocations and coordinate conventions |
| color | Raw color data/API dependency | Generic fallbacks must not imply characterization |
| brand | Upstream required dependency | Keep upstream identity internal; public branding undecided |
| testkit | Resolve upstream development manifests | No product runtime role |

Direct crate dependencies added for the Rust port's R1 (2026-10-26), all already in the
lockfile through the vendored crates, so nothing new was downloaded: `half` 2.7 (MIT/Apache-2.0,
16-bit float frame stores), `quick-xml` 0.41 (MIT, lensfun XML), `rayon` 1.12
(MIT/Apache-2.0, parallel decoding), and the vendored `lcc-meta` (capture times). R5–R6 added
`jpeg-encoder` 0.7 (MIT/Apache-2.0, review JPEGs), `wgpu` 30 with `pollster` 1 and `bytemuck`
1 (MIT/Apache-2.0; the GPU sky stack), all from the local registry cache LightCraft uses. R7
added `sysinfo` 0.37 (MIT, free memory for the default memory budget). The built-in colour
matrices (`data/colour-matrices.json`) are Adobe's published ColorMatrix values for the Z6 II
and D700, read from LibRaw's table through the rawpy oracle. The desktop app
(`app/`) uses eframe/egui 0.36 (MIT/Apache-2.0) with `vendor/egui-wgpu`: egui-wgpu
0.36.2 (MIT OR Apache-2.0) copied from LightCraft Classic's vendor folder with its HDR display
patch (an `Rgba16Float` extended-linear-sRGB surface and egui drawn at SDR white), wired in through
`[patch.crates-io]`; licence texts are in that folder. The app's typeface is Inter 4
(Regular and SemiBold, SIL OFL 1.1) copied from LightCraft's `assets/fonts/` with its licence
(`app/assets/fonts/OFL-Inter.txt`; SHA-256 40d692fc… Regular, 78a843fa… SemiBold). The app
reads treatment previews with `image` 0.25 (MIT/Apache-2.0, JPEG decoding only). On macOS the menu bar comes from `muda` 0.21 (MIT/Apache-2.0).

Source files are copied verbatim from the pinned upstream commit. The subset workspace
Cargo.toml is adapted to expose only these crates and their dependency declarations.
No UI, profiles or image assets are adopted from Classic. The upstream brand config is
retained solely to satisfy upstream constants; this does not name the new product.

## Refresh 6f48dbd → dbe90dc (adopted)

Subset paths, snapshot → dbe90dc: 16 added, 58 modified, 19 renamed files
(+12,402/−430 lines). Additions include a GoPro VC-5 decoder, X-Trans
demosaic/benchmarks, Minolta MRW and PSD metadata, more packed-NEF layouts,
Kelvin presets, reduced-size DNG decoding and hostile-input size bounds.

Manifest adaptations: workspace version/rust-version (1.95), `chrono` 0.4.45,
`quick-xml` 0.41, and a declared but unvendored `lcc-codecs` entry used only by one
raw example. New lockfile crates: chrono, num-traits, autocfg (all MIT/Apache-2.0).
First-party code needed no change. Before adoption, a scratch build gave identical
`inspect` output for all 262 NEFs; after adoption, corpus validation passed and
full-frame TIFF pixel data for DSC_0226/0357/0487 hashed identically to the snapshot
decoder's output, with reports differing only in decoder revision.

## Attribution

Retain upstream LICENSE, LICENSE-APACHE, NOTICE and assets/ATTRIBUTION.md.
Most adopted code is MIT; raw spectral_table.rs has generated camera data derived
from rawtoaces-data v1.1.0 (Apache-2.0), including cited CIE CC-BY-4.0 source data.
Preserve the upstream attribution verbatim. Verify complete distribution obligations
and transitive dependency licenses before public release. Public project license
is not selected by copying the upstream workspace license field.

Vendored arw1 and crw decoders cite dcraw.c as a format reference (rw2, sr2 and the
NEF Huffman decoder declare clean-room work); the snapshot NOTICE does not mention it. Upstream NOTICE now states that some decoders
were translated from or written with reference to dcraw.c outside its restricted
Foveon block (adds orf). The refresh also adds vc5.rs, ported from GoPro's GPR SDK
(Apache-2.0 OR MIT), with a verbatim SDK table under tests. Upstream renamed its
product to Chromaforge in brand.toml; the repository URL is unchanged. These notices
are now vendored verbatim.

## Important behavior findings

- `RawImage::develop` includes opcodes and, for non-square sensor pixels, resampling.
  Initial diagnostic rejects sensor-stage opcodes/non-square resampling and records
  but defers post-demosaic opcodes; uses explicit normalization/demosaic/crop.
- `normalized` can replace non-finite samples with zero and substitute a unit scale
  for invalid white/black ranges. Wrapper rejects invalid samples/ranges beforehand.
- `normalized` scales each site by `1/(white − own black)`, not DNG's
  `1/(white − maximum plane black)`, and does not clip. Equivalent for uniform black
  (all local NEFs); differs when BlackLevelDeltaH/V vary.
- Camera transforms can use a generic model when no matrix exists. Initial output
  stays camera RGB and carries unapplied color/WB data instead.
- Local Z6 II NEFs carry no color matrix. The spectral table aliases `Z 6_2` to
  Nikon Z f data ("different model, same sensor"), and `stand_in` supplies a
  per-format camera; neither is called by the wrapper. Any use needs explicit provenance.
- Bayer AHD is available; stellar fidelity has not been established.
- White balance, baseline exposure and profile looks must not be applied invisibly.
- Header probing is not full pixel/compression validation.
- Table-less 34713 NEFs (Packed 12/14 bits) with a truncated strip decode with
  zero rows (snapshot and dbe90dc); the wrapper cannot yet detect this. Lossless Huffman
  NEFs fail explicitly: truncated copies of DSC_0226 (99/90/60%) were rejected.
- dbe90dc estimates Coolpix big-endian 16-bit black as `min(200, darkest sample)`
  when no maker-note black exists; the inferred value is not marked.
- Canon CR2 black is measured per frame from masked columns (black 0 when too few
  are found) and white is inferred from the data histogram, falling back to 16383.
  Level provenance is not reported. On a 40D, darks and ISO 800 lights decode white
  16383 while stars clip near 16003, so saturation goes unflagged (ADR 0006).

## Existing open-source data and oracles (not adopted)

Camera level and geometry data already maintained elsewhere; Pictor should not
re-derive it by measurement. RawTherapee `rtengine/camconst.json` (GPL-3.0) lists
per-ISO white levels, masked areas and crops (Canon EOS 40D: ISO 100 typical 13825,
matching local measurement). rawspeed `data/cameras.xml` (LGPL-2.1) lists black, white,
black areas and crops (40D black 1021, white 13600; Nikon Z 6_2 black 1008, white
15520 versus the decoder's 16383). LibRaw (LGPL-2.1/CDDL-1.0) derives many levels from
maker notes. Lens distortion/vignetting: lensfun (library LGPL-3.0, database
CC-BY-SA-3.0). The prototype reads its vignetting entries as an oracle and starting value
(all three owner lenses are profiled: uncorrected corners at 35%, 30% and 14% of centre for
the AF 24mm f/2.8D at f/2.8, Z 20mm f/1.8 S at f/1.8 and Z 24-70mm f/4 S at 24 mm f/4);
self-calibration from star motion is the goal, since profiles miss focus and copy variation. Calibration references: Siril (GPL-3.0), ccdproc (BSD-3-Clause),
DeepSkyStacker (BSD-3-Clause); no maintained Rust calibration crate was identified.
The vendored decoder is clean-room by upstream policy and uses none of these;
LightCraft's rules forbid GPL/LGPL data, so it will not adopt these white levels.

LightCraft session review (2026-10-25) of the relayed decoder findings: upstream
LightCraft rejects truncated packed NEF strips (since 001943f01) while Classic keeps
zero rows by design, so Pictor needs its own guard or a different source for
that path. The fallbacks are intended upstream; an issue requesting level provenance
is drafted for owner approval. Classic HEAD has since moved to 08f7526c8.

Per ADR 0007 these are test oracles first. rawspeed `cameras.xml` and RawTherapee
`camconst.json` are pinned in [oracle-lock.json](oracle-lock.json) (ignored
`.sources/oracles/`); `tools/oracle_levels.py` compares them with `pictor levels`.
Results over 528 local frames (no decode errors):

| Camera / ISO | Decoded white | Largest stored sample | RawTherapee | rawspeed |
|---|---|---|---|---|
| Canon 40D ISO 100 | 16383 (fallback) | 13824–13826 | 13700 | 13600 (all ISOs) |
| Canon 40D ISO 640 | 12522–12523 | 12573–12575 | 12600 | 13600 |
| Canon 40D ISO 800 | 16383 (fallback) | up to 16005 | 15900 | 13600 |
| Canon 40D ISO 1600 | 15940–15942, 16383 | up to 16013 | 14900 | 13600 |
| Nikon Z6 II ISO 1600 | 16383 | 16383 | none | 15520 |

RawTherapee whites are safe values just below typical clip; rawspeed uses one
conservative value per mode and measures 40D black from black areas. On 21 Z6 II
frames every CFA site piles up exactly at 16383 with no pile-up near 15520, so the
decoded Z6 II white matches observed clipping; whether 15520–16383 is linear is
unmeasured. Shipping any of this data would need a license decision; the oracle
path needs none.

## Star detection and registration candidates (2026-10-25)

| Candidate | License | Use so far |
|---|---|---|
| sep (Source Extractor library) | LGPL-3.0+ (MIT wrapper) | Research detection; oracle only |
| photutils, astropy | BSD-3-Clause | Installed for research; not yet used |
| astroalign | MIT | Research: first-pair triangle matching |
| rawpy / LibRaw 0.22.1 | MIT / LGPL-2.1 or CDDL-1.0 | Research RAW reading; decoder oracle |
| Siril | GPL-3.0 | Not used (calibration/registration oracle candidate) |
| astrometry.net | BSD-3-Clause (verify bundled parts) | Not used (plate solving) |
| Rust crates stardetect, twirs, arcsec | Licenses not yet verified | Not evaluated |
| Sequator, Starry Landscape Stacker | Closed source | Behavior references only |

Rust port R2 (2026-10-26): star detection is a clean-room implementation after the Source
Extractor documentation, checked against sep as a black-box oracle (sep's LGPL code is not
read or copied); triangle matching is a port of astroalign 2.6.2 (MIT, attribution kept in
`src/stars/triangles.rs`); RANSAC, least squares and Lucas–Kanade tracking are written in
the crate (OpenCV remains the oracle). No new crates.

No maintained Rust library covering wide-angle nightscape registration was found;
[ADR 0008](adr/0008-sky-registration.md) records the model measured with the research
tools. LibRaw reports Z6 II camera white 15311 (rawspeed 15520; decoder 16383).

## Candidates, not adopted implementations

Classic merge: HDR/panorama/focus, registration/RANSAC, transform refinement and
linear outputs. Assess geometry and scene assumptions before nightscape reuse.
Classic plugins: WASM sandbox (256 MiB default, 8 MiB file/HTTP body); later adapter
should hand jobs to native processing, not hold full-resolution stacks in WASM.
Classic external-editor support: useful future launch/return primitives; many-to-one
result import needs explicit design. UI and GPU crates remain audit candidates.

The two snapshots are available for future comparison. This milestone adopts the
Classic subset only; it does not claim the LightCraft implementations are equivalent.
