# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: IN_PROGRESS — journal-only durability admission and reconnect substrate
mode: IMPLEMENT
status: IN_PROGRESS
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-durability-journal
issue: 167
pr: 212
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
foundation_terminal_repair_issue: 208
foundation_terminal_repair_pr: 210
foundation_terminal_repair_merge_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
ownership_correction_authority: Oteryn/Oteryn-Game#187 comment 5424765487
ownership_correction_scope: active Durability task status/provenance/blocker/no-write/next-action only; no worker or runtime change
architecture_hold_main_sha: 007183ac7ef09dd4ae8d8f476d7ac943541d7d48
worker_branch_provenance: remote
worker_branch_remote_head: 79ec09b0d2b13aca4355a66b91ac392474ca467c
local_unpublished_documentation_checkpoint: 3adf13ef17b3b7811aa4f73971456ecd321afcc2
local_checkpoint_delivery_status: not_a_remote_delivery
prior_resume_base_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
resume_base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
resume_admission_main_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
resume_strategy: normal_non_force_merge_up_existing_worker_branch
base_sha: 4c395ece416c3c56aed5607653a0730c52dcb3fd
head_sha: 79ec09b0d2b13aca4355a66b91ac392474ca467c
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl durability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-27T07:06:23Z
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
  - issue:208 completed by pr:210 / main:f056cd38dde6065a3154e256d01aea9e5a09e5f4
blocks:
  - Server Seam remains WAITING_DEPENDENCY until this resumed Durability worker merges the real durable adapter
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
shared_paths: none
external_repositories: []
```

## Outcome

Architecture #187/#190, transport-ref semantics #197/#200, the retained-attempt registry #193/#195, Foundation reconnect boundary #192/#199 and Foundation terminal reconciliation repair #208/#210 are merged. PR #210 merged as protected `main@f056cd38dde6065a3154e256d01aea9e5a09e5f4`, so Durability write authority is restored only on its exact owned paths.

The worker must preserve its published branch history, refresh from protected `main@f056cd38dde6065a3154e256d01aea9e5a09e5f4`, verify the constrained terminal reconciliation API and this restored task authority, then perform a normal non-force merge-up into `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`. Only then may it resume TDD inside the owned paths. Do not reset/recreate/force-push the worker branch merely because main advanced.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are not writable by this task; PR #182 already merged the accepted shared SQLx/Cargo/PostgreSQL prerequisite.
- `PROVEN`: the retained remote worker branch was non-force merged with protected `main@4c395ece416c3c56aed5607653a0730c52dcb3fd` at `2c03415c85a3621fcf6564a88f15f62398d8a790`, advanced to Task 1's deliberate RED contract at `289336df5b58f4dc720861043cc22a881ac3fa33`, then to the first PREPARE-only implementation checkpoint at `79ec09b0d2b13aca4355a66b91ac392474ca467c`; the earlier local unpublished checkpoint `3adf13ef17b3b7811aa4f73971456ecd321afcc2` remains non-authoritative and is not a delivery.
- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #200 merged transport-ref uniqueness as `dc531658c7ffc9af91ccc6719aee80ffe01c22a4`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: PR #199 merged the Foundation V1 reconnect durability boundary as protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0` after exact-head `FOUNDATION_RECONNECT_DURABILITY_V1 / PASS`, full Cargo 1.94/CI and `game-gate` PASS.
- `PROVEN`: PR #210 merged the constrained `ReconnectDurableReconciliationSnapshotV1::terminal(record)` API and ambiguous-same-attempt terminal regression as protected `main@f056cd38dde6065a3154e256d01aea9e5a09e5f4`; exact-head review and all required CI passed.
- `DERIVED`: the former architecture/Foundation dependency blocker is terminally resolved; Server Seam is still blocked on the real durable adapter, not on architecture.

## Acceptance criteria

- [x] #208 was completed by PR #210; branch/history were preserved by normal non-force merge-up of the current protected `main@4c395ece416c3c56aed5607653a0730c52dcb3fd` into the existing worker branch at `2c03415c85a3621fcf6564a88f15f62398d8a790`, followed by an ownership-correct Task 1 RED contract at `289336df5b58f4dc720861043cc22a881ac3fa33`.
- [ ] Real isolated PostgreSQL tests prove migration fresh/compatibility/checksum/ahead/behind/dirty/lock interruption and runtime-DDL denial.
- [ ] Durable journal/adapter consumes the merged Foundation V1 boundary and proves fencing, same-attempt retry/lost-response classification, crash reconciliation and DB outage/recovery without moving Foundation admission/security/controller authority.
- [ ] PREPARE/COMMIT persistence and reconciliation preserve the exact V1 attempt/transport-ref/evidence/deadline semantics; ambiguous outcomes reconcile the same attempt rather than reminting authority.
- [ ] Exact-head persistence/fencing review, CI, expected-head merge and archive lifecycle are complete.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write. No new Cargo/workflow/shared-surface authority is granted by this resume allocation.

## Validation

### Focused

- initial command/run: `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres`
- initial result: `RED` as required for Task 1 at `289336df5b58f4dc720861043cc22a881ac3fa33` — the test imported the allocated but not-yet-implemented `apps/game-server/src/durability/mod.rs` and Cargo failed only with the missing-module error.
- current source evidence at `79ec09b0d2b13aca4355a66b91ac392474ca467c`: `cargo fmt --all -- --check`, strict Clippy for `durability_postgres` and both game-server binaries, and `cargo test --locked -p oteryn-game-server --lib` all pass; the library proof is `153/153` tests.

### Component/integration

- command/run: isolated PostgreSQL worker evidence after normal merge-up from protected `main@4c395ece416c3c56aed5607653a0730c52dcb3fd`
- result: `NOT_EXECUTED_LOCALLY` — the isolated executor has no local PostgreSQL runtime; the exact task-owned `durability_postgres` test is wired to the pinned per-run PostgreSQL 17 GitHub Actions service and will be required there before delivery.

### E2E

- scenario: real isolated PostgreSQL DB E2E required during Durability delivery; gameplay Tier 1/Tier 2 `NOT_APPLICABLE`
- result: pending

## Context checkpoint

```yaml
last_progress: normal non-force merge-up was published at 2c03415c85a3621fcf6564a88f15f62398d8a790; Task 1's guarded PostgreSQL RED specifications were published at 289336df5b58f4dc720861043cc22a881ac3fa33; PREPARE-only migration/schema/journal implementation plus collision, idempotency, 8/9 and same-attempt race contracts were published at 79ec09b0d2b13aca4355a66b91ac392474ca467c in draft PR #212
status: IN_PROGRESS
branch: impl/game-durability-journal
head_sha: 79ec09b0d2b13aca4355a66b91ac392474ca467c
resume_base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
pr: 212
final_head_sha: null
owner_action_required: null
blocker: null
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
next_action: implement the remaining allocated V1 COMMIT/CAS and restart-safe reconciliation path with new isolated PostgreSQL RED contracts; retain PREPARE completion as non-authoritative until Foundation performs its separate final revalidation
```
