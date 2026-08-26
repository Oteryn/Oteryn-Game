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
updated_at: 2026-08-26T18:20:00+02:00
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
- `PROVEN`: this branch was created from that current protected main and owns only the exact paths listed above.
- `PROVEN`: PR #199 is the worker delivery PR; the first TDD RED test for `AuthenticatedTransportRefV1` is committed on the branch and production code has not yet been written.

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

- RED candidate: first failing test committed on PR #199; awaiting GitHub-runner classification before production implementation.
- unpublished workstation scratch: non-authoritative and not used as merge evidence.

### Exact-head CI

- final head: pending
- result: pending

## Context checkpoint

```yaml
last_progress: PR #199 opened; first RED test is committed and production code remains absent pending GitHub-runner failure evidence
status: implementing
branch: impl/foundation-reconnect-durability-v1
head_sha: null
pr: 199
final_head_sha: null
owner_action_required: null
blocker: GitHub runner must classify the RED candidate before production implementation
next_action: observe the expected runner compile failure for the missing AuthenticatedTransportRefV1 symbol
```
