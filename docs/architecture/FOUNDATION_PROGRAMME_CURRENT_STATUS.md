# Oteryn v2 Foundation Programme — Current Status

- Status: **Canonical current execution-status overlay**
- Date: 2026-08-17
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: current DecisionStatus / DeliveryStatus / ImplementationStatus and the next safe architecture or implementation-proof gate
- Detailed historical review/CI/repair evidence: accepted contracts, owner baselines, archived task records and merged PR discussions

## 1. Reading rule

This document is the maintained current-status source. Accepted ADRs/contracts/owner baselines remain semantic authority for their owned scope. Historical backlog/horizon prose is not current execution truth when it conflicts with this file or a later explicit owner-acceptance baseline.

`ARCHITECTURE_STATUS_MODEL.md` remains normative:

```text
DecisionStatus != DeliveryStatus != ImplementationStatus
```

Architecture acceptance never implies runtime implementation, Reference parity or production enablement.

## 2. Current programme headline

The native foundation, durability/content/determinism architecture, the complete first A-F gameplay/client/analytics architecture wave, and the bounded Stage-C movement/combat/content architecture are owner-accepted.

Recent canonical deliveries are:

- `GAME-ABILITY-01` owner acceptance: PR #306 / merge `e2bb284f56f39d8fa01a843d098bcb21d17d77ac`; lifecycle closeout #307 / `dfc75d1332f710d6ac85009653579f7bc51ccc59`;
- remaining first-wave owner acceptance: PR #309 / merge `bf2a2ae279516f62626a5d8f4dc1aeb587535c62`; its bookkeeping closeout is consolidated into PR #314;
- Stage-C `VSL-MOVE-01`, `VSL-COMBAT-01`, `VSL-CONTENT-01` owner acceptance delivery: PR #311 / merge `e0ea9ef87c01dec720a22e8df6d54bfd669cb62c`; lifecycle/status closeout is PR #318.

All accepted first-wave and Stage-C gameplay/client/analytics gates remain `ImplementationStatus=NOT_STARTED` unless explicitly stated otherwise below.

## 3. Current three-axis status

