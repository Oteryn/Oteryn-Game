# GAME-ABILITY-01 — Whole-Gate Owner Decision Package

- Package status: `DECISION_RESOLVED_BY_LATER_OWNER_ACCEPTANCE`
- Prepared: 2026-08-16
- Trusted base: `main@d2af53855046df25b4e52edbd5ec14e0513a63ec`
- Decision target: merged `GAME-ABILITY-01_WHOLE_GATE_CONTRACT_CANDIDATE.md`
- Historical gate state at package preparation: `CANDIDATE / LIFECYCLE_CLOSED / NOT_STARTED`
- Coordinator recommendation: `ACCEPT`
- Owner disposition: `ACCEPT` — recorded later in `GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md`
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**

This file is the durable pre-decision package. It is intentionally not rewritten into the normative acceptance contract. The later owner-acceptance baseline supersedes only the unresolved decision-status boundary and preserves this package as historical evidence.

## 1. Decision summary

The owner-accepted GAME-ABILITY partial baselines already define the core execution model. The whole-gate candidate closes the remaining seams so delayed work, repeated timers, reactions/procs, partial commits, continuation, resource exhaustion and client prediction cannot acquire incompatible implementation-local semantics.

At preparation time the verified facts were:

- `PROVEN` — the merged whole-gate candidate was `CANDIDATE / LIFECYCLE_CLOSED / NOT_STARTED`.
- `PROVEN` — accepted partial baselines bind one typed Effect Plan pipeline, authoritative targeting/legality, explicit cast/channel/commit anchors, typed cooldown/charge/condition state, staged deterministic damage/heal composition and typed effect-family/domain-routing boundaries.
- `PROVEN` — Agent A promoted `0/4` registered `ABILITY_COMBAT` cases; target evidence remained `UNKNOWN`, provenance/legal `PENDING`, implementation `NOT_STARTED`, parity `PARITY_PENDING_EVIDENCE`.
- `PROVEN` — historical material review findings on candidate PR #268 were repaired before final head `a65680d9504b3a4e6394ad3bb3dc25c6630cd098`; exact-head self-review and required repository gates were clean and the final owner-authorized independent review found no major issue.
- `PROVEN` — GAME-AI, GAME-INTERACTION and ALPHA-CLIENT packages were merged/lifecycle-closed but remained unaccepted whole gates.
- `DERIVED` — no material semantic contradiction with the GAME-ABILITY candidate was present on the trusted base.
- `CONFLICT` — no material accepted-source conflict was found.
- `UNKNOWN` — exact Reference values/formulas/timing, provenance clearance, numeric resource ceilings, concrete survival policies and executable foreign-domain APIs remained unresolved by design.

## 2. Accepted semantic closure proposed by the package

The package recommended acceptance of these whole-gate clauses together with all existing GAME-ABILITY partial baselines:

1. one authoritative typed execution pipeline for player, AI, NPC and system origins;
2. revision-bound semantic occurrence/lineage for delayed, repeated and reactive work;
3. owner-scoped commit groups with explicit ordered sub-occurrences for intentional partial/sequential behavior and no invented distributed transaction;
4. one bounded future-occurrence model for channel pulses, condition ticks, delayed hits, recharges and other future mutating work;
5. explicit FND-03-compatible repeated-timer catch-up semantics, including hard-bounded/fair `RUN_EACH_BOUNDED` and the non-semantic-only restriction on `SKIP_TO_LATEST`;
6. explicit continuation semantics for future-authoritative cast/channel/cooldown/charge/condition state at every exercised lifecycle boundary, without implicit persistence or removal;
7. typed deterministic pre-commit contributions and bounded post-commit reaction/proc descendants with stable lineage, deterministic ordering and cycle/work bounds;
8. mandatory implementation resource-limit registration while exact numeric ceilings remain measurement/evidence driven;
9. strict client prediction/presentation non-authority with reconciliation to authoritative server results;
10. strict separation of architecture acceptance from implementation evidence and Reference evidence/parity.

## 3. Binding upstream invariants

Acceptance was conditioned on preserving:

- server-authoritative gameplay and client-intent-only trust;
- native Rust / `protocol-oteryn` target architecture;
- FND-03 owner/order/generation, repeated-timer and asynchronous-work boundaries;
- FND-04 fencing/recovery authority;
- SIM-DETERMINISM arithmetic, RNG identity/purpose, normalized time/order/revision/replay rules;
- GAME-ITEM/DUR-03 conservation, idempotency and owner authority;
- DUR-04 bounded proposal-only content/Wasm capability model;
- GAME-AI and GAME-INTERACTION foreign-domain ownership;
- ANL observational/read-only authority;
- fail-closed Reference evidence and separate implementation/parity axes.

## 4. Cross-domain blockers preserved

