# GAME-ABILITY-01 — Whole-Gate Gap Analysis

- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-08-15
- Gate: `GAME-ABILITY-01`
- Worker issue: #260
- Worker branch: `docs/arch-b-game-ability-gap`
- Trusted worker base: `main@088b46638ac014cd7928d6b0b75cee44902fe22c`
- Canonical dependency re-read: `main@dc1eecae7952902bee3fb1e2d88aefc2be792cae`
- Runtime/client/protocol/content/DDL/Platform/production authority: **NONE**
- Merge authority: **ARCHITECTURE_COORDINATOR_ONLY**

## Executive conclusion

`GAME-ABILITY-01` does not need another redesign of targeting, cast timing, cooldowns, conditions, damage/healing or effect-family ownership. Those subjects already have binding partial baselines. The remaining whole-gate architecture gap is narrower: the partial contracts need one explicit closure envelope for **future occurrences**, **repeated-timer catch-up semantics**, **reactive/proc descendants**, **commit-group boundaries**, **continuation declaration**, **resource-bound obligations**, **client-authority separation** and **implementation/parity evidence readiness**.

The analysis therefore supports a bounded whole-gate contract candidate. That candidate can make the architecture coherent without deciding exact Reference spell values, target geometry, formula arithmetic, cooldown durations, condition values, persistence schema, protocol messages or executable content.

Architecture acceptance and mechanic parity remain separate:

```text
whole GAME-ABILITY architecture accepted
!= any concrete Reference mechanic parity confirmed
!= executable runtime implemented
!= Alpha content complete
```

The canonical Agent-A continuity package merged as PR #271 is consumed exactly: **0/4 registered `ABILITY_COMBAT` cases are promoted**; target evidence remains `UNKNOWN`; source/case provenance and legal review remain `PENDING`; implementation remains `NOT_STARTED`; parity remains fail closed. This worker does not promote, rewrite or own those classifications.

## Problem

The current architecture is strong locally but still leaves several composition questions at the seams between accepted partial baselines:

1. channel pulses and condition ticks are bounded, but there is no single whole-gate statement covering delayed/periodic/future mutating occurrences generally;
2. FND-03 requires each behavior-affecting repeated timer family — including periodic combat/damage/healing — to select an explicit catch-up policy, but the whole-gate candidate did not yet bind that obligation, leaving missed ticks vulnerable to implementation-defined replay/coalescing/skipping;
3. damage/heal reactions are bounded, but the gate still needs a general deterministic proc/trigger lineage and loop-prevention rule;
4. an `Effect Plan` has an authoritative commit concept, while cross-domain effects deliberately do not create a global transaction; the whole gate needs to say how partial/ordered consequences are represented without hidden partial mutation;
5. cooldown/condition baseline deliberately defers survival across logout/recovery/migration; the gate needs a declaration boundary so implementation cannot silently choose persistence behavior;
6. multiple partial baselines require bounded work, but GAME-ABILITY-specific resource dimensions and failure obligations have not yet been collected into one implementation-readiness rule;
7. client cast bars/prediction remain non-authoritative, but the whole gate needs one explicit server/client authority statement so presentation work cannot become a second legality/commit engine;
8. Reference catalogue/manifest work and executable fixtures need a clear prerequisite ladder so architecture acceptance is not conflated with evidence or runtime parity.

Leaving these seams entirely to implementation would allow incompatible timer, catch-up, proc, partial-failure, client-prediction and resource-exhaustion semantics to emerge while still claiming conformance to each individual partial baseline.

## Constraints

The closure must preserve, not reopen:

