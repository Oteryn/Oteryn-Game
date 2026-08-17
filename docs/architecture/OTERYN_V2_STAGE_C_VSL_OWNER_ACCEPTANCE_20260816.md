# Oteryn v2 — Stage-C VSL Owner Acceptance

- Date: 2026-08-16
- Coordination issue: #310
- Pull request: #311
- Owner disposition source: project owner explicitly clarified that the prior acceptance applies to all remaining architecture decisions, including Stage-C.
- DecisionStatus: `ACCEPTED`
- ImplementationStatus: `NOT_STARTED`
- DeliveryStatus: `IN_REVIEW`
- Executor prompts: `HOLD`

## Accepted gates

```text
VSL-MOVE-01:    ACCEPTED
VSL-COMBAT-01:  ACCEPTED
VSL-CONTENT-01: ACCEPTED
```

The accepted scope is exactly the bounded architecture described by the corresponding candidate contracts and the Stage-C owner-decision package in PR #311.

## What acceptance binds

### VSL-MOVE-01

The current `ChannelRuntime` / `InstanceRuntime` remains the one authoritative local position owner. Local step and same-scope relocation remain distinct from scope handoff. Movement commits server-authoritative state; GAME-INTERACTION owns trigger/retry/reconciliation semantics; FND-02 owns authoritative projection/reconciliation. Exact Reference movement/LOS/timing values remain evidence-gated.

### VSL-COMBAT-01

GAME-ABILITY remains the sole effect/combat mutation pipeline. Creature death has one stable semantic occurrence. Durable loot materialization and pickup remain DUR-03/GAME-INTERACTION governed and idempotent; XP is a separate stable descendant workflow owned by GAME-CHAR. No distributed death/loot/XP transaction is introduced. Test-only fixture profiles remain non-shipping and cannot establish Reference parity.

### VSL-CONTENT-01

The minimum native semantic graph/compiler/projection/loader/activation seam is accepted for the first vertical slice. A bounded non-production evidence profile may prove deterministic compilation/loading/activation. The permanent World Project/World Bundle physical encoding remains explicitly undecided and still requires the DUR-04 format spike and later owner format-selection decision.

## Explicit non-authorizations

This acceptance does **not** authorize or prove:

- runtime/client/server/protocol/content implementation;
- production deployment or protected-environment changes;
- PostgreSQL DDL/migrations or live data mutation;
- final content physical-format selection;
- Reference parity or any promotion in the Reference evidence manifest;
- entitlement/Premium/VIP implementation under `PROD-ENTITLEMENTS-01`;
- concrete numeric resource limits that have not yet been registered and validated;
- any cross-repository write.

`ImplementationStatus` remains `NOT_STARTED` until the separately audited implementation executor handoff is released.

## Review requirement before merge

`VSL-COMBAT-01` touches durable loot/value invariants. Under root `AGENTS.md`, PR #311 therefore requires a genuinely independent review on the exact final SHA before merge. Self-review and a workflow that reports `NOT_APPLICABLE` do not satisfy that gate.

If independent review produces a finding that moves the head, exact-head CI and independent review must be repeated for the repaired final head.

## Supersession

This acceptance supersedes only the unresolved owner-disposition state in `OTERYN_V2_STAGE_C_VSL_OWNER_DECISION_PACKAGE_20260816.md`. The historical candidate contracts and decision package remain immutable evidence of what was reviewed and accepted.

## Current state

```text
DECISION: ACCEPTED
DELIVERY: IN_REVIEW
IMPLEMENTATION: NOT_STARTED
EXECUTOR_PROMPTS: HOLD
```
