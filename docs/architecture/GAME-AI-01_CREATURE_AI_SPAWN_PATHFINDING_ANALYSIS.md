# GAME-AI-01 — Creature AI, Spawn and Pathfinding Analysis

- DecisionStatus: `PROPOSED`
- DeliveryStatus: `OPEN`
- ImplementationStatus: `NOT_STARTED`
- Gate: `GAME-AI-01`
- Issue: `#261`
- Worker lane: `docs/arch-c-game-ai`
- Trusted base: `088b46638ac014cd7928d6b0b75cee44902fe22c`
- Scope: paper-only architecture analysis; runtime/content implementation authority **NONE**
- Canonicality: **NONCANONICAL WORKER PROPOSAL** until Architecture Coordinator acceptance/merge
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Purpose

This analysis resolves the worker-owned `GAME-AI-01` design envelope for authoritative creature AI, spawn/population control and pathfinding without implementing runtime code or redefining foreign gameplay domains.

The target is an architecture that can support Reference and Evolved profiles through one engine while preserving:

- one authoritative owner for every mutation-capable AI occurrence;
- deterministic/replayable decisions;
- bounded CPU, memory, queue, search and script work;
- exact content/template/spawn provenance;
- stale-result rejection across ownership/revision changes;
- explicit Reference evidence gaps instead of guessed parity;
- a narrow intent boundary into combat, interaction, reward and persistence owners.

## 2. Terminology collision: runtime AI is not Game Intelligence AI

`ADR-0006` / Oteryn Game Intelligence defines external read-only analytics/investigation AI. That subsystem is prohibited from live runtime/database mutation, autonomous sanctions, balance changes and deployment.

`GAME-AI-01` uses **runtime AI** to mean server-authoritative simulation logic for creatures, monsters, summons/pets and NPC-local behavior under the current `ChannelRuntime` or `InstanceRuntime` owner.

The two systems MUST remain separate:

```text
runtime GAME-AI
-> authoritative gameplay proposal/resolution inside current runtime owner

Game Intelligence / Investigation AI
-> read-only derived analysis outside gameplay authority
```

No analytics/investigation model output becomes trusted runtime AI authority merely because both are called "AI".

## 3. Evidence and precedence

### PROVEN — binding upstream architecture consumed

- `FND-03_RUNTIME_EXECUTION_CONTRACT.md`: `ChannelRuntime` owns channel-local creatures/spawns/AI; `InstanceRuntime` owns instance-local creatures; mutation crosses one non-interleaved owner boundary; pathfinding/AI planning may run only as non-authoritative auxiliary work and stale results require owner revalidation.
- `GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md`: runtime locality does not imply reward/source multiplicity; value-producing sources require explicit simulation/eligibility/multiplicity policy and ChannelId must not silently become a reward reset key.
- `DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md`: stable `ContentKey`, immutable locked World Bundle provenance and staged activation; scripts are bounded proposal components with no direct gameplay/SQL mutation.
- `SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md`: exact semantic revision binding, owner-local `RuntimeExecutionOrdinal`, purpose-isolated gameplay RNG, deterministic tie-breaks, normalized time and replay provenance.
- accepted partial `GAME-ABILITY-01` baselines: targeting/legality, cast/commit, cooldown/condition and typed effect boundaries remain combat authority; AI cannot bypass them.
- accepted GAME-VISION Reference rule: first Reference target is the immutable Global Tibia cut after the 2026-07-28 maintenance boundary; unknown/conflicting behavior fails closed rather than being guessed from current Global or OTS code.

### DERIVED — GAME-AI consequences

Because FND-03 already assigns creature/spawn/AI ownership to Channel/Instance runtime, GAME-AI does not create a new process/service mutation owner. Because SIM requires future-determining state and exact revisions to be reproducible, target memory, accepted AI timers, spawn occurrence state and behavior-affecting template revisions must be part of deterministic state/replay evidence whenever they can change a future result without a new external fact.

Because DUR-04 scripts are proposal-only, a scriptable AI leaf cannot become a generic mutable game object or bypass the FSM/owner validation boundary.

### UNKNOWN — Reference behavior evidence not established by this task