- one data-first Ability Definition model with typed bounded Effect Plans and proposal-only DUR-04 Wasm/WIT extensions;
- one authoritative Target Resolver, deterministic resolved target sets, separate legality evaluation and no hidden retargeting during effect application;
- explicit ability lifecycle, logical `PRIMARY COMMIT`, named ancillary commitment anchors, explicit reservations/compensation and bounded channel occurrences;
- typed cooldown/charge state and `ConditionDefinition != ConditionInstance`, deterministic stack/conflict transitions and condition ticks re-entering the authoritative pipeline;
- staged deterministic damage/heal composition, separate damage/heal semantics, SIM-owned RNG identity and bounded reactive descendants;
- a small typed effect-family vocabulary, domain-owned cross-domain transitions, no generic state patch and Reference catalogue metadata separate from executable content/effect vocabulary;
- FND-03 one-writer/order/generation authority **plus explicit repeated-timer catch-up policy/backlog/fairness semantics, including the non-semantic-only `SKIP_TO_LATEST` restriction**, and FND-04 fencing;
- accepted SIM-DETERMINISM arithmetic/RNG/order/replay/revision semantics;
- accepted DUR-04 deterministic content and capability-bounded proposal-only scripting;
- GAME-ITEM/DUR-03 conservation and idempotency for item/currency/value consequences;
- ANL-01 observational/audit authority only;
- the accepted immutable first Reference target and fail-closed evidence/parity manifest semantics;
- no new protocol family, global mutation owner, global gameplay order, process-global timer/RNG/condition registry or hidden distributed transaction.

## Evidence basis reviewed

The worker consumed the following repository truth as read-only input:

- `GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_REFERENCE_MECHANIC_CATALOGUE_ENTRY_PARITY_FIXTURE_CONTRACT.md` together with later accepted Reference-manifest/current programme state;
- canonical Agent-A continuity evidence merged by PR #271 on `main@dc1eecae7952902bee3fb1e2d88aefc2be792cae`;
- `FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md`, especially the accepted repeated-timer catch-up policy section;
- accepted FND-04, GAME-CHANNEL-01, GAME-CHAR-01, GAME-ITEM-01, DUR-02/DUR-03/DUR-04, SIM-DETERMINISM-01 and ANL-01 constraints as applicable;
- `RESOURCE_LIMITS_REGISTRY.json` as the repository mechanism for hard implementation limits;
- `MULTI_AGENT_ARCHITECTURE_ORCHESTRATION.md` for required structured cross-domain findings and canonical status axes;
- `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md` for canonical status/ownership of still-unaccepted GAME-AI-01, GAME-INTERACTION-01 and ALPHA-CLIENT-01 gates;
- exact blocked sibling PR heads only as explicitly **noncanonical proposal evidence**, never as accepted repository architecture;
- coordinator-owned horizon/register/current-status files as read-only context.

No external source is used to promote Reference behavior or claim runtime implementation.

## Reconciliation of accepted partial baselines

| Partial baseline | Binding result consumed | Whole-gate implication | Reopened? |
|---|---|---|---:|
| Typed effect pipeline | definitions are versioned data; invocation produces bounded typed proposals; only authoritative owner commits | every closure rule re-enters the same plan/validation/commit authority | NO |
| Targeting and legality | target discovery is deterministic, authoritative and separate from legality; no hidden effect-stage retarget | future/reaction occurrences either reuse an explicitly retained target policy or invoke the same bounded resolver at a declared anchor | NO |
| Cast/channel/commit | explicit lifecycle, logical primary commit, named ancillary anchors, no hidden rollback, channel pulses are bounded occurrences | delayed/periodic work uses the same occurrence/commit semantics; committed parents are never erased by child failure | NO |
| FND-03 repeated timers | each behavior-affecting repeated timer family declares catch-up policy; periodic combat/damage/healing must select one; run-each catch-up is bounded/fair; `SKIP_TO_LATEST` is only for explicitly non-semantic maintenance/AI-think-like work where skipping cannot alter required gameplay outcomes | GAME-ABILITY must bind catch-up semantics per applicable family and cannot use skip-to-latest to discard required gameplay ticks | NO |
| Cooldown/charge/condition | typed owner-local state, deterministic transitions, condition ticks re-enter pipeline, persistence survival deferred | whole gate needs declaration/continuation boundary, not persistence schema | NO |
| Damage/heal composition | staged read-only calculation, explicit RNG anchors, bounded reactive descendants | general proc/trigger semantics extend the same descendant model without creating event-bus mutation | NO |
| Effect families/catalogue | small typed vocabulary; foreign invariants stay with owning domains; catalogue is provenance metadata | whole gate preserves typed domain routing and refuses invented global atomicity | NO |
| Catalogue/fixture binding | parity is case/scenario scoped and revision/evidence bound | whole gate needs readiness ladder, not second evidence registry | NO |

