# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: PAUSED_BRANCH_PROVENANCE_RECOVERY — journal-only durability admission and reconnect substrate
mode: IMPLEMENT
status: PAUSED_BRANCH_PROVENANCE_RECOVERY
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
worker_branch_remote_head: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
local_unpublished_documentation_checkpoint: 3adf13ef17b3b7811aa4f73971456ecd321afcc2
local_checkpoint_delivery_status: not_a_remote_delivery
prior_resume_base_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
resume_base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
resume_admission_main_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
resume_strategy: normal_non_force_merge_up_existing_worker_branch
base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
head_sha: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: impl durability
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-28T14:14:00Z
execution_budget_minutes: 120
large_budget_reason: SQLx migration safety, durable idempotency/fencing and mandatory isolated PostgreSQL evidence
owned_paths: []
released_paths:
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
  - P0 branch provenance recovery: destructive cross-scope commit cd808d396018832b632be26911105a36f0cb7a20 and unallocated restoration 73e17f418c63ec038f5aa7ef8f0888ac74b75aa2 are retained ancestors of the paused branch
  - Server Seam remains WAITING_DEPENDENCY until the separately allocated clean Durability successor merges the real durable adapter
write_authority: none_legacy_branch_is_immutable_evidence
shared_paths: none
external_repositories: []
```

## Outcome

The legacy Durability branch is paused as immutable evidence. The separately allocated successor is `OTV2-20260828-impl-durability-successor` on `impl/game-durability-journal-recovery-240`; this packet grants it no authority and preserves no legacy runtime write authority.

## Coordinated ownership correction — 2026-08-28

This correction is limited by ownership_correction_authority and ownership_correction_scope above to status, provenance, blocker, no-write state and next action. It changes no worker/runtime authority.

- PROVEN: current Draft PR #212 head is fb30fba2a888835dfc7cbde27f940b79d7bfe05d; its ancestry includes destructive cross-scope commit cd808d396018832b632be26911105a36f0cb7a20 and unallocated restoration 73e17f418c63ec038f5aa7ef8f0888ac74b75aa2.
- PROVEN: recovery allocation PR #241 is merged as protected main a171410de07c2dab718f52f780d4314bdcc53604 and admits `fb30...` only as read-only source evidence.
- PROVEN: no further write, review qualification, readiness handoff or merge is allowed on impl/game-durability-journal / PR #212.
- DERIVED: the existing READY_TO_RESUME instructions below are historical. Only the separately allocated successor task may begin its clean-ancestry TDD lifecycle.

## Historical pre-pause record — non-actionable

Architecture #187/#190, transport-ref semantics #197/#200, the retained-attempt registry #193/#195, Foundation reconnect boundary #192/#199 and Foundation terminal reconciliation repair #208/#210 are historical prerequisites only. They do not restore write authority to this legacy packet.

The former merge-up and TDD-resume procedure is void. No worker may merge, refresh, qualify, or write `impl/game-durability-journal`; it and PR #212 remain immutable evidence.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are not writable by this task; PR #182 already merged the accepted shared SQLx/Cargo/PostgreSQL prerequisite.
- `HISTORICAL`: the remote worker branch was `impl/game-durability-journal@7ac06bd84a1a31fc9a3ea2560de8ae20cea96741`; local unpublished checkpoint `3adf13ef17b3b7811aa4f73971456ecd321afcc2` remains non-authoritative and is not a delivery.
- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #200 merged transport-ref uniqueness as `dc531658c7ffc9af91ccc6719aee80ffe01c22a4`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: PR #199 merged the Foundation V1 reconnect durability boundary as protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0` after exact-head `FOUNDATION_RECONNECT_DURABILITY_V1 / PASS`, full Cargo 1.94/CI and `game-gate` PASS.
- `PROVEN`: PR #210 merged the constrained `ReconnectDurableReconciliationSnapshotV1::terminal(record)` API and ambiguous-same-attempt terminal regression as protected `main@f056cd38dde6065a3154e256d01aea9e5a09e5f4`; exact-head review and all required CI passed.
- `DERIVED`: the former architecture/Foundation dependency blocker is terminally resolved; Server Seam is still blocked on the real durable adapter, not on architecture.

## Historical pre-pause acceptance record — non-actionable

- Historical only: PR #210 and its predecessor chain were recorded before the provenance incident.
- Historical only: the former PostgreSQL and durability-adapter objectives do not authorize work on the legacy branch.
- Current acceptance is defined solely by `OTV2-20260828-impl-durability-successor` and its clean-ancestry TDD gates.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write. No new Cargo/workflow/shared-surface authority is granted by this resume allocation.

## Historical pre-pause validation record — non-actionable

### Focused

- result: superseded; no legacy validation may be run as a delivery gate

### Component/integration

- result: superseded; any new evidence belongs only to the separate successor

### E2E

- result: superseded; this legacy packet provides no executable E2E action

## Context checkpoint

```yaml
last_progress: legacy PR #212 is paused immutable evidence after the P0 provenance incident; PR #241 merged the clean successor allocation as main a171410de07c2dab718f52f780d4314bdcc53604
status: PAUSED_BRANCH_PROVENANCE_RECOVERY
branch: impl/game-durability-journal
head_sha: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
resume_base_sha: null
pr: 212
final_head_sha: null
owner_action_required: null
blocker: P0 branch provenance/authority gap; current branch content is evidence-only and cannot be retrospectively ratified
write_authority: none_legacy_branch_is_immutable_evidence
next_action: do not write, qualify, merge, reset, force-push or delete PR #212; the separately allocated clean successor owns the next Durability implementation lifecycle
```
