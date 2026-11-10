# Determinism primitives and local operations viewer

Input: `Oteryn/Oteryn-Game@4f0b6c62a789489fca6c37dd36d5917d096f328e`.
Reviewer: lead. Status: **REVIEWED**, not runtime VERIFIED.

## oteryn-simulation-determinism

All tracked package files were substantively inspected: its manifest, local governance and complete implementation/inline tests. The crate stays within its numeric, deterministic decision, canonical state and semantic-time role. Checked exact arithmetic, equal-scale enforcement and scale bounds were followed. Ratio multiplication widens two i64 operands into i128 before division; the private rounding helper is reached with that bounded numerator. Sign normalization and all four rounding modes were inspected, including negative values and ties to even. A hypothetical arbitrary i128-min input is not a demonstrated production caller.

Decision derivation binds its fixed domain, profile, root, occurrence, purpose length/content and decision index. Canonical state bounds the entry count and per-entry sizes, sorts keys, rejects duplicates and length-delimits hashed fields. Semantic time uses checked arithmetic. The seven inline tests and cross-platform golden assertions were inspected but not executed. Actual combat-rule decision consumers were traced. No additional confirmed defect was identified in the reviewed scope. The pre-existing RNG-001 seed-secrecy question remains a separate LIKELY finding with its original unresolved production-evidence requirement; this package review neither upgrades nor dismisses it.

## oteryn-game-ops-top

Both tracked files were fully inspected, including the terminal cleanup guard, response limit, read timeout, refresh/diagnose state, filtering, event loop, rendering and inline test. Health deserialization was checked against the actual server control-health producer and literal health request route in `apps/game-server/src/node/serve.rs:2571-2588,2635-2658`. The producer supplies the fields consumed by the viewer. Unknown/placeholder producer metrics are not treated as verified live measurements by this audit.

The tool is read-only. It sends local health/diagnose requests, shuts down its write half and bounds response reads to 64 KiB. Terminal mode restoration is guarded on return/error. Failed refresh leaves an explicit unavailable status while retaining the last health snapshot. A local socket-connect delay was considered, but a concrete accepted-contract violation or production failure was not established; no speculative finding was counted. No terminal/Unix-socket integration execution was performed.

## Limits

Source and test inspection establish review coverage. They do not prove Windows/native rendering, production service health, cross-platform execution or the full Cargo dependency graph. No source change was made.
