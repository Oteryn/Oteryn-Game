# OTV2-20260826-impl-foundation-reconnect-durability

```yaml
task_id: OTV2-20260826-impl-foundation-reconnect-durability
title: WAITING_ARCHITECTURE_DECISION_MERGE — Foundation reconnect durability boundary v1
mode: IMPLEMENT
status: WAITING_ARCHITECTURE_DECISION_MERGE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/foundation-reconnect-durability-v1
issue: 192
pr: 199
architecture_decision_issue: 187
architecture_decision_pr: 190
architecture_decision_merge_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
transport_ref_decision_issue: 197
transport_ref_decision_pr: 198
transport_ref_decision_head_sha: 55c96131745454e7b3c5b81781e865f17b285968
allocation_pr: 194
allocation_merge_sha: 1063caf409af6cd4b25fa844e17a483b87e76ad6
registry_issue: 193
registry_pr: 195
registry_merge_sha: 9878d42a21815027ef88067bfc59f8b40e78b473
base_sha: f31453f65477ae9966d724d67bdd2c1857318be1
head_sha: null
final_head_sha: null
owner: Oteryn: impl foundation
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-26T18:32:00+02:00
execution_budget_minutes: 120
large_budget_reason: FND-03/FND-04 reconnect authority, fencing, async handoff and security evidence are XHigh-risk Foundation semantics
owned_paths:
  - apps/game-server/src/foundation/admission.rs
  - apps/game-server/src/foundation/admission_facade.rs
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/recovery_tests.rs
  - docs/agents/tasks/active/OTV2-20260826-impl-foundation-reconnect-durability.md
public_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1 after PR #198 merges
  - FND-03_RUNTIME_EXECUTION_CONTRACT
  - FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT
  - FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
  - issue:193 completed by pr:195 / main:9878d42a21815027ef88067bfc59f8b40e78b473
  - issue:197 / pr:198 must merge and protected main must be read back before runtime TDD resumes
blocks:
  - issue:167
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
write_authority: no_runtime_write_while_transport_ref_decision_unmerged
shared_paths: none
external_repositories: []
```

## Outcome

After the transport-ref uniqueness decision becomes canonical, expose the smallest Foundation-owned V1 reconnect persistence boundary: stable transport reference, complete durable authority/reconciliation evidence, split-phase PREPARE/COMMIT request-completion semantics and final FND-04 revalidation. Foundation retains all admission/security/controller authority; this task never implements SQLx or the Durability module.

## Architecture and source of truth

- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #194 merged the exact Foundation successor allocation as `1063caf409af6cd4b25fa844e17a483b87e76ad6`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: Issue #197 is an active architecture holding gate and PR #198 is the exact docs-only decision candidate at `55c96131745454e7b3c5b81781e865f17b285968`.
- `PROVEN`: PR #198 selects PREPARE CAS as the sole durable uniqueness reservation, one immutable ref per `ReconnectAttemptRef`, terminal collision for that attempt, and replacement only under a new bounded attempt.
- `PROVEN`: the earlier #199 RED test commit was reverted from the worker branch because #197 forbids #192 runtime/test mutation before PR #198 merges; that unpublished/superseded experiment is not merge evidence.
- `PROVEN`: current #199 branch runtime paths are byte-equivalent to protected `main`; only this worker-owned task packet remains changed while the hold is active.

## Acceptance criteria

- [ ] After PR #198 merges, merge/reconcile its exact protected-main result before the first runtime test commit.
- [ ] TDD RED/GREEN proves one immutable `AuthenticatedTransportRefV1` per attempt and rejects same-attempt remint.
- [ ] Typed PREPARE completions preserve collision, existing-prepared, unavailable, ambiguous, capacity, stale-authority and idempotency-conflict classes.
- [ ] Collision may schedule at most one new attempt after terminal classification, fresh authority checks and remaining capacity; attempt 9 is rejected before allocation.
- [ ] V1 record/authorization preserves AccountId, exact session/lease/scope/controller/ControlLoss/proof/security/trust/revision and FND-02 reconciliation fences.
- [ ] Split-phase PREPARE and COMMIT yield the FND-03 logical writer; durable completions re-enter as new normalized authoritative inputs.
- [ ] Complete final revalidation occurs after PREPARE and immediately before COMMIT authorization.
- [ ] Controller projection is installable only after exact durable reconciliation of committed attempt/generation/transport ref.
- [ ] Full Cargo 1.94 focused/component/workspace validation, independent exact-head security review and exact-head CI pass before merge.

## Excluded scope

No SQLx/query/migration/schema work, Durability module, Cargo/workflow/registry/lib.rs lease, listener/Server Seam, gameplay semantics, production database/secrets or external-repository mutation.

## Validation

### TDD

- status: `NOT_STARTED_ON_AUTHORIZED_BASE` while PR #198 remains unmerged.
- superseded pre-decision test commit: reverted; not evidence.

### Exact-head CI

- final head: pending
- result: pending

## Context checkpoint

```yaml
last_progress: #199 runtime/test paths were restored to protected-main bytes after discovering the active #197 architecture hold; worker now waits only for PR #198 merge/readback
status: WAITING_ARCHITECTURE_DECISION_MERGE
branch: impl/foundation-reconnect-durability-v1
head_sha: null
pr: 199
final_head_sha: null
owner_action_required: null
blocker: PR #198 must pass its exact-head Merge authority audit and merge before #192 runtime TDD resumes
next_action: after PR #198 merges, reconcile this branch to that exact protected-main SHA and begin TDD RED for one-ref-per-attempt semantics
```
