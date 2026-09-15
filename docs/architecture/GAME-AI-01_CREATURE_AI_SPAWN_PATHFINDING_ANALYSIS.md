# GAME-AI-01 — Creature AI, Spawn and Pathfinding Analysis

- DecisionStatus: `PROPOSED`
- DeliveryStatus: `OPEN`
- ImplementationStatus: `NOT_STARTED`
- Gate: `GAME-AI-01`
- Successor issue: `#275`
- Successor task: `OTV2-20260815-game-ai-successor`
- Successor branch: `docs/arch-c-game-ai-successor`
- Predecessor issue: `#261`
- Predecessor PR: `#272`
- Predecessor final reviewed head: `f977a2865c6210f2962a24fa9c00d556acf76122`
- Trusted successor base: `cb98fd32a2bb71fce83234ebf8bf69bdd1a1970e`
- Scope: paper-only architecture analysis; runtime/content implementation authority **NONE**
- Canonicality: **NONCANONICAL WORKER PROPOSAL** until Architecture Coordinator acceptance/merge
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Purpose and successor boundary

This document is the bounded successor/re-scope of the worker proposal developed in #261/#272 for `GAME-AI-01`.

It exists because the predecessor task recorded `repair_cycles_for_current_gate: 5`, while repository governance requires repair cycling to stop after three attempts. The Architecture Coordinator therefore ended the predecessor review with `BLOCKED` rather than authorizing a sixth repair cycle.

This successor does **not** create a new stable architecture gate. The gate remains `GAME-AI-01`. It does **not** erase or reset the predecessor repair history, and it does not authorize this worker to close, supersede, merge or lifecycle-transition #261/#272.

The semantic re-scope is intentionally narrow. It resolves only the final #272 findings concerning:

1. decision timing for the typed bounded FSM representation;
2. atomicity when a semantic AI resolution exceeds its execution budget;
3. spawn occupancy retry policy;
4. task-readiness truth;
5. repair-cycle history/governance.

All other correct predecessor boundaries are carried forward below.

## 2. Binding inputs and precedence

### 2.1 PROVEN — consumed architecture

This proposal composes with the following accepted/current architecture and does not replace it:

- `FND-03_RUNTIME_EXECUTION_CONTRACT.md` — `ChannelRuntime`/`InstanceRuntime` ownership, owner-local ordering, normalized timers, auxiliary proposal work, generation fencing and current-owner revalidation;
- `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md` — deterministic semantic revisions, owner-local execution ordinals, stable ordering, bounded work, normalized time, replay provenance and fail-closed resolution behavior;
- `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` — channel simulation locality is not reward/source multiplicity; value-producing sources require explicit eligibility/multiplicity semantics;
- `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md` — immutable content provenance and bounded script/component proposal semantics; invalid plans reject atomically by default unless another accepted domain contract explicitly defines partial-plan semantics;
- accepted `GAME-ABILITY-01` partial owner baselines — targeting/legality, cast/channel/commit, cooldown/charge/condition and typed effect ownership remain downstream; semantic diagrams do not imply a mandatory physical Rust enum/scheduler representation;
- `DISCONNECT_REENTRY_PVE_PROTECTION_OWNER_DECISION.md` and its parent baseline — valid re-entry protection suppresses new PvE monster offensive actions for the accepted window while preserving the same authoritative actor, threat/encounter obligations and already committed effects.

### 2.2 DERIVED — GAME-AI consequences

From those inputs:

- local AI/spawn mutation has no new service/process owner; it remains with the current `ChannelRuntime` or `InstanceRuntime`;
- pathfinding/planning workers and scripts can only produce bounded proposals;
- any future-determining AI state must be deterministic/replayable when required by SIM/recovery semantics;
- GAME-AI may select typed intents but cannot bypass GAME-ABILITY/GAME-INTERACTION/reward owners;
- a representation choice such as FSM, behavior tree or statechart is not automatically an architecture invariant merely because the semantic behavior is finite and typed;
- a semantic resolution that may still fail for budget/validation reasons cannot mutate authoritative AI state incrementally before the whole AI-local resolution is accepted.

