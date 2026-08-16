# GAME-AI-01 — Creature AI, Spawn and Pathfinding Contract Candidate

- DecisionStatus: `PROPOSED`
- DeliveryStatus: `OPEN`
- ImplementationStatus: `NOT_STARTED`
- Gate: `GAME-AI-01`
- Successor issue: `#275`
- Successor task: `OTV2-20260815-game-ai-successor`
- Predecessor issue: `#261`
- Predecessor PR: `#272`
- Predecessor final reviewed head: `f977a2865c6210f2962a24fa9c00d556acf76122`
- Analysis source: `GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_ANALYSIS.md`
- Scope: paper-only candidate contract; executable runtime/content authority **NONE**
- Canonicality: **NONCANONICAL WORKER PROPOSAL** until Architecture Coordinator acceptance/merge
- Merge authority: `ARCHITECTURE_COORDINATOR_ONLY`

## 1. Contract purpose and successor status

This candidate is a new bounded successor/re-scope for the same stable `GAME-AI-01` architecture gate. It is not a sixth repair cycle on predecessor task `OTV2-20260815-game-ai-architecture`.

The predecessor recorded five repair cycles and received a final coordinator `BLOCKED` disposition. That history remains material and MUST NOT be reset or hidden. #261/#272 remain under Architecture Coordinator lifecycle authority.

This successor changes only the final predecessor findings concerning representation decision timing, over-budget resolution atomicity, finite spawn occupancy retries and delivery-governance truth. All preserved GAME-AI ownership and cross-domain boundaries below remain consistent with the predecessor package.

`MUST`, `MUST NOT`, `SHOULD` and `MAY` are normative inside this candidate proposal. They become canonical only if the Architecture Coordinator accepts/merges them.

## 2. Composed upstream contracts

This candidate composes with and does not replace:

- FND-03 runtime owner/order/timer/fencing/auxiliary-work semantics;
- SIM-DETERMINISM-01 deterministic ordering, revisions, normalized time, bounded work and replay semantics;
- GAME-CHANNEL-01 simulation locality and value-source multiplicity/eligibility semantics;
- DUR-04 immutable content and proposal-only bounded scripting/component semantics;
- accepted GAME-ABILITY-01 partial targeting/legality/cast/cooldown/effect owner baselines;
- GAME-ITEM/DUR-03 and reward-owner value conservation/settlement boundaries;
- accepted disconnect/re-entry PvE protection owner decisions;
- future GAME-INTERACTION/event/encounter contracts at their owning boundaries.

No runtime implementation, physical schema, concrete framework/library, path algorithm, DDL, Platform or production authority is granted.

## 3. Core authority invariant — preserved

For every local runtime AI actor:

```text
one current semantic ChannelRuntime or InstanceRuntime owner
+ one current ownership generation
+ one bounded deterministic semantic AI state
+ one exact behavior-affecting provenance/revision set
-> one authoritative local mutation boundary
```

Public-world AI/spawn state belongs to the current `ChannelRuntime`. Instance-local AI/spawn state belongs to the current `InstanceRuntime`.

A path/planning worker, script/component worker, client, persistence callback, analytics/Game Intelligence system or foreign runtime MUST NOT commit local AI/spawn mutation directly.

A world/event/encounter owner MAY send normalized facts/commands into the current local owner but MUST NOT silently turn GAME-AI into a cross-owner transaction coordinator.

## 4. Immutable actor/template/spawn provenance — preserved

Every active AI actor/source MUST bind enough immutable semantic provenance to reproduce and attribute future-affecting behavior, including as applicable:

- semantic runtime scope and current ownership generation;
- actor semantic identity/local generation fence;
- exact World Bundle artifact/content revision;
- map/navigation revision;
- ruleset/world-policy/SIM determinism profile revision;
- spawn/source stable `ContentKey` plus package/revision provenance;
- behavior-template stable `ContentKey` plus package/revision provenance;
- script artifact/WIT/execution-profile revision when used;
- stable spawn/event occurrence context when retry/recovery/reward behavior depends on it;
- validated controller/principal context for controlled actors.

Runtime index, file path, display name, NodeId, worker identity or mutable `latest` lookup MUST NOT substitute for semantic provenance.

A retry, recovery or delayed result MUST NOT silently reinterpret one accepted semantic occurrence under an incompatible newer revision.

## 5. Behavior representation decision timing

