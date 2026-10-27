# Processing specification

Status: product contract; algorithm choices require validation.

## Data and provenance

Every frame identifies original bytes by SHA-256, decoder/source revision, camera,
compression when known, sensor/active/crop dimensions, orientation, CFA, levels,
exposure metadata, calibration inputs, white balance, color characterization and
processing settings. Missing values stay missing. Record supplied vs inferred values.
Preserve negative linear samples and highlight headroom; do not clamp for display.
Use explicit units, pixel origin and coordinate transforms at every boundary.

Sensor mosaic, normalized mosaic, camera RGB, characterized scene-linear RGB and
display rendering are distinct stages. A camera-RGB diagnostic has no standard
RGB primaries and must never be mislabeled sRGB/Rec.2020.
TIFF display referents are not recovered RAWs merely by reversing gamma.

## RAW and calibration investigation

Specify and test the ordering of linearization, black subtraction, dark correction,
flat normalization, bad-pixel handling, demosaic and lens/geometric corrections.
Do not subtract darks from normalized/rendered data by default. Do not double-subtract
bias already present in another calibration master. Proposed units, order and matching
rules: [ADR 0006](adr/0006-calibration.md); library core only, not end-to-end.
Keep lens geometry deferred to a combined registration/resampling transform where
possible. Separate sensor corrections from aesthetic profile looks and local tone maps.
Inspect decoder opcodes, hot-pixel interpolation, clipping and camera fallback policies.

A sequence uses a selected shared development recipe. Automatic suggestions must
show evidence; differing capture metadata requires a decision, not per-frame auto tone.
White balance can use a reference or explicit shared setting. Capture-time metadata
and calibration compatibility remain inspectable. Finishing controls operate after
integration and cannot silently alter scientific master data.

## Registration and integration

Model sky and foreground separately. A single global mean residual is insufficient:
inspect spatial errors and independent matched stars, especially wide-angle corners.
Mask each input's foreground/occlusions before warping sky samples. Combine transforms
to minimize interpolation. Propagate validity, saturation, coverage and rejection.
Do not imply removed satellite/aircraft trails always disappear when observations
are insufficient. Record exclusions, reasons, parameters and remaining uncertainty.
Define whether output is an average or summed exposure; record effective exposure
and weights. Keep comparison brightness identical. A monitor rendering is not a
literal prediction of dark-adapted human vision.

## Foreground modes

Reference mode anchors scene content to a chosen moment. Flash reference may be
different from the sky alignment reference. Cleanup mode explicitly permits
removing transient objects using observed background from other frames.
Stable regions may be averaged; unstable leaves/branches/water/clouds can fall back
to reference data. Never invent hidden detail. Allow manual mask/reference overrides.

## Resource and backend contract

Stages declare capability, resource estimates, cancellation points and deterministic
settings. A scheduler budgets input buffers, analysis, CPU/GPU residency, caches and
scratch together. Parallelism is bounded. Projects with hundreds of frames must not
require holding all decoded frames in RAM. Preview proxies never become final data.
Acceleration changes implementation, not undocumented color or precision semantics.
Missing GPU capabilities fall back to CPU; incomplete algorithms remain explicit.

## Outputs

Primary: reproducible scene-linear master, format/profile choices pending ADR.
Compatibility: color-managed 16-bit TIFF with documented transfer function.
Optional: natural finished render, separate sky/ground/mask/coverage and report.
Diagnostic outputs must state stage, primaries, orientation and processing limitations.
Project saves include recipe, source hashes, corrections and algorithm/backend versions.
New output paths never overwrite originals. Interrupted writes must not look complete.