This task has no sufficient evidence to claim exact Reference values/algorithms for:

- perception range/shape and hidden/invisible target treatment;
- aggro/threat scoring, retention and retarget thresholds;
- monster memory lifetime and home/leash/reset behavior;
- path cost model, corner/diagonal rules, obstacle quirks and path preference tie-breaks;
- spawn counts, respawn delays, occupancy retry rules and crash-reset semantics;
- summon/pet command rights, lifetime and exact XP/loot attribution;
- NPC movement/idle/dialogue coupling;
- boss phase/recovery semantics;
- loot, XP or reward contribution thresholds.

These remain evidence/profile-owned and MUST NOT be marked `PARITY_CONFIRMED` by this architecture package.

## 4. Domain ownership boundary

### 4.1 Local runtime owner

The current authoritative owner of an AI actor is exactly the owner of its spatial simulation:

```text
public-world actor -> current ChannelRuntime(WorldId, ChannelId)
instance actor     -> current InstanceRuntime(WorldId, InstanceId)
```

AI state is actor/scope-local authoritative state. It MUST NOT be mutated by:

- auxiliary pathfinding/planning workers;
- content/script execution threads;
- client callbacks;
- persistence callbacks;
- analytics/investigation AI;
- another Channel/Instance owner.

World-shared boss/event/spawn orchestration is not implicitly GAME-AI-owned. If a named world/encounter owner exists, that owner sends typed normalized facts/commands into the local runtime; local creature mutation remains local.

### 4.2 Foreign-domain outputs

GAME-AI may propose or consume typed facts, but does not own:

- combat legality/formulas/effects/death resolution — GAME-ABILITY/SIM and downstream combat owners;
- item/currency creation/conservation/loot transactions — GAME-ITEM/DUR-03/reward/content owners;
- doors/teleports/world interaction legality — GAME-INTERACTION;
- quest/dialogue/trade durable business state — their owning domains;
- world event durable occurrence/eligibility — event/reward owners;
- persistence schema — DUR;
- analytics/audit schemas — ANL.

## 5. Candidate runtime model

### 5.1 Exact provenance is mandatory

Every active AI actor must be attributable to an immutable behavior-affecting provenance set. The physical Rust types/serialization remain implementation-owned, but the semantic record must identify, as applicable:

```text
semantic runtime scope
current scope ownership generation
actor semantic/local-generation identity
exact World Bundle artifact digest
content_revision
map_revision/navigation revision
ruleset_revision
world_policy_revision
SimulationDeterminismProfileRevision
spawn/source ContentKey + immutable package/revision provenance
AI behavior template ContentKey + immutable package/revision provenance
script component/WIT/script_execution_profile_revision when used
stable spawn/event occurrence context where behavior/reward depends on it
validated controller/principal context for controlled actors
```

A mutable file path, display name, runtime array index or "latest" content lookup is not sufficient provenance.

### 5.2 Runtime-local state

An actor may hold bounded deterministic local state such as:

- current FSM state key;
- current target reference + target local generation/revision evidence;
- bounded threat/stimulus memory;
- home/spawn anchor if the active policy defines one;
- pending authoritative think/action timer semantics;
- path request identity and current validated route cursor;
- finite policy-local counters/hysteresis values;
- compatible controlled-actor principal/command-right evidence where applicable.

Unbounded blackboard maps, arbitrary script globals and process-global mutable AI state are rejected.

## 6. Behavior representation decision

### 6.1 Decision matrix

| Option | Deterministic auditability | Bounded-cost proof | Content expressiveness | Failure isolation | Decision |
|---|---|---|---|---|---|
| Typed bounded FSM | strong: explicit state/transition graph | strong: compile-time graph + per-input transition bound | sufficient for first authoritative package; hierarchical states can be added explicitly | strong | **SELECTED for v1 semantic execution model** |
| Runtime behavior tree | possible but traversal/abort/decorator semantics require a second complex execution contract | weaker until node/depth/visit/continuation rules are frozen | high | medium | **DEFERRED**; no framework/library authority |
| Direct authoritative script | weakens host/domain invariants and cost predictability | bounded only through script profile but broad mutation would violate DUR-04 | high | poor if made owner | **REJECTED as mutation owner** |
| FSM + bounded proposal leaf/script | strong if every leaf returns typed proposal to owner | strong with inherited DUR-04 + AI bounds | high enough for complex authored policies | strong | **ALLOWED extension point** |

