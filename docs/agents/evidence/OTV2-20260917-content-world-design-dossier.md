# OTV2 Content / World design dossier — 2026-09-17

```yaml
status: RETAINED_ANALYSIS
classification: PROPOSED_NONCANONICAL
repository: Oteryn/Oteryn-Game
base_main: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
related: [162, 64, 483, 486, 499, 500, 504]
implementation_authority: NONE
format_acceptance: PROPOSED_NOT_ACCEPTED
registry_mutation_authority: NONE
production_authority: NONE
crystal_runtime_reuse: NO
crystal_reference_and_import_evidence: YES
```

## Purpose

Retain the complete substantive design developed with the owner on 2026-09-17 for Oteryn world/content authoring, migration/import, map structure, attributes, interaction semantics, client/server projection, runtime state, durability and Reference migration.

This dossier deliberately records both the initial design and the corrections discovered during deeper review. Later statements in this document supersede earlier candidates where they conflict. It does not amend accepted ADRs/contracts or allocate implementation.

## 1. Architectural baseline that remains binding

The design builds on the accepted native-world direction rather than replacing it:

```text
legacy/external inputs
        -> bounded importers
        -> Legacy / Source IR
        -> typed Canonical Oteryn World/Content Model
        -> editable Oteryn World Project
        -> deterministic validation/compiler
        -> separate server/client projections
        -> immutable World/Content artifacts
        -> isolated staging
        -> explicit activation
        -> runtime
```

Key invariants retained from the existing architecture:

- OTBM, OTB, Crystal/TFS conventions, legacy numeric item IDs and sprite IDs are migration evidence, not canonical Oteryn semantics.
- Stable namespaced `ContentKey`-like identities are canonical; compact numeric runtime IDs are revision-scoped implementation details only.
- Authored static world/content, live runtime state and durable player/value state are distinct.
- `WorldId` and `ChannelId` remain distinct in the multichannel model.
- Client data is a non-authoritative allowlisted projection. Server/domain owners decide legality and mutation.
- Loading/staging and activation are separate. An active immutable content generation is not edited in place.
- GAME-ITEM owns item legality; DUR-03 owns durable value/item location transition, conservation, retry/idempotency and anti-duplication semantics.
- GAME-INTERACTION/Movement/Ability and other accepted owners continue to own runtime behavior; Content supplies versioned authored semantics, not a second runtime authority.
- DUR-04 remains the eventual scripting boundary: bounded Component Model/WIT components propose typed actions and never directly own SQL, arbitrary object mutation or trusted value creation.

## 2. What can start now

There is no architectural reason to postpone large-scale content extraction until Server Seam or every gameplay subsystem is complete.

We can already build and populate a non-runtime source/authoring layer for:

- items and their physical/presentation candidates;
- creatures, combat/reward candidates and loot candidates;
- abilities/effects/formula bindings;
- NPC definitions, services, shops, travel and dialogue candidates;
- world/map geometry, terrain, placements, zones and transitions;
- appearances/presentation metadata;
- quest source records and candidate graph reconstruction even while the canonical quest/runtime contract remains incomplete.

What must remain blocked is not collection itself, but promotion of unresolved/unsupported semantics into an executable Reference profile.

## 3. Proposed source-project layout

The exact physical format is not accepted by this dossier. The following is the current authoring candidate because it is Git-friendly, bounded and easy to inspect:

```text
content/projects/reference/
  project.json
  content.lock.json
  profiles/
    reference-playable.json
  packages/reference-base/
    package.json
    items/
    creatures/
    abilities/
    effects/
    loot/
    behaviors/
    npcs/
    dialogues/
    shops/
    quests/
    encounters/
    world-objects/
    terrains/
    presentations/
    localization/
    worlds/reference/
      world.json
      regions/
      areas/
      subareas/
      zones/
      transitions/
      spawns/
      npc-placements/
      houses/
      encounters/
      shards/
  provenance/
    sources.lock.json
    bindings/
    declarations/
  migration/
    id-maps/
    coordinate-profiles/
    composite-patterns/
    corrections/
    reports/
```

Use one package initially unless independent versioning/distribution creates a real need for more. Directory grouping is not semantic package ownership.

