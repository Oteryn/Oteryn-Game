# Oteryn Game — Wave-2 Resource Limits Decision Packet

- Date: 2026-08-24
- Issue: `#93`
- Task: `OTV2-20260824-wave2-resource-limits`
- Mode: `CONTRACT / PREPARATION`
- Allocation merge: `0d9012bd20049754989d4d83c00c325a2bafe666`
- Runtime/client/server/protocol/content/DDL/Platform/production authority: **NONE**
- `RESOURCE_LIMITS_REGISTRY.json` mutation authority: **NONE in this task**

## 1. Decision purpose

This packet closes the inventory/evidence-preparation part of Issue #93 without inventing product policy or numeric gameplay ceilings. It identifies every material resource dimension required by the accepted `GAME-ABILITY-01`, `GAME-INTERACTION-01`, `GAME-AI-01` and `VSL-MOVE-01` scopes, records the exact existing inherited envelopes that are already registered, and makes the unresolved owner decisions explicit before any executable lane may claim acceptance.

The only permitted classifications are:

- `REGISTERED_EXACT` — the exact same resource already has an accepted registry entry and numeric hard maximum;
- `CONTRACT_EXACT_UNREGISTERED` — an accepted owning contract supplies an exact number but the registry does not yet contain it;
- `EVIDENCE_CANDIDATE` — evidence supports a concrete candidate value but it is not yet owner-accepted;
- `OWNER_DECISION_REQUIRED` — the owning contract requires a finite limit but no accepted/candidate exact number may be promoted by this worker;
- `NOT_APPLICABLE_TO_FIRST_SLICE` — an exact allocated executable slice explicitly excludes the dimension fail-closed.

`PROVEN`: no current first-slice allocation exists for Ability, Interaction, AI or Movement. Therefore this packet does not classify any required semantic dimension as `NOT_APPLICABLE_TO_FIRST_SLICE`; that classification is legal only against a later exact child plan.

`PROVEN`: the accepted gameplay contracts deliberately require finite resource bounds while deferring their exact numeric maxima. No current accepted source inspected for this task provides an exact numeric Ability, Interaction, AI or Movement semantic-work ceiling that is missing only from the registry. Therefore no row is currently `CONTRACT_EXACT_UNREGISTERED`.

`PROVEN`: no reviewed evidence artifact currently establishes a concrete candidate numeric value for the unregistered gameplay dimensions below. Therefore no row is currently `EVIDENCE_CANDIDATE`.

`UNKNOWN`: the exact numeric values for every row classified `OWNER_DECISION_REQUIRED`.

## 2. Registry invariants and inherited envelopes

`RESOURCE_LIMITS_REGISTRY.json` requires an unambiguous unit, an absolute hard maximum, failure category, allocation impact, client-visibility statement and boundary tests. Externally controlled counts/depths/lengths/byte sizes must be registered before implementation acceptance, and missing required limits fail review rather than imply unlimited capacity.

The following existing entries are relevant inherited envelopes. They MUST NOT be copied into gameplay-semantic limits unless the owning contract proves that the resource is exactly the same resource.

| Registry ID | Exact hard maximum | Unit | Exact inherited meaning used by this packet |
|---|---:|---|---|
| `FND02-OUTSTANDING-COMMANDS` | 64 | commands | contiguous commands sent but not terminally acknowledged for one `GameSession`; retry of an already-counted command consumes no new slot |
| `FND02-COMMAND-PAYLOAD-BYTES` | 65,536 | bytes | encoded `ClientCommand.payload` envelope only |
| `FND02-COMMAND-RESULT-PAYLOAD-BYTES` | 65,536 | bytes | encoded `CommandResult.payload` envelope only |
| `FND02-STATE-DOMAINS-PER-SYNC` | 256 | entries | state domains in one resync/snapshot exchange |
| `FND02-STATE-DELTA-PAYLOAD-BYTES` | 262,144 | bytes | one encoded `StateDelta.payload` envelope |
| `FND02-SNAPSHOT-CHUNK-COUNT` | 256 | chunks | chunks in one `SnapshotBody` transfer |
| `FND02-SNAPSHOT-CHUNK-BYTES` | 524,288 | bytes | one `SnapshotChunk.data` payload |
| `FND02-SNAPSHOT-ASSEMBLED-BYTES` | 16,777,216 | bytes | complete encoded `SnapshotBody` before protobuf decode |

These entries constrain protocol/transport allocation. They do **not** answer semantic questions such as how many targets an ability may resolve, how many AI path nodes may be searched, how deep an interaction cascade may run, or how many entities a Movement visibility delta may semantically include.

## 3. Common decision/boundary-test rule for unregistered rows

Unless a row states a narrower behavior, every `OWNER_DECISION_REQUIRED` dimension has the following mandatory acceptance shape:

