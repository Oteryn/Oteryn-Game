# GAME-ABILITY-01 — Whole-Gate Contract Candidate

- DecisionStatus: `CANDIDATE`
- DeliveryStatus: `IN_REVIEW`
- ImplementationStatus: `NOT_STARTED`
- Date: 2026-08-15
- Gate: `GAME-ABILITY-01`
- Worker issue: #260
- Companion analysis: `GAME-ABILITY-01_WHOLE_GATE_GAP_ANALYSIS.md`
- Runtime/client/protocol/content/DDL/Platform/production authority: **NONE**
- Merge authority: **ARCHITECTURE_COORDINATOR_ONLY**

## 1. Purpose

This candidate composes the existing GAME-ABILITY partial baselines into one implementable semantic contract without rewriting or superseding them. It adds only the minimum whole-gate rules needed at their seams: future occurrences, repeated-timer catch-up policy, reactive descendants, owner-scoped commit grouping, lifecycle-continuation declaration, resource-bound obligations, client trust separation and evidence/readiness layering.

If this candidate is later accepted, it means the **architecture of the ability/spell/condition execution domain is closed enough for separately authorized implementation work**. It does not mean any runtime exists, any Reference mechanic is parity-confirmed, any Alpha content is complete or any downstream domain integration is implemented.

## 2. Normative composition and non-supersession

The following existing GAME-ABILITY documents remain binding for their declared scopes and are incorporated by reference rather than restated here:

- `GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_CAST_CHANNEL_COMMIT_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_COOLDOWN_CHARGE_CONDITION_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_EFFECT_COMPOSITION_DAMAGE_HEAL_OWNER_BASELINE.md`;
- `GAME-ABILITY-01_EFFECT_FAMILIES_REFERENCE_CATALOGUE_OWNER_BASELINE.md`;
- catalogue-entry/parity-fixture binding semantics from `GAME-ABILITY-01_REFERENCE_MECHANIC_CATALOGUE_ENTRY_PARITY_FIXTURE_CONTRACT.md`, read together with the later accepted Reference-manifest pin/current programme state that supersedes only its historical statement that the manifest was still candidate;
- the FND-03 repeated-timer/catch-up contract in `FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md`.

This candidate MUST NOT be interpreted to reopen their deliberate deferrals. Where this candidate appears less specific than an accepted partial baseline, the partial baseline governs. Where a later accepted owner contract explicitly supersedes a rule, that later decision governs only its stated scope.

## 3. Whole-gate invariant

Every authoritative ability/mechanic consequence MUST remain explainable as a bounded deterministic sequence of typed occurrences and owner-domain commits:

```text
versioned Ability / Mechanic Definition
+ normalized invocation/origin context
+ exact behavior-affecting revision set
        |
        v
Prepare / Admission
-> explicit Target Resolution + Legality anchors as required
-> read-only typed calculation / contribution stages
-> bounded owner-scoped Effect Plan or ordered sub-occurrence plans
-> authoritative validation
-> named commit anchor(s), including PRIMARY COMMIT where applicable
-> typed committed result/domain evidence
-> optional bounded future/reactive occurrences
        |
        +--> every mutating descendant re-enters applicable authoritative boundaries
```

Client input, content, scripts, AI and system mechanics MAY originate intent/proposals only under their accepted capability/authority context. None receives a second mutation engine.

## 4. Semantic occurrence envelope

An **ability occurrence** in this contract is a semantic execution envelope, not a new global Foundation identity, protocol ID, persistence key or ownership token.

Every occurrence that can affect authoritative state MUST be attributable to an existing accepted stable parent identity/context suitable for its source, for example a CommandRef-like player command, accepted server-side event/timer/operation occurrence or another stable parent lineage. The exact identity type remains owned by FND/SIM/DUR contracts.

The semantic envelope MUST bind or deterministically derive all behavior-affecting context needed for replay and stale-result rejection, including as applicable:

- source/origin authority and capability context;
- exact Ability/Mechanic Definition revision;
- ruleset/content/world-policy revision context;
- target/legality policy revision;
- lifecycle/cost/commit/cooldown/charge/condition policy revisions;
- effect-composition/formula revision;
- `SimulationDeterminismProfileRevision` and named RNG purpose semantics where used;
- DUR-04 script artifact/WIT/`script_execution_profile_revision` where used;
- owning-domain integration revisions where behavior-affecting;
- parent/descendant semantic lineage where the occurrence is future/reactive work.

An occurrence MUST NOT silently switch to a newer incompatible semantic revision because execution is delayed, retried, recovered or ownership moves.