### 5.1 Decision

**Must decide now whether typed bounded FSM is the normative v1 representation? `NO`.**

No current paper-only downstream gate is blocked specifically on FSM identity. The future executable `GAME-AI-01` acceptance proof requires a concrete implementation to prove this contract, but it does not require that implementation to be an FSM if another representation proves the same semantics.

Therefore this candidate MUST NOT freeze FSM, behavior tree, statechart, decision graph, Rust enum layout, scheduler type or any concrete framework/library as normative v1 representation.

### 5.2 Representation-neutral semantic execution contract

Any accepted implementation representation MUST realize one semantic resolution equivalent to:

```text
exact owner/revision-bound snapshot
+ one accepted owner input/timer/outcome occurrence
-> finite deterministic bounded evaluation
-> staged typed AI-local plan + staged typed foreign-domain proposals
-> complete applicable preflight/validation
-> atomic AI-local commit OR complete rejection
-> one typed normalized outcome
```

The representation MUST provide/prove:

- finite validated authored/evaluable structure;
- explicit hard bound on work per semantic resolution, expressed through the accepted resource-limit profile;
- deterministic trigger/guard/node/action eligibility and tie-break ordering;
- no recursive or otherwise unbounded continuation inside one resolution;
- exact semantic revision/provenance binding;
- deterministic typed failure outcomes;
- owner-local atomicity from section 6;
- no direct foreign-domain mutation authority.

### 5.3 Superseding evidence and migration discipline

A later contract MAY freeze a concrete representation only when it records the mandatory decision-timing evidence, including the specific downstream proof blocked without the choice, migration/reversal cost, superseding evidence and deliberately unresolved subjects.

Sufficient evidence MAY include representative behavior-corpus expressiveness, deterministic/replay proof, bounded-cost/performance evidence, recovery/continuation identity needs or authoring/tooling evidence together with a reviewed compatibility/migration plan.

Premature representation coupling to content schema, persisted/checkpointed state identity, replay fixtures, diagnostics or Studio tooling is prohibited by this candidate.

### 5.4 Deliberately undecided

This candidate deliberately leaves undecided:

- FSM vs behavior tree vs statechart/decision graph/other finite representation;
- concrete AI framework/library;
- physical Rust types/enums/scheduler;
- authoring UI and lowering strategy;
- exact Reference state/transition names and tuning.

## 6. Semantic-resolution atomicity

### 6.1 Default rule

One GAME-AI semantic resolution MUST be **all-or-nothing for authoritative AI-local mutation** unless a later accepted domain-specific contract explicitly defines intentional partial-plan semantics.

The following pattern is prohibited:

```text
commit action/state change A
-> commit action/state change B
-> discover work/transition/action bound exceeded
-> retain A/B
-> fallback
```

### 6.2 Required staged/preflight flow

The owner MUST resolve an AI occurrence through semantics equivalent to:

1. bind the current owner/scope/generation and exact semantic revisions;
2. obtain the immutable/revalidatable snapshot required by the policy;
3. evaluate only into a **staged** plan while charging all applicable hard work budgets;
4. stage AI-local candidate mutations and foreign-domain proposals without committing them;
5. validate the complete staged AI-local plan, including current ownership/generation/revision compatibility and required resource bounds;
6. if the complete plan is valid and within bounds, commit its authoritative AI-local mutations as one owner-local resolution;
7. otherwise discard the staged plan in full and commit zero AI-local mutation from that resolution;
8. return one deterministic typed normalized outcome.

No action/state/memory/timer mutation from a still-abortable semantic resolution may become authoritative before the resolution passes its complete applicable preflight.

### 6.3 Over-budget behavior

If evaluation reaches any hard semantic-resolution bound before producing a complete valid terminal plan:

- the result MUST be a typed deterministic budget-exhausted failure;
- all staged AI-local mutation MUST be discarded;
- no partial staged action proposal from that rejected resolution may be published as an accepted AI decision;
- the rejected plan MUST NOT recursively continue;
- the actor's authoritative AI-local state remains as it was before that rejected resolution, except for owner/execution evidence whose semantics are independently defined by FND-03/SIM and are not a gameplay action from the rejected plan.

A policy MAY react to the budget-exhausted outcome only through a **separate bounded owner occurrence/resolution** with fresh deterministic ordering and budget accounting. The fallback is not a partially committed tail of the failed resolution.

