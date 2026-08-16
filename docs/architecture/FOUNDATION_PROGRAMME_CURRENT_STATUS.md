# Oteryn v2 Foundation Programme — Current Status

- Status: **Canonical current execution-status overlay**
- Date: 2026-08-16
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

The native foundation, durability/content/determinism architecture and the complete first A-F gameplay/client/analytics architecture wave are now owner-accepted where listed below.

The most recent owner decisions are:

- `GAME-ABILITY-01` owner acceptance: PR #306 / merge `e2bb284f56f39d8fa01a843d098bcb21d17d77ac`; lifecycle closeout #307 / `dfc75d1332f710d6ac85009653579f7bc51ccc59`;
- remaining first-wave owner acceptance is being delivered by PR #309 through `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md`.

The owner explicitly accepted:

```text
GAME-INTERACTION-01
ALPHA-CLIENT-01
GAME-AI-01
ANL-02
ANL-03
```

All remain `ImplementationStatus=NOT_STARTED`.

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
| `GAME-INTERACTION-01` | `ACCEPTED` | `IN_REVIEW` on #309 | `NOT_STARTED` | owner accepted successor child-identity/retry/reconciliation architecture; target lifecycle closes after #309 delivery/closeout |
| `ALPHA-CLIENT-01` | `ACCEPTED` | `IN_REVIEW` on #309 | `NOT_STARTED` | owner accepted native-client composition/authority/projection/release/test architecture |
| `GAME-AI-01` | `ACCEPTED` | `IN_REVIEW` on #309 | `NOT_STARTED` | owner accepted bounded deterministic AI/spawn/path proposal architecture |
| `ANL-02` | `ACCEPTED` | `IN_REVIEW` on #309 | `NOT_STARTED` | owner accepted read-only gameplay/balance/world analytics architecture |
| `ANL-03` | `ACCEPTED` | `IN_REVIEW` on #309 | `NOT_STARTED` | owner accepted read-only economy/integrity/security analytics architecture |
| `QA-E2E-01` | `ACCEPTED` | `LIFECYCLE_CLOSED` | `NOT_STARTED / EVIDENCE_REQUIRED` | three-tier E2E architecture accepted; executable evidence still blocks terminal VSL claims |
| `VSL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | vertical-slice programme remains a proof programme, not an implementation claim |
| `VSL-MOVE-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | **architecture contract still required before movement/collision/visibility executor** |
| `VSL-COMBAT-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | **minimal combat/death/loot slice contract still required before combat executor** |
| `VSL-CONTENT-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | **minimal native map/compiler/loader slice contract still required before content executor** |
| `PERF-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | measured capacity/resource values required before capacity/performance claims |
| `OPS-CHANNEL-01` | `PLANNED` | `PLANNED` | `NOT_STARTED` | required before automatic production channel scaling/recovery claims |
| `PROD-ENTITLEMENTS-01` | `PROPOSED` | `PLANNED` | `NOT_STARTED` | Platform producer prerequisite satisfied; Oteryn-v2 consumer/enforcement contract remains unaccepted; Premium/VIP executor/activation blocked |

During final #309 merge/closeout, the five rows in `IN_REVIEW` transition to `LIFECYCLE_CLOSED` without changing `ImplementationStatus`.

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

## 5. Accepted first-wave boundaries now binding

### GAME-ABILITY-01

One typed server-authoritative semantic ability/effect pipeline, revision-bound occurrence lineage, owner-scoped commit groups, explicit bounded future work/timer catch-up, deterministic reaction descendants, client non-authority and explicit resource-limit registration before executable acceptance. No formulas/Reference values/runtime APIs are implied.

### GAME-INTERACTION-01

Stable recursive child occurrence identity, deterministic order/RNG, exactly-once/reconciliation semantics, truthful pending/committed/rejected results and no generic distributed transaction. Movement/handoff, writable text, physical FND-02 registration and numeric limits remain downstream owners.

### ALPHA-CLIENT-01

One production client composition root, Platform/Gateway/pre-admission/final-game authority separation, non-authoritative projection/presentation, implementation-backed capabilities, client-safe content, deterministic settings/privacy, atomic release activation and real-boundary Tier 1/2/3 evidence. Native gameplay runtime remains unavailable until separately implemented/proven.

### GAME-AI-01

One local authoritative runtime owner, finite deterministic bounded representation-neutral AI resolution, staged all-or-nothing AI-local commit, bounded proposal pathfinding/scripts, finite spawn retry/provenance and no value authority. Concrete framework/path algorithm/resource maxima/event owner/reward attribution remain downstream.

### ANL-02 / ANL-03

Read-only analytics/integrity/security evidence. Explicit quality/privacy/revision semantics, fail-closed regression evaluation, deterministic invariant evaluation when evidence is complete, immutable human-review lifecycle, referral-after-disposition, non-adverse optional client diagnostics. No gameplay mutation, sanction or enforcement authority.

## 6. Cross-domain implementation blockers that remain real

Architecture acceptance deliberately does not invent these dependencies:

- `VSL-MOVE-01`: authoritative movement/collision/floor/teleport/interest/snapshot-delta slice contract is not yet accepted;
- `VSL-COMBAT-01`: minimal combat/death/corpse/loot/XP/pickup slice integration contract is not yet accepted;
- `VSL-CONTENT-01`: minimum World Project/World Bundle/compiler/loader physical vertical-slice contract is not yet accepted;
- GAME-INTERACTION movement/handoff owner and durable writable-text owner remain unresolved for mechanics that need them;
- GAME-AI event/encounter durable multi-actor owner, concrete resource ceilings and controlled-actor reward attribution remain downstream;
- `GAME_EVENT_FOUNDATION_REGISTRY.json` still requires concrete producer event families before ANL-02/03 can claim real metric/detector coverage;
- FND-02/FND-04/client/server executable protocol/admission path remains unimplemented;
- DUR-01/02/03 physical PostgreSQL migration/transaction implementation remains unimplemented;
- DUR-04 physical content format/compiler/loader/WIT-host implementation and accepted resource values remain unimplemented;
- `PERF-01` and `OPS-CHANNEL-01` own measured numeric production capacity/orchestration decisions;
- `PROD-ENTITLEMENTS-01` blocks only entitlement implementation/activation, not unrelated first vertical-slice work.

## 7. What may be implemented before Stage-C gameplay slice contracts

A later explicitly authorized implementation programme may safely work only on bounded layers whose semantic architecture is already accepted and whose task does not cross an unaccepted owning gate. Examples include:

- workspace/build/test/governance maintenance;
- GameNode/bootstrap/runtime-owner scaffolding under FND-03 without movement/combat/content semantics;
- `protocol-oteryn` transport/codec/admission plumbing under FND-02/FND-04 using accepted registries and resource limits, without claiming complete gameplay slice;
- PostgreSQL migration/persistence substrate under DUR-01/DUR-02 where physical design is itself covered by an authorized implementation task and tests;
- Character/session persistence where exact accepted Character/DUR/FND contracts suffice;
- QA-E2E harness/platform implementation that does not fake product-boundary proof;
- client composition/capability/projection scaffolding that remains fail-closed before native gameplay availability.

No executor may silently fill a missing Stage-C mechanic contract from implementation convenience.

## 8. Current paper-only next action

Before issuing **movement/combat/content implementation executors**, complete one bounded Stage-C architecture package covering:

```text
VSL-MOVE-01
VSL-COMBAT-01
VSL-CONTENT-01
```

The package must consume the now accepted GAME-ABILITY / GAME-INTERACTION / GAME-AI / ALPHA-CLIENT / DUR / SIM / QA-E2E boundaries and must not re-open accepted domain semantics.

After those Stage-C contracts are owner-accepted and lifecycle-closed, perform one executor-readiness/prompt audit. `PROD-ENTITLEMENTS-01` remains excluded unless separately accepted.

## 9. Executor prompt state

```text
EXECUTOR_PROMPTS: HOLD
```

Reason: the first-wave architecture is accepted, but three named `BLOCKS_VERTICAL_SLICE` mechanic/content slice contracts remain unaccepted. Releasing a global implementation prompt set now would invite movement/combat/content agents to make architecture decisions inside code.

## 10. Runtime / production authority

Nothing in this status overlay authorizes production deployment, protected-environment approval, live data/session/account mutation, PostgreSQL migration execution, Platform writes, broad content import, entitlement activation or production traffic.

Implementation requires a separate explicit owner-authorized executor task/PR with exact scope and evidence.

`IMPLEMENTATION_AUTHORITY: NONE`