## 5. Definition, policy and engine boundary

GAME-ABILITY retains the accepted **data-first, not data-only** architecture.

Versioned content/ruleset definitions SHOULD express concrete target policy, timing, costs, cooldown/charge policy, conditions, effect-family composition, formula references, continuation policy and reaction policy through typed validated semantics. Core engine/domain code owns invariants, authoritative state transitions, deterministic ordering/validation and domain ownership boundaries.

DUR-04 bounded Wasm/WIT remains the escape hatch for exceptional behavior. A component may consume only approved snapshot/capability inputs and return bounded typed proposals. It MUST NOT own authoritative timers, target sets, condition collections, cooldown maps, RNG streams, persistence, domain mutation or final combat results.

This contract does not select a physical authoring format, generated representation or Rust type graph.

## 6. Targeting and legality integration

Every occurrence uses the accepted target/legality boundary.

- Client/source target identifiers or coordinates are intent only.
- Target discovery/resolution is authoritative, bounded and deterministic.
- Effect planning consumes only targets admitted by the relevant legality boundary.
- Dynamic retargeting is an explicit bounded Target Resolution Step, never a hidden effect-stage world scan.
- A future/reactive occurrence that needs a new target set MUST use an explicitly declared resolver/revalidation policy at a named lifecycle anchor.
- A future occurrence that intentionally retains an earlier stabilized target/snapshot semantics MUST declare that behavior explicitly and bind the required revision/snapshot evidence.

Exact range, geometry, LoS, floor, PvP/PZ, relationship, immunity, partial-target and error-precedence rules remain versioned ruleset/Reference policy.

## 7. Lifecycle, commitment and owner-scoped commit groups

The accepted cast/channel/commit model remains authoritative.

A named commitment anchor commits only the consequence(s) assigned to that anchor. A committed cost/cooldown/charge/effect is history and MUST NOT be silently undone because later work fails. Compensation is a new explicit domain action.

### 7.1 Owner-scoped commit group

Within one accepted authoritative owner/transaction scope, a candidate commit group MUST be bounded and fully validated before that group's authoritative commit. Implementation MUST NOT accidentally commit an iteration prefix and call the remainder a normal failure.

A mechanic requiring intentional sequential or partial resolution MUST represent it as explicit deterministic ordered sub-occurrences/commit groups with declared failure semantics. This applies to multi-hit, multi-target and other ordered mechanics when prior commits are allowed to influence later results.

This rule is semantic. It does not require one SQL transaction, one Rust function or one thread operation.

### 7.2 Cross-domain consequences

An ability may correlate/order typed proposals for multiple domains but does not acquire those domains' mutation authority. A shared plan is not proof of distributed atomicity.

If a mechanic requires atomicity stronger than all participating accepted owner contracts can supply, the affected mechanic MUST fail closed until a separately accepted workflow/transaction contract defines that behavior. Hidden partial mutation or a generic `StatePatch` is forbidden.

## 8. Future and periodic occurrences

Channel pulses, condition ticks, delayed hits, scheduled secondary effects, recharge work and any other future **mutating** behavior MUST use one bounded future-occurrence model rather than private callbacks/timers.

A future occurrence MUST preserve or deterministically derive, as applicable:

- stable parent lineage and bounded occurrence/sub-occurrence index or equivalent stable purpose identity;
- exact semantic revision bindings required to reproduce the work;
- normalized SIM logical time/eligibility semantics, never unsynchronised wall clock as gameplay authority;
- owning semantic scope and order/fence context;
- target retention versus explicit re-resolution/revalidation policy;
- required cost/cooldown/charge/condition anchor policy;
- termination, count/duration/work bounds;
- deterministic cancellation/expiry/failure policy.

When eligible, every future mutating occurrence re-enters the applicable Target/Legality, typed Effect Plan, owner validation and commit boundaries. It does not receive direct mutation authority because it was scheduled earlier.

The contract does not require every deterministic future consequence to be materialized as a timer object. An implementation MAY use an equivalent deterministic derivation if replay, ordering, revision binding, bounds and failure behavior are identical.

### 8.1 Repeated-timer catch-up policy

Every repeated timer family whose individual occurrences can affect gameplay MUST declare an explicit **catch-up/coalescing policy**. There is no implicit global default. The semantic policy MUST be equivalent to one of the FND-03 conceptual classes, or to a later accepted class with the same explicitness and bounds:

- `DEADLINE_STATE` — compute state from the authoritative logical deadline rather than replaying every missed callback;
- `RUN_EACH_BOUNDED` — execute each missed occurrence only within an explicit hard backlog/work bound and carry remaining overdue work fairly across later turns;
- `COALESCE_ELAPSED` — combine missed elapsed periods only where the owning gameplay semantics explicitly prove a deterministic equivalent result;
- `SKIP_TO_LATEST` — **only** for explicitly non-semantic maintenance/AI-think-like work where skipping intermediate occurrences cannot alter any required gameplay outcome; it is not an allowed catch-up choice for required gameplay-affecting ability/combat/damage/healing ticks;
- `EXPIRE_OR_CANCEL` — terminate overdue work when its accepted semantic deadline/eligibility has expired.

**Periodic combat, damage and healing families MUST select their catch-up policy in the owning ability/condition/ruleset definition or accepted semantic policy.** A scheduler implementation MUST NOT decide this by convenience. `SKIP_TO_LATEST` MUST NOT be used to discard required periodic combat/damage/healing occurrences merely because a mechanic definition names that policy; the FND-03 non-semantic restriction is binding.

`RUN_EACH_BOUNDED` MUST NOT produce an unbounded same-turn catch-up storm. Its backlog participates in the same per-scope deterministic work budget/fairness policy as other authoritative work. Zero-delay recursive rescheduling that bypasses the bound is forbidden.

Coalesced, expired and budget-deferred work — and any permitted non-semantic skipped work — MUST remain observable enough to explain player-visible timing/recovery behavior where applicable and to reproduce the deterministic disposition under the same semantic revisions. Exact backlog counts/time limits are implementation evidence and resource-registry decisions; missing required numeric bounds block implementation acceptance rather than implying unlimited catch-up.

## 9. Conditions, cooldowns and charges as future-authoritative state

The accepted cooldown/charge/condition baseline remains binding. In particular, `ConditionDefinition` is immutable/versioned semantics while `ConditionInstance` is authoritative runtime state, and mutating condition ticks re-enter the same pipeline.

This whole-gate candidate adds one requirement: any policy that creates state capable of changing a future authoritative outcome MUST explicitly declare its continuation semantics for every lifecycle boundary that the mechanic exercises.

The semantic model MUST be capable of expressing, without silent defaults:

- owner-local continuation while the authoritative scope remains live;
- reconnect/same-scope recovery eligibility when an accepted FND recovery contract supports it;
- durable/restart continuation when an accepted DUR contract supports it;
- Channel/Instance/world-transfer handling when an accepted transfer/handoff contract supports it;
- explicit non-survival/removal where that is the ruleset behavior.

These bullets are required semantic capabilities, not a frozen enum or a statement that any concrete condition/cast survives a listed boundary.

A positive remaining duration does not imply survival. Session end does not imply removal. Missing required ruleset/Reference policy remains fail closed.

GAME-ABILITY owns the gameplay meaning and revision of the state. FND/GAME-CHANNEL/DUR/Character owners own safe fencing, transfer, recovery and physical persistence when survival is accepted.

## 10. Calculation and typed contributions

Damage/healing and other authoritative calculations remain read-only until their typed consequences enter a validated commit group.

A behavior that changes the current occurrence's magnitude/admission/result MUST be represented at an explicit typed pre-commit calculation/admission/contribution stage. It MUST NOT be implemented as an unordered mutable callback that happens to run before commit.

Contribution ordering, arithmetic, rounding, overflow, invalid-value handling and gameplay RNG follow the accepted SIM-DETERMINISM and versioned formula/ruleset descriptors. Exact formula values and Reference order remain evidence/policy-owned.

State-consuming mitigation such as shields/absorbs MUST expose correlated typed transition consequences; a calculator may not secretly mutate authoritative state.

## 11. Reactions, procs and trigger lineage

A proc/trigger that creates a consequence beyond the current validated commit group MUST become an explicit bounded descendant occurrence. Generic authoritative event-bus mutation is not accepted.

A reaction opportunity MUST be anchored to a named typed lifecycle/result stage. Its read inputs MUST be immutable/stabilized for that stage. If a mechanic must alter the current parent calculation, it participates as a typed pre-commit contribution under section 10 rather than mutating the parent from a descendant callback.

For post-commit descendants, the semantic flow is:

```text
committed parent result/evidence
-> deterministic reaction eligibility evaluation
-> deterministic ordered descendant proposals
-> bounded descendant occurrences
-> each descendant re-enters its applicable authoritative pipeline
```

Every reactive edge/definition MUST provide stable semantic provenance and an explicit versioned re-entry/cycle policy. Re-entry MUST NOT be inferred from handler registration or implementation recursion. Undeclared re-entry is fail closed.

