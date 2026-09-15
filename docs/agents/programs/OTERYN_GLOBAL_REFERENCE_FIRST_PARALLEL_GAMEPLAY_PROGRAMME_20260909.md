# Oteryn Global Reference-first parallel gameplay/content programme

- Status: `PROPOSED_COORDINATION_PROGRAMME`
- Date: 2026-09-09
- Tracker: #486
- Parent control plane: #162
- Repository: `Oteryn/Oteryn-Game`
- Admission protected main: `4d06bad1c0d21f2237df290865be55d8f7ed4f02`
- Implementation authority granted by this document: **NONE**
- Allocation authority granted by this document: **NONE**
- Production authority: **NONE**

## 1. Objective

Prepare and deliver as much of the playable game as can truthfully progress in parallel while the current durability/admission/Server Seam dependency chain continues, without creating duplicate workers or bypassing the existing control plane.

The programme is explicitly **Reference-first**:

1. establish and reproduce the accepted external Global Tibia Reference behavior cut mechanic by mechanic;
2. compose those evidenced semantics into the native `Oteryn Reference` product profile;
3. only after a reviewed Reference readiness gate may a later programme activate `Oteryn Evolved` gameplay differences.

This document is a scheduling/execution plan over already accepted architecture. It does not redefine product-profile authority.

## 2. Binding terminology and sequencing

The accepted first external behavior target is:

> Global Tibia production-observable behavior after the 2026-07-28 server-save/maintenance change boundary.

For this programme use these unambiguous terms:

- `GLOBAL_REFERENCE_TARGET` — the immutable external behavior oracle/evidence target above;
- `OTERYN_REFERENCE` — the Oteryn fidelity profile that implements that target inside the shared native engine/client/`protocol-oteryn` stack;
- `OTERYN_EVOLVED` — the later Oteryn-designed profile containing explicit Oteryn gameplay differences.

There is no separate production runtime profile called `Tibia Global`. Global is evidence/behavior authority, not an Oteryn runtime fork.

The execution order is therefore:

```text
GLOBAL_REFERENCE_TARGET evidence
        -> per-domain parity model/fixtures
        -> native shared/domain implementation
        -> OTERYN_REFERENCE composition + parity gates
        -> Reference readiness checkpoint
        -> explicit later owner/control-plane release
        -> OTERYN_EVOLVED work
```

`OTERYN_EVOLVED` mechanics must not be used to fill a missing Reference rule.

## 3. Evidence rule

Every Reference-sensitive behavior remains classified using the accepted evidence discipline:

- `PROVEN`
- `OBSERVED`
- `DERIVED`
- `UNKNOWN`
- `CONFLICT`
- `DECLARED_DIFFERENCE` when separately authorized

The first three may be consumed only within their actual evidence strength. `UNKNOWN` and `CONFLICT` block the affected parity claim instead of being guessed.

Source preference remains:

1. official Tibia/CipSoft public evidence and owner primary captures;
2. lawful controlled black-box observation;
3. reputable public community corroboration;
4. Canary/Crystal/other OTS repositories only as `OTS_HYPOTHESIS_ONLY` discovery/test-case inputs.

Current Global behavior after the target date does not silently move the target.

## 4. Control-plane and anti-collision rule

#162 remains the sole product allocation/integration control plane. #486 is a programme tracker, not a second scheduler.

Before any tracked-file runtime/content/client write, the active control plane must freshly prove:

- current protected `main`;
- existing canonical Issue/task/branch/PR for the lane;
- exact owned paths and shared leases;
- dependencies required by the concrete child slice;
- absence of a competing mutating writer;
- exact current allocation authority.

If a canonical worker already exists, continue the same lineage. Do not create a replacement branch/PR merely because this programme was introduced.

A blocked mutating lane should continue legal read-only evidence/readiness work where useful. A dependency blocker in one lane does not freeze independent path-disjoint work.

## 5. Programme lanes

### R0 — Reference evidence and parity registry

**Purpose:** establish the evidence required by all other Reference lanes.

Primary live anchor: #483 plus the accepted Reference evidence manifest/contracts.