### 6.2 Why bounded FSM is selected

The authoritative execution model is a **typed finite state machine with bounded transition work per accepted owner input**.

A behavior template declares a finite state/transition graph. Exact gameplay state names are content/profile-owned; this contract does not invent Reference states. The runtime must provide:

- a finite validated state set;
- a finite validated transition set;
- typed triggers/guards/actions;
- one deterministic initial/fallback state;
- a hard maximum of transitions/actions per authoritative resolution;
- no recursive transition dispatch;
- deterministic state/transition ordering where more than one is eligible;
- atomic rejection before partial mutation when a transition/action plan is invalid.

Behavior trees remain a future authoring option only if a later accepted decision proves need and either:

1. lowers them deterministically into the same bounded execution semantics; or
2. freezes an equally explicit bounded traversal/continuation contract.

No BT library is selected here.

### 6.3 Script composition

A DUR-04 component may implement a bounded decision leaf:

```text
immutable snapshot + explicit capability set + semantic revisions
-> bounded component
-> typed AI proposal
-> FSM/host/domain revalidation
-> authoritative owner resolution
```

A script cannot directly move an actor, set HP, mint loot, write SQL, mutate another runtime, modify arbitrary AI memory or bypass GAME-ABILITY/GAME-INTERACTION legality.

## 7. Authoritative scheduling

GAME-AI introduces no universal fixed game tick.

An AI evaluation/think occurrence is a mutation-capable owner-scoped input/timer under FND-03. It receives `RuntimeExecutionOrdinal` when accepted for resolution.

Each AI timer family must declare an explicit FND-03 catch-up policy. `SKIP_TO_LATEST` is allowed only when skipped evaluations are semantically equivalent maintenance/observation work; it must not silently delete occurrence-producing combat/spawn semantics.

Equal-deadline AI inputs use the FND-03 deterministic timer order. OS wake-up/thread order is never AI priority.

## 8. Perception, aggro, threat and target selection

The architecture freezes a bounded pipeline, not Reference tuning:

```text
bounded candidate enumeration from current authoritative snapshot
-> eligibility/scope/lifecycle/perception filter
-> merge bounded current-target + threat/stimulus memory
-> versioned policy score/priority
-> deterministic stable tie-break
-> target retain/switch/clear decision
-> action intent selection
-> downstream legality revalidation at commit
```

Rules:

1. candidate enumeration is bounded and has stable/canonical order before scoring;
2. only actors valid in the same current simulation scope may become local spatial targets unless a named cross-scope contract exists;
3. stale actor generations/revisions are rejected;
4. target memory is bounded by entries and lifetime/occurrence semantics;
5. score arithmetic follows SIM numeric/formula rules;
6. ties use a stable semantic comparator, never pointer/hash-map/worker order;
7. a retained target is revalidated before movement/action commit;
8. a client cannot author authoritative threat, target priority or NPC hostility directly;
9. ability legality remains downstream and may reject an AI intent without AI bypass;
10. a rejected intent becomes a deterministic typed outcome/stimulus and cannot trigger unbounded immediate retry.

Reference aggro/threat formulas stay `UNKNOWN` until evidence resolves them.

## 9. Memory and leash/reset

AI memory is bounded authoritative runtime state whenever it can alter future behavior. Memory categories and expiry/reset rules are template/profile-owned.

Optional home/leash policy may use a validated spawn/home anchor and bounded distance/time/state conditions. This package does not claim that Reference uses any particular leash radius, reset timeout, healing/reset behavior or retarget rule.

If a reset requires combat-state, HP, condition, reward-contribution or encounter mutation, GAME-AI must issue a typed request/proposal to the owning domain. It cannot directly erase combat/reward state merely by entering an AI state.

## 10. Pathfinding ownership and execution

### 10.1 Pathfinding is auxiliary proposal work

Expensive path search must not run unbounded on the Channel/Instance writer.

