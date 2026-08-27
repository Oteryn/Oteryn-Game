# OTV2-20260825-impl-durability

```yaml
task_id: OTV2-20260825-impl-durability
title: IN_PROGRESS — journal-only durability admission and reconnect substrate
mode: IMPLEMENT
status: IN_PROGRESS
integration_state: REVIEW_RECONCILIATION_REQUIRED
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
worker_branch_remote_head: f11986f8219eb7b401af8ef942377758c4e85fe9
local_unpublished_documentation_checkpoint: 3adf13ef17b3b7811aa4f73971456ecd321afcc2
local_checkpoint_delivery_status: not_a_remote_delivery
prior_resume_base_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
resume_base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
resume_admission_main_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
current_protected_main_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
current_main_merge_up_sha: c8a27bbae6c531ba625aee76f347388c2a447034
resume_strategy: normal_non_force_merge_up_existing_worker_branch
base_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
head_sha: f11986f8219eb7b401af8ef942377758c4e85fe9
validated_implementation_head_sha: 46148c92bc2e27b2c9523a08f8a8e3b6f7deb735
format_successor_head_sha: f11986f8219eb7b401af8ef942377758c4e85fe9
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: sol durability lead
created_at: 2026-08-25T23:24:03+02:00
updated_at: 2026-08-27T20:36:00Z
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
  - Server Seam remains WAITING_DEPENDENCY until this Durability worker is independently reviewed and integrated
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
shared_paths: none
external_repositories: []
shared_supply_chain_status: BLOCKED_SHARED_SUPPLY_CHAIN
shared_supply_chain_detail: cargo-deny rejects yanked chacha20 0.10.1 through rand 0.10.2 -> sqlx-postgres 0.9.0; this task has no Cargo/lockfile authority and PR #212 changes neither
```

## Outcome

The real PostgreSQL reconnect journal/adapter is implemented on the existing `impl/game-durability-journal` worker branch and remains within the exact #167-owned paths. Protected `main@6e6e37852b7a050a1c7117ab2a9f316907d09daf` was merged normally and non-force into the retained worker branch at `c8a27bbae6c531ba625aee76f347388c2a447034`; there has been no reset, branch recreation or force-push.

The first independent exact-head audit of PR #212 at `c79ab0627cf50c9c02296711fc76436b692143c7` returned three P1 findings and two P2 findings. The implementation findings were corrected at `46148c92bc2e27b2c9523a08f8a8e3b6f7deb735`: recovery-grant nonce is now durably single-consumed atomically at COMMIT, stale/expired retained attempts consume the same eight-attempt epoch capacity, PREPARE checks durable RECONNECTABLE/no-current-controller/current-generation fencing, and the migration interruption proof now cancels/drops the blocked operation before starting a fresh retry. The only CI-local defect on that implementation SHA was rustfmt in the new regression tests; the formatting-only successor is `f11986f8219eb7b401af8ef942377758c4e85fe9`.

This task is not `READY_FOR_INTEGRATION` yet. A fresh genuinely independent exact-head persistence/fencing/schema review is still required after the refreshed checkpoint and exact-head CI. The repository-wide cargo-deny failure for yanked `chacha20 0.10.1` is a separate shared-surface blocker and must not be repaired from this lane.

## Architecture and source of truth

- `PROVEN`: accepted topology is a game-server-local module, one game-owned migration ledger, dedicated migration execution and `PREPARE -> DB COMMIT/CLASSIFY -> RECONCILE`.
- `PROVEN`: DUR03-RL-01..08 and all item/value transactions remain fail-closed excluded.
- `PROVEN`: `lib.rs`, Cargo/workspace/workflow/gitattributes are not writable by this task; PR #182 already merged the accepted shared SQLx/Cargo/PostgreSQL prerequisite.
- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #200 merged transport-ref uniqueness as `dc531658c7ffc9af91ccc6719aee80ffe01c22a4`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: PR #199 merged the Foundation V1 reconnect durability boundary as protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0`.
- `PROVEN`: PR #210 merged the constrained terminal reconciliation API as protected `main@f056cd38dde6065a3154e256d01aea9e5a09e5f4`.
- `PROVEN`: protected `main` advanced to `6e6e37852b7a050a1c7117ab2a9f316907d09daf` and was normally merged into this worker at `c8a27bbae6c531ba625aee76f347388c2a447034`; the branch is not relying on a stale Foundation base.
- `PROVEN`: the independent review of `c79ab0627cf50c9c02296711fc76436b692143c7` identified concrete persistence/fencing gaps rather than architecture ambiguity; those implementation gaps are addressed in the owned Durability surfaces at `46148c92bc2e27b2c9523a08f8a8e3b6f7deb735`.
- `DERIVED`: remaining Durability work is exact-head verification/re-review/integration lifecycle, not additional product or authority design.

## Acceptance criteria

- [x] #208 was completed by PR #210 and the retained worker history was preserved; current protected `main@6e6e37852b7a050a1c7117ab2a9f316907d09daf` is present through normal non-force merge commit `c8a27bbae6c531ba625aee76f347388c2a447034`.
- [x] Real isolated PostgreSQL 17.6 tests prove fresh migration, missing-ledger/runtime-DDL denial, checksum mismatch, ahead/behind/dirty ledger, migration lock cancellation plus fresh retry, and DB outage/recovery.
- [x] The durable journal consumes the merged Foundation V1 boundary and proves durable fencing, same-attempt retry/lost-response classification, cross-process replay, crash/reconciliation behavior, transport-ref collision, recovery-grant nonce single-consumption and DB outage/recovery without moving Foundation admission/security/controller authority.
- [x] PREPARE/COMMIT persistence and reconciliation preserve exact V1 attempt/transport-ref/evidence/deadline semantics: stale/expired attempts remain inside the retained eight-attempt bound, PREPARE requires durable RECONNECTABLE/no-current-controller/current-generation state, recovery grants are single-consumed at COMMIT, and ambiguous outcomes reconcile the same attempt rather than reminting authority.
- [ ] Fresh exact-head independent persistence/fencing/schema review, final exact-head CI reconciliation, expected-head merge and archive lifecycle are complete.