### 6.4 Cross-domain proposal rule

Atomicity here is owner-local. This contract does not create cross-owner distributed transactions.

A GAME-ABILITY/GAME-INTERACTION/reward intent produced by AI is a staged proposal until the AI resolution is accepted. It is not the foreign-domain mutation.

GAME-AI MUST NOT commit local state whose semantic truth depends on successful foreign execution before receiving the owning domain's normalized accepted/rejected result. Success-dependent state transitions consume that result as a subsequent/current normalized owner input according to the owning contract.

### 6.5 Partial-plan exception policy

No partial-plan exception is accepted here.

A future intentional partial-success mechanic requires a separate material architecture decision that defines at least step ordering, allowed commit subsets, idempotency, failure/retry semantics, replay/recovery behavior, transaction ownership and product evidence justifying partial success.

Absent that accepted contract, partial authoritative AI-local mutation after validation/budget failure MUST NOT occur.

## 7. AI scheduling and time — preserved

GAME-AI MUST NOT introduce a universal fixed simulation tick.

Mutation-capable think/evaluation/spawn/respawn/retry occurrences enter through FND-03 owner-scoped normalized input/timer semantics and receive deterministic owner-local ordering when accepted.

Gameplay deadlines/windows use normalized monotonic/semantic time, not direct wall-clock reads. Repeating timer families require an explicit bounded catch-up policy; skipped evaluations MUST NOT silently suppress required occurrence-producing gameplay semantics.

## 8. Perception, threat and target selection — preserved

The authoritative target-selection pipeline MUST remain bounded and deterministic:

```text
bounded candidate enumeration
-> scope/lifecycle/perception eligibility
-> bounded current-target/threat/stimulus memory
-> versioned policy scoring/priority
-> stable semantic tie-break
-> retain/switch/clear decision
-> typed downstream action intent
```

Requirements:

- candidate enumeration and memory are bounded;
- canonical ordering exists before order-sensitive evaluation;
- stale scope/local-generation/revision evidence fails closed;
- pointer/hash/thread/worker/storage-default order MUST NOT select a target;
- score arithmetic and any gameplay RNG obey SIM;
- current target/action eligibility is revalidated at the owning boundary;
- downstream rejection cannot be bypassed and cannot cause unbounded immediate retry.

Exact Reference aggro/threat/retarget/memory/leash semantics remain evidence-gated.

## 9. Disconnect/re-entry PvE suppression — preserved downstream legality input

GAME-AI does not own re-entry protection eligibility, duration or protocol state.

When the accepted downstream protection fact says a character is inside the active PvE re-entry protection window:

- monsters MUST NOT begin a new offensive action against that protected character;
- a monster that already had the character in threat/target memory MAY retain that bounded memory according to ordinary evidence-backed policy;
- GAME-AI MUST NOT clear threat, aggro, encounter state, position or committed gameplay history solely because new offense is temporarily suppressed;
- a prohibited attack intent MUST NOT be buffered for automatic burst execution when protection expires;
- already committed pre-protection effects remain with their owning combat/effect contract;
- after the protection fact expires, normal PvE target/action eligibility resumes from the continuing authoritative state.

The exact four-second owner decision and eligibility rules are consumed, not redefined, by this contract.

## 10. Pathfinding — preserved proposal/revalidation model

Potentially expensive pathfinding MUST execute only as bounded auxiliary proposal work.

A path request MUST bind enough immutable/revalidatable context to detect stale work, including as applicable scope identity, ownership generation, actor local generation, source state revision, start/movement capability, goal/revision evidence, map/navigation/content revision, behavior/ruleset/SIM context, logical work identity and budget profile.

The current owner MUST revalidate a returned route before adoption, including current ownership/generation, actor/source/goal compatibility, map/navigation revision, current request identity, movement legality and resource bounds.

Stale, late, misrouted or superseded proposals are discarded without rollback because they never had mutation authority.

The accepted path profile MUST define deterministic neighbor/cost/tie/bounded-termination/route-normalization semantics. This contract selects no concrete pathfinding algorithm/library.

Repath requires a typed deterministic trigger and cannot become an unbounded retry loop.

## 11. Scripts/components — preserved proposal-only model

A DUR-04 component MAY participate only as bounded proposal work:

```text
immutable snapshot + explicit capabilities + exact revisions
-> bounded component
-> typed proposal
-> host/owner/domain validation
-> accepted commit or rejection
```

A script/component MUST NOT directly move actors, mutate arbitrary runtime state, write SQL, mint value or bypass GAME-ABILITY/GAME-INTERACTION legality.

Trap, fuel exhaustion, invalid proposal or capability violation commits zero authoritative mutation from the rejected proposal.

## 12. Spawn definition and provenance — preserved

Every spawn/population source MUST be immutable authored content bound to stable content/package/World Bundle provenance.

A source MUST declare as applicable:

- simulation scope;
- actor and behavior-template references;
- bounded population/placement constraints;
- deterministic placement candidate/selection semantics;
- respawn/timer semantics;
- occupancy/retry policy;
- recovery class;
- required semantic revisions/capabilities;
- GAME-CHANNEL multiplicity/eligibility classification for value-producing sources.

Missing mandatory classification, invalid bounds or incompatible references/revisions block staging/activation. There is no permissive runtime multiplicity default for a value-producing source.

Every accepted spawn occurrence MUST carry stable semantic occurrence context sufficient to distinguish retry/replay/recovery of the same occurrence from a genuinely later occurrence. NodeId, wall-clock time and ownership generation alone MUST NOT become durable reward/source identity.

## 13. Spawn occupancy and finite retry policy

### 13.1 Placement attempt

Spawn commit MUST revalidate current spatial legality/occupancy.

One placement attempt MAY examine a finite bounded canonical alternative set under deterministic selection semantics. Failure to find a legal placement in that bounded set ends that attempt; it MUST NOT expand into unbounded tile probing.

### 13.2 Postponed retry policy

If the source policy permits postponed retries, the policy MUST define all of:

1. `configured_retry_count` or semantically equivalent finite retry-count field;
2. an accepted finite **hard maximum** that the configured retry count cannot exceed;
3. a normalized retry window/deadline;
4. deterministic versioned retry cadence/schedule;
5. deterministic/canonical candidate ordering or bounded deterministic selection semantics on each attempt;
6. stable semantic occurrence identity across all retries;
7. stable retry index/attempt identity sufficient for replay and deterministic RNG isolation when applicable;
8. semantic revision-compatibility rules across delayed retry/recovery;
9. explicit terminal skip/fail/cancel/reconcile disposition when count, deadline/window or hard maximum is exhausted.

A configured retry policy that lacks any required finite bound or terminal disposition MUST fail activation/validation rather than default to retrying.

### 13.3 Hard-stop rules

Postponed retries MUST stop when the earliest applicable terminal condition is reached, including:

- configured retry count exhausted;
- retry window/deadline expired;
- shared/accepted hard maximum reached;
- source occurrence cancelled/superseded;
- incompatible revision/recovery condition prevents safe continuation.

Unbounded immediate retries, recursive retry dispatch and “random candidate until success” loops are prohibited.

If a profile defines randomized placement, it MUST use SIM-compliant purpose-isolated deterministic RNG over a bounded candidate space with stable logical decision identity; randomness does not remove finite count/window/hard-max obligations.

### 13.4 Reference evidence

Exact Reference retry count, cadence, window/deadline, placement ordering and recovery behavior remain `UNKNOWN` until evidence proves them. This candidate authorizes only the finite policy shape, not a guessed Reference value.

## 14. Spawn recovery classes — preserved

Each spawn/encounter family whose failure/restart behavior can affect gameplay/value MUST select an explicit compatible recovery class rather than inherit an accidental process-reset default.

Semantically valid classes may include:

- ephemeral scope reset only when product/economy semantics explicitly permit it and it cannot create forbidden duplicate availability/value;
- checkpointed runtime continuity for future-determining local state;
- durable event occurrence under a named event/world owner.

Old-generation actors/work are fenced before new local projection becomes authoritative. High-impact/value-producing sources MUST NOT silently reset if reset can duplicate reward/eligibility.

## 15. GAME-CHANNEL multiplicity/eligibility — preserved

Channel locality MUST NOT silently imply one independent reward/value source per channel.

A value-producing AI/spawn source MUST have explicit GAME-CHANNEL simulation/eligibility/multiplicity classification before activation. ChannelId, local ownership generation, retry index or process restart MUST NOT silently become a reward reset/settlement key.

## 16. Controlled actors — preserved

