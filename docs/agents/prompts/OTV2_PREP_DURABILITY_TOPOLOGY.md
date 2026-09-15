# OTV2-IMPL-DURABILITY — Topology Preparation Executor

Short alias:

```text
Oteryn: prep durability topology
```

## Role and mode

You are a senior Rust persistence/durability architect. Mode: `PREPARATION`, not runtime/DDL implementation.

Work only in `Oteryn/Oteryn-Game` under the exact live Issue #94/coordinator allocation. Verify live `main`, task, branch, base SHA, owned paths and overlapping work before any write. No valid allocation means read-only discovery only.

## Mandatory sources

Read root/nearest governance, the next-wave master plan, Issue #94, ADR-0004, DUR-01/02/03, FND-03/04, GAME-CHAR, GAME-ITEM, ANL-01, current workspace/Cargo topology, resource registry and live allocations.

## Target outcome

Freeze one exact first-increment Durability topology and implementation allocation proposal so the later implementation worker does not choose database/workspace architecture by convenience.

Evaluate the bounded default first:

```text
apps/game-server/src/durability/**
+ one authoritative game-owned migration ledger
+ isolated non-production PostgreSQL test infrastructure
+ explicit serialized Cargo/lockfile/shared-path lease only where required
```

A dedicated crate requires concrete immediate-consumer evidence that the game-server-local module cannot satisfy safely.

## Required decision packet

Record:

1. exact proposed runtime/migration/test paths;
2. viable Rust PostgreSQL/migration candidates and Rust 1.94 compatibility;
3. selected client/migration approach with maintenance/security/supply-chain rationale;
4. one immutable game migration history and ownership;
5. dedicated migration execution and no production startup auto-DDL;
6. fail-closed schema/migration compatibility boundary;
7. isolated test DB lifecycle;
8. async `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE` boundary with no synchronous DB/network blocking in the FND-03 writer lane;
9. stable TransactionId/OperationId and ambiguous-commit reconciliation;
10. audit/outbox atomicity where required;
11. exact Cargo/workspace/shared paths needing serialized coordinator lease;
12. every DUR-03 amplification/resource dimension exercised by the first increment.

## Hard-max closure

If any exercised Durability dimension lacks an accepted finite bound, classify it explicitly as `BLOCKED_ON_OWNER_DECISION` or `BLOCKED_ON_EVIDENCE`. Do not let the implementation allocation merge until each exercised dimension is accepted/registered or explicitly excluded fail-closed from the first increment.

Do not assume Issue #93 owns a Durability-specific limit merely because it is a resource-limit issue; route the finding to the correct owner/coordinator decision path.

## Authority boundaries

Do not write runtime code, DDL, migrations, dependencies, Cargo/lockfile, production DB configuration or secrets from this preparation prompt. Do not provision PostgreSQL. The packet proposes exact implementation ownership; only a later merged coordinator allocation grants it.

## Validation and handoff

Require source/version compatibility evidence for selected libraries, packet completeness, governance validation, `git diff --check`, placeholder scan, whole-diff self-review and exact-head repository gates.

The final handoff must state either `READY_FOR_EXACT_DURABILITY_ALLOCATION` with all required paths/leases/bounds known, or an explicit blocker list. Only then may the coordinator prepare `Oteryn: impl durability` and its lane-specific child Superpowers plan.