The package classified these as implementation/integration blockers rather than reasons to reject the GAME-ABILITY architecture:

| Dependency | What remains blocked until its owning evidence/contract exists |
|---|---|
| FND-03 / DUR-02 / recovery owners | claiming future ability state survives recovery/restart/handoff |
| GAME-ITEM / DUR-03 | item/currency/value-consuming or producing mechanics outside owner invariants |
| GAME-INTERACTION / world owner | executable teleport/push/pull/occupancy/world-object consequences needing its legality/commit surface |
| GAME-AI | authoritative AI selection/threat/spawn/path semantics |
| SIM / Reference evidence | exact formulas/RNG facts and Reference parity |
| FND-02 / ALPHA-CLIENT | exact result/error/prediction/reconciliation protocol/UI integration |
| Reference evidence/provenance | any current `ABILITY_COMBAT` promotion or aggregate parity |
| ANL / producer registry | complete concrete ability telemetry/event-family coverage |
| resource-limit registry | implementation acceptance for any exercised dimension lacking tested hard ceilings |

## 5. Security, determinism and exploitability review

- `PROVEN` — client, scripts, analytics and catalogue records have no fallback mutation authority.
- `DERIVED` — revision binding and SIM-owned RNG purpose avoid retry rerolls, latest-definition reinterpretation and hash/thread/registration-order combat differences.
- `PROVEN` — future/reaction work is bounded; budget exhaustion cannot silently erase committed history or bypass limits through recursive scheduling/direct scripts.
- `DERIVED` — these rules address proc storms, catch-up storms, unbounded target enumeration and recursive reaction amplification.
- `UNKNOWN` — exact numeric ceilings remain implementation evidence and therefore block implementation until measured and registered.

## 6. Player, producer and operational impact

Player-facing benefits derived from acceptance include deterministic/fair ordering, no scheduler-convenience loss of required periodic gameplay effects, no silent rollback of committed outcomes, server authority over client prediction and fail-closed unresolved Reference behavior.

Producer benefits include one compositional execution model, explicit timing/continuation/reaction/resource policies, safer Studio/content validation and fewer bespoke mutation callbacks. The cost is more explicit semantic metadata and owner integrations for unusual mechanics.

Operationally, explicit occurrence/reaction lineage and deterministic failure disposition improve postmortem, exploit and balance analysis while bounded work provides a basis for overload/capacity tests.

## 7. Mandatory decision-timing test

**Must decide now: YES** for the whole-gate seam semantics above.

Broad GAME-ABILITY implementation, representative executable combat/condition/timer/reaction fixtures, resource-limit strategy, client reconciliation consumption and broad content authoring cannot safely assume one coherent whole-gate model until these seams are binding.

Deferring them until content/runtime exists risks incompatible timer catch-up, retry/revision behavior, hidden partial commits, recursive reaction authority, implicit continuation and inconsistent resource/client-authority behavior.

Supersession evidence includes representative mechanics that cannot be expressed safely, replay/retry failures, cross-domain workflow impossibility, measured performance failure after semantic-preserving optimization, Studio authoring evidence, security findings or later accepted FND/SIM/DUR/domain boundary changes.

## 8. DECISIONS_NOT_TAKEN

The package deliberately did not decide or authorize:

- actual Reference mechanic facts, provenance/legal clearance or parity promotion;
- exhaustive ability/effect/condition/proc catalogues;
- exact target grammar/geometry/range/LoS/PvP/PZ/immunity/error precedence;
- exact cast/channel timing, interruption, costs/refunds, cooldowns/charges, condition values/ticks/stacking/dispel precedence;
- exact formulas, arithmetic values, RNG probabilities/order or combat roll semantics;
- concrete timer policy for unresolved mechanics or numeric timer/resource ceilings;
- concrete continuation/survival policy for specific mechanics;
- physical content/catalogue/Rust/scheduler representations;
- Wasmtime/WIT implementation beyond accepted DUR-04 capability semantics;
- SQL DDL/migrations/checkpoint representation or cross-domain transaction protocol;
- GAME-AI, GAME-INTERACTION, item/value, entity/world or Character foreign-domain APIs;
- FND-02 wire fields/error encoding or ALPHA client UI/prediction design;
- ANL event schema/retention, fixture runner, runtime crate/service decomposition;
- runtime/client/server/content implementation, deployment, SLOs, LiveOps or production authority;
- acceptance of sibling whole gates.

## 9. Owner disposition

The later owner decision is now durable:

```text
OWNER DECISION: ACCEPT
```

Normative acceptance is recorded in `GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md`.

That acceptance means:

```text
GAME-ABILITY semantic architecture accepted for the declared scope
!= runtime implemented
!= Alpha gameplay complete
!= Reference parity confirmed
!= foreign-domain contracts accepted
!= production authorized
```
