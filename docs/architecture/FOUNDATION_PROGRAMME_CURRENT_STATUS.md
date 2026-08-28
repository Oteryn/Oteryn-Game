# Oteryn v2 Foundation Programme — Current Status

- Status: **Canonical current execution-status overlay**
- Date: 2026-08-24
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Reconciliation issue: `#97`
- Reconciliation snapshot: `main@e1a942675f98b4a42ce9d021773702e727c23574`
- Applies to: current DecisionStatus / DeliveryStatus / ImplementationStatus and the next safe execution gate

## 1. Reading rule

This document is the maintained current-status overlay. Accepted ADRs/contracts/owner baselines remain semantic authority for their owned scope. GitHub Issue/PR/CI state and `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` outrank stale prose when the repository has advanced.

`ARCHITECTURE_STATUS_MODEL.md` remains normative:

```text
DecisionStatus != DeliveryStatus != ImplementationStatus
```

Architecture acceptance, merged implementation, executable proof and production enablement are separate facts. This file therefore preserves the distinctions `ACCEPTED`, `IMPLEMENTED`, `PROVEN`, `BLOCKED` and `NOT_EVALUATED` rather than promoting a merged bounded seam into whole-programme proof.

## 2. Programme headline

Verified merged-main state at the reconciliation snapshot:

- `PROVEN`: Bootstrap delivery PR #10 merged as `0809004252db228e8f3fac3cdb6638c3c2a7fbda`; its lifecycle closeout completed through PR #11.
- `PROVEN`: Simulation delivery PR #14 merged as `66619daf5837f31f7c54676e9f8351ed4ae220b0`; the bounded production `oteryn-simulation-determinism` core is implemented and consumed by `apps/game-server`.
- `IMPLEMENTED`: Foundation protocol/runtime/admission delivery PR #59 merged as `a70318484b1ffdd328b53cdc70a4386a516d0109`; lifecycle closeout completed through PR #74.
- `PROVEN`: Foundation post-merge independent audit PR #81 merged as `55e30e23c3d5775ce760c6b210ea77f152b359ae` with P0/P1/P2 = 0 for the audited implementation tree.
- `NOT_PROVEN`: the historical mandatory independent review on the exact final **pre-merge** Foundation head cannot be reconstructed as proven and remains a retained process caveat. The later post-merge audit does not rewrite that history.
- `IMPLEMENTED`: Domain Character/Item core PR #56 merged as `0facd7f89edc1b0685e67c5531839e8e6f04c466`; lifecycle/ownership release completed through PR #82.
- `IMPLEMENTED` for bounded non-production evidence only: Content PR #58 merged as `8f99f25d0b1b3472d40504cd54b463cf752ebe7a`. Its activation-boundary P0 was repaired by PR #87 (`db95bc720529b643531c79f708086f69dd612d22`) and terminally closed by #89. Production Content acceptance/activation remains separately owner-gated by Issue #54.
- `IMPLEMENTED`: the QA evidence shell PR #98 merged as `dc22e0da8efcc6f4458416191261063b295af5b4`; Issue #91 is closed completed. This is test/evidence infrastructure only.
- `NOT_EVALUATED`: real gameplay Tier 1 and native-client Tier 2 journeys. Current product state still lacks a production gameplay listener/client-entry seam.
- `NOT IMPLEMENTED`: Durability, Ability, Interaction, AI, native gameplay Client, Movement and Combat on merged `main`.
- `BLOCKED`: production gameplay listener/client-entry remains absent pending the #96 preparation/allocation path; Client cannot be released before that seam is merged and qualified.

No statement above promotes Reference parity, production Content activation, a permanent World Bundle format, production deployment or real gameplay E2E.

## 3. Current three-axis status

