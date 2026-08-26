# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: WAITING_DEPENDENCY — journal-only durability admission and reconnect substrate
mode: IMPLEMENT
status: WAITING_DEPENDENCY
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-durability-journal
issue: 167
pr: null
architecture_decision_issue: 187
architecture_decision_pr: 190
architecture_decision_merge_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
architecture_decision_url: https://github.com/Oteryn/Oteryn-Game/issues/187
foundation_boundary_issue: 192
registry_issue: 193
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
updated_at: 2026-08-26T15:04:00+02:00
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
  - issue:192 Foundation boundary must merge before Durability resume
  - issue:193 registry bound must merge before Foundation child final acceptance
  - main@c57ddb5253cdfec126a768232d53f8a9bb292e3f or recorded successor
blocks:
  - Server Seam remains WAITING_DEPENDENCY until #192 merges, #167 receives a fresh exact-base resume allocation, and the durable adapter itself merges
write_authority: none_while_waiting_dependency
external_repositories: []
```

## Outcome

The architecture question is resolved by PR #190. This task remains fail-closed and may not resume implementation until Foundation successor #192 merges and the coordinator issues a fresh exact-base Durability resume allocation.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are coordinator-owned serialized paths and are not writable by this task.
- `PROVEN`: PR #182 merged the shared SQLx/Cargo/PostgreSQL prerequisite only; it did not create a Durability worker PR or authorize implementation.
- `PROVEN`: the only remote worker-branch provenance is `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`; local unpublished documentation checkpoint `3adf13ef17b3b7811aa4f73971456ecd321afcc2` is not a remote delivery.
- `PROVEN`: PR #190 / `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7` accepts `DUR-RECONNECT-AUTHORITY-V1`; Foundation successor #192 must implement and merge that boundary before this task can receive a fresh exact-base resume allocation.

## Acceptance criteria

- [ ] Real isolated PostgreSQL tests prove migration fresh/compatibility/checksum/ahead/behind/dirty/lock interruption and runtime-DDL denial.
- [ ] Durable journal proves fences, retry/lost-response classification, crash reconciliation and DB outage/recovery.
- [ ] Exact-head persistence/fencing review, CI, expected-head merge and archive lifecycle are complete.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write. While `WAITING_DEPENDENCY`, no code, schema, migration, test or worker-task mutation is authorized until #192 merges and a fresh exact-base resume allocation is recorded.

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
last_progress: PR #190 merged DUR-RECONNECT-AUTHORITY-V1 as main@2394f6f4633b8c6662d8d79a84110cc2ae13dcb7; architecture is resolved and #167 now waits on Foundation successor #192
status: WAITING_DEPENDENCY
branch: impl/game-durability-journal
head_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
pr: null
final_head_sha: null
owner_action_required: null
blocker: Foundation reconnect durability boundary Issue #192 is not yet merged
write_authority: none_while_waiting_dependency
next_action: keep this existing worker branch/task fail-closed until #192 merges, then receive a fresh exact-base resume allocation
```
