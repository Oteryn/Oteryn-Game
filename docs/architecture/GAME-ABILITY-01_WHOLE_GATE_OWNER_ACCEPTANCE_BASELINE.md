# GAME-ABILITY-01 — Whole-Gate Owner Acceptance Baseline

- Status: **OWNER-ACCEPTED WHOLE-GATE ARCHITECTURE**
- DecisionStatus: `ACCEPTED`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-08-16
- Gate: `GAME-ABILITY-01`
- Owner disposition: `ACCEPT`
- Accepted source candidate: `GAME-ABILITY-01_WHOLE_GATE_CONTRACT_CANDIDATE.md`
- Decision-preparation source: `GAME-ABILITY-01_OWNER_DECISION_PACKAGE.md`
- Candidate delivery: PR #268 / merge `0cfd8d8ee3ecf4fbb1cb76cbc9680b53a152e3c1`
- Candidate exact reviewed head: `a65680d9504b3a4e6394ad3bb3dc25c6630cd098`
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**

## Owner decision

The repository owner explicitly selected:

```text
ACCEPT
```

for the bounded `GAME-ABILITY-01` whole-gate owner decision presented in `GAME-ABILITY-01_OWNER_DECISION_PACKAGE.md`.

This later baseline preserves the merged whole-gate candidate unchanged and keeps the decision package as a separate non-normative historical decision record. It supersedes only the unresolved/candidate **decision status** for the candidate's declared whole-gate scope.

The current three-axis status during this acceptance delivery is:

```yaml
GAME-ABILITY-01:
  DecisionStatus: ACCEPTED
  DeliveryStatus: IN_REVIEW
  ImplementationStatus: NOT_STARTED
```

Delivery/lifecycle bookkeeping remains a separate axis and will advance through normal repository merge/closeout governance without changing the accepted semantic scope.

## Accepted scope

The following whole-gate semantic closure is now binding together with all previously owner-accepted GAME-ABILITY partial baselines:

1. one authoritative data-first typed GAME-ABILITY execution pipeline for player, AI, NPC and system origins;
2. revision-bound semantic occurrence/lineage for delayed, repeated and reactive work;
3. owner-scoped commit groups with explicit ordered sub-occurrences for intentional partial/sequential behavior and no invented distributed transaction;
4. one bounded future-occurrence model for channel pulses, condition ticks, delayed hits, recharge and other future mutating work;
5. explicit FND-03-compatible repeated-timer catch-up/coalescing semantics, including hard-bounded/fair `RUN_EACH_BOUNDED` and the non-semantic-only restriction on `SKIP_TO_LATEST`;
6. explicit continuation semantics for future-authoritative cast/channel/cooldown/charge/condition state at every exercised lifecycle boundary, without implicit persistence or implicit removal;
7. typed deterministic pre-commit contributions and explicit bounded post-commit reaction/proc descendants with stable lineage, deterministic ordering and cycle/work bounds;
8. mandatory implementation resource-limit registration for all content/external-controlled work dimensions, while exact numeric ceilings remain measurement/evidence driven;
9. strict client presentation/prediction non-authority with reconciliation to authoritative server results/revisions;
10. strict separation of architecture acceptance from implementation evidence, Reference evidence/provenance and parity confirmation.

## Binding upstream invariants

This acceptance does not supersede and must be read together with the accepted GAME-ABILITY partial baselines for:

- typed Effect Plan / authoritative validation and commit;
- deterministic authoritative Target Resolver plus separate Legality Evaluation;
- explicit cast/channel lifecycle and named commit anchors;
- cooldown/charge/condition typed state and `ConditionDefinition != ConditionInstance`;
- staged deterministic damage/heal composition and SIM-owned RNG purpose semantics;
- small typed effect-family vocabulary, owner-domain routing and Reference catalogue separation.