## Remaining-gap classification

`ARCH-BLOCKER` means the gap must be resolved for a coherent whole-gate architecture candidate. `IMPLEMENTATION/PARITY` means it can remain unresolved after architecture acceptance but blocks the affected executable/parity claim.

| ID | Gap | Owner | Decide now? | Whole-gate architecture | Executable/parity impact | Treatment |
|---|---|---|---:|---|---|---|
| `GA-GAP-01` | general delayed/periodic/future mutating occurrence semantics | GAME-ABILITY | YES | `ARCH-BLOCKER` | blocks timers/ticks/channels/delays | one bounded future-occurrence envelope re-enters accepted pipeline |
| `GA-GAP-01A` | repeated-timer catch-up/coalescing/backlog/fairness semantics | GAME-ABILITY semantic policy consuming FND-03 | YES | `ARCH-BLOCKER` | blocks periodic combat/damage/healing and other repeated authoritative timers | each family selects explicit FND-03-compatible policy; run-each is hard-bounded/fair; `SKIP_TO_LATEST` is forbidden for required gameplay-affecting ticks |
| `GA-GAP-02` | proc/trigger/reactive ordering, lineage and cycle prevention | GAME-ABILITY + SIM | YES | `ARCH-BLOCKER` | blocks safe proc/reaction implementation | deterministic descendant generation, explicit re-entry and bounded lineage |
| `GA-GAP-03` | plan partition, partial failure and cross-domain atomicity boundary | GAME-ABILITY + affected domain owners | YES for boundary | `ARCH-BLOCKER` | blocks multi-effect mechanics | validate owner-local group before commit; explicit sub-occurrences; no invented distributed transaction |
| `GA-GAP-04` | continuation across logout/reconnect/recovery/transfer | GAME-ABILITY declaration + FND/GAME-CHANNEL/DUR/Character owners | YES for declaration; NO for exact policy | undeclared path blocks architecture | affected mechanic blocked | explicit continuation policy binding; no silent default |
| `GA-GAP-05` | GAME-ABILITY resource limits | GAME-ABILITY + implementation registry | YES for dimensions; NO numeric ceilings | obligation is architecture blocker | implementation blocked until maxima/tests | enumerate dimensions including catch-up backlog/work; exact values measured |
| `GA-GAP-06` | client prediction/presentation authority | GAME-ABILITY + FND-02/ALPHA-CLIENT | YES trust boundary | `ARCH-BLOCKER` | client implementation later | presentation-only; server result/commit authority |
| `GA-GAP-07` | architecture vs executable/Reference evidence | GAME-ABILITY + Reference evidence + QA/runtime | YES | `ARCH-BLOCKER` | current Reference cases blocked | separate architecture, invariant-fixture and parity prerequisites |
| `GA-GAP-08` | exact target rules | ruleset/Reference | NO globally | not blocker | exercised mechanics blocked until known | versioned per-mechanic policy |
| `GA-GAP-09` | exact cast/cost/cooldown/condition values | ruleset/Reference | NO globally | not blocker | exercised mechanics blocked | revision/evidence-owned content |
| `GA-GAP-10` | exact formulas/RNG | ruleset/Reference + SIM | NO globally | not blocker | affected parity/implementation blocked | SIM envelope; facts evidence-owned |
| `GA-GAP-11` | physical schema/types/scheduler/Wasmtime | DUR-04/tooling/runtime | NO | not blocker | implementation-specific | semantic contract only |
| `GA-GAP-12` | persistence representation | DUR-02 + owning domain | NO | not blocker | required when survival accepted | report-only dependency |
| `GA-GAP-13` | protocol result/error/prediction encoding | FND-02/client | NO | not blocker | integration | semantic authority only |
| `GA-GAP-14` | AI selection/use/spawn behavior | GAME-AI-01 | NO | not blocker | AI integration | registered future owner; no noncanonical proposal is treated as accepted |
| `GA-GAP-15` | movement/world interaction execution | GAME-INTERACTION/world owner | NO | not blocker | affected mechanics fail closed | registered future owner; typed proposal only until accepted owner contract |