A file path is not object identity. Moving `items/currency/gold_coin.json` to another folder does not change the stable content key.

## 4. Text/source format direction

The current recommendation is strict UTF-8 JSON for authored definitions and JSON Lines for sparse/generated map shards, with structural schema validation plus typed domain validation.

This supersedes the earlier automatic JSON5 recommendation. JSON5 may later be an editor input adapter, but should not create a second semantic model merely to gain comments/trailing commas.

Important parser rules:

- reject duplicate authoritative fields;
- reject unsupported critical fields;
- fail closed on unresolved typed references;
- never treat filesystem enumeration order as semantic order;
- canonicalization/hash rules must be versioned;
- arrays that are semantically ordered must not be globally sorted by a generic canonicalizer;
- every externally controlled length/count/depth requires the appropriate registered resource ceiling before production implementation acceptance.

`32x32` remains only a useful source-shard experiment candidate. It is not frozen as a permanent authoring size, compiled-bundle chunk or runtime spatial sector.

## 5. Three independent partition domains

Do not conflate:

```text
SOURCE/AUTHORING SHARD
  -> Git diff locality, partial edit/save, author collaboration

COMPILED BUNDLE CHUNK
  -> runtime loading/streaming/patch/cache locality

RUNTIME SPATIAL SECTOR
  -> actor lookup, movement/spectator/pathfinding locality
```

Crystal's separate runtime sectoring reinforces that these dimensions solve different problems. Repartitioning any of them must not renumber canonical world positions or stable placement/content identities.

## 6. Identity model

The design must preserve the following distinctions:

| Identity | Meaning |
|---|---|
| Content key | What kind of thing this is |
| Placement key | This exact authored/static occurrence in the world |
| Definition revision/digest | Exact semantic definition context |
| Source key | Stable source/materialization source where applicable |
| Runtime instance identity | Concrete live actor/item/object lifecycle |
| Reward/materialization occurrence | One authoritative chance/claim/event occurrence |
| Transaction/command identity | One mutation attempt/retry lineage |

Rules:

- Move of the same authored object may preserve its PlacementKey.
- Copy creates a distinct PlacementKey.
- Rechunking changes no semantic identity.
- A presentation-only change must not reset a materialization source or reissue a reward.
- If a legacy source has no stable placement identity, the importer maintains an explicit identity mapping. Ambiguous matching is reported as `AMBIGUOUS`, not silently guessed by nearest-position/name similarity.

## 7. Definition versus runtime instance

### Items

```text
ItemDefinition / ItemType
  = authored immutable kind and capabilities

ItemInstance
  = one concrete durable mutable item/value lifecycle
```

A static map placement that references an item-like definition is not automatically a durable ItemInstance. Materializing value into live custody is a domain/transaction operation.

### Creatures

```text
CreatureDefinition
  = authored stats/behavior/abilities/reward/presentation semantics

CreatureInstance
  = runtime identity, current HP, position, target, conditions, etc.
```

### World objects

A wall, door, fountain, lever or decoration may be a `WorldObjectDefinition`/placement without becoming an economy item. If a movable box with durable contents is a true value-bearing item, its live location/custody must remain in GAME-ITEM/DUR-03 rather than being duplicated in a second spatial state owner.

## 8. Item capability model

Do not build one unbounded arbitrary attribute bag and do not require one rigid class hierarchy for every object type.

Use typed capability families, conceptually including:

```text
Physical
Stack
Charges
Durability
Temporal/Decay
Container
Equipment
Weapon/Use
Protection/Modifiers
Binding/Transfer Restrictions
Imbuement
Light
Readable/Writeable
Fluid/Field
Attachment/Hangable
Wrap/Rotate
Materialization/Pickup eligibility
Presentation
```

Every authoritative mutable state field must be permitted by the resolved definition. Arbitrary JSON/EAV state is not an authoritative gameplay extension mechanism.

### Units

Every gameplay numeric field needs an explicit semantic unit or an owning profile that fixes the unit. Avoid ambiguous names such as `weight_oz` when the actual stored scale is unclear. Exact decimal/rational values should not be accidentally converted through floating-point authoring when determinism matters.

### Imbuement

