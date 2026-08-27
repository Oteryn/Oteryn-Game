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
owner: Oteryn: sol durability lead
architecture_decision_issue: 187
architecture_decision_pr: 190
architecture_decision_merge_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
foundation_boundary_issue: 192
foundation_boundary_pr: 199
foundation_boundary_merge_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
registry_issue: 193
registry_pr: 195
registry_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
transport_ref_decision_issue: 197
transport_ref_decision_pr: 200
transport_ref_decision_merge_sha: dc531658c7ffc9af91ccc6719aee80ffe01c22a4
foundation_terminal_repair_issue: 208
foundation_terminal_repair_pr: 210
foundation_terminal_repair_merge_sha: f056cd38dde6065a3154e256d01aea9e5a09e5f4
current_protected_main_sha: d48e746ec4b001b1f210119cadabc256dd8656b7
current_main_merge_up_sha: 819976b788a3ef1232ba5409cb29a5847104670f
base_sha: d48e746ec4b001b1f210119cadabc256dd8656b7
validated_deadline_fix_head_sha: 2ffbf4006f0ad686a6965fa8c89cfdc935caae39
format_successor_head_sha: 268cc378c38104eba5fbce47d6041cb097ddebe9
checkpoint_parent_head_sha: 268cc378c38104eba5fbce47d6041cb097ddebe9
final_head_sha: null
final_head_frozen_at: null
updated_at: 2026-08-27T21:15:00Z
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
shared_paths: none
external_repositories: []
shared_supply_chain_status: BLOCKED_SHARED_SUPPLY_CHAIN
shared_supply_chain_detail: cargo-deny rejects yanked chacha20 0.10.1 through rand 0.10.2 -> sqlx-postgres 0.9.0; this task has no Cargo/lockfile authority and PR #212 changes neither
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
```

## Outcome

The real PostgreSQL reconnect journal/adapter is implemented on the retained `impl/game-durability-journal` branch and remains inside the exact #167-owned paths. Protected `main` advanced to `d48e746ec4b001b1f210119cadabc256dd8656b7`; that head was merged normally and non-force into the worker branch by two-parent merge commit `819976b788a3ef1232ba5409cb29a5847104670f`. No reset, branch recreation or force-push occurred.

The first independent audit at `c79ab0627cf50c9c02296711fc76436b692143c7` returned three P1 and two P2 findings. All five were materially closed before the next audit: durable RecoveryGrantNonce single-consumption, retained eight-attempt accounting for stale/expired PREPARE, durable PREPARE reconnectable/no-controller/current-generation fencing, true migration cancellation plus fresh retry, and checkpoint refresh.

The subsequent independent re-review at `a73b40aed1979fb7050da37aa4de4e200e1b0c14` confirmed those five findings closed and identified one new PR-local P1: `CURRENT_TIMESTAMP`/transaction-start time could become stale while PREPARE or COMMIT waited on the per-session PostgreSQL row lock, allowing a deadline to expire during contention without being observed after lock acquisition.

That P1 is fixed at `2ffbf4006f0ad686a6965fa8c89cfdc935caae39`. The durability adapter now reads real database wall-clock time with PostgreSQL `clock_timestamp()` after the authoritative row lock has been obtained. Two real PostgreSQL contention regressions hold the session row lock, start PREPARE/COMMIT while the deadline is valid, wait until the database clock crosses the absolute deadline, release the lock, and prove fail-closed stale rejection without controller/generation advancement or recovery-grant consumption. The exact implementation SHA passed all `23/23` PostgreSQL 17.6 tests. Formatting-only successor `268cc378c38104eba5fbce47d6041cb097ddebe9` preserves that behavior and also passed the exact-head PostgreSQL harness and formatting/policy checks.

This lane remains `REVIEW_RECONCILIATION_REQUIRED`: a genuinely independent exact-head persistence/fencing/schema re-review is still mandatory after this checkpoint commit and its exact-head CI. The repository-wide yanked `chacha20 0.10.1` cargo-deny failure is a separate shared-surface blocker and must not be repaired from this lane.

## Architecture and invariants

- `PROVEN`: Foundation remains the admission/security/final-revalidation/controller authority; Durability persists/classifies/reconciles the exact accepted Foundation V1 record only.
- `PROVEN`: retained transport-ref uniqueness and the frozen `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` semantics are preserved.
- `PROVEN`: RecoveryGrantNonce is durably single-consumed atomically with COMMIT and validated on committed replay/reconciliation.
- `PROVEN`: PREPARE publishes only while the locked durable session is reconnectable, has no current controller, and exact epoch/predecessor/lease/scope/current-generation fences match.
- `PROVEN`: PREPARE/COMMIT deadline checks use actual database time after lock acquisition; transaction-start timestamps are not accepted as post-contention freshness evidence.
- `PROVEN`: runtime startup performs schema inspection only and does not execute DDL; migration execution remains a separate path.
- `PROVEN`: Cargo/workspace/workflow/Foundation/shared surfaces are outside this task and were not modified.

## Acceptance criteria

- [x] Foundation dependencies and retained worker history are reconciled; current protected `main@d48e746ec4b001b1f210119cadabc256dd8656b7` is present through normal non-force merge commit `819976b788a3ef1232ba5409cb29a5847104670f`.
- [x] Real isolated PostgreSQL tests prove fresh migration, missing-ledger/runtime-DDL denial, checksum/ahead/behind/dirty incompatibility, migration cancellation plus fresh retry, outage/recovery, replay/collision/capacity and restart behavior.
- [x] PREPARE/COMMIT/reconciliation preserve exact V1 attempt, transport-ref, evidence, authority and deadline semantics, including durable recovery nonce single-consumption and post-lock deadline expiry under real row-lock contention.
- [ ] Fresh exact-head independent persistence/fencing/schema review, final exact-head CI reconciliation, expected-head integration merge and archive lifecycle are complete.

## Validation

### Deadline/fencing implementation

- implementation SHA: `2ffbf4006f0ad686a6965fa8c89cfdc935caae39`
- Rust workspace run: `33117100729`
- job: `98674435312` — `Rust / Durability PostgreSQL harness`
- environment: pinned PostgreSQL `17.6-bookworm`
- result: `PASS`, `23/23`, `0 failed`
- new contention proofs:
  - `commit_row_lock_wait_cannot_outlive_authorization_deadline`
  - `prepare_row_lock_wait_cannot_outlive_prepared_deadline`
- assertions include stale rejection after lock wait crosses deadline, no controller/generation advance and no RecoveryGrantNonce consumption on expired COMMIT.

### Formatting successor

- successor SHA: `268cc378c38104eba5fbce47d6041cb097ddebe9`
- Rust workspace run: `33117483359`
- Durability PostgreSQL harness job: `98675748924` — `SUCCESS`
- Windows SIM golden: `SUCCESS`
- Architecture semantic audit: `SUCCESS`
- Merge authority audit: `SUCCESS`
- Agent governance: `SUCCESS`
- merge-gate run `33117483394`: scope, dependency review, Rust policy/metadata including `cargo fmt --check`, governance and both CodeQL jobs passed at checkpoint time; Linux/Windows full jobs were still completing when this documentation checkpoint was authored.
- supply-chain remains independently `FAIL` on the shared yanked `chacha20 0.10.1` baseline; no Cargo/lockfile change is authorized here.

## Independent review reconciliation

First audit findings at `c79ab0627cf50c9c02296711fc76436b692143c7`, all closed:

1. RecoveryGrantNonce replay state was not durably single-consumed.
2. Stale/expired PREPARE could bypass the eight-attempt retained bound.
3. PREPARE did not prove durable reconnectable/no-current-controller/current-generation authority.
4. Migration interruption test resumed one future instead of proving cancellation plus fresh retry.
5. Active checkpoint evidence was stale.

Second independent audit at `a73b40aed1979fb7050da37aa4de4e200e1b0c14`:

- confirmed the five findings above materially closed;
- found one P1: transaction-start `CURRENT_TIMESTAMP` was unsafe as deadline evidence after `FOR UPDATE` contention;
- required actual post-lock database time and real PREPARE/COMMIT contention regressions.

The second-audit P1 is addressed by `clock_timestamp()` and the two passing PostgreSQL contention tests at `2ffbf4006f0ad686a6965fa8c89cfdc935caae39`, with rustfmt-only successor `268cc378c38104eba5fbce47d6041cb097ddebe9`.

A fresh independent review must evaluate the exact PR head created by this checkpoint after its CI completes. This lane must not self-approve or self-merge.

## Context checkpoint

```yaml
last_progress: protected main d48e746ec4b001b1f210119cadabc256dd8656b7 merged normally at 819976b788a3ef1232ba5409cb29a5847104670f; second-audit deadline P1 fixed at 2ffbf4006f0ad686a6965fa8c89cfdc935caae39 with PostgreSQL 17.6 23/23 PASS; rustfmt-only successor 268cc378c38104eba5fbce47d6041cb097ddebe9 also has exact-head PostgreSQL PASS and format/policy PASS
status: IN_PROGRESS
integration_state: REVIEW_RECONCILIATION_REQUIRED
branch: impl/game-durability-journal
validated_deadline_fix_head_sha: 2ffbf4006f0ad686a6965fa8c89cfdc935caae39
checkpoint_parent_head_sha: 268cc378c38104eba5fbce47d6041cb097ddebe9
current_protected_main_sha: d48e746ec4b001b1f210119cadabc256dd8656b7
current_main_merge_up_sha: 819976b788a3ef1232ba5409cb29a5847104670f
pr: 212
final_head_sha: null
owner_action_required: fresh independent exact-head persistence/fencing/schema review after exact-head CI
blocker: independent re-review required; separately shared cargo-deny is blocked by yanked chacha20 0.10.1 outside this lane
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
next_action: freeze this checkpoint successor, reconcile its exact-head CI, obtain genuinely independent persistence/fencing/schema re-review, then hand off to integration authority without self-merging
```
