# Architecture decision records

Accepted means agreed product/architecture direction. Proposed means not selected.

| Record | Status |
|---|---|
| [0001 Standalone Rust architecture](0001-standalone-rust.md) | Accepted |
| [0002 Capability layers](0002-capabilities.md) | Accepted |
| [0003 Guided realism](0003-guided-realism.md) | Accepted |
| [0004 Pinned source reuse](0004-source-reuse.md) | Accepted for development; vendored Classic current at dbe90dc |
| [0005 RAW stages](0005-raw-stages.md) | Accepted stage separation; scientific methods pending |
| [0006 Sensor-stage calibration](0006-calibration.md) | Proposed; core and synthetic-truth tests exist, real captures pending |
| [0007 Open-source first](0007-open-source-first.md) | Accepted |
| [0008 Sky registration](0008-sky-registration.md) | Proposed; prototype evidence on three Z6 II sequences |
| [0009 Sky separation](0009-sky-separation.md) | Proposed; prototype evidence on three Z6 II sequences |
| [0010 Sequence check and drift](0010-sequence-check-and-drift.md) | Proposed; prototype evidence on Yellowstone (D700) and BWCA |
| [0011 Desktop toolkit](0011-desktop-toolkit.md) | Accepted: egui on a shared wgpu device |

Future ADRs: public license, GPU framework, color characterization,
master/export formats, distortion model, registration and rejection methods.