Work that can progress independently:

- official-first source indexing;
- behavior/parity case inventory;
- target-date continuity checks;
- exact `PROVEN / OBSERVED / DERIVED / UNKNOWN / CONFLICT` classification;
- fixture/test-case proposals;
- gap packets for each gameplay lane.

Do not duplicate #483. Extend or consume its output through the live control plane.

### R1 — World / Map

**Purpose:** create Reference-capable world semantics independently from graphics representation and without freezing a permanent World Bundle format prematurely.

Scope families:

- OTBM/legacy corpus as migration input only;
- canonical positions/cells/floors;
- ordered tile/stack semantics;
- terrain/collision/navigation;
- roofs/occlusion/elevation;
- teleport/door/house/static interaction topology;
- spawn-area topology;
- deterministic conversion diagnostics and no-silent-loss evidence;
- server-authoritative vs client-safe world projections.

Can progress before Server Seam through docs/evidence/import fixtures/non-production experiments and exact content-owned work where already allocated.

Do not select `.omap`/`.owb`, atlas/array, chunk shape or streaming format without their own evidence gate.

### R2 — Content catalogue

**Purpose:** provide the native definitions required by Reference gameplay.

Consume #54/#433 and current `apps/game-server/src/content/**` authority rather than creating a parallel content graph.

Reference-oriented content includes:

- items/equipment definitions;
- creatures;
- spawn definitions;
- spell/ability definitions;
- presentation keys;
- relocation/interactions needed by the slice;
- provenance and exact revision binding.

Reference content values must come from evidence. Test-only fixture profiles stay explicitly non-shipping.

### R3 — Movement / Interaction

**Purpose:** establish authoritative movement/spatial/basic interaction behavior and the client-visible results needed by the Reference loop.

Preparation that can run independently includes:

- Reference movement evidence;
- collision/spatial cases;
- walk/turn/use/look/container interaction cases;
- resource-bound inventories;
- client presentation fixtures.

Production Movement remains dependency-gated by current admission/Server Seam/client/QA authority. Do not bypass #247 or current Movement allocation rules.

### R4 — Ability / Spells

**Purpose:** use the existing typed ability/effect architecture to reproduce Reference spells without creating a second combat engine.

Reference responsibilities:

- spell identity/provenance;
- mana/soul/requirements when applicable;
- cooldown/group semantics;
- targeting/geometry;
- range/line-of-sight;
- damage/heal/effect formulas;
- runes and other ability carriers where in scope;
- conditions/durations;
- server-authoritative commit timing;
- presentation event mapping kept separate from authority.

The generic ability engine may use structural fixture values, but a Reference spell is not `PARITY_CONFIRMED` until its material observable behavior is evidenced and tested.

### R5 — Combat / Death / Loot / XP

**Purpose:** reproduce the first real Reference PvE combat journey.

Reference responsibilities include:

- targeting/attack lifecycle;
- attack timing/range legality;
- physical/elemental damage;
- armor/resistance/mitigation rules;
- creature death occurrence;
- XP consequence;
- corpse creation;
- loot selection/materialization;
- pickup/ownership interaction;
- player death/respawn minimum as a later bounded child when its exact dependencies are ready.

Production combat integration remains gated by merged Movement and current Server Seam/durability prerequisites. Evidence, formulas, deterministic fixtures and non-authoritative presentation work may progress earlier when path-disjoint.

### R6 — Creature AI / Spawns

**Purpose:** reproduce Reference-observable creature behavior while keeping AI proposal-only relative to authoritative Movement/Ability owners.

Reference responsibilities:

- perception/target selection;
- chase/flee/retarget behavior;
- attack/spell choice;
- path proposals;
- spawn lifecycle/area/timing;
- deterministic decision ordering where required;
- creature-specific behavior data.

Do not allow AI to mutate position/combat/value directly.

### R7 — Character / Item / Progression

**Purpose:** supply the player-state semantics needed by Reference gameplay.

Reference responsibilities include:

- vocation/class and promotion behavior;
- level/XP/skills progression;
- regeneration/stamina/training when in bounded scope;
- inventory/container/equipment legality;
- item stats/resists/imbuement behavior where Reference-visible;
- death/loss/blessing/protection semantics;
- economy/source/sink rules needed by the selected playable slice.

Evolved-only systems such as owner-designed death/progression redesigns must not enter this lane unless separately accepted as a Reference difference.

### R8 — NPC / Quest / Services

**Purpose:** reproduce the minimum Reference service loop and prepare broader content systems.

Initial bounded targets:

- one representative NPC dialogue/service;
- minimal buy/sell or trade behavior needed by the slice;
- depot/container service where required;
- quest-state evidence/model preparation;
- service interaction through canonical Interaction/Item/Durability owners.

Broader quest scripting, market/bank and economy breadth remain later children unless independently allocated.

### R9 — Client / Presentation / Graphics

**Purpose:** make the Reference loop physically recognizable/playable without moving gameplay authority client-side.

Consume protected graphics direction and #480 rather than creating another renderer programme.

Parallel work includes:

- floor/tile/stack/roof rendering;
- movement interpolation;
- object/outfit/effect/missile presentation;
- spell/projectile/hit/heal VFX;
- names/HP/basic HUD;
- lighting/day-night/weather/season presentation where shared and semantically neutral;
- Classic/Enhanced/HD presentation families only where gameplay semantics stay identical;
- synthetic/mock `RenderSnapshot + PresentationEvents + EnvironmentState` consumers until the real producer is composition-ready.

Server remains authoritative for damage/hit/AoE/timing.

### R10 — Reference QA / composition

**Purpose:** prove the assembled `OTERYN_REFERENCE` profile rather than merely collecting locally green components.

This lane owns parity/composition evidence, not domain authority.

Required progressive journeys:

1. world entry -> presence;
2. movement -> interaction;
3. representative heal + attack ability;
4. creature combat -> kill -> XP;
5. corpse -> loot -> pickup;
6. inventory/equipment basics;
7. death -> respawn/re-entry minimum;
8. representative creature AI/spawn;
9. minimal NPC/depot/trade service;
10. reconnect/restart/durability where required;
11. native client presentation of the complete journey.

## 6. Safe parallelism matrix

| Lane | Useful work now while Server Seam chain is incomplete | Production/runtime composition dependency |
|---|---|---|
| R0 Evidence | YES | none for evidence work |
| R1 World/Map | YES: evidence/import/prototype/bounded content work | final server/client world runtime contracts |
| R2 Content | YES under #54/#433 current authority | production activation/publication gates |
| R3 Movement/Interaction | YES: evidence/readiness/fixtures | current admission + Client/QA + exact Movement allocation |
| R4 Ability/Spells | YES: evidence/data/engine-ready work when allocated | real player command/runtime composition for live journey |
| R5 Combat | YES: evidence/formulas/fixtures/readiness | merged Movement + durability + Server Seam composition |
| R6 Creature AI | YES: evidence/data/proposal logic when allocated | authoritative Movement/Ability runtime |
| R7 Character/Item/Progression | YES where existing contracts/allocations permit | durability/profile composition for persistent state |
| R8 NPC/Quest/Services | YES: evidence/model/readiness | Interaction/Item/Durability composition for persistent services |
| R9 Client/Graphics | YES; #480 explicitly supports non-production evidence | real authoritative snapshot/event producer for final integration |
| R10 QA | YES for fixture/parity harness planning | full end-to-end gates require owning runtime seams |

`YES` never means implicit write authority. It means there is useful path-disjoint work that the current control plane may legally allocate or that a read-only worker may prepare.

## 7. Dependency DAG

The programme is not a single serial chain. Use this DAG:

