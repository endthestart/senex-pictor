# ADR 0003: Guided realism and explicit scene treatment

Status: accepted, 2026-10-25.

Prioritize a faithful, aligned long-exposure sky and believable foreground. Produce
an editable master and optional natural finished rendering. Start with guided
checkpoints, inspectable uncertainty and corrections. Promote automatic decisions
only after evaluation against corrected and held-out sessions.

Offer reference-moment preservation and explicit cleanup of transient objects.
Allow a separate flash-lit foreground reference. Unstable foliage may require
reference fallback; do not assume reconstruction of every leaf or hidden surface.

Consequences: preserve masks, source choices, coverage and decision provenance.
No generative replacement of missing scene content in the first-release workflow.
Finishing changes must not silently alter the master or comparison brightness.
