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
current_protected_main_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
current_main_merge_up_sha: e3524615917f3a0b89d4ef33a5826c36a855eb1e
base_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
validated_deadline_fix_head_sha: 2ffbf4006f0ad686a6965fa8c89cfdc935caae39
validated_contract_repair_head_sha: null
checkpoint_parent_head_sha: 851d3fedee186d4977ede6e9d83b65c90f0b5444
final_head_sha: null
final_head_frozen_at: null
updated_at: 2026-08-28T09:49:49Z
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
shared_paths: none
external_repositories: []
shared_supply_chain_status: RESOLVED_ON_PROTECTED_MAIN
shared_supply_chain_detail: protected main carries the shared Cargo.lock repair to chacha20 0.10.2; exact merged-up Durability CI proved the Rust supply-chain gate green without any #167-owned Cargo/lockfile mutation
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

The real PostgreSQL reconnect journal/adapter is implemented on the retained `impl/game-durability-journal` branch and remains inside the exact #167-owned paths. Worker history has been preserved throughout with normal non-force updates only.

The first independent audit at `c79ab0627cf50c9c02296711fc76436b692143c7` returned three P1 and two P2 findings. All five were materially closed: durable RecoveryGrantNonce single-consumption; retained eight-attempt accounting for stale/expired PREPARE; durable PREPARE reconnectable/no-controller/current-generation fencing; true migration cancellation plus fresh retry; and checkpoint refresh.

The subsequent independent re-review at `a73b40aed1979fb7050da37aa4de4e200e1b0c14` confirmed those five findings closed and identified one new PR-local P1: transaction-start `CURRENT_TIMESTAMP` could become stale while PREPARE or COMMIT waited on the per-session PostgreSQL `FOR UPDATE` row lock. That P1 was closed by using PostgreSQL `clock_timestamp()` at the post-lock fence and by real PREPARE/COMMIT contention regressions.

A later independent read-only contract review of the `536e46527662ebd1b370ed293a35b094f86f2c79` candidate identified two remaining PR-local gaps despite the then-green 23-test harness: canonical PostgreSQL schema encoding for frozen identifiers/CommandId values, and incomplete Phase-D current-authority reconciliation after COMMIT. Both were repaired inside the allocated Durability paths.

The canonical schema repair now uses PostgreSQL native `uuid` for UUID-backed durable identities/scopes and `NUMERIC(20,0)` for full-range `u64` FND02 CommandIds. The journal writes and validates a typed canonical mirror for identity/scope/FND02 fields rather than relying on `record_json` as the only representation; pending command IDs are retained in a typed child table. Opaque reconnect-attempt and transport references remain byte-oriented as required by their contracts.

Protected-main Codex policy now classifies this lane as `CODEX_REQUIRED`. A fresh native GitHub Codex review of `e3524615917f3a0b89d4ef33a5826c36a855eb1e` found one P1 and two P2 gaps: the durable session row did not bind AccountId/CharacterId/WorldId/RuntimeScopeRef, uint64-class authority fences narrowed through PostgreSQL `BIGINT`/Rust `i64`, and typed attempt mirrors did not compare `control_loss_epoch` plus `transport_ref` against the canonical record. TDD regressions reproduced all three findings before repair. The current local successor persists the complete session actor/scope binding, stores every uint64-class reconnect authority fence losslessly as `NUMERIC(20,0)`, validates typed epoch/transport mirrors, preserves deterministic terminal replay, and supports a new non-reused ControlLossEpoch without assigning ordering semantics to the opaque epoch fence.

Phase-D reconciliation now fail-closes unless the current durable session still matches the committed record's control-loss epoch, predecessor generation, character-lease generation, scope-ownership generation, candidate/current generation, transport ref, ACTIVE session state, absence of a prepared attempt, and recovery-grant binding where applicable. PREPARED reconciliation equivalently requires exact reconnectable/no-current-controller authority.

The historical canonical/Phase-D repair culminated in `5ef45f94ef615a6d2ca139f5e12e1f167483f241`, where PostgreSQL 17.6 executed **25/25 PASS** and all PR-local Linux/Windows/format/governance/CodeQL/dependency jobs were green. The then-separate shared `cargo-deny` blocker is no longer current: protected main subsequently repaired the shared lockfile to `chacha20 0.10.2`, and the normal non-force merge-up `e3524615917f3a0b89d4ef33a5826c36a855eb1e` proved the Rust supply-chain gate green. The current successor has local pinned PostgreSQL 17.6 **33/33 PASS**, strict Clippy PASS for the Durability test target and migration binary, and the complete game-server package test suite PASS; exact-head CI must be rerun after the successor is pushed.

