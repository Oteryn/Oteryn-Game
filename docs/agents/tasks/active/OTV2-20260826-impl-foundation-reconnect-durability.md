# OTV2-20260826-impl-foundation-reconnect-durability

```yaml
task_id: OTV2-20260826-impl-foundation-reconnect-durability
title: Implement Foundation reconnect durability boundary v1
mode: IMPLEMENT
status: waiting_allocation_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/foundation-reconnect-durability-v1
issue: 192
pr: null
allocation_pr: pending
base_sha: null
head_sha: null
final_head_sha: null
owner: Oteryn: impl foundation
created_at: 2026-08-26T15:02:00+02:00
updated_at: 2026-08-26T15:02:00+02:00
execution_budget_minutes: 120
large_budget_reason: FND-03/FND-04 reconnect authority, fencing, async handoff and security evidence are XHigh-risk Foundation semantics
owned_paths:
  - apps/game-server/src/foundation/admission.rs
  - apps/game-server/src/foundation/admission_facade.rs
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/foundation/fnd04_verifier.rs
  - apps/game-server/src/foundation/recovery_tests.rs
  - docs/agents/tasks/active/OTV2-20260826-impl-foundation-reconnect-durability.mdpublic_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
  - FND-03_RUNTIME_EXECUTION_CONTRACT
  - FND-04A_AUTHORITY_FRESH_ADMISSION_CONTRACT
  - FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT
depends_on:
  - issue:187 resolved by pr:190 / main:2394f6f4633b8c6662d8d79a84110cc2ae13dcb7
  - issue:193 retained-attempt registry row must merge before final acceptance
blocks:
  - issue:167
  - OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM
write_authority: none_until_allocation_pr_merges
shared_paths: none
external_repositories: []
```

## Outcome

Expose the smallest Foundation-owned V1 reconnect persistence boundary accepted by PR #190: stable transport reference, complete durable authority/reconciliation evidence, split-phase PREPARE/COMMIT request-completion semantics and final FND-04 revalidation. Foundation retains all admission/security/controller authority; this task never implements SQLx or the Durability module.

## Acceptance criteria

- [ ] TDD RED proves the current synchronous/generic boundary cannot satisfy the V1 contract.
- [ ] `AuthenticatedTransportRefV1` codec, all-zero rejection, non-reuse/collision handling and equality-only semantics are proven.
- [ ] V1 record/authorization preserves AccountId, exact session/lease/scope/controller/ControlLoss/proof/security/trust/revision and FND-02 reconciliation fences.
- [ ] Split-phase PREPARE and COMMIT yield the FND-03 logical writer; durable completions re-enter as new normalized authoritative inputs.
- [ ] Complete final revalidation occurs after PREPARE and immediately before COMMIT authorization.
- [ ] One-live-PREPARED and registered 8/9 distinct-attempt boundaries fail closed before authority mutation.
- [ ] Full Cargo 1.94 focused/component/workspace validation, independent exact-head security review and exact-head CI pass before merge.
## Excluded scope

No SQLx/query/migration/schema work, Durability module, Cargo/workflow/registry/lib.rs lease, listener/Server Seam, gameplay semantics, production database/secrets or external-repository mutation.

## Context checkpoint

```yaml
last_progress: PR #190 merged the accepted reconnect durability boundary; exact implementation allocation is prepared but not yet merged
status: waiting_allocation_merge
branch: impl/foundation-reconnect-durability-v1
head_sha: null
pr: null
final_head_sha: null
owner_action_required: null
blocker: allocation PR must merge before first write
next_action: after allocation merge, create the named branch from that exact protected-main SHA and execute TDD RED
```
