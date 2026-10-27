# ADR 0001: Standalone Rust architecture

Status: accepted, 2026-10-25.

Build a public open-source native Rust standalone application before editor adapters.
Keep image processing independent of UI, catalog and host APIs. Use a headless path
for repeatable validation. Reuse suitable open-source components with provenance (ADR 0007).

First supported task: fixed-tripod nightscapes. Other astronomy tasks use distinct
future workflows sharing useful primitives. No empty modules or general catalog editor.

Consequences: CPU correctness is available without a desktop UI; adapters remain thin.
No Swift-only implementation or continuation of the Python POC is required.
Product identity, UI toolkit and public license are separate decisions.