Model explicit slots and per-slot allowed families/tiers rather than only `slots: N`. Active imbues and remaining time belong to ItemInstance state. The number and type of imbuement slots are part of the item's total gameplay power and must be included when balancing items.

### Definition evolution

Presentation-only changes, semantically compatible authoritative changes, migrations and unsupported changes are different compatibility classes. A newer definition must not silently reinterpret persisted instance state solely because the stable key stayed the same.

## 9. Creature definition coverage

Real content needs more than a name and presentation. The canonical model should be able to cover, when the selected milestone/evidence requires them:

```text
identity
presentation
stats / max HP / speed
locomotion and environment rules
behavior / targeting strategy
ability loadout
attack profiles
armor/defense/mitigation semantics
resistance/element profiles
immunities/conditions
corpse
loot and reward bindings
XP/reward definition
voices/light
public/bestiary metadata
event/native-rule references
provenance/evidence
```

Crystal's real creature definitions are a useful field-coverage checklist, not automatic Reference truth. OTS values remain migration/hypothesis evidence until qualified through the Reference evidence process.

## 10. Ability/effect direction

Content definitions should bind typed ability/effect/formula definitions while Ability/SIM remains the execution owner.

Conceptual flow:

```text
AbilityDefinition
  -> requirements/costs/cooldowns/targeting
  -> EffectDefinition(s)
  -> Formula/Ruleset binding
  -> Presentation
```

Do not embed arbitrary direct authoritative mutation scripts in item/spell files. Damage, heal, geometry, ordering and rounding remain owned by the accepted gameplay/SIM contracts and evidence target.

## 11. NPC model

Separate authored identity from placement and services:

```text
NpcDefinition
NpcPlacement
DialogueDefinition
ServiceDefinition
Shop/CatalogueDefinition
TravelDefinition
PriceRuleDefinition
Presentation/Behavior
```

An NPC's position can change without rewriting its dialogue. A catalogue can evolve without changing NPC locomotion.

The NPC architecture boundary remains server-owned. Conversation state, pending transactions, item/currency mutation and durable quest state do not belong in the Content artifact.

Crystal NPC files demonstrate that a large amount of base data, shops and service facts are extractable automatically. Procedural dialogue/quest branches should become candidate graph/native-rule requirements, not imported Lua runtime.

## 12. Quest direction

Do not preserve `storage[key] = integer` as Oteryn's canonical quest model.

The target needs stable semantic identities such as:

```text
QuestKey
QuestStageKey
ObjectiveKey
Condition
Transition
RewardDefinition
RepeatabilityPolicy
Reward/Claim Occurrence Identity
Quest Progress Revision Context
```

Parallel objectives and explicit transitions must be representable. `possess item` and `consume/hand over item` are different operations. Reward issuance requires idempotent occurrence identity and the owning durable transaction path; reconnect/retry must not duplicate rewards.

Legacy storage arithmetic is migration evidence for reconstructing candidate states/transitions. A raw numeric comparison is not enough to infer the semantic meaning of a stage without tracing reads/writes and surrounding behavior.

## 13. World/map semantic model

Canonical world coordinates retain the accepted native spatial profile:

```text
x: i32
y: i32
floor: i16
```

with explicit world bounds and declared floors. Legacy Tibia `z`/stack conventions are mapped through versioned importer profiles, not promoted by direct cast.

The conceptual cell model is:

```text
Cell(position)
  -> TerrainDefinition ref
  -> ordered static semantic placements
  -> derived/static spatial facts/indexes
  -> zone/house/interaction refs where applicable
  -> local relocation refs where applicable
```

Presentation order must be explicit and deterministic. Serializer order, item ID and filename order are not rendering or interaction authority.

## 14. Source map shards

A source shard may use sparse JSONL records. Its header explicitly states its world/floor/address/span; the filename is only a convention.

Important rules:

- missing required shard != empty world;
- explicit void != unresolved/lost import record;
- unknown legacy record is diagnostic/fail-closed for the affected executable subset, never silently dropped;
- negative coordinates use well-defined Euclidean division/indexing;
- a placement is authored once even when its footprint crosses shard boundaries;
- cross-shard indexes may reference one placement, but they must never materialize multiple gameplay instances.