## Closure decision A — future occurrences

Existing partial baselines describe channel pulses and condition ticks, but future gameplay also requires delayed hits, secondary effects, timed expiries, scheduled recharge and other bounded work. Allowing each mechanic to invent a timer/callback model would reintroduce hidden mutation and replay ambiguity.

Selected model: any future mutating work is an explicit bounded occurrence/sub-occurrence with parent/provenance, exact behavior revision binding, normalized SIM time/order semantics and the same applicable target/legality/plan/commit path. Private callbacks/timers are rejected as semantic authority. Server-origin work need not masquerade as client commands.

A future occurrence carries enough semantic state to reconstruct why it exists, which revision set governs it, when/order-wise it is eligible, which bounded occurrence lineage it represents and which target/revalidation policy applies. Physical Rust/persistence representation remains deferred.

### Closure decision A1 — repeated-timer catch-up

FND-03 already decides that each repeated timer family whose individual occurrences matter must select a catch-up policy; there is no global scheduler default. GAME-ABILITY therefore consumes that rule explicitly.

Conceptual policies are:

- `DEADLINE_STATE`;
- `RUN_EACH_BOUNDED`;
- `COALESCE_ELAPSED`;
- `SKIP_TO_LATEST` — only for explicitly non-semantic maintenance/AI-think-like work where skipping cannot alter required gameplay outcomes;
- `EXPIRE_OR_CANCEL`.

A later accepted equivalent is possible only if it states the same semantic choice and hard bounds explicitly.

Periodic combat, damage and healing must select the policy in the owning ability/condition/ruleset semantics. `SKIP_TO_LATEST` is **not** a legal way to discard required gameplay-affecting periodic ability/combat/damage/healing occurrences. `RUN_EACH_BOUNDED` requires hard backlog/work bounds, fair carry-over of overdue work and participation in the normal per-scope deterministic work budget. It may not execute unlimited missed ticks in one turn or recursively schedule zero-delay work around the budget. Coalesced/expired/deferred work and any permitted non-semantic skipped work must be observable/replayable enough to explain relevant player-visible timing behavior. Numeric backlog/time ceilings remain measured registry evidence.

This is a gameplay-semantic decision, not a timer-wheel implementation choice.

## Closure decision B — reactions, procs and loop prevention

The damage/heal baseline already rejects hidden recursive mutation and requires bounded reactive descendants. General post-commit reactions follow:

```text
validated owner-scoped occurrence
-> COMMIT
-> typed committed result/evidence
-> deterministic reaction eligibility
-> ordered bounded descendant proposals
-> each accepted descendant becomes a new occurrence
-> applicable target/legality/effect-plan/commit pipeline
```

Pre-commit formula contributions are typed contributions, not post-commit reactions. Reactive edges carry provenance, parent lineage and explicit versioned re-entry/cycle policy. Undeclared re-entry is fail closed. Ordering may not depend on registration/hash/pointer/plugin/thread/wall-clock accidents.

Hard bounds cover reaction depth, descendants and total root work. Budget exhaustion never erases committed history; it deterministically prevents uncommitted descendants and emits bounded evidence.

## Closure decision C — commit groups and partial behavior

