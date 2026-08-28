# Oteryn Game Durability Journal — Implementation Plan

**Goal:** Deliver the accepted journal-only PostgreSQL substrate for admission/reconnect reconciliation, preserving a fail-closed boundary around all value and general durability work.

**Scope and exact worker paths:**

- `apps/game-server/src/durability/mod.rs`
- `apps/game-server/src/durability/db.rs`
- `apps/game-server/src/durability/schema.rs`
- `apps/game-server/src/durability/admission_journal.rs`
- `apps/game-server/src/durability/reconcile.rs`
- `apps/game-server/src/bin/oteryn-game-migrate.rs`
- `apps/game-server/migrations/0001_admission_reconnect_journal.sql`
- `apps/game-server/build.rs`
- `apps/game-server/tests/durability_postgres.rs`
- `apps/game-server/tests/support/postgres.rs`
- `docs/agents/tasks/active/OTV2-20260825-impl-durability.md`

The worker receives no shared-path authority. `apps/game-server/src/lib.rs`, `apps/game-server/Cargo.toml`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/rust.yml`, and `.gitattributes` are a serialized coordinator lease; a worker that needs a change there returns `BLOCKED` with evidence.

## Global constraints

- Start only from the allocation-recorded `main` SHA and write only listed paths on one branch/PR for Issue #167.
- Use the accepted SQLx `0.9.0`, game-server-local module, one game-owned migration ledger, dedicated migration execution, isolated non-production PostgreSQL E2E, and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE` topology.
- Implement a durable adapter for existing `foundation::admission_facade::ReconnectAttemptJournal`. The database is never a second runtime simulation writer; startup cannot migrate and runtime DDL must be denied.
- Verify migration fresh/compatibility/checksum/ahead/behind/dirty/lock-interrupt behavior; verify reconnect fencing, lease scope, exact retry, lost response/ambiguous classification, crash window and DB outage/recovery.
- Exclude transactions/outbox, all items/value custody/mint/XP/rewards, Foundation semantics, `main.rs`, registries, production database/config/secrets, Platform/Atlas/META and external repositories. DUR03-RL-01..08 stay fail-closed excluded.

## Task 1: Establish isolated PostgreSQL test and migration failures first

1. Create guarded test infrastructure that refuses non-local/non-test targets and guarantees cleanup.
2. Write failing migration and journal tests for fresh application, compatibility/checksum, ahead/behind/dirty/interrupt locks, startup non-migration, and runtime DDL denial.
3. Write failing admission-journal tests for fence/lease scope, duplicate retry, lost-response classification, reconciliation after crash, and DB outage/recovery.
4. Record the exact isolated test preconditions and red-suite results in the task packet.

## Task 2: Implement the journal-only durable substrate

1. Implement schema/version inspection, migration command, and database connector in the allocated paths using accepted SQLx topology only.
2. Implement the durable `ReconnectAttemptJournal` adapter with idempotent attempt identity and fail-closed classification/reconciliation.
3. Ensure runtime code cannot run migrations or DDL and cannot use the database as a live simulation writer.
4. Run focused tests after every increment; do not add value, outbox, account policy, or generalized persistence abstractions.

## Task 3: Prove failure behavior and exact-head quality

1. Run isolated PostgreSQL focused/component tests for all named migration and reconnect cases, including exact max/retry/ambiguous failure behavior.
2. Run workspace/governance checks required for the exact head; Tier 1/Tier 2 gameplay E2E remains `NOT_APPLICABLE`, while database E2E is mandatory and must be real, not mocked.
3. Perform full-diff self-review and genuinely independent exact-head persistence/fencing review under root policy. Fix material findings only inside allocation.

## Task 4: Delivery and Server Seam handoff

1. Freeze exact head; verify CI and zero review threads; merge only with expected head.
2. Post-merge read back protected `main` and archive/release this worker task.
3. Re-evaluate Server Seam only after the merged durable adapter proves compatibility; it receives no implicit listener/client/deployment authority from this plan.