Whenever order can affect gameplay, eligible reactions MUST have a deterministic total ordering policy based on semantic inputs/revisions. Registration order, hash iteration, pointer address, thread completion, plugin load order, database default order and wall-clock observation are forbidden tie-breakers.

## 12. Reaction and occurrence loop prevention

Every implementation MUST enforce hard bounds for reactive/future work. The semantic model MUST support at least:

- maximum reaction depth;
- bounded descendants per parent/root;
- bounded total reaction/future work for one root occurrence;
- bounded dynamic retarget/re-resolution depth where applicable;
- bounded multi-hit/multi-target sub-occurrences;
- bounded channel/periodic count/duration;
- bounded repeated-timer catch-up backlog/work per authoritative scope.

Exact maxima are implementation-evidence decisions and MUST be registered through the repository's resource-limit mechanism before implementation acceptance.

If a post-commit descendant budget is exhausted, the parent and already committed descendants remain committed. Further uncommitted descendants are deterministically rejected/terminated under the owning fail-closed policy and bounded evidence is emitted. The engine MUST NOT roll back history, silently disable the safety limit, reroute through direct script mutation or continue unbounded.

Content/static validation SHOULD prove semantic maxima are within accepted runtime bounds where possible. Safety bounds MUST NOT become undocumented balancing rules for otherwise valid mechanics.

## 13. Resource-limit contract

Before executable GAME-ABILITY implementation may be accepted, every externally/content-controlled work or allocation dimension used by that implementation MUST have an explicit hard maximum, unambiguous unit, failure category, allocation impact and boundary tests in the accepted resource-limit mechanism.

At minimum, applicable entries MUST cover:

- target candidate count, resolved-target cardinality, query/geometry complexity and dynamic retarget depth;
- Effect Plan entry count and any relevant bounded encoded/in-memory plan size;
- calculation/contribution stage cardinality;
- multi-hit/multi-target sub-occurrence count;
- channel/periodic occurrence count, outstanding future work and `RUN_EACH_BOUNDED` catch-up backlog/work;
- condition instance/stack cardinality and pending scheduled condition work;
- reaction depth, descendant cardinality and total root reaction work;
- cross-domain proposal count;
- GAME-ABILITY-specific variable diagnostic/evidence payloads where they can cause unbounded work;
- inherited DUR-04 script/query/action-plan constraints for script-backed mechanics.

This candidate intentionally does not select numeric values. Missing required limits block implementation acceptance; they are not interpreted as unlimited.

Untrusted or oversized candidate work MUST be rejected before unchecked allocation and, for a pre-commit plan, before partial authoritative mutation.

## 14. Cross-domain routing

Effect vocabulary does not transfer domain ownership.

Representative routing remains:

- item/stack/item-charge/currency/loot/conserved value -> `GAME-ITEM` / `DUR-03`;
- movement/teleport/push/pull/occupancy/world-object activation -> registered future `GAME-INTERACTION-01` / world owner; affected mechanics remain blocked until an accepted owner contract exists;
- spawn/summon/despawn/entity lifecycle -> authoritative entity/world lifecycle owner;
- threat/aggro/AI-control state -> registered future `GAME-AI-01` owner; no AI contract is treated as canonical until accepted/merged;
- character-owned state/progression/resource semantics outside explicitly GAME-ABILITY-owned availability/condition surfaces -> Character/simulation owner;
- persistence/recovery of surviving future state -> FND/DUR owner;
- observation/investigation -> ANL owner, read-only.

The list is illustrative. Each actual integration MUST use a bounded typed owner-approved surface. No generic resource delta, state patch, script callback or catalogue metadata may bypass owner invariants.

## 15. Client presentation and prediction boundary

Client prediction/presentation is permitted only as non-authoritative UX over client-safe information.

A client MAY tentatively present cast progress, target previews, cooldown countdowns, condition/status indicators, animation or pending action state. It MAY send cancellation/target/action intent as permitted by protocol.

The server remains authoritative for:

- accepted invocation/admission;
- target membership/order and legality;
- interruption/cancellation ordering;
- resource/cost/cooldown/charge/condition commitments;
- final damage/healing and other effect results;
- RNG/proc/reaction outcomes;
- commit success/failure and authoritative revisions.

Client presentation MUST reconcile to authoritative results/revisions and MUST NOT promote local prediction to state authority. Server-only data, hidden rules/RNG or security-sensitive evidence MUST NOT be exposed merely to make prediction exact.