A path request binds immutable/revalidatable context including, as applicable:

```text
scope identity
scope ownership generation
actor local generation
source actor/state revision
start + normalized movement capabilities
goal/goal-revision evidence
map/navigation/content revision
behavior/ruleset/SIM revision set
logical path-operation identity
budget profile
```

An auxiliary worker receives immutable search input and returns a route proposal/result only.

### 10.2 Result revalidation

Before adopting any route, the current owner revalidates:

- same semantic scope and current ownership generation;
- actor still exists with matching local generation;
- source/goal state remains compatible;
- map/navigation/content revision remains compatible;
- request is still current and not cancelled/superseded;
- route satisfies current movement/legality constraints;
- returned work stayed within declared resource bounds.

Late/stale/misrouted results are discarded without rollback because they never had mutation authority.

### 10.3 Determinism

The selected pathfinding implementation is not frozen. Any implementation must define a versioned deterministic search profile covering:

- movement/cost semantics;
- neighbor enumeration;
- equal-cost tie-breaks;
- bounded termination;
- route canonicalization;
- supported authoritative targets.

The same accepted search input/profile must produce the same normalized path result on supported targets. Worker completion order must not select which route becomes gameplay authority.

### 10.4 Repath and failure

Repath is triggered only by typed deterministic conditions such as target/goal revision change, route invalidation, explicit semantic deadline or accepted movement failure. It is never an unbounded "try again until it works" loop.

Terminal path outcomes must be typed at least semantically as:

- route found;
- no route under current semantics;
- bounded search budget exhausted;
- cancelled/superseded;
- stale revision/generation;
- capacity unavailable/overloaded;
- unsupported/invalid request.

On failure, the actor enters a finite policy-defined fallback such as wait/re-evaluate/clear objective. The fallback itself is bounded and deterministic.

## 11. Spawn and population model

### 11.1 Spawn definitions

Spawn/population definitions are immutable content semantics identified by stable `ContentKey`/package revision and included in the World Bundle. Relevant definitions must declare, as applicable:

- simulation scope (`CHANNEL`, `INSTANCE` or a named external world/event owner with local projection);
- actor/template reference and behavior template reference;
- bounded population/placement rules;
- candidate placement region/cells in deterministic order or deterministic selection policy;
- respawn/scheduling semantics;
- occupancy handling;
- recovery class;
- GAME-CHANNEL multiplicity/eligibility classification for value-producing sources;
- required semantic revision compatibility.

Missing mandatory classification or incompatible references block activation; there is no permissive runtime default for value-producing sources.

### 11.2 Spawn occurrence provenance

Every spawned actor must retain enough stable semantic occurrence context to distinguish one accepted spawn occurrence from duplicate/retry/recovery work.

GAME-AI does not mandate a new globally public UUID. A valid identity may be derived from an owning source/event occurrence model, provided it is stable across idempotent retry/replay and cannot be confused with a later occurrence. A runtime ownership generation alone is insufficient as durable reward identity because generation changes during valid recovery.

### 11.3 Occupancy

Placement is revalidated against current authoritative spatial state before spawn commit.

If the preferred placement is unavailable, the policy must use a finite deterministic outcome:

- try a bounded canonically ordered alternative set;
- postpone through one bounded owner timer; or
- fail/skip that occurrence according to explicit policy.

Unbounded random tile probing, recursive immediate respawn attempts or displacement of players to force a spawn are prohibited.

### 11.4 Respawn/recovery classes

Each spawn/encounter family must explicitly select a compatible recovery class rather than inherit an accidental process-restart default:

- `EPHEMERAL_SCOPE_RESET` — state may reconstruct from immutable content on scope activation only when product/economy semantics explicitly permit reset;
- `CHECKPOINTED_RUNTIME_CONTINUITY` — future-determining local spawn/AI state is recovered from deterministic runtime checkpoint/replay evidence;
- `DURABLE_EVENT_OCCURRENCE` — a named event/world owner persists occurrence/eligibility and commands local projection.

High-impact/value-producing sources cannot silently use `EPHEMERAL_SCOPE_RESET` if reset can duplicate rewards or availability.