The table reports accepted architecture against merged implementation evidence. `IMPLEMENTED` means the named bounded merged scope exists; `PROVEN` is reserved for the exact evidence boundary stated here and does not imply whole-contract or production proof.

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus |
|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` |
| `FND-ID-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` |
| `NET-TRANSPORT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` for bounded framing/ingress primitives; production gameplay listener absent |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` |
| `FND-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `DUR-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` for production durability/content-format authority |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-VISION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-CHANNEL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-CHAR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` |
| `GAME-ITEM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` |
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED`* |
| `GAME-ABILITY-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-INTERACTION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ALPHA-CLIENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `GAME-AI-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `ANL-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `QA-E2E-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` evidence shell; real Tier 1/2 `NOT_EVALUATED` |
| `VSL-MOVE-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-COMBAT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-CONTENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `IMPLEMENTED` bounded non-production evidence seam; production `BLOCKED` by #54 |
| `PROD-ENTITLEMENTS-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
| `VSL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `PERF-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `OPS-CHANNEL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |

\* `SIM-DETERMINISM-01 = IMPLEMENTED` means the bounded executor-defined SIM core from PR #14 exists on merged `main`. It is not a claim that every replay envelope, downstream consumer, formula descriptor, external-fact integration or VSL proof is complete.

Foundation rows marked `IMPLEMENTED` are bounded to the merged PR #59 scope: typed IDs, FND-02 ingress/framing primitives, FND-03 generation/ordinal fencing, FND-04 admission/session/lease/reconnect semantics and reconciliation. PR #59 explicitly did **not** add a production gameplay listener, gameplay command/state IDs or production persistence schema.

## 4. Reference evidence/parity

Current `ABILITY_COMBAT` truth remains evidence-gated:

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

Merged Foundation/Domain/Content/QA work does not promote Reference evidence or parity.

## 5. Current implementation and preparation state

```text
BOOTSTRAP:   COMPLETED + LIFECYCLE_CLOSED
SIM:         COMPLETED + LIFECYCLE_CLOSED
FOUNDATION:  IMPLEMENTED + LIFECYCLE_CLOSED
             post-merge independent audit PASS P0/P1/P2=0
             historical pre-merge independent exact-head gate NOT_PROVEN
DOMAIN:      IMPLEMENTED + LIFECYCLE_CLOSED + OWNERSHIP_RELEASED
CONTENT:     NON_PRODUCTION EVIDENCE SEAM IMPLEMENTED
             activation-boundary repair TERMINAL
             production acceptance/activation BLOCKED by #54
QA:          EVIDENCE SHELL IMPLEMENTED + Issue #91 CLOSED
             real gameplay Tier 1/2 NOT_EVALUATED