### 2.3 UNKNOWN — Reference behavior remains evidence-gated

This successor establishes no new Reference parity evidence. Exact Reference values/algorithms remain `UNKNOWN`, `CONFLICT` or `PENDING` unless separately proven, including perception, aggro/threat, memory/leash, path preference, spawn retry count/cadence/deadline, controlled-actor details, NPC behavior and reward contribution semantics.

Architecture completeness MUST NOT be used as evidence of Reference parity.

## 3. Preserved predecessor authority model

### 3.1 Local runtime ownership

For a public-world actor, the current `ChannelRuntime(WorldId, ChannelId)` is the local mutation authority. For an instance-local actor, the current `InstanceRuntime(WorldId, InstanceId)` is the local mutation authority.

Auxiliary pathfinding/planning workers, scripts, clients, persistence callbacks, analytics/Game Intelligence and foreign runtimes MUST NOT directly mutate local AI/spawn state.

A world/event/encounter owner may supply normalized facts or commands, but it does not silently transfer local actor mutation authority into GAME-AI.

### 3.2 Foreign-domain boundaries

GAME-AI does not own:

- damage/healing/effect/death/cast/cooldown legality;
- item/currency/XP/loot creation or settlement;
- door/teleport/world-interaction legality;
- durable quest/dialogue/trade/economy state;
- persistence schema/DDL;
- analytics evidence schemas or sanctions;
- global coordinator overlays.

## 4. Preserved deterministic perception and target pipeline

The candidate keeps the predecessor's bounded deterministic pipeline:

```text
bounded authoritative snapshot
-> bounded candidate enumeration
-> scope/lifecycle/perception eligibility filter
-> bounded current-target/threat/stimulus memory
-> versioned policy score/priority
-> stable semantic tie-break
-> retain/switch/clear decision
-> typed action intent
-> downstream legality validation/commit
```

Required properties remain:

- bounded candidate and memory sets;
- canonical ordering before order-sensitive evaluation;
- stable tie-breaks independent of hash/pointer/thread/worker completion order;
- stale actor generation/revision rejection;
- downstream legality may reject an intent and GAME-AI cannot bypass that rejection;
- rejected intents cannot create unbounded immediate retry loops.

Exact Reference tuning remains evidence-gated.

## 5. Final finding 1 — decision timing for typed bounded FSM

### 5.1 Subject under decision

The material decision is **not** whether authoritative AI execution must be bounded, deterministic, typed and owner-controlled. Those are already required by upstream contracts.

The material decision is whether `GAME-AI-01` must now freeze **typed bounded FSM as the normative v1 representation/model identity** for authored/runtime behavior execution.

### 5.2 Mandatory decision-timing test

**Must decide now? `NO`.**

#### Concrete downstream gate/proof blocked by deferral

**None at the current paper-only architecture stage is blocked specifically by not choosing FSM representation.**

The future `GAME-AI-01` executable implementation-acceptance proof is blocked until an implementation candidate demonstrates the accepted semantic execution contract, but that proof can be satisfied by any representation that proves the same bounded deterministic semantics. No current GAME-ABILITY, GAME-INTERACTION, FND-03, SIM, DUR-04 or GAME-CHANNEL gate requires FSM identity.

Therefore the architecture must freeze the semantic obligations now and defer physical/model representation until evidence makes it necessary.

#### Cost if the representation were frozen now and reversed later

Prematurely freezing FSM identity could couple:

- content authoring schema to state/transition IDs;
- checkpoints/replay evidence to FSM-specific state layout;
- Studio/tooling UX to one graph model;
- migration/compatibility rules to FSM-specific revisions;
- script leaf contracts to FSM-specific call sites;
- test fixtures and diagnostics to FSM-only transition vocabulary.

Reversing that later could require content migration, compatibility adapters, replay/checkpoint conversion, tooling changes and deprecation policy. Because no executable model has yet been accepted, that coupling cost is avoidable now.

#### Evidence sufficient to supersede this deferral

A later decision may freeze FSM or another representation only when evidence demonstrates a material need, for example:

- representative Reference/Evolved behavior corpus proves an expressiveness or auditability requirement that materially favors one model;
- an implementation proof shows deterministic bounded semantics cannot be met acceptably without a particular representation or lowering model;
- measured resource/performance evidence demonstrates a material bounded-cost difference;
- recovery/replay evidence requires a particular stable continuation/state identity;
- authoring/Studio evidence proves one representation materially reduces unsafe ambiguity while preserving migration compatibility;
- a reviewed migration/compatibility plan justifies replacing any already accepted representation.

Any later supersession must name the exact clause being superseded and preserve semantic invariants unless separately re-decided.

#### Deliberately undecided

This successor deliberately does not choose:

- FSM versus behavior tree versus statechart/decision graph or another finite semantic representation;
- a concrete AI framework/library;
- physical Rust enums/types/scheduler structures;
- authoring UI/Studio representation;
- lowering strategy between authoring and runtime models;
- concrete pathfinding algorithm/library;
- exact Reference state names/transitions/tuning.

### 5.3 Semantic execution contract frozen now

What **is** required now is a representation-neutral bounded semantic resolution contract:

```text
exact owner/revision-bound snapshot + one accepted owner input/occurrence
-> finite deterministic evaluation under declared hard bounds
-> staged typed AI-local plan + staged foreign-domain proposals
-> complete AI-local validation/preflight
-> one atomic AI-local commit OR complete rejection
-> typed normalized outcome
```

Any later representation MUST prove:

- finite validated authored/evaluable structure;
- bounded transitions/nodes/visits/actions/work units per semantic resolution;
- deterministic trigger/guard/selection ordering;
- no recursive/unbounded continuation;
- exact revision/provenance binding;
- deterministic failure/typed outcome;
- owner-local atomicity defined in section 6;
- no direct foreign-domain mutation.

This is the v1 architecture invariant. **FSM identity is not.**

## 6. Final finding 2 — over-budget semantic-resolution atomicity

### 6.1 Selected model

For one GAME-AI semantic resolution, the default is **all-or-nothing for authoritative AI-local mutation**.

A path of the form:

```text
execute authoritative action A
-> execute authoritative action B
-> discover bound exceeded
-> keep A/B and fall back
```

is prohibited.

The successor selects staged/preflighted resolution:

```text
1. bind immutable/revalidatable owner snapshot + exact semantic revisions
2. evaluate into a staged plan while charging all declared work budgets
3. stage candidate AI-local state/timer/memory changes and any foreign-domain proposals
4. validate the complete staged AI-local plan and required resource/ownership/revision constraints
5a. if valid and within bounds -> commit the complete AI-local plan atomically
5b. if invalid/stale/over-budget -> discard the complete staged plan; commit zero AI-local mutation
6. expose one typed normalized result
```

### 6.2 Budget exhaustion

Budget counters are charged during read-only/staged evaluation. If evaluation cannot reach a complete terminal plan before any hard bound is exceeded:

- the staged plan is discarded in full;
- no staged AI-local target/state/memory/timer mutation commits;
- no staged action proposal from that rejected resolution becomes authoritative or publishable as an accepted AI decision;
- the result is a deterministic typed failure such as semantic `BUDGET_EXHAUSTED`;
- there is no recursive continuation of the rejected plan.

A policy may react to that typed failure only through a **separate bounded owner occurrence/resolution** (for example wait/re-evaluate/clear objective) with fresh ordering/budget semantics. The failed resolution itself does not partially mutate into its fallback.

### 6.3 Cross-domain composition

This atomicity is deliberately owner-local; GAME-AI does not invent cross-owner distributed transactions.

A foreign-domain intent (for example an ability request) is a proposal, not the foreign mutation. GAME-AI may commit only local state whose truth does not depend on foreign acceptance. Any local state transition whose semantics require successful foreign execution must wait for the normalized accepted/rejected result returned by the owning domain as a later/current owner input under that domain contract.

Thus downstream rejection never requires GAME-AI to roll back a foreign mutation it did not own.

### 6.4 Partial-plan semantics