## 12. Summons, pets and controlled actors

Controlled actors remain server-authoritative AI actors in the current Channel/Instance owner.

A summon/pet command must arrive as a validated normalized command/fact and bind current control-right evidence. A client may request a target/mode/action but never writes movement, threat, combat result or reward attribution directly.

The AI actor retains bounded controller/principal provenance needed to route attribution. Exact ownership persistence, command rights, XP sharing, loot eligibility and despawn rules remain downstream gameplay/reward decisions.

One controlled actor must not create duplicate principal credit merely because both owner and controlled actor appear in contribution evidence. Exact dedup/business rules are foreign-domain work.

## 13. NPC boundary

NPC-local idle/movement/perception behavior may use the same bounded FSM/pathing kernel.

Dialogue, trade, quest, bank, economy and durable social state are not absorbed into GAME-AI. A scripted NPC interaction remains a DUR-04 proposal to the owning domain. No NPC script receives ambient SQL, filesystem/network or mutable global Game authority.

## 14. Boss and encounter extension point

Actor-local phase behavior may be represented by the bounded FSM when all affected state is genuinely actor-local.

If a boss phase coordinates multiple actors, objectives, world events, durable eligibility or rewards, the encounter/event owner must hold that semantic state. GAME-AI consumes normalized phase facts and emits typed actor intents; it does not become a hidden cross-owner transaction coordinator.

Crash recovery must reconstruct one semantic occurrence and fence old-generation actors/work before new local projection can publish authority.

## 15. Loot/reward eligibility and abuse boundary

GAME-AI does not mint loot, XP, currency or item instances.

The downstream reward path must consume authoritative combat/death outcome plus exact spawn/source/controller provenance. The architecture requires these anti-duplication properties where AI provenance participates:

1. one authoritative death/source occurrence cannot settle the same reward twice after retry/replay/recovery;
2. Channel multiplicity/eligibility policy is explicit before a value-producing source is activated;
3. client-reported damage/participation is never trusted reward evidence;
4. summon/pet contribution is attributable to an accepted principal model without double-counting one semantic contribution;
5. stale old-generation death/spawn work cannot create a new reward occurrence;
6. AI reset/leash/despawn cannot erase or fabricate reward eligibility outside the reward owner;
7. analytics/investigation output cannot mint or revoke rewards.

Exact loot tables, contribution thresholds, XP division and transaction keys remain GAME-ITEM/DUR-03/content/reward-owned.

## 16. Overload and degradation

The authoritative Channel/Instance writer must remain serviceable under path/AI pressure.

Rules:

- every AI/path/spawn queue and pending set is bounded;
- expensive path search/planning runs in bounded auxiliary capacity;
- no per-actor unbounded task spawn;
- no unbounded retry/repath loop;
- no unbounded candidate/threat/blackboard growth;
- control/fencing work retains FND-03 reserved capacity;
- capacity refusal returns a typed deterministic outcome to AI policy;
- best-effort precomputation may be dropped/coalesced only when it cannot change semantic outcome;
- already accepted semantic timers/actions are never silently discarded merely to reduce load;
- overload must not convert Reference behavior into an untracked Evolved difference.

## 17. Required resource-limit dimensions

No numeric values are invented by this paper contract. Before implementation can claim `GAME-AI-01`, hard maxima and boundary tests must exist in `RESOURCE_LIMITS_REGISTRY.json` or an accepted superseding registry for at least:

- active AI actors per authoritative scope;
- states/transitions per behavior template;
- transitions/actions per authoritative AI resolution;
- AI memory/threat/stimulus entries per actor;
- spatial candidates returned/evaluated per decision;
- pending AI timers/operations per actor/scope;
- queued and in-flight path requests per actor/scope/GameNode executor;
- path search nodes/work units per request;
- path result/route length and bytes;
- repath/retry attempts over a bounded semantic window;
- spawn sources/controllers per scope;
- spawn population and placement candidates/attempts per source/resolution;
- controlled-actor command backlog;
- AI/script proposal size and inherited DUR-04 fuel/memory/host-call/query bounds;
- diagnostic/replay evidence volume when attacker-amplifiable.

