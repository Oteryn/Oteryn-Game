# Oteryn v2 — Remaining First-Wave Owner Acceptance Baseline

- Status: `OWNER-ACCEPTED FIRST-WAVE ARCHITECTURE`
- Date: 2026-08-16
- Coordination issue: #308
- Delivery PR: #309
- Owner disposition: `ACCEPT ALL FOUR DECISION ROWS`
- Serial canonicalization order: `GAME-INTERACTION-01 -> ALPHA-CLIENT-01 -> GAME-AI-01 -> ANL-02/ANL-03`
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**
- Implementation authority: **NONE**

## 1. Owner disposition

On 2026-08-16 the repository owner explicitly accepted all four decisions prepared in `OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_DECISION_PACKAGE_20260816.md`:

```yaml
GAME-INTERACTION-01:
  DecisionStatus: ACCEPTED
  ImplementationStatus: NOT_STARTED

ALPHA-CLIENT-01:
  DecisionStatus: ACCEPTED
  ImplementationStatus: NOT_STARTED

GAME-AI-01:
  DecisionStatus: ACCEPTED
  ImplementationStatus: NOT_STARTED

ANL-02:
  DecisionStatus: ACCEPTED
  ImplementationStatus: NOT_STARTED

ANL-03:
  DecisionStatus: ACCEPTED
  ImplementationStatus: NOT_STARTED
```

This baseline is a later owner-acceptance record. It does not rewrite the historical merged proposal/candidate artifacts or their review history.

## 2. GAME-INTERACTION-01 accepted scope

The owner accepts the merged successor semantic scope delivered by PR #277 / merge `c8d8ae20471acf004db7bbf6015a2d1b710aa8af` and final reviewed head `2a13f789bb988a7e8eeca1c387173960708d506a`.

Binding architecture includes:

- stable recursive source-derived child-occurrence identity;
- canonical deterministic child ordering and retry-stable RNG identity;
- exactly-once/at-most-once authoritative child commit semantics;
- explicit `PENDING | COMMITTED | REJECTED` outcome truth;
- same-occurrence reconciliation for ambiguous foreign operations;
- timeout/cancellation/stale-completion behavior that never guesses rollback;
- `PENDING` forbids blind duplicate fresh semantic attempts;
- named coupled workflows rather than a generic distributed transaction;
- foreign owner authority remains foreign-owned.

Still deliberately unresolved or implementation-blocking where applicable:

- movement/relocation/handoff owner and operation contract;
- durable writable-text owner;
- concrete FND-02 gameplay payload/registry representation;
- numeric cascade/retry/resource ceilings;
- physical IDs/digests/storage/schema/Rust types;
- runtime/client/server implementation and production proof.

GAME-ABILITY is now an accepted owning effect/combat boundary consumed by GAME-INTERACTION; this acceptance does not transfer GAME-ABILITY mutation authority to GAME-INTERACTION.

## 3. ALPHA-CLIENT-01 accepted scope

The owner accepts the merged native-client architecture scope delivered by PR #273 / merge `b7f239a32081fc43f5d3306517eadde850b5be6b` and final reviewed head `fe85600806979812f63dfb7b2c2a7e85cfecc943`.

Binding architecture includes:

- `apps/client` as the sole production client composition root unless a later accepted process/security boundary supersedes it;
- application/screens depending on semantic ports/view state instead of directly owning infrastructure;
- strict Platform Identity -> one-time Game Login Ticket -> Platform Game Gateway -> selected route/pre-admission -> FND-02 transport -> final game-owned FND-04 admission chain;
- non-authoritative client gameplay projection and presentation/scene/audio non-authority;
- runtime capability evidence distinct from architecture target vocabulary;
- semantic input before protocol egress and bounded validated ingress/reconciliation;
- client-safe content/release compatibility and one-way revision-safe activation;
- low-level client/Studio sharing without composition/session/UI coupling;
- deterministic settings scope/precedence and privacy fail-closed behavior;
- release activation outside the active authority-bearing gameplay process;
- ADR-0007 Tier 1/2/3 evidence separation plus FND-02 independent wire proof.

Still deliberately unresolved or implementation-blocking where applicable:

- concrete UI/scene/render/network/audio/updater/installer libraries;
- gameplay transport adapter/listener/runtime availability;
- FND-02 codecs/registry/reconciliation implementation;
- FND-04 runtime counterpart and authoritative server endpoint;
- final client-safe content projection implementation;
- Tier 1/2/3 executable evidence;
- exact numerical cache/queue/audio/scene/resource ceilings.

Architecture acceptance does not make the native gameplay client available.

## 4. GAME-AI-01 accepted scope

The owner accepts the merged successor semantic scope delivered by PR #276 / merge `f1bd64a62b9392223589e6b0609149570f5a76b5` and final reviewed head `200267c946e0c78b15ce0d17c82454622d036abc`.

Binding architecture includes:

- ChannelRuntime/InstanceRuntime as the only local AI/spawn mutation authority;
- representation-neutral finite deterministic bounded semantic execution;
- staged/preflighted all-or-nothing AI-local commit or complete rejection;
- bounded auxiliary pathfinding/proposal work with current-owner revalidation;
- deterministic target/perception/threat selection and stable tie-break semantics;
- proposal-only bounded DUR-04 component/script participation;
- finite spawn occupancy retry count/window/cadence/order plus hard maximum;
- stable occurrence provenance/recovery and GAME-CHANNEL source multiplicity classification;
- controlled-actor principal/control provenance;
- GAME-AI never mints/settles item/currency/XP/reward value;
- GAME-ABILITY owns action/effect legality/commit while GAME-AI proposes typed intent;
- GAME-INTERACTION owns environmental interaction/reconciliation semantics at its boundary;
- Reference `UNKNOWN/CONFLICT/PENDING` remains fail closed.