1. select an absolute hard maximum only from accepted evidence or explicit owner disposition;
2. register the exact unit and any configurable range without allowing configuration to remove the hard maximum;
3. reject/terminate excess work deterministically before unchecked allocation and, for pre-commit plans, before partial authoritative mutation;
4. test `N` at the accepted maximum and `N+1` rejection/termination, plus zero/empty, checked-arithmetic overflow and retry/replay stability where applicable;
5. preserve already committed history when a post-commit descendant/future-work budget is exhausted;
6. expose only bounded stable failure/outcome information to clients where the domain result is client-visible; the numeric ceiling itself need not be disclosed;
7. do not create a new wire/error numeric identifier from this packet.

Evidence for an owner decision SHOULD include the exact proposed executable slice, representative validated content, deterministic worst-case/stress fixtures, measured CPU/memory/latency or retained-state cost, and a safety margin justified by the owning semantic contract rather than by a generic Foundation envelope.

## 4. Ability inventory

Owning semantic source: accepted `GAME-ABILITY-01`, composed from the whole-gate owner acceptance and its preserved whole-gate/partial contracts.

| ID | Dimension | Classification | Unit | Amplification/control source | Failure behavior / category | Allocation impact | Client visibility | Boundary-test obligation | Evidence / owner action |
|---|---|---|---|---|---|---|---|---|---|
| `AB-RL-01` | target candidates examined by one resolution step | `OWNER_DECISION_REQUIRED` | candidate entities/positions | player/content target query and world density | fail before unbounded candidate materialization; `CAPACITY_EXCEEDED` | bound enumeration/sort/filter storage before growth | yes — bounded rejection/outcome only | max vs max+1 candidate set; stable ordering; overflow-safe accounting | representative single/area/chain targeting corpus + measured resolver cost + owner maximum |
| `AB-RL-02` | resolved-target cardinality | `OWNER_DECISION_REQUIRED` | targets | authored geometry/query and dynamic world population | reject pre-commit plan exceeding bound; `CAPACITY_EXCEEDED` | cap resolved-set allocation and downstream fan-out | yes — outcome only | max legal targets vs max+1; no partial target commit from rejected plan | representative AoE/multi-target content + downstream cost + owner maximum |
| `AB-RL-03` | target query/geometry complexity | `OWNER_DECISION_REQUIRED` | deterministic work units / spatial candidates | authored query shapes, LoS/geometry and world density | deterministic budget exhaustion before effect planning; `CAPACITY_EXCEEDED` | bound spatial traversal/work queues | yes — outcome only | exact budget exhaustion, canonical-order invariance, no hidden rescan | geometry corpus + spatial-index measurements + owner work-unit definition/maximum |
| `AB-RL-04` | dynamic retarget/re-resolution depth | `OWNER_DECISION_REQUIRED` | resolution steps / levels | chain/jump/retarget semantics | terminate further uncommitted retarget work; `CAPACITY_EXCEEDED` | prevent recursive/iterative unbounded resolver growth | yes — outcome/effect result | depth max accepted; max+1 deterministically terminates; replay stable | representative chain/jump mechanics + owner depth maximum |
| `AB-RL-05` | Effect Plan entry count | `OWNER_DECISION_REQUIRED` | typed plan entries | content/effect composition/targets | reject complete staged plan before commit; `CAPACITY_EXCEEDED` | bound vector/plan allocation | yes — outcome only | max entries vs max+1; zero partial commit | representative effect-family composition + measured plan cost + owner maximum |
| `AB-RL-06` | Effect Plan encoded/in-memory size | `OWNER_DECISION_REQUIRED` | bytes | content/effect payload shape and target fan-out | reject before retain/copy/commit; `CAPACITY_EXCEEDED` | bound plan serialization and retained staging memory | no numeric detail; client may see failure | exact bytes max/max+1; checked cumulative arithmetic | measured representative/worst-case plans + owner byte maximum |
| `AB-RL-07` | calculation/contribution stages per occurrence | `OWNER_DECISION_REQUIRED` | stages | ruleset/formula/contribution composition | reject staged calculation before authoritative commit; `CAPACITY_EXCEEDED` | bound stage list and calculation work | yes — final outcome only | max stages vs max+1; deterministic order independent of registration/container order | formula/effect corpus + measured stage cost + owner maximum |
| `AB-RL-08` | multi-hit/multi-target sub-occurrences | `OWNER_DECISION_REQUIRED` | sub-occurrences per parent/root | authored hit count × target fan-out | reject/terminate uncommitted sub-occurrences under declared commit-group semantics; `CAPACITY_EXCEEDED` | bound sub-occurrence plan/work | yes — visible committed results remain final | max vs max+1; committed-prefix semantics only where explicitly authored; replay stable | representative multi-hit mechanics + owner maximum |
| `AB-RL-09` | channel/periodic occurrence count or duration-derived count | `OWNER_DECISION_REQUIRED` | occurrences per root | channel/condition/periodic definitions | no unbounded scheduling; terminate/reject excess future work; `CAPACITY_EXCEEDED` | bound timer/future-occurrence retention | yes — timing/effect outcome | max occurrences, max+1, expiry/cancel, restart/replay | representative channels/DoTs/HoTs + lifecycle cost + owner maximum |
| `AB-RL-10` | outstanding future ability work | `OWNER_DECISION_REQUIRED` | pending occurrences per root/owner scope | delayed hits, recharge, channel/condition work | reject new excess future work before enqueue; `CAPACITY_EXCEEDED` | bound scheduler/retained occurrence state | yes — outcome/timing state | queue at max, next enqueue rejected, retry same occurrence no duplicate slot | scheduler/storage measurement + owner per-root/scope maximum |
| `AB-RL-11` | `RUN_EACH_BOUNDED` catch-up backlog/work | `OWNER_DECISION_REQUIRED` | overdue occurrences or work units per scope/window | stalls/recovery/repeated timers | defer/terminate according to declared catch-up policy; never same-turn storm; `CAPACITY_EXCEEDED` | bound catch-up queue and same-turn work | yes — timing/outcome may be visible | backlog max/max+1, fairness, replay, no zero-delay bypass | recovery/stall fixtures + scheduler budget evidence + owner maximum |
| `AB-RL-12` | condition instance/stack cardinality | `OWNER_DECISION_REQUIRED` | instances/stacks per authoritative entity/scope | content, repeated applications and stacking policy | reject excess application before mutation; `CAPACITY_EXCEEDED` | bound condition collections/indexes | yes — status/outcome only | max active/stacked vs max+1; replace/refresh policy remains semantic, not inferred | representative condition catalogue + memory/work measurement + owner maximum |
| `AB-RL-13` | pending scheduled condition work | `OWNER_DECISION_REQUIRED` | future condition occurrences per entity/scope | periodic conditions and delayed expiry/ticks | reject excess scheduling before retention; `CAPACITY_EXCEEDED` | bound scheduler/state retention | yes — status/timing outcome | max pending vs max+1; cancellation/recovery/replay stable | condition timing corpus + scheduler measurement + owner maximum |
| `AB-RL-14` | reaction depth | `OWNER_DECISION_REQUIRED` | descendant levels | procs/triggers/re-entry | committed ancestors remain; further uncommitted descendants terminate; `CAPACITY_EXCEEDED` | bound recursion/lineage retention | yes — committed outcomes remain visible | depth max/max+1, cycles/re-entry, replay stable | reaction graph corpus + owner maximum |
| `AB-RL-15` | descendants per parent/root | `OWNER_DECISION_REQUIRED` | descendant occurrences | proc fan-out and authored reactions | terminate/reject excess descendants, preserve committed history; `CAPACITY_EXCEEDED` | bound child vectors/queues | yes — final committed results | fan-out max/max+1; canonical ordering; duplicate lineage stable | reaction catalogue + measured fan-out + owner maximum |
| `AB-RL-16` | total reaction/future work for one root occurrence | `OWNER_DECISION_REQUIRED` | deterministic work units / occurrences | combined reaction × future-work amplification | stop further uncommitted work at root budget; `CAPACITY_EXCEEDED` | cap aggregate CPU/queue/retained lineage | yes — bounded outcome/evidence | aggregate max/max+1 across mixed descendants; no bypass by reschedule | mixed worst-case reaction/channel fixture + owner root-work maximum |
| `AB-RL-17` | cross-domain proposals emitted by one ability root/commit group | `OWNER_DECISION_REQUIRED` | typed proposals | effects routed to Item/Movement/Interaction/AI/other owners | reject staged group exceeding bound before publication/commit; `CAPACITY_EXCEEDED` | bound proposal vectors and foreign-operation fan-out | yes — final owning-domain outcomes only | max proposals vs max+1; no hidden distributed partial commit | representative cross-domain mechanics + owner maximum |
| `AB-RL-18` | variable diagnostic/evidence payload/work | `OWNER_DECISION_REQUIRED` | bytes and/or records per root | adversarial invalid input, high fan-out failures, tracing | truncate/reject bounded diagnostics without changing gameplay truth; `CAPACITY_EXCEEDED` | bound log/event/evidence allocation | no raw internal volume; safe correlation only | max/max+1, truncation marker, gameplay outcome unchanged | diagnostics schema + observability budget + owner maximum |
| `AB-RL-19` | inherited DUR-04 script fuel/memory/host-call/query/result/action-plan bounds for script-backed mechanics | `OWNER_DECISION_REQUIRED` | fuel; bytes/pages; calls; query/results; proposals | content/script code under approved capabilities | script trap/exhaustion or host rejection commits zero rejected proposal mutation; `CAPACITY_EXCEEDED` | bound guest memory/execution/host result/action plans | outcome only; no internal capability detail | each applicable resource max/max+1, deterministic fuel outcome across supported targets | DUR-04/script implementation profile must first receive accepted registry values; Ability may consume, not invent them |