| Gate | DecisionStatus | DeliveryStatus | ImplementationStatus | Current note |
|---|---|---|---|---|
| `FND-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | canonical Rust workspace/dependency cutover applied by PR #50 / `78988f72a80cc904aa9176ae850c50d4efa0b0f0` |
| `VSL-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `PROVEN` | native-client repository cutover/migration complete |
| `FND-ID-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | semantic identity vocabulary accepted; runtime identity implementation remains separate |
| `FND-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | `protocol-oteryn` v1 architecture accepted; TCP profile 1 registered architecturally, no gameplay adapter/listener |
| `NET-TRANSPORT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | TCP-default / future QUIC-opt-in transport architecture; QUIC profile/runtime still future work |
| `FND-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | one-writer GameNode/Channel runtime execution architecture accepted |
| `FND-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | admission/GameSession/CharacterLease/reconnect architecture accepted |
| `DUR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | durable identifier representation accepted |
| `DUR-02` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | Persistence-v1 architecture accepted; no PostgreSQL DDL/migration execution |
| `DUR-03` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | item/currency/value transaction, conservation and anti-duplication architecture accepted |
| `DUR-04` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | content/world/compiler/bundle/scripting architecture accepted |
| `ANL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | event/audit foundation accepted |
| `GAME-VISION-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | minimum product direction and immutable first Reference target accepted |
| `GAME-CHANNEL-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | channel product/lifecycle/multiplicity policy accepted |
| `GAME-CHAR-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | Character Stage A/B semantics accepted; unresolved Reference rules remain parity-gated |
| `GAME-ITEM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | item model/equipment/container semantics accepted |
| `SIM-DETERMINISM-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | arithmetic/RNG/order/replay/state-hash architecture accepted |
| `GAME-ABILITY-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | owner acceptance baseline merged via #306; exact Reference mechanic evidence remains independent |
| `GAME-INTERACTION-01` | `ACCEPTED` | `IN_REVIEW` closeout on #314 | `NOT_STARTED` | owner acceptance delivery merged via #309; consolidated lifecycle bookkeeping remains on #314 |
| `ALPHA-CLIENT-01` | `ACCEPTED` | `IN_REVIEW` closeout on #314 | `NOT_STARTED` | owner acceptance delivery merged via #309; consolidated lifecycle bookkeeping remains on #314 |
| `GAME-AI-01` | `ACCEPTED` | `IN_REVIEW` closeout on #314 | `NOT_STARTED` | owner acceptance delivery merged via #309; consolidated lifecycle bookkeeping remains on #314 |
| `ANL-02` | `ACCEPTED` | `IN_REVIEW` closeout on #314 | `NOT_STARTED` | owner acceptance delivery merged via #309; consolidated lifecycle bookkeeping remains on #314 |
| `ANL-03` | `ACCEPTED` | `IN_REVIEW` closeout on #314 | `NOT_STARTED` | owner acceptance delivery merged via #309; consolidated lifecycle bookkeeping remains on #314 |
| `QA-E2E-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED / EVIDENCE_REQUIRED` | three-tier E2E architecture accepted; executable evidence still blocks terminal VSL claims |
| `VSL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | vertical-slice programme remains a proof programme, not an implementation claim |
| `VSL-MOVE-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | Stage-C local movement/collision/visibility/reconciliation architecture accepted via #311; implementation not released |
| `VSL-COMBAT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | Stage-C combat/death/loot/XP/pickup architecture accepted via #311; Reference values remain evidence-gated |
| `VSL-CONTENT-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED` | Stage-C semantic content/compiler/loader evidence slice accepted; permanent physical format remains undecided |
| `PERF-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | measured capacity/resource values required before capacity/performance claims |
| `OPS-CHANNEL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | required before automatic production channel scaling/recovery claims |
| `PROD-ENTITLEMENTS-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | Platform producer prerequisite satisfied; Oteryn-v2 consumer/enforcement contract remains unaccepted; Premium/VIP executor/activation blocked |

The five first-wave rows still shown `IN_REVIEW` are not waiting on owner acceptance or PR #309; only their separately consolidated lifecycle-bookkeeping delivery on #314 remains. Stage-C architecture lifecycle closes through PR #318 without changing its `ImplementationStatus`.

## 4. Reference evidence/parity state

The first immutable Reference target remains Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance boundary.

The accepted Reference evidence/parity manifest remains paper-only evidence authority. Current `ABILITY_COMBAT` state is exactly:

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

Agent A PR #271 promoted **0/4** cases. None of the gameplay architecture acceptances changes that result.

## 5. Accepted gameplay/client/analytics and Stage-C boundaries now binding

### GAME-ABILITY-01

One typed server-authoritative semantic ability/effect pipeline, revision-bound occurrence lineage, owner-scoped commit groups, explicit bounded future work/timer catch-up, deterministic reaction descendants, client non-authority and explicit resource-limit registration before executable acceptance. No formulas/Reference values/runtime APIs are implied.

### GAME-INTERACTION-01

Stable recursive child occurrence identity, deterministic order/RNG, exactly-once/reconciliation semantics, truthful pending/committed/rejected results and no generic distributed transaction. Cross-scope handoff, durable writable text, physical FND-02 registration and numeric limits remain downstream owners.

### ALPHA-CLIENT-01

One production client composition root, Platform/Gateway/pre-admission/final-game authority separation, non-authoritative projection/presentation, implementation-backed capabilities, client-safe content, deterministic settings/privacy, atomic release activation and real-boundary Tier 1/2/3 evidence. Native gameplay runtime remains unavailable until separately implemented/proven.

### GAME-AI-01

One local authoritative runtime owner, finite deterministic bounded representation-neutral AI resolution, staged all-or-nothing AI-local commit, bounded proposal pathfinding/scripts, finite spawn retry/provenance and no value authority. Concrete framework/path algorithm/resource maxima/event owner/reward attribution remain downstream.

### ANL-02 / ANL-03

Read-only analytics/integrity/security evidence. Explicit quality/privacy/revision semantics, fail-closed regression evaluation, deterministic invariant evaluation when evidence is complete, immutable human-review lifecycle, referral-after-disposition, non-adverse optional client diagnostics. No gameplay mutation, sanction or enforcement authority.

### Stage-C VSL architecture

`VSL-MOVE-01`, `VSL-COMBAT-01` and `VSL-CONTENT-01` are accepted architecture. They freeze only the minimum movement, combat/value and native-content integration seams required for the first real-boundary vertical slice. They do not implement those seams, select the permanent world format, supply unknown Reference formulas/values or authorize production.

## 6. Cross-domain implementation blockers that remain real

Stage-C architecture is no longer an unresolved architecture blocker. Remaining implementation/evidence dependencies include:

- GAME-INTERACTION cross-scope movement/handoff owner and durable writable-text owner for mechanics that require those broader capabilities;
- GAME-AI event/encounter durable multi-actor owner, concrete resource ceilings and controlled-actor reward attribution remain downstream;
- `GAME_EVENT_FOUNDATION_REGISTRY.json` still requires concrete producer event families before ANL-02/03 can claim real metric/detector coverage;
- FND-02/FND-04/client/server executable protocol/admission path remains unimplemented;
- DUR-01/02/03 physical PostgreSQL migration/transaction implementation remains unimplemented;
- DUR-04 physical content format/compiler/loader/WIT-host implementation and accepted resource values remain unimplemented;
- the permanent World Project/World Bundle encoding still requires the mandated DUR-04 evidence spike and later owner format decision;
- required Stage-C/resource numeric ceilings must be registered before affected executable acceptance; missing values are blockers, not infinity;
- `QA-E2E-01` executable Tier 1/2/3 evidence remains required for terminal vertical-slice proof;
- `PERF-01` and `OPS-CHANNEL-01` own measured numeric production capacity/orchestration decisions;
- `PROD-ENTITLEMENTS-01` blocks only entitlement implementation/activation, not unrelated first vertical-slice work.

## 7. Implementation boundary after Stage-C acceptance

A later explicitly authorized implementation programme may work only on bounded layers whose semantic architecture is accepted and whose task does not cross an unresolved owner/evidence/resource gate. Examples include:

- workspace/build/test/governance maintenance;
- GameNode/bootstrap/runtime-owner scaffolding under FND-03;
- `protocol-oteryn` transport/codec/admission plumbing under FND-02/FND-04 using accepted registries and resource limits;
- PostgreSQL migration/persistence substrate under DUR-01/DUR-02 where physical design is covered by an authorized implementation task and tests;
- Character/session persistence where exact accepted Character/DUR/FND contracts suffice;
- QA-E2E harness/platform implementation that does not fake product-boundary proof;
- client composition/capability/projection scaffolding that remains fail-closed until compatible native gameplay seams exist;
- Stage-C movement/combat/content implementation only under a released coordinator allocation that consumes the accepted Stage-C contracts and preserves their remaining evidence/resource/foreign-owner gates.

No executor may silently fill an unresolved mechanic, authority, numeric limit, Reference fact or permanent content-format decision from implementation convenience.

## 8. Current next action

The Stage-C architecture decision package is complete. The next repository action before implementation workers are released is the separately owned final implementation-executor handoff on PR #314:

1. reconcile #314 against the exact post-Stage-C-closeout `main`;
2. update stale prerequisite wording without changing accepted architecture;
3. re-evaluate any prompt-content delta required by that reconciliation;
4. pass exact-head governance/merge gates and mandatory self-review;
5. release only the coordinator/DAG if every declared release condition remains satisfied.

`PROD-ENTITLEMENTS-01` remains excluded unless separately accepted.

## 9. Executor prompt state

```text
EXECUTOR_PROMPTS: HOLD
```

Reason: Stage-C architecture is accepted/lifecycle-closed, but the final implementation executor package on PR #314 has not yet been reconciled, validated, merged and released against the exact post-closeout `main`. No implementation worker may infer authority directly from architecture acceptance.

## 10. Runtime / production authority

Nothing in this status overlay authorizes production deployment, protected-environment approval, live data/session/account mutation, PostgreSQL migration execution, Platform writes, broad content import, entitlement activation or production traffic.

Implementation requires a separate explicit owner-authorized executor task/allocation under the released implementation coordinator programme.

`IMPLEMENTATION_AUTHORITY: NONE`
