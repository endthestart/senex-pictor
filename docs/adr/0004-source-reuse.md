# ADR 0004: Isolated, pinned source reuse

Status: accepted for development, 2026-10-25. Vendored Classic refreshed to upstream dbe90dc.

Never update or reset a dirty sibling checkout. Attempt upstream clones into ignored
`.sources/`; if network is unavailable, use isolated local HEAD snapshots and record
the limitation. Pin exact revisions and vendor only required source components with
licenses, attribution and SHA-256 inventory. Do not use absolute sibling build paths.

Vendored source is third-party material retained verbatim, including original file
boundaries and long files; the 300-line rule applies to new/edited first-party files.
Do not split third-party files merely to satisfy a first-party formatting convention.
Manifest adaptation is recorded; upstream tests remain upstream tests.

Consequences: local builds remain reproducible offline from the saved subset and
Cargo.lock. Update requires a new audit and hash manifest, not an unreviewed moving
branch dependency. Obtain upstream freshness before M0 is declared complete.