## 15. Composite/multi-tile world objects

Do not assume one legacy sprite fragment equals one gameplay object and do not merge neighboring fragments merely because they visually appear related.

Keep independent concepts:

```text
semantic/gameplay footprint
collision footprint
interaction footprint
visual footprint
presentation anchor/fragments
```

A large tree may have a wide crown but a small blocking trunk. A roof can have a wide visual footprint with no independent movement block. A recognized fountain composite may become one semantic object only when a versioned mapping/pattern proves the source structure; otherwise preserve safe lower-level placements and emit diagnostics.

## 16. Spatial capability model

The earlier `blocking=true` idea is too weak. Static semantics should be able to express, when evidence supports the distinction:

```text
actor occupancy / hard movement block
projectile or line-of-effect block
pathfinding block/avoidance
height/stack/placement support
attachment/hangable surface + orientation
visual footprint
gameplay/collision/interaction footprint
```

Effective movement legality is a server-side composition of authored static facts plus current object state, dynamic actors/items, zones/house/access policy and authority context.

Do not infer two independently proven Reference values where a legacy source exposes only one combined flag. For example, a single legacy `unsight`-like fact may feed multiple old behaviors; provenance must preserve that it was one source fact.

## 17. Stateful objects

Prefer one stable logical object/placement with explicit states over representing ordinary state changes as unrelated item types.

Conceptual door:

```text
DoorDefinition
  states: closed/open/locked/...
  per-state presentation
  per-state spatial contributions
  allowed state transitions
  access/manipulation/crossing policies
```

Important corrections:

- `A -> B -> A` in a legacy source is only a state-machine candidate. It may instead represent charges, quantity, ownership or lifecycle replacement.
- permission to manipulate a door and permission to cross its threshold are distinct policies.
- a repeated operation after a lost response must not accidentally toggle state back. Command/occurrence identity and idempotency semantics must be explicit where required.
- closing behavior must define what happens when the footprint is occupied; do not invent implicit displacement of actors.
- state is scoped to the appropriate runtime world/channel/instance/generation. A PlacementKey is not a global mutable singleton.

## 18. Interaction capability and binding graph

The client must not infer behavior from sprite IDs or visual names. Authored definitions/bindings declare typed behavior; client-safe data may expose UI hints, but the server resolves legality.

Interaction families discovered as practically necessary include:

```text
USE
USE_WITH
OPEN/CLOSE state transition
OPEN_CONTAINER / container access
READ
WRITE
ROTATE
WRAP / UNWRAP
SLEEP/BED
STEP_IN
STEP_OUT
ADD_ITEM
REMOVE_ITEM
EQUIP
DEEQUIP
PICKUP
MOVE/PUSH
TELEPORT / RELOCATION
FLOOR_TRANSITION
HANG/ATTACH
NPC_TALK / SERVICE
QUEST_TRIGGER
```

Do not collapse client intent, an authoritative occurrence and a proposed downstream mutation into one callback.

### Binding resolution

Do not inherit implicit Crystal precedence such as `unique id -> action id -> item id -> position`.

Use stable typed references conceptually equivalent to:

```text
Placement/Definition target
TriggerKind
InteractionDefinitionKey / NativeRuleKey
CompositionPolicy
Priority only where the owning semantic contract actually permits it
```

Ambiguous or shadowed bindings should fail compilation unless explicit composition semantics make the combination legal.

## 19. Client projection and authority

The client may receive:

- presentation handles/data;
- visible object state;
- bounded interaction hints/menu options;
- non-authoritative movement/path hints;
- revision/generation metadata required for reconciliation.

It must not receive server-only secrets merely to hide them in UI.

A modified or stale client cannot make a movement/use/item action legal. The server resolves the target under the authoritative current generation and validates position/range/LOS, state, access, item legality and other domain conditions.

Interaction hints are hints, not permanent client-side prohibitions: stale client knowledge must reconcile when another actor changes the world.

## 20. Runtime spatial indexes

The runtime should not parse JSON and recompute all static facts on every step.

The compiler/runtime may precompute bounded revision-scoped indexes such as:

```text
CellSpatialFlags / compact spatial handle
interaction presence/index
relocation presence/index
cross-chunk footprint index
static navigation data
```