A partial-plan model is **not selected** by this successor. If a future mechanic truly requires intentional partial success inside one semantic resolution, it needs a separate material decision and accepted domain-specific contract defining:

- which steps may commit;
- ordering and idempotency;
- failure/retry semantics;
- replay/recovery behavior;
- downstream transaction boundaries;
- evidence that partial success is product-semantic rather than an implementation leak.

Absent that accepted contract, partial authoritative AI-local mutation on resolution failure is prohibited.

## 7. Preserved scripts and auxiliary planning

DUR-04 script/component leaves remain proposal-only:

```text
immutable bounded snapshot + explicit capabilities
-> bounded component
-> typed proposal
-> host/owner/domain validation
-> accepted commit or rejection
```

Script trap, fuel exhaustion, invalid output or capability violation commits zero authoritative AI/game mutation from that proposal.

Pathfinding/planning remains bounded auxiliary work. A worker receives immutable/revalidatable input and returns a proposal only. The current owner revalidates scope ownership generation, actor local generation, source/goal compatibility, map/navigation/content revisions, current request identity, legality and resource bounds before adoption. Stale/late/misrouted proposals are discarded without rollback.

No concrete pathfinding algorithm or library is chosen here.

## 8. Final finding 3 — spawn occupancy retries

### 8.1 Problem with predecessor wording

The predecessor froze occupancy recovery to “one bounded postponed retry/deadline”. That was more specific than available Reference/product evidence and unnecessarily constrained legitimate finite policies.

The successor replaces that clause with a **policy-defined finite retry contract**.

### 8.2 Required finite retry policy

When the primary placement and the bounded candidate alternatives for one placement attempt cannot produce a legal spawn, an authored/accepted occupancy policy MAY schedule postponed retry attempts only when it defines all of:

- **retry count** — a finite configured number of postponed retries/attempts;
- **hard maximum** — an accepted finite ceiling that the configured count cannot exceed;
- **retry window/deadline** — normalized semantic-time window/deadline after which the occurrence cannot retry;
- **deterministic cadence** — exact versioned schedule/rule for retry deadlines, not worker wake-up or wall-clock jitter;
- **deterministic order** — canonical placement candidate ordering/selection semantics for every attempt;
- **terminal disposition** — explicit skip/fail/cancel/reconcile result when count, deadline or hard maximum is reached;
- **stable occurrence identity** — all retries remain the same semantic spawn occurrence unless the owning source explicitly creates a later new occurrence;
- **revision compatibility** — retry/recovery cannot silently reinterpret the same occurrence under incompatible content/policy revisions.

No numeric value is invented by this paper task. Before executable acceptance, configured values and the hard ceiling must be defined in accepted content/resource-limit policy and have boundary tests.

### 8.3 Attempt semantics

A bounded canonical alternative set MAY be evaluated inside one placement attempt without scheduling a new retry occurrence.

Postponed retry attempts:

- use stable retry indices/occurrence provenance;
- enter through normalized owner timer semantics;
- use FND-03 deterministic equal-deadline ordering;
- cannot recursively dispatch an immediate retry loop;
- stop when any configured finite count/window/deadline/hard maximum is exhausted.

If a profile permits randomized placement/selection, randomness must follow SIM purpose-isolated deterministic RNG with stable logical decision identity and bounded candidate space. “Random tile until success” remains prohibited.

### 8.4 Reference posture

Exact Reference retry count, cadence, deadline, placement preference and crash/recovery behavior remain `UNKNOWN` until evidence resolves them. The engine contract allows finite versioned policy; it does not claim any particular policy as Reference.

## 9. Spawn provenance, recovery and GAME-CHANNEL multiplicity — preserved

Every accepted spawn occurrence must retain stable semantic provenance sufficient to distinguish:

- idempotent replay/retry of the same occurrence;
- recovery continuation of the same occurrence;
- a genuinely later occurrence.

NodeId, wall-clock timestamp or current ownership generation alone is insufficient durable source/reward identity.

Each source/encounter with gameplay/value impact must select an explicit compatible recovery class such as ephemeral scope reset only when safe, checkpointed runtime continuity, or durable event occurrence under a named owner. Old-generation work must be fenced.