Summons/pets/controlled actors remain server-authoritative actors in the current Channel/Instance owner.

Controller/client commands are validated normalized requests bound to current control-right/principal evidence. The client MUST NOT directly author position, target, threat, damage, lifetime or reward credit.

Controlled actors retain enough principal/control provenance for stale-control rejection and downstream attribution. Exact command vocabulary, lifetime, persistence, XP/loot attribution and contribution-dedup rules remain downstream/profile-owned.

## 17. NPC and boss/encounter boundaries — preserved

NPC-local idle/movement/perception MAY reuse the accepted bounded semantic execution/pathing contract. Dialogue, trade, quest, bank, economy and durable business state remain with their owning domains.

Actor-local boss phase behavior MAY be represented locally only when the affected semantic state is genuinely actor-local. Multi-actor objectives, durable world event occurrence, shared eligibility and reward settlement require a named encounter/event owner.

GAME-AI MUST NOT invent cross-owner atomicity.

## 18. Combat/action boundary — preserved

GAME-AI MAY select a typed action/ability intent. It MUST NOT directly:

- apply damage/healing/effects;
- bypass targeting/range/line-of-sight/cast legality;
- change cooldown/charge/condition state outside the owning contract;
- declare death or reward settlement independently;
- turn downstream rejection into local legality override.

The owning domain validates/commits/rejects the proposal. GAME-AI consumes the normalized outcome under the staged/atomic rules in section 6.

## 19. Loot, XP and reward boundary — preserved

GAME-AI MUST NOT mint item instances, currency, XP, loot or reward eligibility.

Where AI/spawn provenance participates in downstream reward logic, the composed system MUST prove at least:

- one semantic source/death occurrence settles at most once under retry/replay/recovery;
- source multiplicity/eligibility is explicit under GAME-CHANNEL;
- stale old-generation work cannot settle value;
- client-authored contribution is not trusted;
- controlled-actor contribution maps through a defined principal model without accidental double credit;
- leash/reset/despawn cannot erase/fabricate reward eligibility outside the reward owner.

Exact formulas, thresholds and transaction identities remain foreign-domain work.

## 20. Overload and degradation — preserved

GAME-AI MUST preserve FND-03 owner/control/fencing progress under overload.

At minimum:

- AI/path/spawn/retry queues and pending sets are bounded;
- expensive search/planning uses bounded auxiliary capacity;
- no per-actor unbounded task spawn exists;
- candidate/memory/threat collections are bounded;
- capacity exhaustion returns typed deterministic outcomes;
- ordinary AI traffic cannot consume reserved control/fencing capacity;
- accepted semantic timers/actions are not silently discarded merely to reduce load;
- best-effort precomputation may be dropped/coalesced only when outcome-equivalent;
- overload policy cannot silently change claimed Reference semantics.

## 21. Mandatory resource-limit dimensions

Before executable `GAME-AI-01` acceptance, the accepted shared resource-limit policy/registry or superseding machine-readable contract MUST provide concrete finite ceilings, units, failure categories and boundary tests for at least:

1. active AI actors per authoritative scope;
2. representation-specific authored states/nodes/transitions as applicable;
3. semantic evaluation work/transitions/nodes/visits/actions per resolution;
4. AI memory/threat/stimulus entries per actor;
5. perception/target candidates per decision;
6. pending AI timers/operations per actor/scope;
7. queued/in-flight path requests per actor/scope/executor;
8. path search work units/nodes per request;
9. path route/result length/bytes;
10. repath/retry work over a bounded semantic window;
11. spawn sources/controllers and population per scope;
12. placement candidates per attempt;
13. postponed occupancy retry count **and hard maximum per occurrence/window**;
14. controlled-actor command backlog;
15. inherited script fuel/memory/host-call/query/proposal bounds;
16. replay/diagnostic evidence volume where amplification-prone.

No numeric value is invented by this paper contract. Missing required limits block executable acceptance.

## 22. Fail-closed matrix