Exact FND-02 messages, error categories/codes, replication payloads, prediction algorithms, UI timing and animations are downstream protocol/client decisions. `ALPHA-CLIENT-01` remains a registered but currently noncanonical/blocked downstream gate; this contract does not consume its blocked proposal as accepted architecture.

## 16. Failure semantics

The following whole-gate rules are mandatory:

- invalid/illegal/oversized pre-commit work fails before its candidate commit-group mutation;
- a failed/rejected script proposal does not receive fallback direct mutation;
- an interruption/cancellation prevents only consequences not already committed at their named anchors;
- committed costs/effects/cooldowns/charges/ticks are never erased as implicit rollback;
- any reversal/refund after commit is a new explicit compensation action under the owning domain;
- a failed reactive descendant does not erase its committed parent or earlier committed siblings;
- missing required revision, unsupported owner integration, unknown required Reference behavior, missing resource limit, missing repeated-timer catch-up policy or stale fence fails closed for the affected executable path;
- retry MUST NOT reroll or reinterpret the same logical occurrence under a newer incompatible revision;
- no client, analytics, script or catalogue record can be used as fallback gameplay authority.

Exact player-facing failure precedence/text remains ruleset/protocol/client owned.

## 17. Replay, provenance and observability

Every behavior-affecting execution path MUST preserve enough typed evidence to explain and reproduce, as applicable:

- parent invocation/occurrence lineage;
- exact semantic revision set;
- target resolution/legality stage outcome;
- named commitment anchors and owner-scoped commit groups;
- formula/SIM/RNG purpose evidence needed by accepted replay policy;
- cooldown/charge/condition transitions and future-occurrence lineage;
- repeated-timer catch-up/coalescing/expiry/deferred-backlog disposition and any permitted non-semantic skip disposition;
- reaction/proc eligibility/order/descendant lineage;
- cross-domain routing result;
- deterministic capacity/failure disposition.

ANL-01 remains observational/read-only and owns event/audit envelope semantics. This contract does not create an analytics mutation path or require unbounded production traces. Security-sensitive RNG roots/seeds remain protected under SIM/security policy.

## 18. Reference catalogue and evidence boundary

Reference Mechanic Catalogue metadata remains separate from executable content and core effect vocabulary.

The accepted Reference manifest is the evidence/parity authority. A catalogue entry MAY bind to manifest cases and exact GAME-ABILITY revisions; it MUST NOT upgrade evidence or parity.

The canonical Agent-A continuity result merged by PR #271 is consumed exactly as-is: **0/4 registered `ABILITY_COMBAT` cases are promoted**; target evidence remains `UNKNOWN`; source/case provenance and legal review remain `PENDING`; implementation remains `NOT_STARTED`; parity remains fail-closed. This whole-gate candidate does not reinterpret or improve those classifications.

For every concrete Reference mechanic exercised by a Reference milestone:

```text
sufficient target evidence for the exercised cases/aspects
+ required provenance/legal state cleared
+ exact behavior-affecting revision bindings
+ exact Oteryn implementation revision
+ passing bounded executable fixture/test coverage
+ no unresolved required cross-domain/limit/invariant gap
=> candidate for parity confirmation under the accepted manifest contract
```

Absent any required element, the affected parity claim remains fail closed. Aggregate parity cannot be inferred from catalogue presence, content similarity, source-code similarity or a single happy-path scenario.

## 19. Architecture evidence versus executable evidence

Acceptance of this paper contract, if granted later, requires architecture/governance evidence only. It does not require pretending an unimplemented engine can pass runtime fixtures.

Future executable GAME-ABILITY acceptance MUST add at least representative evidence for:

1. deterministic target membership/order and legality staging under shuffled underlying enumeration;
2. no hidden effect-stage retargeting;
3. pre-commit invalid/oversized plan rejection with zero partial owner-local mutation;
4. explicit ordered partial/sequential sub-occurrence behavior where configured;
5. commit-anchor behavior showing committed consequences are not implicitly rolled back by later interruption/failure;
6. cooldown/charge/condition transition determinism and bounded future occurrences;
7. retry/replay preserving exact semantic revision and RNG outcome/identity as applicable;
8. periodic/delayed occurrence ordering independent of OS/thread/wall-clock scheduling;
9. deterministic repeated-timer catch-up fixtures for every implemented periodic family, including missed-deadline/backlog, coalescing/expiry behavior, any permitted non-semantic skip behavior, fairness and hard-bound exhaustion;
10. reaction/proc ordering independent of registration/hash/thread order;
11. reaction cycle/re-entry rejection and all configured hard-bound boundary tests;
12. post-commit descendant failure/budget exhaustion preserving committed parent history;
13. bounded Wasm proposal failure/trap/fuel exhaustion causing zero unauthorized mutation;
14. unsupported/missing cross-domain owner integration failing closed;
15. continuation/recovery fixtures for every lifecycle boundary an implemented mechanic claims to survive;
16. client prediction mismatch reconciling to authoritative result without local authority;
17. Reference fixture evidence only after manifest/provenance/revision prerequisites are met.

