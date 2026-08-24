# OTV2-DURABILITY-TOPOLOGY — Durability Topology Preparation

Short alias:

```text
Oteryn: prep durability topology
```

Mode: `PREPARE / DECIDE`, not runtime implementation.

Work only under an exact coordinator allocation for Issue #94. Read live `main`, root/nearest governance, ADR-0004, DUR-01/02/03, FND-03/FND-04, GAME-CHAR, GAME-ITEM, ANL-01, current workspace/Cargo topology and the current next-wave master plan.

Produce `docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md` with exact proposed runtime/migration/test/Cargo/shared paths, PostgreSQL client/migration candidate comparison, Rust 1.94 compatibility, immutable migration history, dedicated migration execution, fail-closed schema compatibility, isolated test database strategy and async PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE boundaries.

Evaluate the bounded default first: `apps/game-server/src/durability/**` plus one game-owned migration ledger and isolated non-production PostgreSQL tests. A dedicated crate requires demonstrated immediate-consumer need.

Explicitly inventory every DUR-03 amplification/count/depth/work dimension exercised by the first increment. If any required finite bound remains unresolved, mark it as a blocking owner-decision/registry gate; do not leave it as a non-blocking note and do not let the later implementation worker select a number.

This task grants no code, DDL, dependency, migration or production database authority. End with an exact implementation-allocation proposal only. Validate governance, diff quality and whole-diff self-review before merge/closeout.
