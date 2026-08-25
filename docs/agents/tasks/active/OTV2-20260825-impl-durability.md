# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: Implement journal-only durability admission and reconnect substrate
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-durability-journal
issue: 167
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl durability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-25T23:24:03+02:00
execution_budget_minutes: 120
large_budget_reason: SQLx migration safety, durable idempotency/fencing and mandatory isolated PostgreSQL evidence
owned_paths:
  - apps/game-server/src/durability/**
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/migrations/**
  - apps/game-server/build.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/active/OTV2-20260825-impl-durability.md
public_contracts:
  - OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24
  - foundation::admission_facade::ReconnectAttemptJournal
depends_on:
  - issue:162 allocation merge
  - issue:167
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - Server Seam re-evaluation after durable adapter merge
external_repositories: []
```

## Outcome

Fail-closed SQLx 0.9.0 journal-only migration/reconnect substrate that never acts as a live runtime simulation writer.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are coordinator-owned serialized paths and are not writable by this task.

## Acceptance criteria

- [ ] Real isolated PostgreSQL tests prove migration fresh/compatibility/checksum/ahead/behind/dirty/lock interruption and runtime-DDL denial.
- [ ] Durable journal proves fences, retry/lost-response classification, crash reconciliation and DB outage/recovery.
- [ ] Exact-head persistence/fencing review, CI, expected-head merge and archive lifecycle are complete.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write.

## Validation

### Focused

- command/run: pending allocation merge
- result: pending

### Component/integration

- command/run: pending allocation merge
- result: pending

### E2E

- scenario: real isolated PostgreSQL DB E2E required after allocation; gameplay Tier 1/Tier 2 `NOT_APPLICABLE`
- result: pending

## Context checkpoint

```yaml
last_progress: task packet created; no worker authority before allocation merge
status: waiting
branch: impl/game-durability-journal
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: allocation authority is unmerged
next_action: coordinator merges exact allocation then records worker base SHA
```
