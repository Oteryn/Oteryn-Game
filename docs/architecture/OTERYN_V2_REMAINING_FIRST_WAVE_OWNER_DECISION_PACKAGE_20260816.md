# Oteryn v2 — Remaining First-Wave Owner Decision Package

- Status: `RESOLVED / HISTORICAL DECISION RECORD`
- Date: 2026-08-16
- Coordination issue: #308
- Delivery PR: #309
- Trusted preparation base: `main@dfc75d1332f710d6ac85009653579f7bc51ccc59`
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**
- Normative owner-acceptance source: `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md`

## Purpose

This file preserves the coordinator decision-preparation record that re-evaluated the remaining first-wave proposal/candidate packages after `GAME-ABILITY-01` became owner-accepted.

The package originally presented `ACCEPT | REWORK | DEFER` choices for:

1. `GAME-INTERACTION-01`;
2. `ALPHA-CLIENT-01`;
3. `GAME-AI-01`;
4. paired `ANL-02` + `ANL-03`.

On 2026-08-16 the repository owner explicitly selected **ACCEPT for all four decision rows** and authorized the `ready`/automatic Codex path for PR #309. The later owner-acceptance baseline is normative for those DecisionStatus values; this preparation record is retained for rationale/audit history and must not be read as an unresolved owner request.

## Verified preparation state

Before owner disposition the coordinator established:

- `GAME-ABILITY-01 = ACCEPTED / LIFECYCLE_CLOSED / NOT_STARTED`;
- Agent-A Reference evidence remained 0/4 promoted with target `UNKNOWN`, provenance/legal `PENDING`, implementation `NOT_STARTED` and parity `PARITY_PENDING_EVIDENCE`;
- GAME-INTERACTION #277, GAME-AI #276, ALPHA-CLIENT #273 and ANL-02/03 #270 were merged/lifecycle-closed with clean final review/gate evidence but had not yet been owner-accepted;
- no material accepted-semantic conflict was found between GAME-ABILITY acceptance and those packages;
- concrete runtime APIs, numeric limits, producer events, movement/handoff ownership, event/encounter ownership, Reference behavior and executable proof remained downstream by design.

## Decision rationale preserved

### GAME-INTERACTION-01

Recommendation was `ACCEPT` because stable child occurrence identity, deterministic ordering/RNG, exactly-once/reconciliation semantics and truthful `PENDING | COMMITTED | REJECTED` outcome state are safety-critical before implementation. GAME-ABILITY acceptance removed the historical noncanonical effect-owner blocker while leaving movement/handoff, writable-text, physical FND-02 registration and numeric limits downstream.

### ALPHA-CLIENT-01

Recommendation was `ACCEPT` because production composition-root, Platform/Gateway/FND admission separation, non-authoritative client projection, scene/audio non-authority, client-safe content, settings/privacy, release atomicity and Tier-1/2/3 evidence boundaries need to be fixed before native gameplay implementation. Concrete libraries, transport runtime, protocol implementation and E2E evidence remain downstream.

### GAME-AI-01

Recommendation was `ACCEPT` after GAME-INTERACTION because one-writer local AI authority, representation-neutral bounded deterministic resolution, staged all-or-nothing AI-local commit, proposal-only pathfinding/scripts, finite spawn retry and no value authority are correctness/safety boundaries. Concrete representation/path algorithms, hard numeric limits, event/encounter owner and reward attribution remain downstream.

### ANL-02 + ANL-03

Recommendation was `ACCEPT TOGETHER` because their read-only evidence-quality/privacy/regression/invariant/human-disposition semantics must constrain later producer and analytics design, while concrete event IDs, thresholds, technologies, resource ceilings and enforcement remain downstream.

## Serial canonicalization order

The coordinator recommended and the owner accepted this recording order:

```text
GAME-INTERACTION-01
-> ALPHA-CLIENT-01
-> GAME-AI-01
-> ANL-02 / ANL-03
```

The order is coordination/dependency ordering only. It does not create cross-domain mutation authority.

## Preserved exclusions

The owner disposition did not authorize or imply:

- runtime/client/server/protocol/content implementation;
- DDL/migrations or Platform writes;
- production/protected-environment mutation;
- Reference evidence/parity promotion;
- concrete libraries/algorithms/frameworks;
- numeric formula/resource-limit values;
- producer event registration;
- Premium/VIP/game-consumed entitlement activation;
- `PROD-ENTITLEMENTS-01` acceptance.

## PROD-ENTITLEMENTS-01

The entitlement gate remains separately deferred for unrelated foundation work and mandatory before any entitlement executor/activation. Platform producer remediation is proven/pinned, but the Oteryn-v2 consumer/enforcement contract remains not accepted.

## Final disposition

```yaml
GAME-INTERACTION-01: ACCEPT
ALPHA-CLIENT-01: ACCEPT
GAME-AI-01: ACCEPT
ANL-02: ACCEPT
ANL-03: ACCEPT
implementation_authority: NONE
```

See `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md` for the normative accepted scope and remaining blockers.

`IMPLEMENTATION_AUTHORITY: NONE`