The exact test framework, fixture runner and implementation architecture are not selected here.

## 20. Definition/build/activation validation obligations

Once executable content/runtime authority exists, the semantic validator/compiler/activation path MUST reject or block, as applicable:

- unresolved or incompatible behavior-affecting revision references;
- unsupported effect family or owner-domain integration;
- generic authoritative state patches;
- hidden/unbounded targeting, future work or reaction graphs;
- a repeated timer family with no explicit accepted catch-up policy or required hard backlog/work bound;
- a gameplay-affecting repeated timer configured with `SKIP_TO_LATEST` contrary to FND-03's non-semantic restriction;
- a reaction/re-entry policy without deterministic ordering/loop bounds;
- a future-authoritative state path whose exercised continuation behavior is undeclared;
- required resource dimensions with no accepted hard limit;
- a script requiring undeclared capability or direct mutation authority;
- a Reference-profile behavior claimed as confirmed when required evidence/parity prerequisites are not satisfied;
- a cross-domain mechanic that requires atomicity unavailable from accepted owner contracts.

Physical compiler/schema details remain DUR-04/tooling implementation choices.

## 21. Dependency matrix

| Dependency/owner | Binding use in this contract | Not owned/decided here | Failure posture |
|---|---|---|---|
| FND-03 | authoritative owner order/generation, stale-work rejection, repeated-timer catch-up/coalescing policy and fairness/backlog requirements | runtime scheduler/process internals; exact numeric catch-up limits | fail closed on stale/ambiguous authority or missing/invalid catch-up policy/bounds |
| FND-04 | session/lease/recovery fences | admission/reconnect implementation | no ability authority bypass |
| GAME-CHANNEL-01 | Channel/Instance locality/transfer policy boundary | channel selection/transfer implementation | no implicit cross-Channel mutable state |
| GAME-CHAR-01 | character-owned state/progression boundary | exact character rules/formulas | typed owner route only |
| GAME-ITEM-01 + DUR-03 | item/value ownership/conservation/idempotency | item transaction runtime/schema | no generic resource delta/value mint |
| DUR-02 | accepted durability/recovery envelope | DDL/checkpoint/migration representation | survival path blocked until safe owner carrying exists |
| DUR-04 | versioned content and bounded proposal-only Wasm/WIT | physical format/compiler/engine selection | missing capability/revision fails closed |
| SIM-DETERMINISM-01 | numeric/RNG/order/time/replay semantics | formula values/RNG algorithm/runtime | missing profile/evidence fails closed |
| ANL-01 | typed evidence/audit boundary | concrete event schema/storage | read-only observation only |
| Reference manifest/catalogue | evidence/revision/parity binding | evidence acquisition/classification | `UNKNOWN/CONFLICT/PENDING` stays fail closed |
| GAME-AI-01 | registered required-for-alpha owner; current successor proposal remains noncanonical/blocked | AI choice/threat/spawn/path behavior | affected AI-origin ability integration blocked until accepted owner contract |
| GAME-INTERACTION/world owner | registered required-for-alpha owner; current successor proposal remains noncanonical/blocked | world interaction legality/commit API | affected mechanic blocked until accepted owner contract |
| FND-02 / ALPHA-CLIENT-01 | server-authority contract + registered client architecture gate; current ALPHA-CLIENT proposal remains noncanonical/blocked | wire layout, error protocol, prediction/UI | client never becomes authority; integration blocked on accepted client contract where required |
| RESOURCE_LIMITS_REGISTRY | hard implementation bounds/boundary tests | exact numeric maxima | missing required entry blocks implementation acceptance |

## 22. Whole-gate acceptance boundary

This candidate is suitable for owner acceptance only if the Architecture Coordinator verifies all of the following on the exact worker head and current merged dependencies:

- no accepted partial baseline is weakened or contradicted;
- one authoritative typed pipeline remains the only GAME-ABILITY mutation path;
- future occurrences and reactions are explicit, bounded, deterministic and revision-bound;
- every behavior-affecting repeated timer family has an explicit FND-03-compatible catch-up policy and required bounds/fairness behavior;
- `SKIP_TO_LATEST` remains restricted to FND-03's explicitly non-semantic maintenance/AI-think-like class and cannot discard required gameplay outcomes;
- owner-scoped commit groups cannot masquerade as distributed atomicity;
- continuation semantics are explicit obligations rather than implementation defaults;
- resource dimensions/failure rules are architecturally mandatory without speculative numeric ceilings;
- client prediction remains presentation-only;
- Reference evidence/parity remains a separate fail-closed axis and current canonical Agent-A state is consumed without promotion;
- foreign-domain work is reported as dependency/finding and not silently absorbed;
- every cross-domain `evidence` field resolves to canonical repository truth or explicitly labels an exact noncanonical PR/head as such rather than presenting a nonexistent canonical path;
- ordinary exact-head repository checks and worker self-review are clean.

Even after owner acceptance, implementation remains `NOT_STARTED` until separately authorized and proven.

## 23. Decision timing

**Must decide now: YES** for the semantic closure rules in sections 4, 7.1/7.2, 8/8.1, 9's declaration boundary, 11, 12, 13's obligation, 15's trust boundary and 18/19's evidence separation.

These choices shape identity/lineage, ordering, catch-up/fairness, rollback, recovery, resource safety and trust boundaries. Deferring them until content/runtime implementation would make incompatible local designs expensive and unsafe to reconcile.

**Must decide now: NO** for exact mechanic values, physical representations, algorithms, numeric catch-up maxima and foreign-owner APIs listed in `DECISIONS_NOT_TAKEN`. They are either evidence-dependent, measurement-dependent or owned elsewhere. Freezing them now would be speculative.

## 24. Supersession criteria

Reopen this candidate after acceptance only with concrete evidence such as:

- representative Reference/Evolved mechanics cannot be represented without unsafe complexity or pathological primitive/policy proliferation;
- deterministic replay or retry evidence disproves the occurrence/lineage/catch-up model;
- required cross-domain atomicity cannot be safely expressed through accepted owner workflows;
- measured performance remains unacceptable after semantics-preserving optimization;
- Studio/content-production evidence shows the explicit model is impractical at scale;
- security/abuse findings demonstrate the reaction, capability or resource model is insufficient;
- a later accepted Foundation/SIM/DUR/domain contract materially changes an authority boundary this contract depends on.

Supersession MUST explicitly preserve or replace server authority, deterministic ordering/revision binding, bounded work, explicit catch-up semantics, domain ownership, no-hidden-rollback and fail-closed evidence properties.

## 25. CROSS_DOMAIN_FINDINGS