Derived indexes are not second authored sources of truth. Changing definitions and rebuilding the same semantic graph must deterministically regenerate them.

## 21. Spawn model

A useful spawn definition needs more than creature + rectangle:

```text
SpawnDefinition
  key
  anchor/zone
  spawn slots[]
    slot key
    exact position/offset
    direction
    candidate creatures
    explicit selection algorithm/weights
  respawn/recovery policy
  player-presence blocking policy
  environment requirements
  aggregate population
  GAME-CHANNEL multiplicity
  durable eligibility scope
  provenance/evidence
```

Spawn selection weights are not loot probabilities. Their algorithms and numeric domains must be typed separately.

## 22. Loot/reward semantics

A loot table must define the selection algorithm rather than only store numbers called `weight` or `chance`. Independent Bernoulli attempts, one weighted selection, guaranteed entries and nested groups are different mechanics.

Unknown target probabilities may remain candidate/UNKNOWN evidence. They must not be silently replaced with zero, guaranteed loot or an OTS value pretending to be Global truth.

Loot selection remains separate from durable materialization/pickup. A selected `Gold Coin x1` intent does not itself authorize duplicate minting after retry/restart.

## 23. Runtime load/unload is not gameplay reset

Distinguish:

```text
client visibility loss
cache eviction
region simulation sleep
instance teardown
encounter reset
source respawn/materialization policy
process restart
```

None of these technical events may implicitly recreate value or reset world state merely because a source file was loaded again.

A materialization/reset must come from an explicit owning policy/occurrence. Rechunking, cache reload and presentation changes are not reward resets.

## 24. World overlays / variants

Legacy systems dynamically load event/world-change map fragments. Oteryn should not parse raw OTBM in authoritative runtime.

Preserve a future seam for precompiled, validated immutable world changes, conceptually:

```text
WorldOverlayDefinition / WorldVariantDefinition
  -> source/import
  -> compile/validate
  -> immutable artifact
  -> authorized activation/deactivation
```

This remains deferred unless the next Reference journey actually needs it.

## 25. Evidence and provenance

A broad package label such as `reference=true` is not enough.

Target-sensitive evidence must be bindable to the actual semantic value, conceptually:

```text
entity/content key
field-family/typed selector
canonical value digest
source observation IDs + exact source revisions
target cut
classification/confidence/continuity status
declared-difference overlay where applicable
normalization/comparison profile revision
```

Changing the value invalidates the binding. Reordering a non-semantic keyed set need not; reordering an execution sequence must.

Evidence strength, implementation readiness and redistribution/license permission remain separate axes.

## 26. Release profile as closed dependency subset

A large source/candidate catalogue may contain unresolved content while a smaller playable profile is complete.

A release/profile names explicit roots and capabilities, then computes the transitive closure of:

```text
world/content roots
  -> referenced definitions
  -> interaction/native-rule bindings
  -> required presentations/client-safe refs
  -> evidence/policy/revision dependencies
  -> exact locked package revisions
```

An unfinished unrelated quest must not block the first combat corridor. A missing item referenced by an active loot table must block that selected profile.

Runtime resolves no floating `latest` package constraints.

## 27. Source locks and reproducible import

A complete migration source is rarely one file.

For Crystal/reference migration, pin exact participating source identities, for example:

```text
world/map archive + digest
appearance/catalogue digest
item definition sources
monster definitions
NPC definitions
spawn sidecars
house/zone sidecars
startup/map-attribute tables
relevant quest/action/service behavior sources
importer revision
semantic mapping profile revision
compiler/canonicalization revision
license/provenance metadata
```

The source lock is reproducibility metadata, not proof of target correctness.

## 28. Reimport and local corrections

Reimport must not blindly overwrite project-owned corrections.

Use a three-way conceptual comparison:

```text
previous imported baseline
new source revision
local Oteryn corrections/decisions
```

Classify unchanged, upstream-only changed, Oteryn-only changed and conflicting edits. Conflicts require explicit resolution. Generated import candidates and authoritative selected definitions should not be confused.

## 29. Legacy IR and diagnostics

