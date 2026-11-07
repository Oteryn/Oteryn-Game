# OTV2-20260826-impl-foundation-reconnect-durability

```yaml
task_id: OTV2-20260826-impl-foundation-reconnect-durability
title: Historical completed Foundation reconnect durability boundary v1
mode: IMPLEMENT
status: COMPLETED_ARCHIVED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
historical_branch: impl/foundation-reconnect-durability-v1
issue: 192
pr: 199
architecture_decision_issue: 187
architecture_decision_pr: 190
architecture_decision_merge_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
transport_ref_decision_issue: 197
transport_ref_decision_pr: 200
transport_ref_decision_head_sha: 55c96131745454e7b3c5b81781e865f17b285968
transport_ref_decision_merge_sha: dc531658c7ffc9af91ccc6719aee80ffe01c22a4
allocation_pr: 194
allocation_merge_sha: 1063caf409af6cd4b25fa844e17a483b87e76ad6
registry_issue: 193
registry_pr: 195
registry_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
base_sha: dc531658c7ffc9af91ccc6719aee80ffe01c22a4
head_sha: null
final_head_sha: null
delivery_merge_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
owner: Oteryn: impl foundation
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-28T14:05:00Z
archived_at: 2026-08-28
execution_budget_minutes: 120
large_budget_reason: FND-03/FND-04 reconnect authority, fencing, async handoff and security evidence are XHigh-risk Foundation semantics
owned_paths: []
released_paths:
  - apps/game-server/src/foundation/admission.rs
  - apps/game-server/src/foundation/admission_facade.rs
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/recovery_tests.rs
  - docs/agents/tasks/archive/OTV2-20260826-impl-foundation-reconnect-durability.md
public_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
  - FND-03_RUNTIME_EXECUTION_CONTRACT
  - FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT
  - FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
  - issue:193 completed by pr:195 / main:9878d42a21815027ef88067bfc59f8b40e78b473
  - issue:197 resolved by pr:200 / main:dc531658c7ffc9af91ccc6719aee80ffe01c22a4
blocks: []
write_authority: none
shared_paths: none
external_repositories: []
```

## Coordinated terminal archive

PR #199 merged the completed Foundation delivery as protected `main@90f30b47ac9b1e5e41cf274caf707aa39109b0c0`. This record is immutable historical evidence, owns no path, and grants no dispatch, review, validation, or runtime-write authority.

## Outcome

Expose the smallest Foundation-owned V1 reconnect persistence boundary accepted by PR #190 and refined by PR #200: stable transport reference, complete durable authority/reconciliation evidence, split-phase PREPARE/COMMIT request-completion semantics and final FND-04 revalidation. Foundation retains admission/security/controller authority; this task never implements SQLx or the Durability module.

## Architecture and source of truth

- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #194 merged the exact Foundation successor allocation as `1063caf409af6cd4b25fa844e17a483b87e76ad6`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: PR #200 merged `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1` as `dc531658c7ffc9af91ccc6719aee80ffe01c22a4` after exact-head governance, architecture, merge-authority, merge-gate/game-gate and SHA-bound independent review PASS.
- `PROVEN`: this worker branch merged/reconciled that exact protected main before runtime TDD; merge-up commit is `4ce471d652e8910e088aeeef3600b4d7d3802d2f`.
- `PROVEN`: later `main@c72ef273aca34c1f421f5af459a4ed31459e654b` was governance-only META execution routing and did not alter Foundation runtime semantics.
- `PROVEN`: earlier pre-#200 experimental RED commits were reverted and are not TDD/merge evidence.

## Acceptance criteria

- [x] TDD RED/GREEN proves exact 16-byte non-zero `AuthenticatedTransportRefV1` semantics.
- [x] One `ReconnectAttemptRef` binds exactly one immutable transport ref; same-attempt different-ref is idempotency conflict and never remints.
- [x] Typed PREPARE completions preserve prepared/existing-prepared/collision/unavailable/ambiguous/capacity/stale-authority/idempotency-conflict classes.
- [x] Collision may schedule replacement only under a new attempt after terminal collision classification, fresh authority checks and remaining capacity; attempt 9 is rejected before allocation.
- [x] V1 record/authorization preserves AccountId, exact session/lease/scope/controller/ControlLoss/proof/security/trust/revision and FND-02 reconciliation fences.
- [x] Split-phase PREPARE and COMMIT yield the FND-03 logical writer; durable completions re-enter as new normalized authoritative inputs.
- [x] Complete final revalidation occurs after PREPARE and immediately before COMMIT authorization.
- [x] Ambiguous/unavailable paths preserve the same attempt/ref and never guess a new durable outcome.
- [x] Controller projection is installable only after exact durable reconciliation of committed attempt/generation/transport ref.
- [x] Existing synchronous reconnect journal remains compatibility/in-memory behavior and is not made to block on SQLx.
- Historical only: this delivery is terminal and merged; no review or CI action remains on this archived record.

## Excluded scope

No SQLx/query/migration/schema work, Durability module, Cargo/workflow/registry/lib.rs lease, listener/Server Seam, gameplay semantics, production database/secrets or external-repository mutation.

## Validation

### TDD

- `PROVEN` transport-ref RED: `7bcffc0f984012050e4a2113c5ded7028661afa1`; Merge gate run `32991976342`, Linux job `98251552930`, Cargo 1.94 failed only on missing `AuthenticatedTransportRefV1`.
- `PROVEN` core RED: exact RED tree later restored byte-equivalent and Cargo 1.94 failed on the missing V1 core boundary symbols before production implementation.
- `PROVEN` core GREEN: `2dabe5d32b65e69f1f865b8d0501f6db71253686`; Merge gate run `32996639028` passed build, strict Clippy, workspace tests, synthetic harness, native server smoke, Windows client, policy/fmt, CodeQL and supply chain.
- `PROVEN` verifier-evidence RED: `85bc3e386ee15da9811071a495e4331413aae775`; Merge gate run `32998197437`, Linux job `98272867347`, Cargo 1.94 failed exactly because `verify_recovery_grant_durability_v1` did not yet exist.
- `PROVEN` verifier GREEN code candidate: `3cecfca6ddc90f4d0dea2fe05668244915574b50`; the new rich result preserves verified recovery nonce/account/character/world plus signed account-security generation, protocol/profile, rules/content/map/world-policy revisions and credential expiry while retaining legacy verifier semantics.

### Exact-head CI

- pre-review code candidate: `3cecfca6ddc90f4d0dea2fe05668244915574b50`.
- Agent governance `33001109299`: PASS.
- Architecture semantic audit `33001109253`: PASS.
- Merge authority audit `33001109406`: PASS.
- Rust workspace `33001109461`: PASS.
- Merge gate `33001109320`: PASS, including Linux build/strict Clippy/tests/harness/server smoke, Windows build/Clippy/smoke/harness, policy/fmt, dependency review, supply chain, CodeQL and canonical `game-gate`.
- independent exact-head security/authority review: historical lifecycle evidence only; no active review is requested from this archived record.

## Context checkpoint

```yaml
last_progress: delivery PR #199 is merged as protected main 90f30b47ac9b1e5e41cf274caf707aa39109b0c0; ownership is released
status: COMPLETED_ARCHIVED
branch: null
historical_branch: impl/foundation-reconnect-durability-v1
head_sha: null
pr: 199
final_head_sha: null
delivery_merge_sha: 90f30b47ac9b1e5e41cf274caf707aa39109b0c0
owned_paths: []
owner_action_required: none — terminal delivery archived and paths released
blocker: none
next_action: retain this record as historical evidence only
```