## Excluded scope

No production database/config/secrets, transaction/outbox, item/value custody/reward, Foundation semantic change, `main.rs`, registry, Platform/Atlas/META or external repository write. No new Cargo/workflow/shared-surface authority is granted by this task. In particular, the yanked `chacha20 0.10.1` supply-chain repair belongs to a shared Cargo/lockfile owner, not this Durability lane.

## Validation

### Focused / real PostgreSQL

- exact implementation SHA: `46148c92bc2e27b2c9523a08f8a8e3b6f7deb735`
- workflow: Rust workspace run `33113636219`, job `Rust / Durability PostgreSQL harness`
- environment: pinned PostgreSQL `17.6-bookworm`
- result: `PASS`, `21/21` tests, `0 failed`
- audit-regression proofs passing on that SHA:
  - `recovery_grant_nonce_is_single_consumed_at_commit`
  - `stale_prepare_attempts_consume_the_retained_attempt_bound`
  - `prepare_requires_reconnectable_state_and_no_current_controller`
  - `migration_lock_interruption_releases_before_any_ddl_and_allows_fresh_retry`
- retained proofs also pass for fresh/compatibility/checksum/ahead/behind/dirty schema, runtime no-DDL, outage/recovery, same-attempt replay, cross-process replacement, concurrent same attempt, transport-ref collision, eight-attempt limit, lost-response COMMIT/reconcile and stale-COMMIT terminalization.

### Build / policy / semantic CI

At implementation SHA `46148c92bc2e27b2c9523a08f8a8e3b6f7deb735`:

- `Rust workspace` run `33113636219`: `SUCCESS`; Windows SIM golden and Durability PostgreSQL harness both passed.
- `Agent governance`: `SUCCESS`.
- `Merge authority audit`: `SUCCESS`.
- `Architecture semantic audit`: `SUCCESS`.
- merge-gate scope/governance/dependency review/CodeQL passed; Linux build, strict Clippy, workspace tests and synthetic harness passed during the run.
- merge-gate policy detected only rustfmt differences in `apps/game-server/tests/support/postgres.rs`; those exact rustfmt changes were applied by formatting-only successor `f11986f8219eb7b401af8ef942377758c4e85fe9`.
- merge-gate supply-chain failed independently on yanked `chacha20 0.10.1`; PR #212 changes no Cargo manifest or lockfile and this task has no authority to repair it.

### E2E

- scenario: real isolated PostgreSQL DB E2E required during Durability delivery; gameplay Tier 1/Tier 2 `NOT_APPLICABLE`
- result: `PASS` for the task-owned PostgreSQL E2E at `46148c92bc2e27b2c9523a08f8a8e3b6f7deb735`; exact successor CI must be reconciled again after this checkpoint-only update before integration.

## Independent review reconciliation

The independent exact-head audit at `c79ab0627cf50c9c02296711fc76436b692143c7` returned `REQUEST_CHANGES` with the following PR-local findings, all now addressed in the owned lane:

1. P1 recovery-grant replay key was not durably single-consumed -> fixed by `game_durability_recovery_grant_consumptions` and atomic COMMIT/replay/reconcile binding.
2. P1 stale/expired PREPARE could bypass the eight-attempt retained bound -> fixed by capacity-before-new-row and attempt-count accounting for stale terminals.
3. P1 PREPARE did not assert durable RECONNECTABLE/no-current-controller/current-generation state -> fixed in the per-session locked CAS predicate.
4. P2 migration interruption resumed the same pinned future -> fixed so timeout owns/drops the blocked future and a distinct fresh migration operation is started after lock release.
5. P2 active checkpoint was stale -> fixed by this checkpoint refresh.

A fresh independent review must evaluate the new exact PR head after this checkpoint. This lane must not self-approve or self-merge.

## Context checkpoint

```yaml
last_progress: independent audit findings were implemented at 46148c92bc2e27b2c9523a08f8a8e3b6f7deb735 with real PostgreSQL 17.6 21/21 PASS; rustfmt-only successor f11986f8219eb7b401af8ef942377758c4e85fe9 was published; this checkpoint refresh closes the final PR-local P2 documentation finding
status: IN_PROGRESS
integration_state: REVIEW_RECONCILIATION_REQUIRED
branch: impl/game-durability-journal
validated_implementation_head_sha: 46148c92bc2e27b2c9523a08f8a8e3b6f7deb735
checkpoint_parent_head_sha: f11986f8219eb7b401af8ef942377758c4e85fe9
resume_base_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
current_protected_main_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
pr: 212
final_head_sha: null
owner_action_required: fresh independent exact-head persistence/fencing/schema review after exact-head CI
blocker: independent re-review required; separately, shared cargo-deny is blocked by yanked chacha20 0.10.1 outside this lane
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
next_action: freeze the new exact PR head, reconcile exact-head CI, obtain genuinely independent re-review of persistence/fencing/schema, then hand off to the integration authority without self-merging
```
