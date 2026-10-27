# ADR 0002: Layered processing availability

Status: accepted, 2026-10-25.

Provide complete CPU processing, portable GPU acceleration, then platform-specific
GPU enhancements. Gate implementations by actual device capabilities, not OS names.
Algorithms declare numerical contracts; backend dispatch cannot silently change them.

Budget RAM, VRAM/unified memory, staging and scratch together. Profile difficult
stages and use hardware fully within those budgets. Validate on Intel Mac/AMD, M2,
Linux/AMD and eventually Windows. Device failures require recoverable CPU fallback.

Consequences: a faster backend is not required for faithful output. Precision
tolerances and expensive optional methods need explicit testing and documentation.
No GPU framework or platform-specific kernel is selected in this record.