| Condition | Required result |
|---|---|
| malformed/unresolved AI or spawn definition | reject compile/staging/activation |
| incompatible behavior/content/ruleset/SIM/script revision | reject/reconcile explicitly; never reinterpret silently |
| runtime actor/template inconsistency | fail/quiesce affected actor/source without permissive mutation |
| semantic resolution exceeds hard work bound before complete plan | discard complete staged plan; zero AI-local mutation; typed budget-exhausted outcome |
| staged plan fails validation/revalidation | discard complete staged plan; zero AI-local mutation |
| fallback needed after rejected resolution | separate bounded owner occurrence; no recursive partial continuation |
| candidate bound cannot be honored safely | deterministic query/decision failure unless an explicit canonical bounded-selection rule exists |
| path capacity unavailable | typed bounded defer/reject/failure outcome |
| path search budget exhausted | terminal typed failure; no partial route authority |
| stale path result | discard proposal |
| actor recycled/despawned | local-generation mismatch rejects late work |
| script trap/fuel exhaustion/invalid proposal | zero authoritative mutation from rejected proposal |
| ownership generation changed | old work/timers cannot publish authority |
| spawn retry policy lacks finite count/window/hard maximum | reject policy/source activation; no retry default |
| spawn retry count/deadline/hard maximum exhausted | terminal policy-defined skip/fail/cancel/reconcile; no further retry |
| missing value-source multiplicity/eligibility | block source activation |
| Reference behavior `UNKNOWN/CONFLICT/PENDING` | no parity claim and no guessed Reference enablement |

## 23. Reference and Evolved profiles

One engine MUST support both profiles without code forks.

### Reference

- exact exercised behavior requires sufficient target evidence before parity claim;
- exact aggro/threat/leash/path/spawn/retry/controlled-actor/reward semantics remain evidence-gated where not proven;
- OTS similarity, library defaults or current behavior outside the accepted Reference cut cannot fill evidence gaps.

### Evolved

- MAY use intentionally different versioned behavior/path/spawn/retry policies;
- each intentional difference remains explicit and revision-bound;
- authority, deterministic replay, bounded-resource, provenance, atomicity and anti-duplication invariants remain the same.

## 24. Explicitly unresolved decisions

This candidate intentionally leaves unresolved:

- concrete AI execution representation/framework/library;
- physical Rust/content schema and authoring representation;
- concrete pathfinding algorithm/library;
- all numeric AI/path/spawn/retry limits;
- exact Reference perception/aggro/threat/retarget/memory/leash semantics;
- exact Reference movement/path cost/tie/corner/obstacle semantics;
- exact Reference spawn retry count/cadence/deadline/recovery semantics;
- exact summon/pet command/persistence/reward rules;
- exact NPC interaction behavior;
- exact boss/world-event durable ownership APIs;
- exact loot/XP/contribution policy;
- DDL/persistence schema;
- Platform/production changes.

These unresolved subjects MUST NOT be converted into permissive defaults.

## 25. Cross-domain findings — report only

```yaml
cross_domain_finding:
  id: GAME-AI-XD-01
  observed_in_domain: GAME-AI-01
  target_owner: GAME-ABILITY
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md §18 and accepted GAME-ABILITY owner contracts
  conflict_or_gap: GAME-AI needs a stable typed action-intent/result and temporary-legality input boundary while combat validation/commit remains outside AI authority
  required_before: executable GAME-AI action integration and Reference/Evolved combat-intent conformance
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-AI-XD-02
  observed_in_domain: GAME-AI-01
  target_owner: GAME-INTERACTION
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md §10/§17
  conflict_or_gap: route invalidation and environmental interaction facts require a normalized owner boundary; GAME-AI must not define door, teleport or world-interaction semantics
  required_before: executable path adoption/revalidation against dynamic interaction state
  worker_action: REPORT_ONLY
```

```yaml
cross_domain_finding:
  id: GAME-AI-XD-03
  observed_in_domain: GAME-AI-01
  target_owner: GAME-ITEM/DUR-03/REWARD
  severity: P1
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md §15-§19
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
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md §20-§22
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
  evidence: docs/architecture/GAME-AI-01_CREATURE_AI_SPAWN_PATHFINDING_CONTRACT_CANDIDATE.md §14/§17/§19
  conflict_or_gap: world-shared durable occurrence and eligibility require a named event/encounter owner where semantics span multiple actors/scopes; GAME-AI cannot invent that durable owner
  required_before: durable multi-actor encounter occurrence, shared eligibility or reward semantics are implemented
  worker_action: REPORT_ONLY
```

No coordinator-only overlay is edited by this worker.

## 26. Future implementation acceptance evidence

An executable implementation MUST prove at least:

1. identical accepted snapshot/input/revisions produce identical semantic result under shuffled backing order;
2. its chosen representation satisfies the representation-neutral finite bounded semantic execution contract;
3. hard execution-budget exhaustion during staging commits zero authoritative AI-local mutation;
4. rejected partial staged work produces no accepted/published action proposal from that rejected plan;
5. fallback after failure occurs only as a separate bounded owner occurrence;
6. success-dependent AI-local state does not commit before normalized downstream acceptance;
7. stale target/local-generation evidence is rejected;
8. downstream legality rejection cannot be bypassed or spin recursively;
9. identical path input/profile yields deterministic normalized route and stale results are discarded;
10. script trap/fuel exhaustion commits zero authoritative mutation;
11. spawn placement alternatives are finite and canonical/deterministic;
12. occupancy retries obey configured finite count, semantic window/deadline, deterministic cadence/order and accepted hard maximum;
13. retry/recovery preserves one semantic occurrence identity and cannot duplicate value availability/settlement;
14. missing GAME-CHANNEL multiplicity/eligibility blocks value-source activation;
15. controlled-actor command remains a request rather than client mutation authority;
16. re-entry protection suppresses new monster offense without clearing threat/encounter state or buffering attacks;
17. overload preserves owner control/fencing progress and all hard queue/work bounds;
18. Reference `UNKNOWN/CONFLICT/PENDING` cannot be promoted to parity by implementation convenience;
19. recovery fences old-generation work and preserves required future-determining state/provenance.

## 27. Coordinator audit matrix

| Requirement | Successor disposition | Status |
|---|---|---|
| stable architecture gate | remains `GAME-AI-01`; no new global gate ID | `PROPOSED` |
| predecessor history | #261/#272 + final head + five repair cycles retained explicitly | `PROPOSED` |
| runtime authority | current ChannelRuntime/InstanceRuntime only | `PROPOSED` |
| behavior representation | **not frozen now**; bounded semantic contract frozen | `PROPOSED` |
| decision timing | `Must decide now: NO`; no current downstream paper gate blocked by representation deferral | `PROPOSED` |
| semantic-resolution atomicity | staged/preflighted all-or-nothing AI-local commit/reject | `PROPOSED` |
| over-budget behavior | reject complete staged plan; zero partial AI-local mutation | `PROPOSED` |
| pathfinding | bounded proposal + current-owner revalidation; no algorithm/library choice | `PROPOSED` |
| perception/targeting | bounded deterministic pipeline; Reference tuning evidence-gated | `PROPOSED` |
| scripts | DUR-04 proposal-only | `PROPOSED` |
| spawn retry | policy-defined finite count/window/cadence/order + hard maximum | `PROPOSED` |
| spawn provenance/recovery | stable occurrence provenance + explicit recovery class | `PROPOSED` |
| GAME-CHANNEL multiplicity | explicit for value-producing sources | `PROPOSED` |
| controlled actors | validated principal/control provenance | `PROPOSED` |
| disconnect/re-entry | downstream legality input; no new offense, no aggro reset/buffering | `PROPOSED` |
| loot/value | never GAME-AI authority | `PROPOSED` |
| resource obligations | concrete hard maxima required before executable acceptance | `PROPOSED` |
| Reference evidence | unknown/conflict/pending fail closed | `PROPOSED` |
| runtime implementation | none authorized | `NOT_STARTED` |

## 28. Candidate conclusion

The successor freezes the safety and determinism semantics that downstream implementation actually needs while deliberately avoiding premature framework/model identity.

For one AI semantic resolution, authoritative AI-local mutation is staged and preflighted, then committed all-or-nothing or rejected in full. Budget exhaustion cannot leave a partially mutated actor. Spawn occupancy recovery is finite but policy-defined across retry count, normalized time window/deadline, deterministic cadence/order and an accepted hard maximum; exact Reference behavior remains evidence-gated.

The predecessor's correct ownership, pathfinding proposal/revalidation, deterministic target pipeline, no-value-authority, DUR-04 proposal-only scripting, provenance/recovery, GAME-CHANNEL multiplicity, controlled-actor provenance, resource-bound, Reference fail-closed and disconnect/re-entry legality boundaries are preserved.

No executable implementation, concrete AI/pathfinding library, DDL, Platform, production change or coordinator-overlay mutation is authorized.

`MERGE_AUTHORITY: ARCHITECTURE_COORDINATOR_ONLY`