**Ability lane disposition: `BLOCKED_ON_OWNER_DECISION`.** It may become `READY_FOR_ALLOCATION` independently only after the coordinator freezes the exact first slice and every exercised row is `REGISTERED_EXACT`, while every omitted row is explicitly excluded fail-closed from that slice.

## 5. Interaction inventory

Owning semantic source: accepted `GAME-INTERACTION-01` successor architecture and its preserved predecessor invariants.

| ID | Dimension | Classification | Unit | Amplification/control source | Failure behavior / category | Allocation impact | Client visibility | Boundary-test obligation | Evidence / owner action |
|---|---|---|---|---|---|---|---|---|---|
| `GI-RL-01` | cascade depth | `OWNER_DECISION_REQUIRED` | child ancestry levels | content-triggered nested interactions | stop creating uncommitted descendants; `CAPACITY_EXCEEDED` | bound ancestry/stack/queue retention | yes — bounded terminal/pending outcome | depth max/max+1; nested collision identities remain distinct; replay stable | representative interaction graphs + owner depth maximum |
| `GI-RL-02` | child fan-out per parent | `OWNER_DECISION_REQUIRED` | child occurrences | authored edges/targets/triggers | reject child plan before first affected commit/delegation if bound exceeded; `CAPACITY_EXCEEDED` | bound child-plan vectors and delegated fan-out | yes — outcome only | max children/max+1; order independent of container insertion | interaction corpus + measured child-plan cost + owner maximum |
| `GI-RL-03` | total descendants/work per root | `OWNER_DECISION_REQUIRED` | occurrences or deterministic work units | cascade depth × fan-out amplification | terminate/reject further uncommitted root work; `CAPACITY_EXCEEDED` | cap aggregate queue/state/cpu | yes — committed children remain committed; root outcome bounded | mixed cascade max/max+1; retry/replay same child identities | worst-case cascade fixtures + owner root-work maximum |
| `GI-RL-04` | outstanding delegated owner operations | `OWNER_DECISION_REQUIRED` | accepted/pending foreign operations per root/scope | cross-owner item/ability/movement/etc. delegation | reject/defer new work before accepting another owner operation; `CAPACITY_EXCEEDED` | bound reconciliation state and foreign-operation handles | yes — `PENDING`/terminal outcome only | max pending operations; next new op refused; duplicate reconcile uses same `OwnerOperationRef` | coupled-workflow corpus + retained-state/latency evidence + owner maximum |
| `GI-RL-05` | automatic reconciliation/retry work | `OWNER_DECISION_REQUIRED` | attempts or deterministic work units per occurrence/window | dependency loss, timeout, failover and ambiguous completion | bounded same-occurrence reconciliation; on exhaustion require terminal/owner-intervention path, never blind fresh execution; `CAPACITY_EXCEEDED` | bound retry queues/timers/state | yes — safe pending/retry authority outcome | max/max+1, timeout before/after acceptance, no duplicate semantic mutation | failure-injection/recovery tests + owner attempt/work/window maximum |
| `GI-RL-06` | content-controlled trigger/eligible-edge candidates | `OWNER_DECISION_REQUIRED` | candidates per source occurrence | authored trigger/edge multiplicity and world targets | reject bounded plan before child acceptance; `CAPACITY_EXCEEDED` | bound candidate enumeration/sort and child planning | yes — outcome only | max/max+1 candidates; canonical ordering | representative authored interaction definitions + owner maximum |
| `GI-RL-07` | retained child-plan/reconciliation entries | `OWNER_DECISION_REQUIRED` | child lifecycle entries per root/scope | fan-out plus ambiguous/PENDING outcomes | refuse excess retained work before unbounded growth; `CAPACITY_EXCEEDED` | bound durable/in-memory reconciliation bookkeeping | no numeric detail; safe correlation only | max entries/max+1; committed/rejected entries do not become executable again | recovery-state measurements + owner maximum/retention policy |