This lane remains `REVIEW_RECONCILIATION_REQUIRED`: the checkpoint successor must receive exact-head CI and a genuinely independent persistence/fencing/schema re-review before any integration handoff. This lane must not self-approve or self-merge.

## Architecture and invariants

- `PROVEN`: Foundation remains admission/security/final-revalidation/controller authority; Durability persists/classifies/reconciles the exact accepted Foundation V1 record only.
- `PROVEN`: retained transport-ref uniqueness and `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` are preserved.
- `PROVEN`: RecoveryGrantNonce is durably single-consumed atomically with COMMIT and validated on committed replay/reconciliation.
- `PROVEN`: PREPARE publishes only while the locked durable session is reconnectable, has no current controller, and exact epoch/predecessor/lease/scope/current-generation fences match.
- `PROVEN`: PREPARE/COMMIT deadline checks use actual database time after lock acquisition; transaction-start time is not accepted as post-contention freshness evidence.
- `PROVEN`: frozen UUID-backed durable identifiers/scopes use native PostgreSQL `uuid`; all uint64-class reconnect authority fences plus full-range FND02 `CommandId` values use `NUMERIC(20,0)` and round-trip without `i64` narrowing.
- `PROVEN`: the durable GameSession row binds AccountId, CharacterId, WorldId and the exact tagged RuntimeScopeRef; actor/scope disagreement fails closed before replay, COMMIT or reconciliation.
- `PROVEN`: typed identity/scope/FND02 mirrors plus `control_loss_epoch` and `transport_ref` are validated against the exact V1 record on replay, COMMIT and reconciliation; typed/serialized disagreement fails closed.
- `PROVEN`: a different ControlLossEpoch is admitted only as an unseen non-reused equality fence under exact current ACTIVE predecessor/lease/scope identity; it never derives authority from numeric epoch ordering, and stale/unseen or historical attempts are retained deterministically under the eight-attempt per-epoch bound.
- `PROVEN`: Phase-D COMMITTED reconciliation revalidates current epoch/predecessor/lease/scope/generation/transport/session-state/prepared-state/nonce binding before returning a committed snapshot.
- `PROVEN`: runtime startup performs schema inspection only and does not execute DDL; migration execution remains separate.
- `PROVEN`: Cargo/workspace/workflow/Foundation/shared surfaces remain outside this task and were not modified.

## Acceptance criteria

- [x] Foundation dependencies and retained worker history are reconciled; current protected `main@7c2da078596a7d2e27c3066ff74ac69b8b7f9af6` is present through normal non-force merge commit `e3524615917f3a0b89d4ef33a5826c36a855eb1e`.
- [x] Real isolated PostgreSQL tests prove fresh migration, missing-ledger/runtime-DDL denial, checksum/ahead/behind/dirty incompatibility, migration cancellation plus fresh retry, outage/recovery, replay/collision/capacity and restart behavior.
- [x] PREPARE/COMMIT/reconciliation preserve exact V1 attempt, transport-ref, evidence, authority and deadline semantics, including durable recovery nonce single-consumption and post-lock deadline expiry under real row-lock contention.
- [x] Canonical PostgreSQL representation proves native UUID round-trip and full-range `u64` CommandId persistence through `NUMERIC(20,0)`, with typed mirror validation against the exact V1 record.
- [x] Phase-D reconciliation proves fail-closed rejection after mutation of current session state, control-loss epoch, predecessor generation, character lease generation or scope ownership generation, and succeeds again only after exact authority is restored.
- [ ] Fresh exact-head independent persistence/fencing/schema review, final exact-head CI reconciliation, expected-head integration merge and archive lifecycle are complete.

## Validation

### Current Codex-finding repair successor (local pre-push evidence)

- checkpoint parent: `851d3fedee186d4977ede6e9d83b65c90f0b5444`
- protected main consumed by branch: `7c2da078596a7d2e27c3066ff74ac69b8b7f9af6`
- pinned PostgreSQL 17.6 Durability harness: **33/33 PASS**, `0 failed`
- strict Clippy: Durability PostgreSQL target `PASS`; `oteryn-game-migrate` binary `PASS`
- complete `oteryn-game-server` package: 153/153 library tests plus all integration/doc-test groups `PASS`
- formatting: touched Rust files pass Rust 1.94 `rustfmt --check` with Unix newline semantics
- exact-head CI / fresh Codex re-review: **PENDING AFTER FAST-FORWARD PUSH**

### Canonical schema + Phase-D repair