Within one accepted authoritative owner/transaction scope, the candidate plan/commit group is validated as a bounded unit before that group's commit. Intentional sequential/partial behavior uses explicit ordered sub-occurrences/commit groups with deterministic failure semantics rather than accidental iteration-prefix commits.

Cross-owner mechanics may correlate/order typed proposals but cannot claim atomicity beyond accepted owner contracts. Stronger required atomicity blocks the mechanic until an accepted workflow/transaction contract exists.

## Closure decision D — continuation and persistence declaration

Every definition/policy that can create future-authoritative ability state binds continuation behavior for each relevant lifecycle boundary it exercises: owner-local continuity; reconnect/same-scope recovery where supported; durable/restart survival where supported; Channel/Instance/world transfer where supported; or explicit non-survival/removal.

There is no implicit survival from remaining duration and no implicit session-local removal. GAME-ABILITY owns gameplay meaning; FND/DUR/Channel/Character owners carry/fence/restore accepted surviving state.

## Closure decision E — resource-bound obligations

Before executable acceptance, every content/external-controlled work or allocation dimension has hard maxima, units, failure categories, allocation impact and boundary tests in the repository resource-limit mechanism. Applicable dimensions include target enumeration, target geometry, Effect Plan size, contribution count, multi-hit/target count, future occurrences, repeated-timer catch-up backlog/work, condition instances/pending work, reaction depth/descendants/root work, cross-domain proposals, diagnostic evidence and inherited DUR-04 limits.

Exact numeric ceilings are not frozen here. Missing required limits block implementation; they do not mean unlimited.

## Closure decision F — client presentation and prediction

Client prediction/presentation is non-authoritative UX over client-safe information. The server owns target legality/membership, interruption ordering, resource/cooldown/condition commits, final damage/heal/effect results, RNG/proc outcomes and authoritative revisions. Client state reconciles to server results. Exact messages, prediction algorithms and UI are downstream FND-02/ALPHA-CLIENT decisions. The ALPHA-CLIENT gate remains registered but its current PR is blocked/noncanonical, so this analysis consumes no blocked client proposal as accepted architecture.

## Closure decision G — executable-fixture prerequisites

Evidence remains three-layered:

1. architecture contract evidence;
2. implementation invariant evidence once runtime exists, including catch-up policy/backlog/fairness boundary tests;
3. Reference parity evidence requiring manifest/provenance/legal/revision/implementation/fixture prerequisites.

Canonical A=`0/4` therefore does not block paper architecture closure but does block all affected Reference parity claims.

## Options for the whole gate

### Option A — keep GAME-ABILITY architecture open until exact Reference mechanics are evidenced
Rejected. It conflates reusable architecture with mechanic evidence and pressures guessing.

### Option B — accept reusable closure envelope while mechanic facts remain fail closed
**SELECTED.** Freeze authority, occurrence/catch-up, ordering, reaction, continuation, bounds and evidence readiness; keep exact mechanics evidence-owned.

### Option C — defer seams to implementation
Rejected. Timer/catch-up/proc/partial-commit/client/resource semantics change correctness, replay, exploit and recovery behavior.

### Option D — generic authoritative event bus/state-patch graph
Rejected. It weakens typed domain-owner boundaries and makes recursion/ownership difficult to prove.

## Trade-offs

The selected closure adds explicit revision/lineage, future-occurrence/catch-up policy, reaction budgets, continuation declarations and owner routing. The complexity is intentional at architecture boundaries rather than hidden in callbacks/schedulers.

Benefits: replayability, safer content scripting, deterministic periodic/proc behavior, Studio/static validation, incident evidence and reduced owner leakage. Costs: validator/tooling and explicit content metadata. Evidence of excessive authoring burden is supersession evidence, not permission to bypass authority.

## Risks and mitigations