Value-producing sources require explicit GAME-CHANNEL multiplicity/eligibility classification before activation. ChannelId or restart/recovery generation must not silently become a new reward reset key.

## 10. Controlled actors — preserved

Summons/pets/controlled actors remain server-authoritative actors in the current Channel/Instance owner.

Controller commands are validated normalized requests and retain principal/control provenance sufficient for stale-control rejection and downstream attribution. Clients do not directly set position, target, damage, threat, lifetime or reward credit.

Exact summon/pet lifetime, command vocabulary, persistence, XP/loot attribution and contribution-dedup rules remain downstream/profile-owned.

## 11. Reward/value authority — preserved

GAME-AI never mints item instances, currency, XP or reward eligibility.

Downstream reward owners may consume exact spawn/source/controller provenance together with authoritative combat/death/contribution facts. Retry/replay/recovery of one semantic occurrence must not create a second value settlement merely because local runtime ownership or retry index changed.

## 12. Disconnect/re-entry suppression as downstream legality input — preserved

The accepted PvE re-entry protection decision is an input to GAME-AI target/action legality, not AI-owned protection state.

During an active accepted re-entry protection window:

- a protected character is ineligible for **new monster offensive actions**;
- a monster may continue to perceive the character or retain bounded threat/target memory according to ordinary evidence-backed policy;
- GAME-AI must not clear aggro/threat/encounter state merely because offense is temporarily suppressed;
- already committed pre-protection effects remain governed by their owning combat contract;
- no prohibited attack intent is buffered for burst execution at protection expiry;
- after protection expires, ordinary downstream PvE attack eligibility resumes from the continuing authoritative state.

This successor does not redefine the four-second product decision, eligibility boundary, combat classification or protocol result.

## 13. Resource-bound obligations — preserved and refined

No numeric maxima are guessed here. Before executable `GAME-AI-01` acceptance, accepted resource policy/registry must define hard maxima and boundary tests for at least:

- active AI actors per authoritative scope;
- authored semantic states/nodes/transitions as applicable to the selected implementation;
- semantic evaluation work units/transitions/nodes/visits/actions per resolution;
- AI memory/threat/stimulus entries per actor;
- perception/target candidates per decision;
- pending AI timers/operations per actor/scope;
- queued/in-flight path requests per actor/scope/executor;
- path search work and route result size;
- repath/retry work over a bounded semantic window;
- spawn sources/population/placement candidates per attempt;
- **postponed spawn occupancy retry count and hard maximum per occurrence/window**;
- controlled-actor command backlog;
- inherited script fuel/memory/host-call/query/proposal bounds;
- diagnostic/replay evidence where amplification-prone.

Missing required limits block executable acceptance; this worker does not edit the coordinator/shared resource registry.

## 14. Reference/Evolved mapping — preserved

One engine supports both profiles through versioned policy and evidence, not code forks.

Reference behavior remains fail-closed when evidence is `UNKNOWN`, `CONFLICT` or `PENDING`. Evolved may intentionally use different versioned behavior/path/spawn policies but keeps the same authority, deterministic replay, bounded-resource, provenance and anti-duplication invariants.

## 15. Decisions not taken

This successor deliberately does **not** decide:

- a concrete FSM/BT/statechart/decision-graph runtime representation;
- any AI framework/library;
- a pathfinding algorithm/library;
- physical Rust/content schema representation;
- numeric resource/retry limits;
- exact Reference aggro/path/spawn/retry/tuning behavior;
- persistence DDL;
- GAME-ABILITY/GAME-INTERACTION formulas or semantics;
- reward settlement formulas/keys;
- Platform/production topology;
- coordinator-only backlog/horizon/global-register entries.

No omission here authorizes a permissive implementation default.

## 16. Cross-domain findings — report only