**Interaction lane disposition: `BLOCKED_ON_OWNER_DECISION`.** The accepted retry vocabulary does not provide a numeric retry/cascade ceiling; the worker must not derive one from Foundation command counts.

## 6. AI inventory

Owning semantic source: accepted `GAME-AI-01` architecture. The preserved candidate explicitly enumerates mandatory resource-limit dimensions; exact numeric values are intentionally deferred.

| ID | Dimension | Classification | Unit | Amplification/control source | Failure behavior / category | Allocation impact | Client visibility | Boundary-test obligation | Evidence / owner action |
|---|---|---|---|---|---|---|---|---|---|
| `AI-RL-01` | active AI actors per authoritative scope | `OWNER_DECISION_REQUIRED` | actors | spawn/population/content and channel/instance load | reject/defer excess activation/spawn under owning policy; `CAPACITY_EXCEEDED` | bound actor state/scheduler load | indirectly through world state only | max actors/max+1, recovery/failover preserves cap | representative population/world density + measured actor cost + owner maximum |
| `AI-RL-02` | authored representation states/nodes/transitions, when selected representation uses them | `OWNER_DECISION_REQUIRED` | authored nodes/states/transitions | content/template complexity | fail validation/activation before runtime; `CAPACITY_EXCEEDED` | bound compiled graph/storage | no | max/max+1 compile validation; no recursive escape | representative behavior corpus after representation choice + owner maximum |
| `AI-RL-03` | semantic evaluation work per resolution | `OWNER_DECISION_REQUIRED` | visits/transitions/actions or deterministic work units | behavior complexity and inputs | reject entire staged AI-local resolution on budget exhaustion; zero AI-local gameplay mutation; `CAPACITY_EXCEEDED` | bound CPU/staged-plan growth | indirectly through no-action/failure behavior | max/max+1, zero partial mutation, deterministic replay | behavior corpus + worst-case evaluation profiling + owner work-unit maximum |
| `AI-RL-04` | memory/threat/stimulus entries per actor | `OWNER_DECISION_REQUIRED` | entries | perception/combat/event inputs | reject/evict only under explicit semantic policy; no accidental unbounded growth; `CAPACITY_EXCEEDED` | bound per-actor maps/queues | indirectly through AI behavior | max/max+1, stable ordering/eviction semantics, recovery | target/threat corpus + memory cost + owner maximum/policy |
| `AI-RL-05` | perception/target candidates per decision | `OWNER_DECISION_REQUIRED` | candidates | world density/perception radius/content | deterministic budget exhaustion/no unbounded scan; `CAPACITY_EXCEEDED` | bound candidate enumeration/sort/scoring | indirectly | max/max+1, stable tie-break, shuffled storage order invariant | dense-world fixtures + spatial/scoring profiling + owner maximum |
| `AI-RL-06` | pending AI timers/operations | `OWNER_DECISION_REQUIRED` | pending occurrences per actor/scope | authored behavior, retries, delayed work | reject/defer new excess work before enqueue; `CAPACITY_EXCEEDED` | bound scheduler/state | indirectly | queue max/max+1; same occurrence retry no duplicate logical work | lifecycle fixtures + scheduler measurement + owner maximum |
| `AI-RL-07` | queued/in-flight path requests | `OWNER_DECISION_REQUIRED` | requests per actor/scope/executor | AI goal churn/repath triggers | reject/supersede/defer under typed policy before queue growth; `CAPACITY_EXCEEDED` | bound path executor queue/retained request state | indirectly | max/max+1, stale/superseded result discarded | representative concurrent path load + queue/latency evidence + owner maximum |
| `AI-RL-08` | path search work per request | `OWNER_DECISION_REQUIRED` | nodes/work units | map complexity, obstacles, goal distance | bounded no-route/budget-exhausted proposal; no mutation authority; `CAPACITY_EXCEEDED` | cap search frontier/CPU | indirectly | max search work/max+1, deterministic neighbor/tie order | representative maps + path-search profiling + owner maximum |
| `AI-RL-09` | route/result length and retained route bytes | `OWNER_DECISION_REQUIRED` | steps and/or bytes | map size/goal distance | reject oversized proposal before adoption/retention; `CAPACITY_EXCEEDED` | bound route vectors/result transport between worker/owner | indirectly | max/max+1 route, stale revision rejection | representative long routes + memory/revalidation cost + owner limits |
| `AI-RL-10` | repath/retry work in one semantic window | `OWNER_DECISION_REQUIRED` | requests/attempts/work units per window | dynamic blockage/goal churn/rejections | deterministic defer/stop; never immediate unbounded retry loop; `CAPACITY_EXCEEDED` | cap repeated path/evaluation load | indirectly | max/max+1 within normalized window; retry schedule replay stable | obstruction/churn fixtures + latency/load evidence + owner maximum/window |
| `AI-RL-11` | spawn sources/controllers and population per scope | `OWNER_DECISION_REQUIRED` | sources/controllers/actors | content/world configuration | fail activation or bounded spawn outcome; `CAPACITY_EXCEEDED` | bound controller/population state | indirectly/world-visible | max/max+1 source/population validation | representative world/spawn corpus + owner maxima |
| `AI-RL-12` | placement candidates examined per spawn attempt | `OWNER_DECISION_REQUIRED` | candidate positions | authored placement area and occupancy | end attempt when bounded candidate set exhausted; no expanding probe loop; `CAPACITY_EXCEEDED` | bound spatial enumeration/work | no direct numeric detail | max/max+1 candidate set, canonical selection, deterministic RNG isolation | dense occupancy fixtures + spatial profiling + owner maximum |
| `AI-RL-13` | postponed occupancy retry count/work per occurrence/window | `OWNER_DECISION_REQUIRED` | retries/attempts per spawn occurrence/window | occupied placement and retry policy | stop at earliest configured count/window/hard max; terminal skip/fail/cancel/reconcile; `CAPACITY_EXCEEDED` | bound timers and repeated placement work | indirectly/world-visible | hard max and window exhaustion, stable retry index, recovery/replay | failure/occupancy fixtures + owner hard maximum and normalized window |
| `AI-RL-14` | controlled-actor command backlog | `OWNER_DECISION_REQUIRED` | pending commands/intents per actor/scope | controller/player/system command production | reject/defer excess new commands without dropping committed semantics; `CAPACITY_EXCEEDED` | bound command queues | yes for controlling player via outcome only | max/max+1, stale controller/fence rejection, deterministic ordering | controlled-actor scenarios + queue cost + owner maximum |
| `AI-RL-15` | inherited DUR-04 script fuel/memory/host-call/query/result/proposal bounds | `OWNER_DECISION_REQUIRED` | fuel; bytes/pages; calls; results; proposals | script/component behavior | trap/exhaustion/reject proposal; zero rejected AI-local mutation; `CAPACITY_EXCEEDED` | bound guest/runtime work | outcome only | resource-specific max/max+1 and deterministic exhaustion | first accepted DUR-04 execution profile/registry values; AI consumes, does not invent |
| `AI-RL-16` | AI replay/diagnostic evidence volume | `OWNER_DECISION_REQUIRED` | bytes/records per occurrence/window | repeated failures, high actor count, path/retry diagnostics | bounded/truncated evidence without altering gameplay truth; `CAPACITY_EXCEEDED` | bound observability allocation/storage | no raw internal volume | max/max+1/truncation; replay evidence remains sufficient | observability schema/cost + owner maximum |

