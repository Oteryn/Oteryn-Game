# OTV2-20260826-repair-foundation-terminal-reconciliation

```yaml
task_id: OTV2-20260826-repair-foundation-terminal-reconciliation
title: Allocation — Foundation terminal reconnect reconciliation snapshot repair
mode: REPAIR
status: ALLOCATION_PENDING_MERGE
repository: Oteryn/Oteryn-Game
issue: 208
admission_main_sha: 341ce4ed752944d4e816ee4d50559557295d2b97
base_branch: main
planned_worker_branch: impl/foundation-terminal-reconciliation
pr: null
owner: Oteryn: work coordinator
execution_budget_minutes: 45
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
public_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
depends_on:
  - issue:199 completed Foundation V1 boundary
  - issue:200 completed transport-ref decision
blocks:
  - issue:167 durable terminal reconciliation adapter completion
shared_paths: none
excluded_scope:
  - no SQLx/Durability schema, migration, Cargo, workflow, registry, listener, production or external-repository change
  - no Foundation authority, fencing, resource-bound or admission-policy change
```

## Repair scope

The accepted Foundation V1 state machine models `DurableReconnectStateV1::Terminal` and consumes it in `ReconnectDurabilityFlowV1::accept_reconciliation`, but sibling Durability code cannot construct the typed terminal snapshot because all fields are private and only prepared/committed constructors exist.

After this allocation merges, one Foundation worker may make the minimum additive repair only in the owned path:

1. add a constrained terminal snapshot constructor that preserves the exact record and has no current generation or transport reference;
2. add a colocated regression for ambiguous same-attempt reconciliation to terminal;
3. preserve all existing public authority/fencing semantics and avoid a generic/unconstrained constructor.

## Authority and lifecycle

This allocation grants no authority until its PR merges. The worker starts only from the resulting protected `main`, on the planned dedicated branch, and may not edit Durability, Cargo/workspace, workflow, registry or task documents outside this record.

## Acceptance

- [ ] Terminal reconciliation snapshot is constructible by a sibling Durability module without exposing fields.
- [ ] The flow transitions from an ambiguous completion through terminal reconciliation to `ReconnectProjectionDecisionV1::Terminal`.
- [ ] Existing prepared and committed paths remain covered.
- [ ] Focused Foundation regression, full required workspace validation, independent exact-head authority review and protected merge gate pass.

## Validation plan

- focused: Foundation-owned regression will run the admission recovery test target that proves ambiguous same-attempt reconciliation reaches the constrained terminal projection.
- component: the worker must run the applicable locked workspace build, strict Clippy and test checks on the exact final head.
- E2E: NOT_APPLICABLE to this additive Foundation API repair; it creates no database, network or player-facing behavior.

## Review plan

- self-review: required against the exact final head; verify no public fields or unconstrained constructor are exposed.
- independent review: required on the same exact final head because this is Foundation reconnect authority/reconciliation API surface.
- closeout: freeze head, require exact-head protected merge gate and zero unresolved threads, then merge with expected head.

## Context checkpoint

```yaml
last_progress: allocation created from protected main 341ce4ed752944d4e816ee4d50559557295d2b97; no Foundation source mutation has been authorized
status: ALLOCATION_PENDING_MERGE
next_action: merge this allocation through exact-head governance and review, then allocate one worker from resulting protected main
```
