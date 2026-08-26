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
foundation_boundary_pr: 199
foundation_boundary_merge_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
transport_ref_decision_issue: 197
transport_ref_decision_pr: 200
transport_ref_decision_merge_sha: dc531658c7ffc9af91ccc6719aee80ffe01c22a4
registry_issue: 193
registry_pr: 195
registry_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
ownership_correction_authority: Oteryn/Oteryn-Game#187 comment 5424765487
ownership_correction_scope: active Durability task status/provenance/blocker/no-write/next-action only; no worker or runtime change
architecture_hold_main_sha: 007183ac7ef09dd4ae8d8f476d7ac943541d7d48
worker_branch_provenance: remote
worker_branch_remote_head: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
local_unpublished_documentation_checkpoint: 3adf13ef17b3b7811aa4f73971456ecd321afcc2
local_checkpoint_delivery_status: not_a_remote_delivery
resume_base_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
resume_strategy: normal_non_force_merge_up_existing_worker_branch
base_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl durability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-26T22:11:00+02:00
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
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
  - Foundation V1 reconnect durability boundary merged by PR #199
  - foundation::admission_facade::ReconnectAttemptJournal remains compatibility/in-memory behavior only
depends_on:
  - issue:162 allocation lineage
  - issue:167
  - issue:192 completed by pr:199 / main:90f30b47ac9b1e5e41cf274caf707aa39109b0c0
  - issue:193 completed by pr:195 / main:9878d42a21815027ef88067bfc59f8b40e78b473
  - issue:197 completed by pr:200 / main:dc531658c7ffc9af91ccc6719aee80ffe01c22a4
blocks:
  - Foundation terminal reconciliation snapshot repair #208 / allocation PR #209 must merge before this worker resumes Task 1 or Task 2
  - Server Seam remains WAITING_DEPENDENCY until this resumed Durability worker merges the real durable adapter
write_authority: none_until_foundation_terminal_reconciliation_repair_merges
shared_paths: none
external_repositories: []
```

## Outcome

Architecture #187/#190, transport-ref semantics #197/#200, the retained-attempt registry #193/#195 and the Foundation reconnect boundary #192/#199 are all merged. After this resume-allocation PR merges, the existing Durability worker may resume on its exact owned paths against protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0` without any owner decision.

The worker must preserve its published branch history. First perform a normal non-force merge-up of the fresh protected main into `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`, review the resulting diff and then continue TDD only inside the owned paths above. Do not reset/recreate/force-push the worker branch merely because main advanced.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are not writable by this task; PR #182 already merged the accepted shared SQLx/Cargo/PostgreSQL prerequisite.
- `PROVEN`: the remote worker branch remains `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`; local unpublished checkpoint `3adf13ef17b3b7811aa4f73971456ecd321afcc2` remains non-authoritative and is not a delivery.
- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #200 merged transport-ref uniqueness as `dc531658c7ffc9af91ccc6719aee80ffe01c22a4`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: PR #199 merged the Foundation V1 reconnect durability boundary as protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0` after exact-head `FOUNDATION_RECONNECT_DURABILITY_V1 / PASS`, full Cargo 1.94/CI and `game-gate` PASS.
- `DERIVED`: the former architecture/Foundation dependency blocker is terminally resolved; Server Seam is still blocked on the real durable adapter, not on architecture.

## Acceptance criteria

- [ ] Worker merge-ups protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0` into the existing branch without force/reset and verifies the post-merge diff is ownership-correct.
- [ ] Real isolated PostgreSQL tests prove migration fresh/compatibility/checksum/ahead/behind/dirty/lock interruption and runtime-DDL denial.
- [ ] Durable journal/adapter consumes the merged Foundation V1 boundary and proves fencing, same-attempt retry/lost-response classification, crash reconciliation and DB outage/recovery without moving Foundation admission/security/controller authority.
- [ ] PREPARE/COMMIT persistence and reconciliation preserve the exact V1 attempt/transport-ref/evidence/deadline semantics; ambiguous outcomes reconcile the same attempt rather than reminting authority.
- [ ] Exact-head persistence/fencing review, CI, expected-head merge and archive lifecycle are complete.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write. No new Cargo/workflow/shared-surface authority is granted by this resume allocation.

## Validation

### Focused

- command/run: worker TDD after resume-allocation merge
- result: pending

### Component/integration

- command/run: isolated PostgreSQL worker evidence after resume-allocation merge
- result: pending

### E2E

- scenario: real isolated PostgreSQL DB E2E required during Durability delivery; gameplay Tier 1/Tier 2 `NOT_APPLICABLE`
- result: pending

## Context checkpoint

```yaml
last_progress: #208 identified the Foundation-owned terminal reconciliation snapshot constructor required by the V1 durable adapter; Durability pauses without Foundation-path ownership
status: WAITING_DEPENDENCY
branch: impl/game-durability-journal
head_sha: 7ac06bd84a1a31fc9a3ea2560de8ae20cea96741
resume_base_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
pr: null
final_head_sha: null
owner_action_required: no architecture decision required; await bounded Foundation repair #208
blocker: Foundation terminal reconciliation snapshot constructor is required for external terminal same-attempt reconciliation and is outside Durability ownership
write_authority: none_until_foundation_terminal_reconciliation_repair_merges
next_action: after #208 merges, refresh protected main into the existing worker branch, verify the repair and restored allocation authority, then resume TDD only on the exact Durability-owned paths
```