```text
                     +--> R1 World/Map --------+
                     |                         |
R0 Evidence ---------+--> R2 Content ----------+------+
                     |                         |      |
                     +--> R4 Ability/Spells ---+--+   |
                     |                            |   |
                     +--> R6 Creature AI --------|---+|
                     |                            |   ||
                     +--> R7 Char/Item/Prog -----|--+||
                     |                            |  |||
                     +--> R8 NPC/Quest ----------|--||+
                                                  |  ||
WP2/WP3 -> WP4 -> WP5 -> G0 -> Server Seam ------+  ||
                                                     ||
R1 + client readiness -> R3 Movement/Interaction ---+|
                                                      |
R3 + R4 + R2 + durability -> R5 Combat --------------+
                                                      |
R1/R2/R4/R5/R6/R7/R8 + real server producer -> R9 final composition
                                                      |
                                                      v
                                             R10 Reference QA
                                                      |
                                                      v
                                      REFERENCE_READINESS_CHECKPOINT
                                                      |
                                     explicit later owner/control-plane release
                                                      v
                                             OTERYN_EVOLVED
```

Graphics #480 can advance alongside this DAG and feeds R9.

## 8. Reference readiness checkpoint

Do not activate a broad Evolved gameplay programme because an individual Reference subsystem appears complete.

A reviewed `REFERENCE_READINESS_CHECKPOINT` must record at least:

- immutable target identity and coherent evidence registry;
- explicit remaining `PARITY_PENDING_EVIDENCE / CONFLICT` list;
- world presence/movement/basic interaction playable;
- representative heal and attack spell;
- kill -> XP -> corpse -> loot -> pickup;
- inventory/equipment minimum;
- death/respawn/re-entry minimum;
- representative creature AI/spawn;
- minimal NPC/depot/trade service;
- native client presentation;
- reconnect/restart/durability proof where required;
- Reference/Evolved profile isolation tests;
- no undocumented Evolved mechanic in Reference;
- exact known declared safety/legal/technical differences.

The checkpoint may still declare bounded parity gaps; it must not conceal them.

Only a later explicit owner/control-plane decision may open Evolved implementation scope.

## 9. Allocation strategy

The active control plane should prefer small child allocations with exact owned paths over one giant Reference branch.

Recommended allocation units:

- evidence/docs-only child;
- data/content-only child;
- domain-engine child;
- non-production experiment child;
- final composition child after prerequisites are protected.

Avoid shared root Cargo/workflow/registry paths until the child genuinely requires them, then serialize the exact lease.

Existing branches/PRs always win over proposed new children when they already own the same material work.

## 10. What this programme deliberately does not freeze

Unless separately decided/evidenced, this programme does not freeze:

- permanent World Project/World Bundle format;
- KTX2 vs DDS;
- atlas vs texture arrays/hybrid;
- final cache/streaming/bundle topology;
- final UI technology;
- compression/mipmap/filter policy;
- original Evolved balance formulas;
- new Evolved death/progression/economy/PvP rules;
- future Reference revision cadence.

## 11. Immediate coordinator actions

From live GitHub, the current #162 control plane should:

1. reconcile #486 with #483, #54/#433, #480, #247 and #364;
2. inventory existing canonical workers and exact path ownership for R0-R10;
3. mark each lane `ACTIVE_EXISTING / READY_PATH_DISJOINT / READ_ONLY_PREP / WAITING_DEPENDENCY / ARCHITECTURE_REQUIRED`;
4. progress existing workers first;
5. allocate only the smallest genuinely path-disjoint children whose prerequisites are already sufficient;
6. keep production Movement/Combat claims blocked until their real dependencies are protected;
7. continuously prevent Evolved mechanics from being used as Reference defaults;
8. converge the outputs into the Reference readiness checkpoint rather than a collection of disconnected PRs.

## 12. Completion

This programme is complete only when either:

- the reviewed Reference readiness checkpoint is protected and the remaining gaps are explicitly bounded; or
- a precise owner/architecture blocker prevents further Reference progression and all independent legal work has been exhausted.

A merged evidence PR, green component CI, completed graphics prototype or Server Seam release is a checkpoint, not programme completion.

`IMPLEMENTATION_AUTHORITY: NONE_BY_THIS_DOCUMENT`
`ALLOCATION_AUTHORITY: #162_CURRENT_CONTROL_PLANE_ONLY`
`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
