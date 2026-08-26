# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: WAITING_ARCHITECTURE — journal-only durability admission and reconnect substrate
mode: IMPLEMENT
status: WAITING_ARCHITECTURE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-durability-journal
issue: 167
pr: null
architecture_escalation_issue: 187
architecture_escalation_url: https://github.com/Oteryn/Oteryn-Game/issues/187
ownership_correction_authority: Oteryn/Oteryn-Game#187 comment 5424765487
ownership_correction_scope: active Durability task status/provenance/blocker/no-write/next-action only; no worker or runtime change
architecture_hold_main_sha: 007183ac7ef09dd4ae8d8f476d7ac943541d7d48
worker_branch_provenance: remote
worker_branch_remote_head: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
local_unpublished_documentation_checkpoint: 3adf13ef17b3b7811aa4f73971456ecd321afcc2
local_checkpoint_delivery_status: not_a_remote_delivery
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl durability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-26T13:42:59+02:00
execution_budget_minutes: 120
large_budget_reason: SQLx migration safety, durable idempotency/fencing and mandatory isolated PostgreSQL evidence
owned_paths:
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/reconcile.rs
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/migrations/0001_admission_reconnect_journal.sql
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
  - Server Seam remains WAITING_DEPENDENCY; it is not released by #167 while Issue #187 is unresolved
write_authority: none_while_waiting_architecture
external_repositories: []
```

## Outcome

No implementation may proceed while Issue #187 determines durable authority. The prior journal-only outcome remains historical intent, not current implementation authority.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are coordinator-owned serialized paths and are not writable by this task.
- `PROVEN`: PR #182 merged the shared SQLx/Cargo/PostgreSQL prerequisite only; it did not create a Durability worker PR or authorize implementation.
- `PROVEN`: the only remote worker-branch provenance is `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`; local unpublished documentation checkpoint `3adf13ef17b3b7811aa4f73971456ecd321afcc2` is not a remote delivery.
- `PROVEN`: the current synchronous Foundation `ReconnectAttemptJournal` cannot express all required FND-04/DUR-02 durable authority, revalidation and async-handoff semantics; [Issue #187](https://github.com/Oteryn/Oteryn-Game/issues/187) requires a durable accepted architect decision before this task can receive fresh authority.

## Acceptance criteria

- [ ] Real isolated PostgreSQL tests prove migration fresh/compatibility/checksum/ahead/behind/dirty/lock interruption and runtime-DDL denial.
- [ ] Durable journal proves fences, retry/lost-response classification, crash reconciliation and DB outage/recovery.
- [ ] Exact-head persistence/fencing review, CI, expected-head merge and archive lifecycle are complete.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write. While `WAITING_ARCHITECTURE`, no code, schema, migration, test or worker-task mutation is authorized.

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
last_progress: #167 is held at main@007183ac7ef09dd4ae8d8f476d7ac943541d7d48 because Issue #187 identifies a durable architecture conflict; PR #182 released only its shared prerequisite
status: WAITING_ARCHITECTURE
branch: impl/game-durability-journal
head_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
pr: null
final_head_sha: null
owner_action_required: durable accepted architect decision on Issue #187
blocker: current synchronous Foundation ReconnectAttemptJournal cannot express all FND-04/DUR-02 durable authority, revalidation and async-handoff requirements
write_authority: none_while_waiting_architecture
next_action: await the accepted architect decision on #187
```
