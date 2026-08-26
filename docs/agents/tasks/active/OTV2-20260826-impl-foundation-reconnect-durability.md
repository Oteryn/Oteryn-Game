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
pr: null
architecture_decision_issue: 187
architecture_decision_pr: 190
architecture_decision_merge_sha: 2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
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
updated_at: 2026-08-26T17:53:00+02:00
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
  - FND-03_RUNTIME_EXECUTION_CONTRACT
  - FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT
  - FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
  - issue:193 completed by pr:195 / main:9878d42a21815027ef88067bfc59f8b40e78b473
blocks:
  - issue:167
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
write_authority: exact_allocated_foundation_and_task_paths
shared_paths: none
external_repositories: []
```

## Outcome

Expose the smallest Foundation-owned V1 reconnect persistence boundary accepted by PR #190: stable transport reference, complete durable authority/reconciliation evidence, split-phase PREPARE/COMMIT request-completion semantics and final FND-04 revalidation. Foundation retains all admission/security/controller authority; this task never implements SQLx or the Durability module.

## Architecture and source of truth

- `PROVEN`: PR #190 merged `DUR-RECONNECT-AUTHORITY-V1` as `2394f6f4633b8c6662d8d79a84110cc2ae13dcb7`.
- `PROVEN`: PR #194 merged the exact Foundation successor allocation as `1063caf409af6cd4b25fa844e17a483b87e76ad6`.
- `PROVEN`: PR #195 merged the required `FND04-RECONNECT-ATTEMPTS-PER-LOSS-EPOCH = 8` registry row as `9878d42a21815027ef88067bfc59f8b40e78b473`.
- `PROVEN`: protected `main@f31453f65477ae9966d724d67bdd2c1857318be1` records the prior worker-packet mismatch as a `CONFLICT` and requires this worker to reconcile its own packet before first runtime write.
- `PROVEN`: this branch is created from that current protected main and owns only the exact paths listed above.
- `DERIVED`: after this packet reconciliation commit, the stale `waiting_allocation_merge` conflict is removed for the worker branch; runtime writes remain limited to the allocated Foundation paths.

## Acceptance criteria

- [ ] TDD RED proves the current synchronous/generic boundary cannot satisfy the V1 contract.
- [ ] `AuthenticatedTransportRefV1` codec, all-zero rejection, non-reuse/collision handling and equality-only semantics are proven.
- [ ] V1 record/authorization preserves AccountId, exact session/lease/scope/controller/ControlLoss/proof/security/trust/revision and FND-02 reconciliation fences.
- [ ] Split-phase PREPARE and COMMIT yield the FND-03 logical writer; durable completions re-enter as new normalized authoritative inputs.
- [ ] Complete final revalidation occurs after PREPARE and immediately before COMMIT authorization.
- [ ] One-live-PREPARED and registered 8/9 distinct-attempt boundaries fail closed before authority mutation.
- [ ] Ambiguous/lost PREPARE or COMMIT completion requires reconciliation of the same attempt and never invents a new candidate.
- [ ] Controller projection is installable only after exact durable reconciliation of the committed attempt/generation/transport reference.
- [ ] Full Cargo 1.94 focused/component/workspace validation, independent exact-head security review and exact-head CI pass before merge.

## Excluded scope

No SQLx/query/migration/schema work, Durability module, Cargo/workflow/registry/lib.rs lease, listener/Server Seam, gameplay semantics, production database/secrets or external-repository mutation.

## Validation

### TDD

- RED/green evidence: pending on GitHub runners from this reconciled branch; unpublished workstation scratch is non-authoritative and is not used as merge evidence.

### Exact-head CI

- final head: pending
- result: pending

## Context checkpoint

```yaml
last_progress: worker-owned packet reconciled from protected main f31453f65477ae9966d724d67bdd2c1857318be1 after PR #194 allocation and PR #195 registry merge
status: implementing
branch: impl/foundation-reconnect-durability-v1
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: null
next_action: commit the first Foundation V1 failing test on this branch and observe the expected GitHub-runner RED before production implementation
```
