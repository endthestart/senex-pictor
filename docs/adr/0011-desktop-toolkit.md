# ADR 0011: egui on a shared wgpu device for the desktop app

Status: accepted, 2026-10-27. Evidence: a survey of Rust UI toolkits and the canvas spike (`spikes/canvas`, later the app crate); the owner ran the
spike on an Apple M2 Max and agreed to proceed.

## Context

The app must show 24–45 MP stacks with smooth zoom, pan and 100% inspection, swap two renderings
with nothing in between (the comparison research's instant toggle), recompute looks live, and
show colour accurately on wide-gamut Mac screens. Pictor already computes on wgpu 30.
LightCraft and LightCraft Classic, the owner's Rust photo editors, use egui/eframe on wgpu 30.

## Decision

1. The desktop UI uses egui/eframe 0.36 with LightCraft Classic's HDR-patched egui-wgpu
   (`vendor/egui-wgpu`, wired through `[patch.crates-io]`), not the newer LightCraft's egui and
   winit fork.
2. The app creates its wgpu device and hands it to eframe (`WgpuSetup::Existing`); rendered
   images stay on the GPU and are drawn by a paint callback. Nothing is read back to show them.
3. The window surface is `Rgba16Float` in extended linear sRGB where offered: colour-managed by
   macOS (P3 screens show P3 colours correctly) and able to show values above paper white when
   the display grants headroom. 8-bit sRGB surfaces are the fallback, encoded explicitly.
4. HDR (EDR) preview is not a requirement. It becomes one only with an HDR output for
   composites; until then values stay within paper white.

## Evidence

| Measure (45 MP canvas, two looks on the GPU) | Apple M2 Max |
|---|---|
| GPU time per frame into a 3456×2234 target | 0.64 ms fit, 0.55 ms 100%, 0.43 ms 200% |
| Look recomputed on the GPU, with mip levels | 5.4 ms median |
| Window, 30 s scripted pan, zoom and A/B swap | 8.3 ms median (120 Hz), p95 9.0 ms, worst 38 ms |
| Device shared with the window; surface | yes; `Rgba16Float`, `ExtendedSrgbLinear` |

The owner's display was in the Photography (P3-D65) reference preset, which grants no headroom;
the spike reported headroom 1.0 correctly. With HDR allowed, the owner saw the EDR test swatches
at 1×, 2× and 4× paper white in increasing brightness: values above white reach the screen.
On the Intel Mac Pro (W5700X, 580X) the same path works, with 24 MP frames under 1 ms of GPU
time; 45 MP timings there were taken while other models occupied both GPUs and are not
evidence. Slint 1.18 was the only credible alternative
(wgpu interop behind an unstable feature, GPLv3 or attribution licence, no LightCraft reuse).

## Consequences

Immediate-mode layout and a non-native look are accepted; LightCraft's theme, text field,
accessibility helpers and native menu/dialog code can be reused (MIT/Apache). Upgrading egui
means re-applying the vendored patch. Images larger than the GPU's texture limit (16384 px on
the tested GPUs) will need tiles; 45 MP fits in one texture.