It also preserves the accepted FND-03/FND-04, SIM-DETERMINISM, DUR-02/DUR-03/DUR-04, GAME-CHANNEL, GAME-CHAR, GAME-ITEM and ANL authority boundaries.

## Explicit non-authority consequences

Architecture acceptance does **not** authorize or prove:

```text
runtime implementation
Reference mechanic parity
Alpha gameplay completeness
foreign-domain API acceptance
PostgreSQL DDL/migrations
Platform mutation
production rollout
```

A separately authorized implementation task is required before executable GAME-ABILITY work.

## Reference evidence remains fail closed

The canonical Agent-A result is unchanged:

```text
registered ABILITY_COMBAT cases: 4
promoted cases: 0/4
target evidence: UNKNOWN
source/case provenance: PENDING
legal review: PENDING
Oteryn implementation: NOT_STARTED
parity: PARITY_PENDING_EVIDENCE
```

This acceptance does not promote, reinterpret or shadow those classifications.

Exact Reference target geometry, timing, values, formulas, RNG probabilities/order, cooldown/condition values and other Reference-sensitive behavior remain evidence-owned and fail closed where unresolved.

## Cross-domain blockers remain in force

Acceptance of the GAME-ABILITY semantic envelope does not accept sibling whole gates or foreign APIs.

- item/currency/value consequences remain under GAME-ITEM / DUR-03 conservation and idempotency;
- movement/teleport/push/pull/occupancy/world-object consequences remain blocked on accepted owner-domain integration where required;
- AI selection/threat/spawn/path semantics remain GAME-AI-owned;
- client result/error/prediction/reconciliation details remain FND-02 / ALPHA-CLIENT owned;
- surviving state representation/recovery remains FND/DUR/Channel/Character owned;
- analytics remains observational/read-only;
- missing required resource ceilings block implementation acceptance.

No shared Effect Plan creates foreign mutation authority or distributed atomicity.

## Decision timing

**Must decide now: YES** for the accepted whole-gate seam semantics above, because broad implementation/content would otherwise create incompatible timer, retry/revision, reaction, continuation, partial-commit, resource-exhaustion and client-trust behavior.

The following remain deliberately deferred because they are evidence-, measurement- or owner-domain-dependent:

- exact mechanic values/formulas/RNG facts;
- concrete timer policy for an unresolved mechanic and numeric ceilings;
- exact continuation/survival policy for concrete mechanics;
- physical content/catalogue/Rust/scheduler representations;
- persistence schema/DDL/checkpoint representation;
- foreign-domain APIs and cross-domain workflow contracts;
- protocol message fields/error encoding and client UI/prediction algorithms;
- ANL event schemas, fixture runner, runtime crate/service decomposition and production SLOs.

## Supersession criteria

Reopen this accepted architecture only with concrete evidence such as:

- representative Reference/Evolved mechanics cannot be represented safely without pathological complexity or primitive proliferation;
- deterministic replay/retry evidence disproves the occurrence/revision/catch-up model;
- accepted owner workflows cannot express required cross-domain atomicity;
- measured performance remains unacceptable after semantics-preserving optimization;
- Studio/content-production evidence demonstrates unacceptable authoring complexity at scale;
- security/abuse findings show the reaction/capability/resource model is insufficient;
- a later accepted FND/SIM/DUR/domain contract materially changes an authority boundary consumed here.

Any supersession must explicitly preserve or replace server authority, deterministic ordering/revision binding, bounded work, explicit timer semantics, domain ownership, no-hidden-rollback and fail-closed evidence properties.

## Current-status precedence

Until the coordinator-owned global status/register surfaces are reconciled after this acceptance delivery, this later owner-acceptance baseline is the authoritative source for the `GAME-ABILITY-01` **DecisionStatus** and supersedes older `CANDIDATE` wording for that axis only.

It does not rewrite historical candidate delivery status and does not change `ImplementationStatus=NOT_STARTED`.

`IMPLEMENTATION_AUTHORITY: NONE`