```yaml
cross_domain_finding:
  id: GAME-AI-XD-01
  observed_in_domain: GAME-AI-01
  target_owner: GAME-ABILITY
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md §3.2/§12 and accepted GAME-ABILITY owner contracts
  conflict_or_gap: GAME-AI requires a stable typed action-intent/result and temporary-legality input boundary, including accepted re-entry suppression, but must not own combat legality or mutation
  required_before: executable GAME-AI action integration and Reference/Evolved combat-intent conformance
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-AI-XD-02
  observed_in_domain: GAME-AI-01
  target_owner: GAME-INTERACTION
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md §3.2 and pathfinding owner-revalidation boundary
  conflict_or_gap: route invalidation and environmental interaction facts need a normalized owner boundary; GAME-AI must not define door, teleport or world-interaction semantics
  required_before: executable path adoption/revalidation against dynamic interaction state
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-AI-XD-03
  observed_in_domain: GAME-AI-01
  target_owner: GAME-ITEM/DUR-03/REWARD
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md §9-§11
  conflict_or_gap: controlled-actor contribution and one-semantic-occurrence value settlement remain foreign authority and need explicit downstream deduplication/attribution semantics
  required_before: any AI/spawn-controlled actor can participate in durable loot, XP, currency or reward settlement
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-AI-XD-04
  observed_in_domain: GAME-AI-01
  target_owner: ARCHITECTURE-COORDINATOR/RESOURCE-LIMITS
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md §13
  conflict_or_gap: executable acceptance still lacks concrete hard maxima and boundary tests for AI/path/spawn/retry amplification dimensions, including a finite postponed-spawn retry ceiling
  required_before: executable GAME-AI-01 implementation acceptance
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-AI-XD-05
  observed_in_domain: GAME-AI-01
  target_owner: EVENT/ENCOUNTER
  severity: P2
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md §9 plus preserved boss/encounter ownership boundary
  conflict_or_gap: world-shared durable occurrence and eligibility require a named event/encounter owner where the semantic event spans multiple actors/scopes; GAME-AI cannot invent that durable owner
  required_before: durable multi-actor encounter occurrence, shared eligibility or reward semantics are implemented
  worker_action: REPORT_ONLY
```

These are reports only; this successor does not edit foreign or coordinator-owned surfaces.

## 17. Future acceptance evidence

A future implementation must prove at least:

1. identical owner state/input/revisions produce identical semantic decisions independent of backing iteration order;
2. the chosen implementation representation satisfies finite bounded semantic-resolution limits;
3. budget exhaustion during staging commits zero authoritative AI-local mutation from the rejected resolution;
4. a rejected/over-budget staged resolution emits no accepted action proposal from its discarded partial plan;
5. any fallback after budget failure occurs only through a separate bounded owner resolution;
6. downstream action rejection cannot be bypassed and success-dependent local state waits for normalized downstream outcome;
7. stale target/generation evidence fails closed;
8. identical path input/profile yields deterministic normalized result and stale results are rejected;
9. script failure commits zero authoritative mutation;
10. spawn placement alternatives and retries are finite, deterministic and bounded by count plus deadline/window plus hard maximum;
11. retry/recovery retains one stable spawn occurrence identity and cannot mint duplicate availability/value;
12. missing GAME-CHANNEL multiplicity/eligibility blocks value-source activation;
13. controlled-actor requests do not become client authority;
14. re-entry protection suppresses new monster offense without erasing threat/encounter state or buffering attacks;
15. Reference `UNKNOWN/CONFLICT/PENDING` cannot be promoted to parity by implementation similarity;
16. overload preserves FND-03 control/fencing progress and all declared hard bounds.

## 18. Successor conclusion

`GAME-AI-01` needs a **bounded deterministic semantic execution contract now**, but it does **not** need a normative FSM representation freeze now.

The safe v1 boundary is representation-neutral staged evaluation with complete preflight and all-or-nothing AI-local commit/reject. Spawn occupancy retry is finite and policy-defined across count, time window/deadline, deterministic cadence/order and a hard maximum; exact Reference behavior remains evidence-gated.

All preserved predecessor ownership, pathfinding proposal/revalidation, deterministic targeting, no-value-authority, script proposal-only, provenance/recovery, GAME-CHANNEL multiplicity, controlled-actor provenance, resource-bound and fail-closed invariants remain in force.

This remains a noncanonical worker proposal until Architecture Coordinator audit and merge.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`