- semantic repair SHA: `dc93f54bb0a31d3e49fafeede573a00624de7dca`
- validated candidate SHA: `5ef45f94ef615a6d2ca139f5e12e1f167483f241`
- Rust workspace run: `33124700824` — `SUCCESS`
- PostgreSQL 17.6 result: `25/25 PASS`, `0 failed`
- new contract proofs:
  - `canonical_uuid_and_full_range_command_ids_round_trip`
  - `committed_reconcile_revalidates_full_current_authority`
- retained proofs also pass for migration compatibility/incompatibility/interruption, DB outage/recovery, durable nonce single-consumption, attempt capacity, PREPARE authority fencing, same-attempt replay, cross-process replay, collision, COMMIT/lost-response reconciliation, stale COMMIT terminalization and row-lock deadline contention.

### Exact PR-local CI at `5ef45f94ef615a6d2ca139f5e12e1f167483f241`

- merge-gate run: `33124700761`
- scope: `PASS`
- governance: `PASS`
- dependency review: `PASS`
- Rust policy/metadata including formatting: `PASS`
- Rust Linux workspace: build `PASS`; strict Clippy `PASS`; workspace tests `PASS`; synthetic `PASS`; native server smoke `PASS`
- Rust Windows client: build `PASS`; strict Clippy `PASS`; pre-native smoke `PASS`; synthetic `PASS`
- CodeQL actions: `PASS`
- CodeQL Python: `PASS`
- Architecture semantic audit: `PASS`
- Merge authority audit: `PASS`
- Agent governance: `PASS`
- separate shared result: Rust supply chain `FAIL` only on yanked `chacha20 0.10.1`; aggregate validate/game-gate fail solely because that shared gate is required.

### Protected-main reconciliation

- protected main: `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`
- latest worker merge-up: `d5025557eb4dfa52ca25919692752c858ad28e5d`, two parents, normal non-force update.
- protected main remained stable while the canonical/Phase-D repair was validated; no further merge-up was required before this checkpoint.
- the exact head produced by this checkpoint must rerun the same CI before independent qualification; no further worker change is authorized unless exact-head evidence finds a PR-local defect.

## Independent review reconciliation

First audit findings at `c79ab0627cf50c9c02296711fc76436b692143c7`, all closed:

1. RecoveryGrantNonce replay state was not durably single-consumed.
2. Stale/expired PREPARE could bypass the eight-attempt retained bound.
3. PREPARE did not prove durable reconnectable/no-current-controller/current-generation authority.
4. Migration interruption test resumed one future instead of proving cancellation plus fresh retry.
5. Active checkpoint evidence was stale.

Second independent audit at `a73b40aed1979fb7050da37aa4de4e200e1b0c14`, closed:

- transaction-start `CURRENT_TIMESTAMP` was unsafe as deadline evidence after `FOR UPDATE` contention;
- fixed with actual post-lock `clock_timestamp()` and real PREPARE/COMMIT contention tests.

Latest independent read-only contract review of the `536e46527662ebd1b370ed293a35b094f86f2c79` candidate required two additional corrections, now implemented and covered by real PostgreSQL tests:

1. canonical schema encoding with native UUID-backed durable identifiers/scopes and full-range `u64` CommandIds stored through `NUMERIC(20,0)`, rather than depending on JSON-only encoding;
2. full Phase-D current-authority reconciliation after COMMIT, fail-closing on current session/epoch/predecessor/lease/scope/generation/transport/prepared/nonce disagreement.

A fresh genuinely independent review must evaluate the exact checkpoint-successor PR head after its CI completes. This lane must not self-approve or self-merge.

## Context checkpoint

```yaml
last_progress: native Codex findings were reproduced by TDD and repaired locally; pinned PostgreSQL 17.6 is 33/33 PASS, strict Clippy and complete game-server package tests PASS; shared chacha20 blocker is resolved on protected main; exact-head CI and fresh Codex re-review remain pending after push
status: IN_PROGRESS
integration_state: REVIEW_RECONCILIATION_REQUIRED
branch: impl/game-durability-journal
validated_contract_repair_head_sha: null
checkpoint_parent_head_sha: 851d3fedee186d4977ede6e9d83b65c90f0b5444
current_protected_main_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
current_main_merge_up_sha: e3524615917f3a0b89d4ef33a5826c36a855eb1e
pr: 212
final_head_sha: null
owner_action_required: none; lane lead is standing-authorized to request fresh native GitHub Codex review after exact-head CI
blocker: exact-head CI and fresh native Codex re-review required; no current shared supply-chain blocker
write_authority: exact_owned_paths_after_foundation_terminal_reconciliation_implementation_merge
next_action: freeze the checkpoint successor, reconcile exact-head CI, obtain genuinely independent persistence/fencing/schema re-review, then hand off to integration authority without self-merging
```