- **Catch-up damage/heal storms:** explicit FND-03-compatible policy per family; hard backlog/work bound; fairness; no unbounded same-turn recursion.
- **Silent required tick loss:** `SKIP_TO_LATEST` is limited to non-semantic work where skipping cannot alter required gameplay outcomes; required gameplay-affecting periodic work must use another valid policy.
- **Coalescing semantic drift:** coalesce only when owning gameplay semantics prove deterministic equivalence; observable disposition.
- **Primitive/policy proliferation:** small core effect vocabulary plus composition/domain routing and bounded DUR-04 extension.
- **Reaction graphs expensive:** static/runtime budgets and deterministic lineage/order.
- **Safety limits alter valid gameplay:** measured safety boundaries + static/content validation, not hidden balance knobs.
- **Continuation becomes pseudo-schema:** semantic contract only; DUR/FND decide physical persistence.
- **Cross-domain plan looks atomic:** owner-scoped commits + fail-closed stronger-atomicity requirement.
- **Client prediction leaks authority:** client-safe projection + reconciliation.
- **Architecture acceptance misreported as parity:** separate status/evidence axes and canonical A=`0/4` fail-closed truth.
- **Noncanonical sibling evidence mistaken as accepted:** canonical register is the source of gate ownership/status; exact sibling PR/head is labeled noncanonical when referenced for proposal context.

## Player, producer and operations impact

**Player:** deterministic proc/periodic/catch-up behavior, no hidden client authority and explicit recovery semantics reduce inconsistent combat outcomes and catch-up spikes.

**Producer/game design:** typed composition and explicit continuation/reaction/catch-up policies allow mechanics without custom mutation engines; validators can reject unsupported/unbounded mechanics before runtime.

**Operations/security:** bounded target/effect/reaction/catch-up work limits DoS/proc/tick-storm risk; lineage/revision/commit/catch-up evidence improves exploit/divergence investigation; analytics/scripts remain non-authoritative.

## Recommendation

Send the companion whole-gate candidate to Architecture Coordinator audit after canonical A and the current review repairs are incorporated. Owner acceptance, if later granted, freezes only semantic architecture. `ImplementationStatus` stays `NOT_STARTED`; Reference facts/parity remain per-case evidence-owned.

## Future impact

If accepted, broad implementation can be sliced as:

```text
semantic definitions + validators
-> occurrence/target/lifecycle core
-> repeated-timer catch-up policy enforcement
-> owner-scoped Effect Plan/commit
-> cooldown/charge/condition future-state handling
-> deterministic composition + reactions
-> bounded owner-domain adapters
-> replay/invariant fixtures
-> evidence-qualified Reference mechanics
-> client presentation/reconciliation
```

Exact slice ordering remains future implementation-programme work.

## Decision timing

For `GA-GAP-01`, `GA-GAP-01A` and `GA-GAP-02` through `GA-GAP-07`, **Must decide now: YES at semantic-boundary level.** These choices affect timer identity/catch-up, proc ordering, transaction/partial-failure, recovery, resource failure, client trust and evidence acceptance.

Exact numeric/gameplay/physical decisions classified NO are deliberately deferred. Evidence that can supersede the candidate includes representative mechanic incompatibility, deterministic replay failure, owner atomicity needs, measured performance, unacceptable Studio authoring burden or security evidence.

## Dependency matrix

