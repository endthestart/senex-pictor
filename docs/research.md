# Research agenda and references

Prior research: official manuals reviewed on 2026-10-25; not a hands-on benchmark.

## Product references

- SLS: https://sites.google.com/site/starrylandscapestacker/processing-images-with-starry-landscape-stacker-version-1-9
  Star correction, sky masks/islands, composition choices, calibration and mask reuse.
- Sequator: https://sites.google.com/view/sequator/manual
  Separate sky/ground integration, distortion handling, linear input expectations,
  memory/disk behavior and exposure-consistency assumptions.
- Siril: https://siril.readthedocs.io/en/stable/
  Calibration, registration, statistics, rejection and scripted repeatability.
- DSS: https://github.com/deepskystacker/DSS/releases
  Focused preprocessing; contemporary releases support macOS/Linux/Windows.
- APP: https://www.astropixelprocessor.com/
  Distortion-aware registration, mosaics, normalization and diagnostic maps.
- PixInsight: https://pixinsight.net/
  Specialist processes, reusable settings and interrupted batch recovery.
- AutoStakkert!: https://www.autostakkert.com/wp/guides/
  Lucky imaging is a distinct future workflow.
- RegiStax: https://www.astronomie.be/registax/linkedwavelets.html
  Separate detail enhancement from integration.
- StarStaX: https://markus-enzweiler.de/software/starstax/
  Trails, gap filling and cumulative sequences belong in a separate module.
- DxO: https://userguides.dxo.com/en/photolab/exporting-images
  External-application handoff is documented; embedded third-party SDK not established.

## Investigations before scientific release claims

1. RAW order: original sensor units, decoder modifications, dark/bias accounting,
   flat-field construction and uncertainty. Use primary specifications and papers.
2. Shared development: compare synchronized pre-stack edits against conservative
   linear development followed by post-stack finishing, with matched output brightness.
3. Stellar demosaic: test AHD/Detail and alternatives for small PSFs, false color,
   undersampling and photometry. Existing availability is not quality validation.
4. Color: measured vs embedded matrices; missing characterization; white balance;
   spectrally unusual sources; highlight handling; preserve provenance.
5. Geometry: camera/lens distortion vs sky projection, occlusions, one-resample path,
   independent validation stars and spatial residuals. First evidence:
   [ADR 0008](adr/0008-sky-registration.md) (sphere rotation with self-calibrated lens).
6. Foreground: stability measures, reference fallback, optional transient removal
   and explicit limits for persistent occlusion and changing foliage.
7. GPU: assess portable Rust frameworks against Intel-Mac AMD, M2 and Linux AMD;
   test precision, device loss, resource accounting and CPU equivalence.
8. UI: evaluate native Rust toolkit quality, accessibility, color-managed display,
   full-resolution inspection and responsiveness. No toolkit accepted yet.

Document experiments, primary sources, algorithms and rejected alternatives in ADRs.
Do not copy proprietary implementations or assume open-source availability alone
establishes licensing compatibility for a particular component or dataset.