Legacy IR is importer-boundary data only. It should preserve enough information to diagnose loss and reproduce mapping, including original IDs/positions/source paths or source-record identities where legally/operationally permitted.

Required migration outcomes should include explicit dispositions such as:

```text
COPY/EXTRACT
CONVERT/NORMALIZE
DERIVE
REWRITE_AS_NATIVE_RULE
REFERENCE_ONLY
REJECT/UNSUPPORTED
AMBIGUOUS
UNKNOWN/CONFLICT
```

No silent loss. Unknown map/object/script semantics are counted and reported.

## 30. Existing Oteryn extraction work to reuse

The repository already contains Game-owned tooling for appearance normalization, full-world/map analysis, creature/NPC extraction, NPC-role derivation and other Atlas-facing projections.

Those tools are not automatically the canonical gameplay importer, but their parsers, pinned-source discipline, deterministic output tests, coordinate conversion and ambiguity diagnostics should be reused or factored into shared import helpers where appropriate.

Do not build a parallel parser merely because the output consumer changes from Atlas to Content if the existing extraction logic already solves the same bounded source-reading problem.

## 31. CrystalServer — reference only

Owner direction is explicit: **do not copy Crystal code. Oteryn is its own implementation.** Crystal is useful only as a source/reference corpus and as evidence of practical field coverage and failure modes.

Material lessons retained:

1. appearance/protobuf data can provide bulk physical/presentation candidates and should be imported with exact source provenance;
2. `items.xml` alone is not a complete source of item semantics;
3. the monolithic Crystal ItemType is a useful field inventory but not Oteryn's target schema;
4. real content requires height, attachment orientation, floor transitions, read/write, fields/fluids, beds, decay, light, transforms, ammo/shoot, imbuements, augments, mantra/proficiency and related families;
5. implicit numeric action/unique IDs create hidden coupling and should become explicit typed bindings/keys;
6. recent Crystal diagnostics prove that shadowed interaction registrations and duplicated map bindings are real production-class content-authoring problems;
7. source map data spans OTBM plus sidecars/startup tables/definitions/scripts; import locks must represent the whole source set;
8. runtime sectoring is separate from source map representation;
9. NPC base/shop/service content is highly extractable;
10. procedural Lua and quest storage arithmetic are migration evidence for native-rule/quest-graph reconstruction, not target runtime architecture;
11. legacy ID presence must not incidentally decide mobility, lootability or custody in Oteryn;
12. dynamic world-change source data motivates precompiled overlays, not runtime OTBM parsing.

The intended relationship is:

```text
Crystal / OTBM / appearances / XML / Lua / other reference sources
                 -> bounded extraction
                 -> Legacy/Source IR
                 -> normalization + provenance
                 -> conflict/evidence analysis
                 -> Canonical Oteryn definitions
                 -> Oteryn-owned Rust implementation
```

Never treat `Crystal code -> transliterate into Rust` as the migration plan.

## 32. Content compiler / bundle direction

The source project is not what normal gameplay should parse directly.

Logical compiler path:

```text
source project + locked dependencies
 -> parse
 -> typed model
 -> structural/semantic validation
 -> typed reference/link resolution
 -> migration normalization
 -> profile/evidence/resource validation
 -> deterministic lowering
 -> server-authoritative projection
 -> client-safe projection
 -> immutable indexed artifacts
 -> integrity verification
```

Keep manifest/section integrity, bounded allocation/decompression, explicit required capabilities and exact profile/revision binding.

The exact binary payload technology and compression remain undecided. FlatBuffers is only a candidate to benchmark against real world data; successful structural forward compatibility is not permission for an old runtime to ignore a new gameplay-critical capability.

## 33. Compilation diagnostics / `content explain`

Provide first-class diagnostics rather than forcing developers to reverse-engineer why an object behaves as it does.

A future command/Studio inspector should be able to explain a definition/placement:

```text
identity + exact revision
source/provenance/evidence
presentation
spatial contributions
current supported authored states
interaction bindings and rule owners
access requirements
spawn/materialization/reset policy where applicable
client-safe projection versus server-only data
legacy mappings and unresolved/lossy conversion records
```

This is the Oteryn-native evolution of the useful debugging information seen in mature OTS systems, without inheriting their implicit resolution precedence.

