# Accepted requirements

Date: 2026-10-25. Owner: Michael Anderson. Source: eight-question planning interview.

## Product

| ID | Requirement | Verification |
|---|---|---|
| P01 | Public open-source standalone application first; integrations later. | Public release checklist and clean-machine install. |
| P02 | First release: fixed-tripod nightscapes with sky and foreground. | Representative end-to-end sequences. |
| P03 | Hundreds of full-resolution exposures; bounded memory; no implicit downsampling. | Large-session benchmark and dimension checks. |
| P04 | Editable master plus optional natural finished rendering. | Export metadata, numerical checks, visual review. |
| P05 | Realistic sky approximating a perfectly aligned long exposure. | Position, PSF, photometry, color and SNR evaluation. |
| P06 | Guided checkpoints first; automatic operation after measured reliability. | Correctable decisions and separate automation evaluation. |
| P07 | Preserve a reference moment or explicitly remove transient objects. | Scene-mode comparisons with visible provenance. |
| P08 | Separate foreground reference, including a flash-lit exposure. | Flash-reference scene review. |
| P09 | Moving foliage requires stability assessment and reference fallback. | Branch/leaf and occlusion cases, no invented detail. |
| P10 | Broad RAW decoder coverage; Nikon files are the first validated group. | Camera/compression-specific support table. |
| P11 | Optional calibration with capture guidance and matching checks. | Missing/mismatched dark, flat and bias cases. |
| P12 | Research RAW processing order; suggest consistent settings and explain uncertainty. | Scientific reference audit and comparative experiments. |
| P13 | macOS first, Linux second, Windows third. | Supported-platform release evidence. |
| P14 | Native Rust; prefer existing open-source libraries, tools and data (LightCraft among them) with attribution; contribute fixes upstream; where licensing is complicated, use them as test oracles and reimplement independently ([ADR 0007](adr/0007-open-source-first.md)). | Dependency/source audit, oracle comparisons, upstream contribution log. |
| P15 | CPU baseline, portable GPU, then platform-specific capabilities. | Backend contract and equivalence tests. |
| P16 | Efficient code may fully utilize hardware for difficult work. | Profiling, bounded concurrency and memory accounting. |
| P17 | No shortcuts: expose uncertainty, unsupported inputs and incomplete methods. | Error/recovery and quality gates; no silent substitutions. |
| P18 | Safe originals, resumable projects and reproducible processing. | Source hashes, persistence and interruption tests. |

## Agreed UX

Import/validate → review frames → review sky/ground → choose scene treatment →
inspect alignment/integration → export. Reliable decisions can be automatic;
uncertain decisions request review. Advanced controls remain available.
Corrections invalidate only dependent stages. Display changes do not rerun registration.

## Release scope

Onboarding, documentation, installable builds, examples with redistribution rights,
support/validation tables, reproducible builds, dependency notices and recovery behavior.
The first release is intended for the public, not only the owner's hardware.

## Deferred

DxO export integration; LightCraft plugin/native integration; tracked-sky plus separate
ground captures; star trails; deep sky; planetary/lunar/solar; mosaics.
Preserve independent processing interfaces for these, without building empty modules.

## Decisions still open

Product identity, outbound distribution license, portable GPU framework,
final master format/profile options, scientific algorithms and numerical acceptance thresholds.
Resolve by evidence and ADRs, without changing accepted product scope implicitly. The desktop
toolkit is decided in [ADR 0011](adr/0011-desktop-toolkit.md).

## Existing work

The Python implementation is a disposable POC. Preserve originals and useful baseline
evidence before retiring it. It imposes no compatibility or architectural requirement.
