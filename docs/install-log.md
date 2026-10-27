# Development install log

Development-only tools installed on the owner's machine. Product code does not depend
on them. Remove entries marked removable once their phase is finished.

| Date | Tool and version | Method / location | Purpose | Phase | Remove with | Status |
|---|---|---|---|---|---|---|
| 2026-10-25 | Python venv (CPython 3.13.16, uv-managed) with numpy 2.5.4, scipy 1.18.1, astropy 8.0.2, photutils 3.1.1, sep 1.4.1 (LGPL-3.0+), astroalign 2.6.2, rawpy 0.27.1 (bundles LibRaw 0.22.1), tifffile 2026.10.15, scikit-image 0.26.0, pillow 13.0.0, opencv-python-headless 5.0.0.93 | `uv venv` + `uv pip install`; `star-photos/.venv/` (ignored, 292 MB). Replaced an earlier 3.14 venv, which could not install ML runtimes | Phase 1 research: frames, stars, registration, separation, stacking; LibRaw as decoder oracle | 1 | `rm -rf .venv` | installed |
| 2026-10-25 | Python venv (CPython 3.12, uv-managed) with torch 2.2.2, torchvision 0.17.2, numpy 1.26.4, segment-anything (git main), opencv-python-headless, pillow | `star-photos/.venv-sam/` (ignored, 877 MB). torch 2.2.2 is the last Intel-macOS build; SAM 2 needs newer torch | Phase 1 sky/ground segmentation (SAM v1) | 1 | `rm -rf .venv-sam` | installed |
| 2026-10-25 | SAM ViT-B weights `sam_vit_b_01ec64.pth` (Apache-2.0; SHA-256 ec2df62732614e57411cdcf32a23ffdf28910380d03139ee0f4fcbe91eb8c912) and LICENSE | `star-photos/.models/sam/` (ignored, 368 MB) from dl.fbaipublicfiles.com | Segmentation model for the above | 1 | `rm -rf .models` | installed |
| 2026-10-25 | uv-managed CPython 3.13.16 and 3.12 | `~/.local/share/uv/python/` | Interpreters for the venvs above | 1 | `uv python uninstall 3.13 3.12` | installed |
| 2026-10-25 | Data (not an install): half-res frame cache of the Z6 II sequence | `/Volumes/Work/star-photos-data/validation/iter1/cache/` (19 GB; moved from the repository 2026-10-26) | Iteration 1 inputs | 1 | `rm -rf /Volumes/Work/star-photos-data/validation/iter1/cache` | present |
| 2026-10-26 | Data (not an install): full-resolution float16 frame cache and stage outputs of the Z6 II sequence | `/Volumes/Work/star-photos-data/validation/iter2/` (38 GB; moved from the repository 2026-10-26) | Iteration 2 inputs and results | 1 | `rm -rf /Volumes/Work/star-photos-data/validation/iter2/cache /Volumes/Work/star-photos-data/validation/iter2/stages` | present |
| 2026-10-26 | Data (not an install): six owner night-sky datasets (RAW, ON1 sidecars, edited JPEGs, videos) with SHA-256 manifests, plus pipeline work folders | `/Volumes/Work/star-photos-data/` (58 GB of copies; work folders grow per run) | Phase 1 development and comparisons | 1–2 | `rm -rf /Volumes/Work/star-photos-data` | present |
| 2026-10-25 | Data (not an install): Canon EOS 40D calibration sets (zips and extracted) | `/Volumes/Work/star-photos-data/validation/datasets/` (6.3 GB; moved from the repository 2026-10-26) | Calibration real-data evaluation | 2 | `rm -rf /Volumes/Work/star-photos-data/validation/datasets` | present |

## 2026-10-26: lensfun vignetting data (read in place, nothing installed)

`tools/proto/vignetting.py` reads lensfun's XML database as bundled with the owner's
darktable install (`/Applications/darktable.app/Contents/Resources/share/lensfun/version_1`,
data CC-BY-SA 3.0) for review renderings. No package or file was added; remove nothing.

## 2026-10-26: Rust crates for R5 and R6, and Rust stores

| Date | What | Where | Why | Phase | Remove | State |
|---|---|---|---|---|---|---|
| 2026-10-26 | Crates `jpeg-encoder` 0.7 (MIT/Apache-2.0), `wgpu` 30 (MIT/Apache-2.0, Metal/Vulkan/DX12 backends), `pollster` 1 (MIT/Apache-2.0), `bytemuck` 1 (MIT/Apache-2.0/Zlib) and their dependencies | `Cargo.toml`, `Cargo.lock`, `~/.cargo/registry` (already cached for LightCraft) | Review JPEGs; GPU sky stack | 1 | remove from `Cargo.toml` | installed |
| 2026-10-27 | Crate `sysinfo` 0.37 (MIT), `system` and `disk` features | `Cargo.toml`, `Cargo.lock`, `~/.cargo/registry` (already cached) | Free memory for the default memory budget; free disk space before ingest | 1 | remove from `Cargo.toml` | installed |
| 2026-10-27 | Rust standard library for `aarch64-apple-darwin` (rustup target, stable 1.99.1) | `rustup target add aarch64-apple-darwin`; `~/.rustup/toolchains/stable-x86_64-apple-darwin/lib/rustlib/aarch64-apple-darwin` | Apple Silicon builds of the canvas spike for the owner's laptop | 1 | `rustup target remove aarch64-apple-darwin` | installed |
| 2026-10-26 | Data (not an install): Rust stores and stage outputs of the five owner sequences | `/Volumes/Work/star-photos-data/*/rust/` (about 190 GB) | Rust port parity runs | 1 | `rm -rf /Volumes/Work/star-photos-data/*/rust` | present |
| 2026-10-26 | Data (not an install): camera colour matrices for the Z6 II and D700, read from LibRaw 0.22.1 through rawpy | `/Volumes/Work/star-photos-data/colour-matrices/` | Review renderings (Nikon RAW files carry no matrix) | 1 | `rm -rf /Volumes/Work/star-photos-data/colour-matrices` | present |