| Dependency | Current relationship | GAME-ABILITY consumes | Outside this worker | Blocks architecture? |
|---|---|---|---|---:|
| FND-03 | accepted upstream | one writer, order/generation, stale-work rejection, catch-up policy/backlog/fairness semantics including non-semantic-only skip | scheduler/queue internals and numeric limits | NO; binding constraint |
| FND-04 | accepted | session/lease/fence/recovery authority | admission/reconnect | NO |
| GAME-CHANNEL-01 | accepted adjacent | locality/transfer boundary | product transfer implementation | NO; unsupported paths fail closed |
| GAME-CHAR-01 | accepted adjacent | character-state owner boundary | exact formulas/policy | NO |
| GAME-ITEM-01 + DUR-03 | accepted adjacent | item/value ownership/conservation/idempotency | transactions/runtime | NO; owner adapter needed |
| DUR-02 | accepted | recovery/persistence envelope | physical schema/migrations | NO unless survival path exercised |
| DUR-04 | accepted | versioned semantics; proposal-only Wasm/WIT | physical compiler/runtime | NO |
| SIM-DETERMINISM-01 | accepted | revision/numeric/RNG/order/time/replay envelope | exact formulas/RNG runtime | NO |
| ANL-01 | accepted | audit/event boundary | concrete producer schemas | NO |
| Reference manifest + canonical A | accepted evidence truth | immutable target + fail-closed 0/4 current state | evidence research/classification | NO architecture; YES parity |
| GAME-AI-01 | registered required-for-alpha; successor PR #276 is blocked/noncanonical | only future server-origin invocation boundary, not proposal semantics | AI selection/threat/spawn/path | NO; integration blocked until accepted owner contract |
| GAME-INTERACTION/world | registered required-for-alpha; successor PR #277 is blocked/noncanonical | only future typed route boundary, not proposal semantics | movement/world commit semantics | NO; affected mechanics blocked |
| FND-02/ALPHA-CLIENT | FND-02 accepted; ALPHA-CLIENT registered required-for-alpha, PR #273 blocked/noncanonical | server authority + future client reconciliation boundary | wire/prediction UX | NO; client integration blocked until accepted contract where required |
| RESOURCE_LIMITS_REGISTRY | governance | required named bounds | measured values/tests | NO paper; YES executable |

## Whole-gate owner-acceptance prerequisites proposed

1. no accepted partial baseline weakened;
2. future occurrences/reactions use one typed bounded authoritative model;
3. each behavior-affecting repeated timer family has explicit catch-up policy; periodic combat/damage/healing cannot inherit scheduler convenience; run-each backlog is bounded/fair; skip-to-latest cannot discard required gameplay outcomes;
4. owner-scoped commit versus cross-domain workflow explicit;
5. continuation obligation explicit, no persistence defaults;
6. resource dimensions/failures mandatory without speculative maxima;
7. client prediction presentation-only;
8. architecture/implementation/Reference evidence separate;
9. cross-domain gaps reported in required structured form with canonical or explicitly noncanonical exact evidence;
10. canonical Agent-A result is consumed as 0/4 without promotion;
11. exact-head checks/self-review/independent review clean.

## CROSS_DOMAIN_FINDINGS