**AI lane disposition: `BLOCKED_ON_OWNER_DECISION`.** In particular, the postponed spawn-occupancy retry contract requires a finite hard maximum but explicitly leaves the exact Reference count/cadence/window unknown.

## 7. Movement inventory

Owning semantic source: owner-accepted `VSL-MOVE-01`, with exact inherited Foundation envelopes only where the resource is genuinely the same resource.

| ID | Dimension | Classification | Unit | Amplification/control source | Failure behavior / category | Allocation impact | Client visibility | Boundary-test obligation | Evidence / owner action |
|---|---|---|---|---|---|---|---|---|---|
| `MOVE-RL-01` | outstanding client movement commands within the GameSession command window | `REGISTERED_EXACT` | commands | client command production/retry | existing `FND02-OUTSTANDING-COMMANDS=64`; 65th new outstanding command rejected; already-counted retry no new slot | protects session ingress queue before runtime work | yes | existing registry boundary tests apply | consume exact FND-02 envelope; any additional Movement-private reservation queue would require a separate row/decision |
| `MOVE-RL-02` | movement inputs processed per owner work cycle | `OWNER_DECISION_REQUIRED` | inputs / cycle | client/system movement production and queue drain | bound/defer/reject work deterministically; `CAPACITY_EXCEEDED` | bound same-cycle CPU/temporary work | yes — command outcome/state | max/max+1 cycle work; fairness/replay | exact Movement first slice + throughput/latency evidence + owner maximum |
| `MOVE-RL-03` | static/dynamic spatial candidates examined per movement decision | `OWNER_DECISION_REQUIRED` | spatial candidates | map density/occupancy/world state | deterministic capacity failure before unbounded scan; `CAPACITY_EXCEEDED` | bound spatial query buffers/work | yes — movement result only | max/max+1, canonical ordering | representative dense map fixtures + spatial profiling + owner maximum |
| `MOVE-RL-04` | occupancy/query result count | `OWNER_DECISION_REQUIRED` | results | world density and query geometry | reject/convert to bounded recovery path before allocation; `CAPACITY_EXCEEDED` | bound result vectors/maps | yes — movement outcome only | max/max+1 and no partial mutation | occupancy/query corpus + owner maximum |
| `MOVE-RL-05` | post-movement interaction descendants | `OWNER_DECISION_REQUIRED` | descendant occurrences per movement root | tiles/triggers/objects crossed/entered | stop/reject uncommitted descendant work under Interaction semantics; `CAPACITY_EXCEEDED` | bound post-move fan-out/queues | yes — resulting interactions only | max/max+1, stable lineage/order | exact Movement↔Interaction first slice plus accepted Interaction bound/equivalence decision |
| `MOVE-RL-06` | local relocation/teleport chain depth | `OWNER_DECISION_REQUIRED` | relocation levels | teleports/push/pull/triggered relocation | finite hard stop with deterministic rejection/termination; `CAPACITY_EXCEEDED` | prevent recursive relocation stack/queue growth | yes — final location/failure | depth max/max+1, loop fixture, replay stable | representative relocation graph + owner depth maximum |
| `MOVE-RL-07` | total relocation work per root | `OWNER_DECISION_REQUIRED` | relocations/work units | depth × fan-out/trigger amplification | stop further uncommitted relocation work at root budget; `CAPACITY_EXCEEDED` | bound aggregate CPU/lineage | yes | aggregate max/max+1 across mixed relocation causes | worst-case relocation fixture + owner root-work maximum |
| `MOVE-RL-08` | interest enter/leave/update entities per delta | `OWNER_DECISION_REQUIRED` | entity changes / delta | player movement through dense visibility sets | use bounded delta/snapshot/resync/degradation path; never unbounded allocation; `CAPACITY_EXCEEDED` | bound visibility delta construction | yes | max/max+1, deterministic recovery to snapshot/resync path | dense-scene fixture + client/server serialization/render cost + owner maximum |
| `MOVE-RL-09` | visibility/spatial query candidates | `OWNER_DECISION_REQUIRED` | candidates / query | world/entity density and interest geometry | bounded query failure/recovery; `CAPACITY_EXCEEDED` | bound query enumeration/sort | yes indirectly | max/max+1, canonical ordering | dense-world visibility profiling + owner maximum |
| `MOVE-RL-10` | visibility/spatial query results | `OWNER_DECISION_REQUIRED` | entities/results / query | visible entity density | bounded snapshot/resync/degradation; no unbounded result vector; `CAPACITY_EXCEEDED` | bound retained/result collections | yes | max/max+1 with explicit recovery path | dense-scene client/server evidence + owner maximum |
| `MOVE-RL-11` | semantic entity count in a Movement visibility snapshot/extension | `OWNER_DECISION_REQUIRED` | entities / snapshot | visible world density | reject/degrade/resync before unbounded semantic snapshot construction; `CAPACITY_EXCEEDED` | bound semantic snapshot model before encoding | yes | max entities/max+1 independent of transport byte/chunk envelopes | exact Movement snapshot schema + dense-scene evidence + owner maximum |
| `MOVE-RL-12` | snapshot transport chunk count | `REGISTERED_EXACT` | chunks | encoded snapshot transfer | existing `FND02-SNAPSHOT-CHUNK-COUNT=256` | bounds assembler tracking | yes | existing 256/257 and index tests apply | consume exact Foundation transport envelope; does not set semantic entity count |
| `MOVE-RL-13` | snapshot chunk payload bytes | `REGISTERED_EXACT` | bytes | encoded snapshot transfer | existing `FND02-SNAPSHOT-CHUNK-BYTES=524288` | bounds retained/copied chunk payload | yes | existing 524,288/524,289 tests apply | consume exact Foundation transport envelope |
| `MOVE-RL-14` | assembled encoded snapshot bytes | `REGISTERED_EXACT` | bytes | encoded snapshot transfer | existing `FND02-SNAPSHOT-ASSEMBLED-BYTES=16777216` | checked cumulative assembly bound | yes | existing 16,777,216/16,777,217 and overflow tests apply | consume exact Foundation transport envelope |
| `MOVE-RL-15` | encoded Movement state-delta payload bytes when carried as FND-02 `StateDelta.payload` | `REGISTERED_EXACT` | bytes | encoded domain delta | existing `FND02-STATE-DELTA-PAYLOAD-BYTES=262144`; domain-specific bound may later be smaller | bound before domain-specific decode/allocation | yes | existing 262,144/262,145 envelope tests plus future domain semantic tests | consume Foundation envelope only; do not infer entity/work cardinality from bytes |
| `MOVE-RL-16` | queued/in-flight auxiliary path/spatial proposals | `OWNER_DECISION_REQUIRED` | proposals / actor/scope/executor | path preview/spatial auxiliary work and goal churn | reject/supersede/defer excess proposal work; `CAPACITY_EXCEEDED` | bound worker queues/retained result state | yes indirectly | max/max+1, stale/superseded result discard | exact Movement auxiliary design + load evidence + owner maximum |
| `MOVE-RL-17` | Movement diagnostic/evidence volume under spam/failure | `OWNER_DECISION_REQUIRED` | bytes/records per session/root/window | movement spam, spatial failures and resync pressure | bounded/truncated diagnostics without changing authoritative result; `CAPACITY_EXCEEDED` | bound observability allocation/storage | no raw internal volume | max/max+1/truncation, safe correlation preserved | diagnostics schema/cost + owner maximum |