DURABILITY:  NOT IMPLEMENTED; #94 topology + applicable #93 hard-max gate
ABILITY:     NOT IMPLEMENTED; applicable #93 hard-max gate
INTERACTION: NOT IMPLEMENTED; applicable #93 hard-max gate
AI:          NOT IMPLEMENTED; applicable #93 hard-max gate
SERVER SEAM: ABSENT; #96 preparation/allocation path
CLIENT:      NOT IMPLEMENTED; blocked on merged production server seam
MOVEMENT:    NOT IMPLEMENTED; blocked on Interaction + Client + real QA readiness + Movement hard maxima
COMBAT:      NOT IMPLEMENTED; blocked on Movement + Ability + Interaction + Durability + Client + QA
```

Preparation Issues #93, #94, #95, #96 and #97 are open at this reconciliation snapshot. They are independent preparation lanes when path-disjoint; unfinished #95 or #97 is not a global barrier to an unrelated implementation lane whose own readiness gates are satisfied.

The exact operational state of a lane must still be resolved from current GitHub Issue/PR/CI state before mutation. Preparation prose grants no implementation write authority.

## 6. Production Content gate

The Content evidence seam is safe to describe as merged and repaired, but production Content remains blocked:

- Issue #54 remains the owner-gated production acceptance/activation lifecycle.
- accepted DUR-04/VSL hard maxima required for production are absent;
- production activation authority is absent;
- permanent physical World Project/World Bundle format selection remains separately owner-gated;
- the evidence activation state machine remains test-only and cannot be imported by production consumers after PR #87.

Therefore `IMPLEMENTED` for `VSL-CONTENT-01` in this status overlay means only the bounded non-production evidence seam, not production-ready Content.

## 7. QA evidence boundary

PR #98 / Issue #91 terminally delivered the allocated QA evidence shell and its exact-head validation. It proves the evidence-envelope/test-side machinery within that scope.

It does **not** prove physical gameplay journeys:

```text
Tier 1 real production server/protocol/persistence journey: NOT_EVALUATED
Tier 2 native-client journey:                          NOT_EVALUATED
```

The production gameplay listener/client-entry seam is still absent, so any synthetic/direct-domain test success remains insufficient to promote Tier 1/2.

## 8. PROD-ENTITLEMENTS-01 reconciliation

`PROD-ENTITLEMENTS-01` remains accepted architecture, not runtime implementation.

- Game consumer/enforcement contract acceptance: PR #20, independently reviewed exact head `0dfa0c5cdcd811c63d6926da166550712dfb59fc`, squash merge `d40a225e5fedca0396f34b4f2b6c1e343161e6ff`.
- Lifecycle closeout: PR #27, squash merge `84f485089b97cfaba1b5c6628ed8e0ba6655dc51`.
- The historical filename `PROD-ENTITLEMENTS-01_GAME_CONSUMER_ENFORCEMENT_CONTRACT_CANDIDATE.md` is retained as source provenance; its filename does not demote the accepted decision state.

Physical persistence/inbox/cursor schema, transport/IDL, crypto/container choices beyond accepted producer/consumer contracts, concrete product catalogue/benefits, numeric lease/skew policy, runtime implementation, payment behavior and production activation remain separately governed.

## 9. FND-04 reconnect/disconnect timing precedence

Later accepted FND-04 architecture still supersedes historical checkpoint wording for reconnect/liveness timing.

Canonical rule:

```text
historical 2s / 5s / 15s reconnect-liveness-grace values -> NON-CANONICAL / DEFERRED
exact defensive PvE re-entry protection after eligible valid re-entry -> 4 seconds ACCEPTED
```

FND-04 intentionally leaves probe cadence/hysteresis/control-loss detection, stale transport cleanup, same-session grace, stable-control re-arm threshold, CharacterLease timing and prepared/rate/resource limits to measured registry/OPS/PERF/DUR evidence before production activation.

Historical disconnect checkpoint documents remain provenance and must not override the accepted FND-04 contract set.

## 10. Holds that remain binding

- exact Reference formulas/mechanics/values remain evidence-gated; fixtures cannot establish parity;
- permanent World Project/World Bundle physical encoding still requires the #95 evidence spike and later owner decision;
- concrete finite resource ceilings are required before affected executable acceptance; #93 must close each exercised dimension or exclude it fail-closed;
- Durability implementation waits for #94 topology/allocation and applicable DUR-03 hard-max closure;
- producer event families must exist before ANL-02/03 can claim real metric/detector coverage;
- production gameplay server-seam implementation requires #96 preparation plus a later exact coordinator allocation;
- native Client waits for the production server seam;
- QA-E2E Tier 1/2 evidence remains mandatory for terminal vertical-slice proof;
- PERF/OPS retain measured production capacity/orchestration authority;
- high-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing changes require genuinely independent exact-head review under root `AGENTS.md`;
- entitlement runtime/product activation remains unallocated and production-disabled even though its paper consumer contract is accepted.

## 11. Executor state

```text
EXECUTOR_PROGRAMME: RELEASED_AND_ACTIVE
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
BOOTSTRAP: COMPLETED
SIM: COMPLETED
FOUNDATION: COMPLETED
DOMAIN: COMPLETED
CONTENT_EVIDENCE: COMPLETED_NON_PRODUCTION
QA_EVIDENCE_SHELL: COMPLETED
REAL_GAMEPLAY_QA_TIER_1_2: NOT_EVALUATED
NEXT_WAVE_PREPARATION: ACTIVE (#93-#97)
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

## 12. Runtime / production authority

Nothing here authorizes production deployment, protected-environment approval, live data/session/account mutation, production PostgreSQL migration execution, Platform writes, external-repository mutation, entitlement activation, Reference parity claims, permanent Content-format selection or owner-funded AI use.

`PRODUCTION_AUTHORITY: NONE`
