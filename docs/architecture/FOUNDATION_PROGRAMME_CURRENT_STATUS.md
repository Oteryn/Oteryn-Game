# Oteryn v2 Foundation Programme — Current Status

- Status: **Canonical current execution-status overlay**
- Date: 2026-08-22
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Reconciliation issue: `#48`
- Reconciliation snapshot: `main@79e2f3baf17bd3b2231ab71c5dc5019e9aa0441e`
- Wave 1 exact-base bind: `fd39c6aa026e82062a8b29af24811d467c115f19`
- Applies to: current DecisionStatus / DeliveryStatus / ImplementationStatus and the next safe execution gate

## 1. Reading rule

This document is the maintained current-status source. Accepted ADRs/contracts/owner baselines remain semantic authority for their owned scope. Historical backlog/proposal/candidate/checkpoint prose is not current execution truth when it conflicts with this file or a later explicit owner-acceptance baseline.

`ARCHITECTURE_STATUS_MODEL.md` remains normative:

```text
DecisionStatus != DeliveryStatus != ImplementationStatus
```

Architecture acceptance, implementation allocation, an active worker branch and production enablement are separate facts. An unmerged worker branch does not change merged-main `ImplementationStatus` for a gate.

## 2. Programme headline

The native foundation, durability/content/determinism architecture, complete first gameplay/client/analytics architecture wave, bounded Stage-C movement/combat/content architecture and the Game-side entitlement consumer/enforcement architecture are accepted and lifecycle-closed for their declared paper scopes.

Implementation is no longer merely released for future invocation:

- `PROVEN`: Bootstrap delivery PR #10 merged as `0809004252db228e8f3fac3cdb6638c3c2a7fbda`; its lifecycle closeout completed through PR #11.
- `PROVEN`: SIM delivery PR #14 merged as `66619daf5837f31f7c54676e9f8351ed4ae220b0`; its lifecycle closeout completed through PR #15.
- `PROVEN`: PR #14 delivered the bounded production `oteryn-simulation-determinism` core consumed by `apps/game-server`: profile revision, checked deterministic numeric helpers, purpose-isolated/retry-stable decision derivation, semantic time and canonical state-hash support.
- `PROVEN`: Wave 1 allocation PR #45 merged as `33cec30b8075c73290d7d76e9f59df4701771650`.
- `PROVEN`: exact-base bind PR #46 merged as `fd39c6aa026e82062a8b29af24811d467c115f19`.
- `PROVEN`: FOUNDATION, DOMAIN, CONTENT and QA worker branches/task records exist from the post-bind main and report `status: implementing`.
- `PROVEN`: Foundation and Domain branches contain implementation commits beyond the exact-base bind; Content and QA are active but remain behind their own acceptance/merge gates.
- `PROVEN`: later main commit `79e2f3baf17bd3b2231ab71c5dc5019e9aa0441e` (PR #47, independent audit prompt) is disjoint from the implementation allocations and this status reconciliation.

Current implementation execution truth is owned operationally by `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` plus exact worker task/branch/PR/CI state. This architecture overlay summarizes that truth; it does not take over worker ownership.

## 3. Current three-axis status

The table below reports canonical merged architecture plus merged implementation evidence. Active but unmerged Wave 1 worker code is called out separately in Section 5 and does not promote these rows.

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus |
|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` |
| `FND-ID-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `NET-TRANSPORT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `FND-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-VISION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-CHANNEL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-CHAR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-ITEM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED`* |
| `GAME-ABILITY-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-INTERACTION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ALPHA-CLIENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-AI-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `QA-E2E-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED / EVIDENCE_REQUIRED` |
| `VSL-MOVE-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-COMBAT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-CONTENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `PROD-ENTITLEMENTS-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `PERF-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `OPS-CHANNEL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |

\* `SIM-DETERMINISM-01 = IMPLEMENTED` means the bounded executor-defined SIM core from PR #14 exists on merged `main`. It is **not** a claim that every full-contract replay envelope, domain consumer, formula descriptor, future external-fact integration or VSL proof is complete. Whole-gate `PROVEN` is deliberately withheld until named downstream evidence exists.

## 4. Reference evidence/parity

Current `ABILITY_COMBAT` truth remains:

```yaml
registered_cases: 4
promoted_cases: 0
target_evidence: UNKNOWN
source_case_provenance: PENDING
legal_review: PENDING
oteryn_implementation: NOT_STARTED
parity: PARITY_PENDING_EVIDENCE
canonical_digest: null
```

Architecture acceptance and implementation activity do not promote Reference evidence or parity.

## 5. Active implementation wave

At the reconciliation snapshot, the implementation coordinator has already passed the serial Bootstrap and SIM gates and released the post-SIM Wave 1 exact-base allocation.

```text
BOOTSTRAP: COMPLETED + LIFECYCLE_CLOSED
SIM:       COMPLETED + LIFECYCLE_CLOSED
WAVE1:
  FOUNDATION -> implementing on agent/otv2-impl-foundation-runtime-01
  DOMAIN     -> implementing on agent/otv2-impl-domain-core-01
  CONTENT    -> implementing on agent/otv2-impl-vsl-content-01
  QA         -> implementing on agent/otv2-impl-qa-e2e-01
```

The worker branches remain noncanonical until their own required review/CI/merge lifecycle succeeds. Foundation retains mandatory genuinely independent exact-head review for protocol/session/admission/fencing work. Shared composition/workspace mutations remain coordinator-serialized, with Foundation holding the first shared lease under the live allocation.

No architecture status document may treat the existence of these branches as proof that FND-02/FND-03/FND-04, Character/Item, Content or QA gates are already implemented on `main`.

## 6. PROD-ENTITLEMENTS-01 reconciliation

`PROD-ENTITLEMENTS-01` is no longer `PROPOSED`.

- Game consumer/enforcement contract acceptance: PR #20, independently reviewed exact head `0dfa0c5cdcd811c63d6926da166550712dfb59fc`, squash merge `d40a225e5fedca0396f34b4f2b6c1e343161e6ff`.
- Lifecycle closeout: PR #27, squash merge `84f485089b97cfaba1b5c6628ed8e0ba6655dc51`.
- The historical filename `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md` is retained as source provenance; the filename does not demote the later accepted decision status.

Acceptance remains architecture-only. Physical persistence/inbox/cursor schema, transport/IDL, crypto/container choice beyond accepted producer/consumer contracts, concrete product catalogue/benefits, numeric lease/skew policy, runtime implementation, payment behavior and production activation remain separately governed.

## 7. FND-04 reconnect/disconnect timing precedence

Later accepted FND-04 architecture supersedes historical checkpoint wording for reconnect/liveness timing.

Canonical rule:

```text
historical 2s / 5s / 15s reconnect-liveness-grace values -> NON-CANONICAL / DEFERRED
exact defensive PvE re-entry protection after eligible valid re-entry -> 4 seconds ACCEPTED
```

FND-04 intentionally leaves probe cadence/hysteresis/control-loss detection, stale transport cleanup, same-session grace, stable-control re-arm threshold, CharacterLease timing and prepared/rate/resource limits to measured registry/OPS/PERF/DUR evidence before implementation activation.

Historical disconnect checkpoint documents remain provenance and must not override the accepted FND-04 A/B/C contract set.

## 8. Holds that remain binding

Implementation progress does not remove lane-specific gates:

- exact Reference formulas/mechanics/values remain evidence-gated; fixtures cannot establish parity;
- permanent World Project/World Bundle physical encoding still requires the DUR-04 format spike and later owner decision;
- concrete finite resource ceilings are required before affected executable acceptance; missing required values fail closed;
- producer event families must exist before ANL-02/03 can claim real metric/detector coverage;
- QA-E2E Tier 1/2/3 evidence remains mandatory for terminal vertical-slice proof;
- PERF/OPS retain measured production capacity/orchestration authority;
- high-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing changes require genuinely independent exact-head review under root `AGENTS.md`;
- entitlement runtime/product activation remains unallocated and production-disabled even though its paper consumer contract is accepted.

## 9. Executor state

```text
EXECUTOR_PROGRAMME: RELEASED_AND_ACTIVE
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_WORKERS_STARTED: YES
BOOTSTRAP: COMPLETED
SIM: COMPLETED
WAVE1: IMPLEMENTING
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

The exact operational state of an implementation lane must be resolved from live coordinator allocation + worker task/branch/PR/CI evidence, not copied from this summary if the repository has moved since the snapshot.

## 10. Runtime / production authority

Nothing here authorizes production deployment, protected-environment approval, live data/session/account mutation, production PostgreSQL migration execution, Platform writes, external-repository mutation, entitlement activation, Reference parity claims or owner-funded AI use.

`PRODUCTION_AUTHORITY: NONE`