**Movement lane disposition: `BLOCKED_ON_OWNER_DECISION`.** The inherited FND entries close only command/snapshot/state-delta transport envelopes. They do not close Movement semantic input/work/spatial/relocation/visibility cardinalities.

Before any `OTV2-IMPL-MOVE` allocation, the coordinator MUST re-read this inventory against the exact proposed Movement child plan and classify every exercised row as `REGISTERED_EXACT` or explicitly exclude it fail-closed from that slice. A missing owner decision/evidence keeps Movement blocked.

## 8. Cross-domain Durability finding

```yaml
finding_id: W2RL-XD-01
observed_in_domain: Wave-2 resource-limit preparation
target_owner: DUR-03 / Issue #94 coordinator path
severity: implementation_gate
status: REPORT_ONLY
proven_evidence:
  - Issue #94 requires its topology packet to identify DUR-03 numeric resource dimensions that remain blocked by hard-max decisions
conflict_or_gap: Issue #94 currently supplies no exact accepted DUR-03 numeric hard maximum that can be registered or absorbed by Issue #93
required_before: any OTV2-IMPL-DURABILITY slice that exercises such an unregistered numeric dimension may claim executable acceptance
worker_action: do not invent or silently absorb a DUR-03 value; return it to the coordinator/owner decision path
```

