# OTV2-20260826-impl-foundation-reconnect-durability

```yaml
task_id: OTV2-20260826-impl-foundation-reconnect-durability
title: Implement Foundation reconnect durability boundary v1
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/foundation-reconnect-durability-v1
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
owner: Oteryn: impl foundation
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-26T19:00:00+02:00
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
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
  - FND-03_RUNTIME_EXECUTION_CONTRACT
  - FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT
  - FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
  - issue:193 completed by pr:195 / main:9878d42a21815027ef88067bfc59f8b40e78b473
  - issue:197 resolved by pr:200 / main:dc531658c7ffc9af91ccc6719aee80ffe01c22a4
blocks:
  - issue:167
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
write_authority: exact_allocated_foundation_and_task_paths
shared_paths: none
external_repositories: []
```

## Outcome

Expose the smallest Foundation-owned V1 reconnect persistence boundary accepted by PR #190 and refined by PR #200: stable transport reference, complete durable authority/reconciliation evidence, split-phase PREPARE/COMMIT request-completion semantics and final FND-04 revalidation. Foundation retains all admission/security/controller authority; this task never implements SQLx or the Durability module.

## Architecture and source of truth

- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #194 merged the exact Foundation successor allocation as `1063caf409af6cd4b25fa844e17a483b87e76ad6`.
- `PROVEN`: PR #195 merged `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: PR #200 merged `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1` as `dc531658c7ffc9af91ccc6719aee80ffe01c22a4` after exact-head governance, architecture, merge-authority, merge-gate/game-gate and SHA-bound independent review PASS.
- `PROVEN`: this worker branch merged/reconciled that exact protected main before runtime TDD; merge-up commit is `4ce471d652e8910e088aeeef3600b4d7d3802d2f`.
- `PROVEN`: earlier pre-#200 experimental RED commits were reverted and are not TDD/merge evidence.
- `DERIVED`: runtime TDD may now proceed only in the exact allocated Foundation paths.

## Acceptance criteria

- [ ] TDD RED/GREEN proves exact 16-byte non-zero `AuthenticatedTransportRefV1` semantics.
- [ ] One `ReconnectAttemptRef` binds exactly one immutable transport ref; same-attempt different-ref is idempotency conflict and never remints.
- [ ] Typed PREPARE completions preserve prepared/existing-prepared/collision/unavailable/ambiguous/capacity/stale-authority/idempotency-conflict classes.
- [ ] Collision may schedule replacement only under a new attempt after terminal collision classification, fresh authority checks and remaining capacity; attempt 9 is rejected before allocation.
- [ ] V1 record/authorization preserves AccountId, exact session/lease/scope/controller/ControlLoss/proof/security/trust/revision and FND-02 reconciliation fences.
- [ ] Split-phase PREPARE and COMMIT yield the FND-03 logical writer; durable completions re-enter as new normalized authoritative inputs.
- [ ] Complete final revalidation occurs after PREPARE and immediately before COMMIT authorization.
- [ ] Ambiguous/unavailable paths preserve the same attempt/ref and never guess a new durable outcome.
- [ ] Controller projection is installable only after exact durable reconciliation of committed attempt/generation/transport ref.
- [ ] Existing synchronous reconnect journal remains compatibility/in-memory behavior and is not made to block on SQLx.
- [ ] Full Cargo 1.94 focused/component/workspace validation, genuinely independent exact-head security review and exact-head CI pass before merge.

## Excluded scope

No SQLx/query/migration/schema work, Durability module, Cargo/workflow/registry/lib.rs lease, listener/Server Seam, gameplay semantics, production database/secrets or external-repository mutation.

## Validation

### TDD

- status: `NOT_STARTED_ON_AUTHORIZED_BASE`.
- authorized base: `main@dc531658c7ffc9af91ccc6719aee80ffe01c22a4`.
- superseded pre-decision tests: reverted; not evidence.

### Exact-head CI

- final head: pending
- result: pending

## Context checkpoint

```yaml
last_progress: PR #200 merged and #199 reconciled to exact protected main dc531658c7ffc9af91ccc6719aee80ffe01c22a4; architecture hold is cleared and runtime authority is active on allocated Foundation paths
status: implementing
branch: impl/foundation-reconnect-durability-v1
head_sha: null
pr: 199
final_head_sha: null
owner_action_required: null
blocker: null
next_action: commit the first test-only RED for AuthenticatedTransportRefV1 on this authorized base and wait for the expected GitHub-runner compile failure before production code
```