## 34. Validation findings from the conversation prototypes

The synthetic v0.1/v0.2 demonstrator was exercised during the analysis. The final reviewed prototype suite reached **63 passing tests** after correcting five categories of demonstration-layer gaps:

- regex/end-of-input acceptance that could admit a decoded trailing newline in candidate identifiers/numbers;
- the same lexical problem for content keys/short identifiers;
- a direct in-memory normalize path that did not reject an unknown cell field even though the normal file-read path had schema validation;
- an empty shard entirely outside world bounds being accepted because only contained cells were checked;
- duplicate candidate field paths, including conflicting values, lacking explicit uniqueness enforcement.

These are demonstrator/test findings, **not claims of five production Oteryn vulnerabilities**.

Additional prototype checks covered negative coordinate shard math, half-open bounds and a semantic snapshot fingerprint wider than a pure spatial-cell fingerprint.

The prototype did not prove Rust integration, renderer correctness, PostgreSQL durability, full-world resource behavior or production Reference parity.

## 35. Corrections to earlier premature conclusions

The following statements from early brainstorming are explicitly superseded:

- JSON5 is not automatically selected as permanent source format.
- `32x32` is not frozen as permanent source/runtime chunking.
- one generic `blocking` boolean is insufficient.
- separate `vision` and `projectile` Reference fields must not be invented when evidence exposes one combined source fact.
- legacy `A -> B -> A` does not automatically prove one stateful object.
- quest integer storages cannot be automatically labeled as named stages without tracing their semantics.
- moving a durable value-bearing box is not merely a world-position mutation; item custody/location ownership still applies.
- reloading/unloading an area is not a respawn/reset operation.
- opening a container and opening a door are distinct capabilities.
- a client hint is never authoritative legality.
- the correct goal is not to copy Crystal architecture/code, but to use its content/reference evidence to improve Oteryn's own model.

## 36. Minimum next build sequence

The highest-value sequence after this analysis is:

1. finalize only the minimum source-contract semantics required by the next playable proof;
2. define typed authored state/interactions/spatial semantics for representative wall, door, container/pickup item and transition;
3. reuse/extend current Oteryn source extractors under one pinned generic legacy-source manifest;
4. implement a bounded source parser/linker/validator with duplicate/unresolved/ambiguous/shadowed binding diagnostics and zero-silent-loss conversion reporting;
5. import one real bounded corridor plus one ordinary creature, one item, one NPC/service and representative spatial/interactable objects;
6. feed that closed subset into the explicit Reference successor profile rather than mutating the bootstrap profile by convenience;
7. exercise the real runtime/client path: movement/collision, one stateful interaction, client observation/reconciliation, creature death/corpse/loot, one durable item materialization + transfer, retry/reconnect/restart;
8. expand the candidate catalogue in parallel, promoting only closed/evidenced executable subsets.

Do not make full Studio, full Global quest graphs, final compression/container technology, full scripting host or full catalogue import prerequisites for the first real playable evidence unless an accepted invariant actually requires them.

## 37. Open decisions that remain real

The analysis intentionally leaves the following unresolved until representative evidence exists:

- permanent source text/container selection;
- final source shard dimensions;
- final compiled chunk/section layout;
- runtime spatial-sector dimensions;
- binary payload technology and compression;
- complete quest graph/runtime/durable migration model;
- broad encounter/raid/world-overlay activation semantics;
- full script/WIT capability surface and host implementation;
- exact resource ceilings for real full-world/profile data;
- exact target values for any Reference field still UNKNOWN/CONFLICT;
- later Oteryn Evolved balance/content differences.

## 38. Final position

The project is ready to stop treating content structure as a distant post-Server-Seam concern. We have enough architecture to begin a real authoring/import/link/validation foundation now, while keeping promotion to executable profiles fail-closed.

The most important next design/build problem is not inventing more folder names. It is connecting authored definitions to **unambiguous runtime execution and state ownership**:

```text
what an object is
+ what it can do
+ where this occurrence is
+ what state it is currently in
+ who owns that state/mutation
+ how retry/restart/reimport behave
+ what the client is allowed to know
= production-usable content semantics
```

This should be proved on a small real path before generalizing further.