This finding does not block completion of the Issue #93 inventory packet and does not grant Issue #93 ownership over DUR-03 policy.

## 9. Lane readiness

| Lane | Readiness | Reason |
|---|---|---|
| Ability | `BLOCKED_ON_OWNER_DECISION` | required semantic targeting/effect/future/condition/reaction/proposal dimensions have no accepted numeric hard maxima |
| Interaction | `BLOCKED_ON_OWNER_DECISION` | cascade/fan-out/root/delegated/reconciliation dimensions have no accepted numeric hard maxima |
| AI | `BLOCKED_ON_OWNER_DECISION` | evaluation/perception/memory/timer/path/spawn/retry/script dimensions have no accepted numeric hard maxima |
| Movement | `BLOCKED_ON_OWNER_DECISION` | Foundation transport envelopes are exact, but Movement semantic work/spatial/relocation/visibility dimensions remain unresolved and must be rechecked against the exact Movement child plan |

No lane is `READY_FOR_ALLOCATION` from this packet alone. No lane is `BLOCKED_ON_EVIDENCE` as a narrower terminal classification because, at the current decision point, the missing numeric value itself requires owner acceptance; evidence is an input to that owner decision. A later coordinator may change a specific row to `EVIDENCE_CANDIDATE` when a concrete reviewed candidate value actually exists.