Still deliberately unresolved or implementation-blocking where applicable:

- concrete FSM/behavior-tree/statechart/framework/library choice;
- concrete pathfinding algorithm/library;
- hard numeric resource ceilings and boundary tests;
- event/encounter durable multi-actor owner contract;
- controlled-actor reward/contribution attribution/dedup owner semantics;
- concrete runtime/content schema and executable APIs;
- exact Reference AI/path/spawn values and behaviors.

No AI runtime/content activation follows from acceptance.

## 5. ANL-02 accepted scope

The owner accepts the merged gameplay/balance/world analytics semantic scope delivered by PR #270 / merge `32ff2ae75530cb9334463833462eb02c44dc435b` and final reviewed head `e78cb7ff5151876643206324cf7e6d2ca8cde8da`.

Binding architecture includes:

- versioned metric-definition identity and revision-aware observations;
- explicit cohorts, denominators, semantic revision context and evidence-quality vector;
- deterministic producer-owned session/hunt grouping rather than analyst timeout guesses;
- canonical UTC half-open reporting windows based on accepted event occurrence time;
- fail-closed dashboard truth, warnings and suppression semantics;
- fail-closed regression disposition: attempted evaluation with any unsatisfied applicable quality/sample/comparability/reconciliation/privacy/provenance prerequisite yields `REGRESSION_EVIDENCE_INSUFFICIENT`;
- privacy-safe projections and bounded evidence packages;
- analytics remains read-only and cannot mutate gameplay/balance/content/release state.

Still deliberately unresolved or implementation-blocking where applicable:

- concrete producer event IDs/payload schemas/grouping identifiers;
- metric thresholds/statistical methods/minimum sample values;
- warehouse/broker/database/dashboard technology;
- physical schemas/topology;
- numeric query/backfill/cardinality/resource ceilings;
- runtime producer/collector/analytics implementation.

## 6. ANL-03 accepted scope

The owner accepts the merged economy/integrity/security analytics semantic scope delivered by the same PR #270 / merge `32ff2ae75530cb9334463833462eb02c44dc435b`.

Binding architecture includes:

- read-only non-authority invariant for detection/investigation;
- deterministic invariant catalogue over accepted durable evidence when completeness prerequisites are met;
- versioned detector/signal evidence and immutable signal/case lifecycle history;
- no-case triage must still produce an immutable substantive disposition;
- referral is routing only and must follow a substantive evidentiary disposition;
- statistical security anomalies remain non-sanctioning evidence and do not become deterministic proof by score or human belief alone;
- optional client diagnostics absence/opt-out is non-adverse and cannot increase suspicion/risk/enforcement priority;
- DUR-03 remains authoritative prevention/conservation owner;
- enforcement, sanctions, rollback/confiscation/value repair and database/gameplay mutation remain foreign authority.

Still deliberately unresolved or implementation-blocking where applicable:

- concrete DUR/FND/gameplay producer event registrations;
- detector algorithms/models/thresholds/frameworks;
- case-management UI and physical storage/topology;
- exact retention/resource ceilings;
- enforcement/GM/account-remediation contract and production implementation.

## 7. Reference evidence state remains unchanged

First Reference evidence/parity state is not promoted by these acceptances:

```yaml
registered_ABILITY_COMBAT_cases: 4
promoted_cases: 0
TargetEvidence: UNKNOWN
SourceCaseProvenance: PENDING
LegalReview: PENDING
OterynImplementation: NOT_STARTED
Parity: PARITY_PENDING_EVIDENCE
```

Architecture acceptance never implies `PARITY_CONFIRMED`.

## 8. PROD-ENTITLEMENTS-01 remains separate

This owner disposition does not accept `PROD-ENTITLEMENTS-01`.

The Platform producer-side prerequisite is satisfied and pinned, but Oteryn-v2 consumer/enforcement architecture remains not accepted. Any Premium/VIP or other game-consumed entitlement executor/activation remains blocked until that gate receives its own accepted consumer contract, security negative-path proof and cross-repository rollout/rollback evidence.

Unrelated foundation/admission/movement/combat vertical-slice work is not blocked by the deferred entitlement gate.

## 9. Decision timing and supersession

All accepted clauses above were classified as `Must decide now? YES` for the named semantic ownership/safety boundary or as part of the paired analytics read-only contract required before downstream implementation design can safely bind data semantics.

Concrete libraries, physical schemas, algorithms and numeric limits explicitly left unresolved remain deferred until implementation/PERF/OPS/security/product evidence requires them.

Supersession requires a later explicit accepted ADR/contract naming the changed clause and preserving the unaffected owner boundaries. Framework preference or implementation convenience alone is insufficient.

## 10. Delivery and implementation status

During PR #309 review:

```text
DecisionStatus: ACCEPTED
DeliveryStatus: IN_REVIEW
ImplementationStatus: NOT_STARTED
```

After lawful merge and lifecycle closeout, target state for every accepted row is:

```text
DecisionStatus: ACCEPTED
DeliveryStatus: LIFECYCLE_CLOSED
ImplementationStatus: NOT_STARTED
```

`IMPLEMENTATION_AUTHORITY: NONE`