`CROSS_DOMAIN_FINDING / REPORT_ONLY`: this worker does not edit the shared resource registry because it is outside the allocated lane. The implementation owner/coordinator must register concrete limits before executable acceptance.

## 18. Failure and recovery analysis

| Failure | Required disposition |
|---|---|
| malformed/unresolved AI/spawn template | fail compile/staging/activation before publication |
| incompatible behavior/SIM/content/script profile | fail activation; no silent reinterpretation |
| actor/template reference missing at runtime | fail actor/source closed; no default behavior mutation |
| FSM transition/action bound exceeded | reject remaining plan deterministically; no recursive continuation |
| candidate query exceeds safety bound | fail the decision/query unless an explicit canonical bounded-selection policy exists; do not arbitrary-truncate by storage order |
| path queue full | no unsafe worker admission; return typed capacity outcome |
| path work exceeds search budget | typed budget-exhausted result; no partial route authority |
| stale path result after target/revision/fence change | discard proposal; no rollback needed |
| actor despawns while worker runs | result becomes stale/cancelled by actor local generation |
| script trap/fuel exhaustion/invalid proposal | zero proposal mutation commits; deterministic failure outcome |
| ownership generation changes | old timers/results/actors cannot publish new authority |
| recovery lacks required future-determining AI/spawn state | scope/source cannot claim equivalent recovery until reconciled/fail-closed |
| high-impact source lacks multiplicity/eligibility policy | source activation blocked |
| reward outcome is ambiguous after retry/recovery | reward owner must reconcile one occurrence; AI cannot re-mint |
| foreign dependency unavailable | actor/source follows explicit bounded degraded/failure policy; no hidden infinite retry |

## 19. Reference versus Evolved mapping

| Dimension | Reference | Evolved | Shared engine invariant |
|---|---|---|---|
| ownership | same accepted runtime scope | same | one current authoritative owner |
| behavior representation | evidence-backed behavior encoded in bounded FSM/policies | may intentionally use different versioned policies | bounded deterministic execution |
| aggro/threat/retarget | exact semantics remain `UNKNOWN` until evidenced | explicit versioned tuning/difference | stable candidate/filter/score/tie-break pipeline |
| path semantics | exact parity-sensitive path behavior remains evidence-gated | may use intentional versioned path policy | bounded deterministic search + stale-result rejection |
| spawn/respawn/occupancy | exact values/rules evidence-gated | explicit authored policy | immutable provenance + bounded placement + recovery class |
| boss/encounter behavior | parity requires evidence per exercised behavior | explicit declared difference | actor-local AI cannot steal event/reward ownership |
| summon/pet control/attribution | evidence-gated | explicit policy | server authority + validated principal provenance |
| loot/XP | foreign-domain Reference evidence required | foreign-domain policy | AI never directly mints value |
| overload | must not silently alter claimed parity | may expose explicit tested policy | writer safety and hard bounds |

An exercised Reference behavior may be enabled as parity only when its case/evidence classification permits it. Architecture completeness is not behavior-parity completeness.

## 20. Decision summary

### SELECTED

- Channel/Instance owner is the only mutation authority for local AI/spawn state.
- Typed bounded FSM is the v1 semantic behavior execution model.
- Optional scripts/complex planning are bounded proposal components only.
- Pathfinding is bounded auxiliary work with owner revalidation and deterministic search profile.
- Spawn definitions are immutable content with exact bundle/template/source provenance.
- Value-producing spawn sources require explicit GAME-CHANNEL multiplicity/eligibility classification.
- AI memory, target state, accepted timers and spawn occurrence state participate in deterministic state when future-determining.
- Controlled actors bind validated controller/principal provenance.
- AI never directly owns combat formula/effect, interaction, durable reward/value or persistence mutation.

### DEFERRED

- behavior-tree authoring/runtime adoption;
- concrete Rust AI/pathfinding libraries;
- concrete pathfinding algorithm;
- numeric budgets;
- physical AI/spawn content schema/serializer;
- exact Reference behavior/tuning where evidence is absent;
- exact world-event/boss persistence owner APIs;
- exact summon/pet reward/accounting business rules.

### REJECTED

