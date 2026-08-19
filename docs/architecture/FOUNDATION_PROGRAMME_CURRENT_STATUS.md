# Oteryn v2 Foundation Programme — Current Status

- Status: **Canonical current execution-status overlay**
- Date: 2026-08-17
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current DecisionStatus / DeliveryStatus / ImplementationStatus and the next safe execution gate

## 1. Reading rule

This document is the maintained current-status source. Accepted ADRs/contracts/owner baselines remain semantic authority for their owned scope. Historical backlog/proposal/candidate prose is not current execution truth when it conflicts with this file or a later explicit owner-acceptance baseline.

`ARCHITECTURE_STATUS_MODEL.md` remains normative:

```text
DecisionStatus != DeliveryStatus != ImplementationStatus
```

Architecture acceptance and prompt release never imply runtime implementation, Reference parity or production enablement.

## 2. Programme headline

The native foundation, durability/content/determinism architecture, complete first gameplay/client/analytics architecture wave, and bounded Stage-C movement/combat/content architecture are owner-accepted and lifecycle-closed.

Canonical recent deliveries:

- `GAME-ABILITY-01`: PR #306 / merge `e2bb284f56f39d8fa01a843d098bcb21d17d77ac`; closeout #307 / `dfc75d1332f710d6ac85009653579f7bc51ccc59`;
- remaining first-wave acceptance: PR #309 / merge `bf2a2ae279516f62626a5d8f4dc1aeb587535c62`; its terminal task bookkeeping is consolidated into PR #314;
- Stage-C acceptance: PR #311 / merge `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c`; genuinely independent exact-head review reported zero material findings on `c5d9f839abd8998d42f4f37b203882f03bb51ce0`;
- Stage-C lifecycle/status closeout: PR #318 / merge `a6a5180d98cf7791e40d9e1d08b25a5c8b4eff96`;
- final implementation-prompt handoff: PR #314 / merge `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815`; the evaluated coordinator programme is released for explicit invocation.

No accepted gameplay/runtime gate below is implemented merely because its architecture or prompt is released.

## 3. Current three-axis status

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
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` |
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
| `VSL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `PERF-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `OPS-CHANNEL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` |
| `PROD-ENTITLEMENTS-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` |

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

Agent A #271 promoted **0/4**. Architecture acceptance and prompt release do not change this.

## 5. Released implementation handoff

PR #314 merged as `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815`; the evaluated implementation coordinator programme is released. Normal owner entry point:

```text
Oteryn: implementation coordinator
```

The coordinator must create bounded allocations. Direct worker aliases remain read-only without a live coordinator allocation naming exact lane, paths, dependencies and merge order.

Canonical order:

```text
BOOTSTRAP [serial]
  -> FOUNDATION + SIM + DOMAIN + CONTENT + QA
  -> DURABILITY after Foundation/Domain
  -> ABILITY + INTERACTION + AI after Foundation/SIM/Domain/Content
  -> CLIENT after compatible Foundation seam
  -> MOVE after Foundation/SIM/Domain/Content/Interaction/Client/QA
  -> COMBAT only after merged MOVE + Foundation/SIM/Domain/Content/Ability/Interaction/Durability/Client/QA

CHANNEL = later after Foundation/Domain/Durability
CONTENT-FORMAT-SPIKE = evidence only
ANALYTICS = later after concrete producer event families exist
```

Stable workspace/registry/ID mutations remain serialized.

## 6. Holds that remain binding

Prompt release does not remove lane-specific gates:

- `PROD-ENTITLEMENTS-01` blocks Premium/VIP/game-consumed entitlement implementation/activation only;
- exact Reference formulas/mechanics/values remain evidence-gated; test fixtures cannot establish parity;
- permanent World Project/World Bundle encoding still requires the DUR-04 format spike and later owner decision;
- concrete finite resource ceilings are required before affected executable acceptance; missing values fail closed;
- producer event families must exist before ANL-02/03 can claim real metric/detector coverage;
- protocol/admission/server/client/persistence implementations remain `NOT_STARTED`;
- QA-E2E Tier 1/2/3 evidence remains mandatory for terminal vertical-slice proof;
- PERF/OPS retain measured production capacity/orchestration authority;
- high-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing changes require genuinely independent exact-head review under root `AGENTS.md`.

## 7. Executor state

After PR #314 / `88f4fb754b5ae11243afd38a9e0b6a8e3b0a5815`:

```text
EXECUTOR_PROGRAMME: RELEASED
DEFAULT_ENTRYPOINT: Oteryn: implementation coordinator
DIRECT_WORKERS: ALLOCATION_GATED
IMPLEMENTATION_WORKERS_STARTED: NO
IMPLEMENTATION_AUTHORITY_OUTSIDE_LIVE_COORDINATOR_ALLOCATION: NONE
```

PR #314 did not start implementation. A later explicit coordinator invocation may create bounded allocations; no worker receives write authority until its live allocation exists.

## 8. Runtime / production authority

Nothing here authorizes production deployment, protected-environment approval, live data/session/account mutation, production PostgreSQL migration execution, Platform writes, external-repository mutation, entitlement activation, Reference parity claims or owner-funded AI use.

`PRODUCTION_AUTHORITY: NONE`
