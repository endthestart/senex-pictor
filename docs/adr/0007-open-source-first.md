# ADR 0007: Open-source first, oracles where licensing is complicated

Status: accepted, 2026-10-25 (owner direction). Supersedes the LightCraft-only reading
of P14; ADR 0004 still governs how vendored source is pinned and inventoried.

Pictor is entirely open source. For every component (RAW decoding, camera level
and color data, calibration, demosaic, lens geometry, star detection, registration,
UI, GPU) first look for maintained open-source implementations and data, and record
license, maintenance and fit in the [reuse audit](../reuse-audit.md).

- License-compatible and suitable: use as a dependency or vendor it (ADR 0004), with
  attribution retained.
- Licensing complicated or incompatible with the outbound license (still undecided):
  use it as a test oracle only. Pin the oracle's revision and hashes in ignored
  `.sources/`, never redistribute it, and implement independently. Copying code or
  data values from an oracle into Pictor is not an independent implementation.
- Oracles come first; adoption is revisited only where oracle comparison is not
  enough.
- Bugs found and features built in upstream projects go back upstream with evidence.
  LightCraft issues go to the LightCraft session; posting to any public tracker or
  repository needs owner approval for that post.

Consequences: building an algorithm or table starts with a search for an existing
implementation. Oracle disagreements are findings, not automatic corrections: each
needs a source or measurement before Pictor changes behavior.