```yaml
cross_domain_finding:
  id: GA-XD-01
  observed_in_domain: game-ability
  target_owner: FND-03/DUR-02/recovery-persistence
  severity: P1
  evidence: docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical DUR-02 accepted persistence gate)
  conflict_or_gap: Surviving ability state needs owner-defined representation, fencing and restoration; GAME-ABILITY defines semantic continuation only.
  required_before: Any ability state is claimed to survive recovery/restart/handoff.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-02
  observed_in_domain: game-ability
  target_owner: GAME-ITEM/DUR-03
  severity: P1
  evidence: docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical GAME-ITEM/DUR-03 ownership state)
  conflict_or_gap: Conserved item/currency/value costs and consequences remain item/value-owner mutations; ability commit anchors do not create authority or distributed atomicity.
  required_before: Ability execution mutates conserved item/currency/value state.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-03
  observed_in_domain: game-ability
  target_owner: GAME-INTERACTION/world-owner
  severity: P1
  evidence: docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical GAME-INTERACTION-01 required-for-alpha registration); PR #277@efa310c5c581f823ab65f497c3968a754cc0eb8f is noncanonical/BLOCKED proposal evidence only
  conflict_or_gap: Teleport/push/pull/occupancy/world-object consequences need an accepted owner legality/order/commit contract; GAME-ABILITY can route typed proposals only.
  required_before: Movement/world-interaction consequences become executable.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-04
  observed_in_domain: game-ability
  target_owner: GAME-AI-01/world-owner
  severity: P2
  evidence: docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical GAME-AI-01 required-for-alpha registration); PR #276@64d92dfb4a933115f0b59814be54e2f0d51edbe4 is noncanonical/BLOCKED proposal evidence only
  conflict_or_gap: AI may originate invocation in future, but target-choice strategy, threat/aggro, spawn/path and AI-control state require an accepted GAME-AI owner contract and remain foreign authority.
  required_before: Authoritative AI-origin ability use is implemented.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-05
  observed_in_domain: game-ability
  target_owner: SIM-DETERMINISM/ruleset/Reference-evidence
  severity: P1
  evidence: docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md + docs/agents/evidence/OTV2-20260815-ability-combat-reference-continuity.md on canonical main@dc1eecae7952902bee3fb1e2d88aefc2be792cae
  conflict_or_gap: GAME-ABILITY fixes staging but not exact numeric formulas, RNG algorithms/probabilities or Reference draw/order facts.
  required_before: Formula/RNG-dependent execution or Reference parity is claimed.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-06
  observed_in_domain: game-ability
  target_owner: FND-02/ALPHA-CLIENT-01
  severity: P1
  evidence: docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical ALPHA-CLIENT-01 required-for-alpha registration); PR #273@e2eb37e1d099d25dd87ebc02a68c111dd8dd91ac is noncanonical/BLOCKED proposal evidence only
  conflict_or_gap: Protocol/client owners must realize authoritative results/errors/reconciliation/prediction UX without moving authority client-side; current ALPHA-CLIENT proposal is not canonical.
  required_before: Native client gameplay exposes ability outcomes/prediction/reconciliation.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-07
  observed_in_domain: game-ability
  target_owner: Reference-evidence/coordinator
  severity: P1
  evidence: main@dc1eecae7952902bee3fb1e2d88aefc2be792cae + docs/agents/evidence/OTV2-20260815-ability-combat-reference-continuity.md
  conflict_or_gap: Canonical Agent-A result is 0/4 promoted with target UNKNOWN and provenance/legal PENDING; GAME-ABILITY cannot shadow parity truth.
  required_before: Any registered ABILITY_COMBAT case or aggregate ability parity is promoted.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-08
  observed_in_domain: game-ability
  target_owner: ANL-01/producer-registry
  severity: P2
  evidence: docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical ANL-01 accepted observational boundary)
  conflict_or_gap: Analytics should receive bounded occurrence/reaction/revision/commit/catch-up evidence as producer-owned events while staying read-only; concrete GAME-ABILITY producer event registration is not owned by this gate.
  required_before: Concrete GAME-ABILITY analytics/audit coverage is claimed complete.
  worker_action: REPORT_ONLY
```

## DECISIONS_NOT_TAKEN

This analysis deliberately does **not** decide:

- any Reference mechanic behavior, evidence promotion or parity result;
- exact target grammar/range/geometry/LoS/PvP/PZ/immunity/error precedence;
- exact costs, cast/channel times, cooldown/charge/condition values;
- exact condition stacking/conflict/tick cadence;
- exact damage/heal formulas, mitigation, crit/block/dodge/proc/lifesteal or RNG probabilities;
- exact catch-up policy for a concrete mechanic not yet defined by owning ruleset/ability semantics, or numeric backlog/time limits;
- exact reaction priorities/re-entry policies or resource maxima;
- physical content schema, Rust layout, scheduler/timer wheel or DB schema;
- Wasmtime/WIT physical implementation;
- survival of concrete state across lifecycle boundaries;
- movement/item/entity/AI/world APIs or distributed transaction protocol;
- protocol message layout, client prediction/UI/animation/error wording;
- runtime/client/server implementation, executable content, DDL, production rollout or capacity values;
- owner acceptance, merge or closeout.

## Worker handoff

This analysis supports the companion candidate. Both remain `CANDIDATE / IN_REVIEW / NOT_STARTED` until exact-head Architecture Coordinator audit and independent review. Canonical A has been consumed without changing its 0/4 fail-closed truth.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`