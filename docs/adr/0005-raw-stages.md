# ADR 0005: Explicit RAW stages and conservative diagnostics

Status: accepted stage separation, 2026-10-25; calibration/color methods pending.

Distinguish sensor mosaic, normalized mosaic, demosaiced camera RGB, characterized
scene-linear working RGB and finished display rendering. Never label unknown camera
primaries as a standard RGB space. Preserve negative samples and floating-point range.
Record levels, crop, orientation, white balance, opcodes and source identity.

The first vertical slice may produce a full-size camera-RGB diagnostic and report.
It is not a calibrated/color-managed public master. Reject unreviewed sensor-stage
opcodes and non-square sensor resampling. Record/defer post-demosaic opcodes rather
than silently modifying geometry or vignetting.
Do not use upstream generic color fallbacks or hot-pixel replacement without an
explicit scientific decision. Audit normalization before treating it as trusted.

Consequences: broad decoder availability is not a camera validation claim. Calibration,
shared white balance, characterization, TIFF input and public exports have separate
completion gates; no placeholder implementation implies they are available.