```yaml
cross_domain_finding:
  id: GA-XD-01
  observed_in_domain: game-ability
  target_owner: FND-03/DUR-02/recovery-persistence
  severity: P1
  evidence: docs/architecture/FND-03_RUNTIME_LIFECYCLE_FAILURE_AND_REPLAY_ANALYSIS_BASELINE.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical DUR-02 accepted persistence gate)
  conflict_or_gap: Any ability state declared to survive recovery/restart/handoff needs owner-defined representation, fencing and restoration; GAME-ABILITY defines semantic continuation only and has no DDL/checkpoint/handoff authority.
  required_before: Any cast/channel/cooldown/charge/condition/future occurrence is claimed to survive a recovery or transfer boundary.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-02
  observed_in_domain: game-ability
  target_owner: GAME-ITEM/DUR-03
  severity: P1
  evidence: docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical GAME-ITEM/DUR-03 ownership state)
  conflict_or_gap: Conserved item/currency/value costs and consequences remain item/value-owner mutations; GAME-ABILITY commit anchors do not create item authority or distributed atomicity.
  required_before: An ability may consume, create, destroy or transfer conserved item/currency/value state.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-03
  observed_in_domain: game-ability
  target_owner: GAME-INTERACTION/world-owner
  severity: P1
  evidence: docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical GAME-INTERACTION-01 required-for-alpha registration); PR #277@efa310c5c581f823ab65f497c3968a754cc0eb8f is noncanonical/BLOCKED proposal evidence only
  conflict_or_gap: Teleport/push/pull/occupancy/world-object consequences require an accepted owner-defined legality/ordering/commit contract; GAME-ABILITY can route typed proposals only and the current sibling proposal is not canonical.
  required_before: Any movement or world-interaction ability consequence becomes executable.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-04
  observed_in_domain: game-ability
  target_owner: GAME-AI-01/world-owner
  severity: P2
  evidence: docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical GAME-AI-01 required-for-alpha registration); PR #276@64d92dfb4a933115f0b59814be54e2f0d51edbe4 is noncanonical/BLOCKED proposal evidence only
  conflict_or_gap: AI-origin ability use may later share the invocation pipeline, but intent selection, threat/aggro, spawn/pathfinding and AI-control state require an accepted GAME-AI owner contract and remain foreign authority.
  required_before: Authoritative AI-origin ability selection/control is implemented.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-05
  observed_in_domain: game-ability
  target_owner: SIM-DETERMINISM/ruleset/Reference-evidence
  severity: P1
  evidence: docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md + docs/agents/evidence/OTV2-20260815-ability-combat-reference-continuity.md on canonical main@dc1eecae7952902bee3fb1e2d88aefc2be792cae
  conflict_or_gap: The whole gate fixes deterministic staging but does not own exact numeric formulas, RNG algorithm/probabilities or Reference draw/order facts.
  required_before: Any affected formula/RNG-dependent mechanic is implemented or claimed Reference-parity-confirmed.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-06
  observed_in_domain: game-ability
  target_owner: FND-02/ALPHA-CLIENT-01
  severity: P1
  evidence: docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical ALPHA-CLIENT-01 required-for-alpha registration); PR #273@e2eb37e1d099d25dd87ebc02a68c111dd8dd91ac is noncanonical/BLOCKED proposal evidence only
  conflict_or_gap: Protocol/client owners must realize authoritative results, errors and reconciliation/prediction UX; client presentation cannot become ability authority, and the current ALPHA-CLIENT proposal is not canonical.
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
  conflict_or_gap: Canonical Agent-A result is 0/4 promoted with target UNKNOWN and provenance/legal PENDING; GAME-ABILITY cannot create or shadow parity truth.
  required_before: Any of the four registered ABILITY_COMBAT cases or aggregate Reference ability parity is promoted.
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GA-XD-08
  observed_in_domain: game-ability
  target_owner: ANL-01/producer-registry
  severity: P2
  evidence: docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md + docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md (canonical ANL-01 accepted observational boundary)
  conflict_or_gap: Analytics schemas should expose bounded occurrence/reaction/revision/commit/failure lineage as producer-owned events without becoming mutation or gameplay-order authority; concrete GAME-ABILITY producer event registration is not owned by this gate.
  required_before: Concrete GAME-ABILITY analytics/audit event coverage is claimed complete.
  worker_action: REPORT_ONLY
```

## 26. DECISIONS_NOT_TAKEN

This candidate deliberately does **not** decide:

- actual Reference mechanic facts, evidence classes, provenance/legal clearance or parity promotion;
- exhaustive ability/effect/condition/proc/trigger catalogues;
- exact target intent/query grammar, range/geometry/LoS/floor/PvP/PZ/friendly-fire/immunity/error precedence;
- exact target snapshot versus re-resolution choice for any concrete mechanic;
- exact cast/channel duration, interruption causes/precedence, cost values, reservation timing, refund policy, cooldown groups/durations, charge capacity/recharge cadence;
- exact condition stack/refresh/replace/match/source partitioning, immunity/resistance/suppression/dispel precedence, durations or tick cadence;
- exact damage/heal formulas, modifier operation values/order, armor/resistance/absorb, crit/block/dodge, proc chances, lifesteal, RNG probability/order or numeric representation;
- exact repeated-timer policy for a concrete mechanic where the owning ruleset/definition has not yet selected it, and exact catch-up/backlog numeric limits;
- exact reaction priorities, concrete re-entry/cycle policy values or hard resource maxima;
- whether any concrete cast/channel/cooldown/charge/condition survives logout, reconnect, GameNode recovery, process restart or Channel/world transfer;
- physical content/catalogue serializer/schema, Rust enums/types/IDs/layout, scheduler/timer wheel, thread/task model or caches;
- exact Wasmtime/WIT implementation beyond accepted DUR-04 public capability semantics;
- physical persistence schema, SQL DDL, migrations, checkpoint formats or cross-domain transaction protocol;
- GAME-AI, GAME-INTERACTION, item/value, world/entity or Character foreign-domain APIs;
- FND-02 wire fields/message IDs/error encoding, ALPHA client prediction algorithm/UI/animation/FX;
- exact ANL event family/schema/retention;
- fixture runner/test framework or runtime crate/service decomposition;
- runtime/client/server/content implementation, production rollout, capacity values, operational SLOs or LiveOps policy;
- owner acceptance, merge or lifecycle closeout.

## 27. Worker status and handoff

This file is a worker-authored candidate, not canonical accepted architecture. Exact-head validation and independent review must use the current repaired head after canonical Agent-A integration.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
`IMPLEMENTATION_AUTHORITY: NONE`