- unbounded pathfinding/planning on the channel writer;
- direct authoritative mutation from worker/script/client callback;
- process-global mutable AI state/RNG;
- runtime "latest" template lookup;
- unbounded behavior tree traversal or recursive FSM transition dispatch;
- missing reward multiplicity policy fallback;
- arbitrary candidate truncation based on unordered storage;
- AI-owned direct loot/XP/item/currency minting;
- analytics/investigation AI feeding live mutation authority.

## 21. Cross-domain findings — report only

1. `CROSS_DOMAIN_FINDING / REPORT_ONLY — GAME-ABILITY`: GAME-AI requires a stable typed server action-intent/rejection boundary that preserves accepted targeting/cast/cooldown/effect authority. Whole-gate GAME-ABILITY reconciliation remains sibling-owned.
2. `CROSS_DOMAIN_FINDING / REPORT_ONLY — GAME-INTERACTION`: dynamic doors/teleports/fields may invalidate routes or require typed interaction actions. GAME-AI needs normalized nav/interaction invalidation facts but must not define door/teleport semantics.
3. `CROSS_DOMAIN_FINDING / REPORT_ONLY — GAME-ITEM/DUR-03/reward`: exact loot/XP contribution, summon principal attribution and one-settlement idempotency require a downstream reward contract; GAME-AI supplies provenance only.
4. `CROSS_DOMAIN_FINDING / REPORT_ONLY — EVENT/ENCOUNTER`: world-shared boss phases, occurrence persistence and shared eligibility need a named world/event owner instead of accidental AI ownership.
5. `CROSS_DOMAIN_FINDING / REPORT_ONLY — RESOURCE LIMITS`: concrete AI/path/spawn hard maxima must be added to the shared registry before implementation acceptance.
6. `CROSS_DOMAIN_FINDING / REPORT_ONLY — ANL`: if production investigation needs durable AI decision traces, event schema/retention/privacy remains ANL-owned; AI diagnostics must not invent a parallel audit vocabulary.

## 22. Architecture acceptance scenarios

A future implementation must provide deterministic evidence for at least:

1. two identical accepted snapshots/revisions produce the same target selection and state transition;
2. equal-score target candidates resolve by the declared stable comparator under shuffled backing storage;
3. stale target/local-generation evidence is rejected before action commit;
4. an AI intent rejected by GAME-ABILITY cannot bypass legality or spin recursively;
5. same path input/profile yields the same normalized route result on supported targets;
6. a path result from an old ownership generation is discarded;
7. a path result after target/map revision change is discarded or explicitly revalidated under a compatible rule;
8. search-budget exhaustion produces one bounded failure with no writer stall;
9. actor despawn while path work is in flight cannot mutate a recycled actor slot;
10. malformed/oversized FSM template is rejected before activation;
11. FSM transition/action bound prevents recursive/unbounded authored behavior;
12. script trap/fuel exhaustion commits zero AI/gameplay proposal mutation;
13. spawn occupancy alternatives are finite, canonically ordered and reproducible;
14. missing multiplicity/eligibility metadata blocks a value-producing source;
15. recovery fences old-generation spawn/AI work and reconstructs one semantic source occurrence;
16. controlled-actor client command cannot directly set authoritative position/target/damage;
17. controlled-actor contribution provenance cannot mint a second reward merely through duplicate owner+pet representation;
18. overload cannot exhaust the owner control/fencing lane or create unbounded path queues;
19. a Reference case classified `UNKNOWN/CONFLICT/PENDING` cannot be claimed `PARITY_CONFIRMED` by implementation similarity;
20. deterministic replay of future-determining AI/spawn state reaches the same canonical state hash/result sequence.

## 23. Worker conclusion

The bounded FSM + proposal-only path/script architecture is sufficient to freeze the `GAME-AI-01` system boundary without inventing Reference mechanics or implementation technology.

The remaining unknowns are predominantly evidence/tuning, downstream reward/interaction/event ownership and implementation-bound numeric/resource choices. They do not justify a second mutation owner, unbounded behavior engine or direct script/client authority.

This document remains a worker proposal until Architecture Coordinator audit and merge.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`