## 10. Serialized owner/registry closure procedure

For each lane independently:

1. freeze the exact first executable child plan and its owned paths;
2. mark which packet rows the slice actually exercises;
3. for every omitted row, prove that the slice cannot reach that resource and explicitly exclude it fail-closed; only then may that row become `NOT_APPLICABLE_TO_FIRST_SLICE` for that slice;
4. gather representative corpus/stress/cost evidence for every exercised unresolved row;
5. obtain explicit owner disposition for each numeric hard maximum and unit/configurable range;
6. through a separate serialized coordinator PR, add/update the corresponding `RESOURCE_LIMITS_REGISTRY.json` entries with failure category, allocation impact, client visibility and boundary tests;
7. validate registry JSON, governance, placeholder/red-flag scan, `git diff --check`, exact-head repository gates and boundary-test obligations;
8. only after the resulting merged registry/explicit exclusions are re-read may the coordinator allocate that executable lane.

Ability, Interaction and AI do not wait for unrelated Movement-only rows. Movement cannot be released merely because those generic lanes have closed their own rows.

## 11. Explicit decisions not taken

This packet does **not**:

- choose any numeric hard maximum for an unregistered gameplay-semantic resource;
- reuse `FND02-ORDINARY-REPEATED-ENTRIES`, frame bytes or another generic Foundation ceiling as a gameplay-work limit;
- mutate `RESOURCE_LIMITS_REGISTRY.json`;
- allocate Ability, Interaction, AI or Movement implementation;
- define protocol/event/state numeric IDs or new public wire errors;
- choose production tuning, balancing values, permanent content format or Durability topology;
- claim Reference parity from a safety ceiling;
- convert a transport byte/count envelope into semantic target/entity/work cardinality.

## 12. Completion disposition

`PROVEN`: the required Issue #93 inventory is complete for the currently accepted Ability, Interaction, AI and Movement contracts.

`PROVEN`: the current registry provides exact inherited Foundation command/snapshot/state-delta envelopes but no lane-specific numeric maxima that close the required Ability, Interaction or AI semantic dimensions, nor the unresolved Movement semantic dimensions.

`DERIVED`: the preparation worker may complete after this packet is reviewed/merged because its obligation is to expose the decision surface without inventing values. Issue #93 itself MUST remain open for owner/evidence decisions and later serialized registry mutation; closing this worker task must not erase Movement obligations.

`UNKNOWN`: all numeric maxima identified above as `OWNER_DECISION_REQUIRED` until owner-accepted evidence is merged through the required serialized path.

`CONFLICT`: none after exact-path allocation precedence and the Foundation-envelope-versus-gameplay-semantic distinction are applied.
