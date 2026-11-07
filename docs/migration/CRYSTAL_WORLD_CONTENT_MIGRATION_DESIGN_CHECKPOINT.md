# Crystal world/content migration design checkpoint

- Status: `PROPOSED / TO_REVISIT`
- Date recorded: 2026-08-24
- Related accepted architecture: `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md`
- Related design tracking: GitHub Issue #64, `Design checkpoint: OTBM migration and composite world presentation`
- Authority: informational migration/design evidence only; this file does **not** amend ADR-0005, freeze schemas, allocate implementation work, or change active Wave-1 ownership.

## 1. Purpose

Preserve the current design conclusions and source-corpus observations so they can be deliberately re-evaluated when Oteryn reaches the implementation stage for the full native world schema, Crystal/OTBM migration, production item/creature content, boss encounters, presentation/assets, renderer, World Bundle, or Oteryn Studio.

The practical starting point is that a rich legacy content corpus already exists. The expected work is therefore primarily **semantic migration and normalization**, not manual recreation of every map tile, item, creature, spawn, boss and visual asset.

The accepted architectural boundary remains:

```text
legacy/external source corpus
        -> bounded importers
        -> Legacy Intermediate Representation
        -> semantic mapping + normalization + diagnostics
        -> Canonical Oteryn World/Content Model
        -> deterministic compiler
        -> client-safe + server-authoritative runtime bundles
```

OTBM, legacy numeric item IDs, old appearance/sprite IDs, Lua organization and Crystal/TFS runtime conventions remain migration evidence, not canonical Oteryn runtime semantics.

## 2. Verified source-corpus snapshot

The following observations were verified on 2026-08-24 against the owner-supplied `data-global.zip`. The archive itself is **not** committed by this note and these numbers are evidence for planning, not a permanent content contract.

### 2.1 Archive shape

- 5,061 files (5,403 ZIP entries including directories);
- 84,362,168 uncompressed bytes across files;
- 4,854 `.lua` files;
- 164 `.xml` files;
- 32 `.otbm` files;
- 1,802 files under `data-global/monster/`;
- 1,112 files under `data-global/npc/`;
- 152 files under `data-global/raids/`;
- 1,897 files under `data-global/scripts/`;
- 54 files under `data-global/world/`.

Monster definitions are not organized as one clean boss/non-boss taxonomy:

- 730 monster Lua files under `monster/quests/`;
- 95 under `monster/bosses/`;
- 55 under `monster/raids/`;
- 296 monster Lua files contain `bosstiary` metadata.

This is evidence that import classification must be semantic and data-driven rather than based only on directory names.

### 2.2 Main map and sidecars

`data-global/world/world.otbm` is 52,836,960 bytes in the archive and starts with a gzip header. After decompression it is 186,660,172 bytes. Its header contains:

- `Saved with RME 4.0.0`;
- references to `world-monster.xml`;
- references to `world-npc.xml`;
- references to `world-house.xml`;
- reference to `world-zones.xml`.

Relevant sidecar sizes in this corpus:

- `world-monster.xml`: 10,097,048 bytes;
- `world-house.xml`: 153,487 bytes;
- `world-npc.xml`: 136,335 bytes;
- `world-zones.xml`: 83 bytes.

Therefore the actual authored world is distributed across binary map data and sidecars rather than contained in one semantically complete file.

### 2.3 Missing item/appearance catalog in this archive

No `items.xml`, `items.otb` or appearance catalog was found in the supplied archive. Full item-definition and presentation migration cannot be considered complete from `data-global.zip` alone. The exact matching item/appearance catalogs and their revisions/digests will be required before full-world import can be verified.

## 3. Accepted baseline from ADR-0005

ADR-0005 remains authoritative. In particular:

- Oteryn will not use OTBM as the canonical editable or runtime world format;
- OTBM/OTB/XML/appearances/sprites are bounded migration inputs;
- canonical editable source and compiled runtime representation are separate;
- stable namespaced content keys are canonical identity;
- legacy numeric IDs are mappings/provenance, not permanent identity;
- authored static world definitions are separate from mutable authoritative runtime/persistence state;
- server-authoritative and client-safe sections may differ;
- chunks are technical streaming/cache units and do not define semantic geography;
- sprites and appearances belong behind a project-owned asset/presentation pipeline;
- import must be deterministic, bounded, provenance-aware and diagnostic rather than silently lossy.

Everything below is candidate detail to revisit, not a schema freeze.

## 4. Map / OTBM migration direction

### 4.1 OTBM is source syntax, not world semantics

The target should not be an `OTBM 2` written in Rust.

Candidate interpretation:

```text
OTBM tile/house-tile/item nodes
        + pinned item/appearance catalogs
        + XML sidecars
        + relevant script semantics
        -> Legacy World Graph
        -> Canonical Oteryn World
```

The running server should consume native compiled world data rather than repeatedly interpret OTBM rules in normal gameplay.

### 4.2 Canonical cell contents

A candidate native cell model should represent meaning rather than source encoding:

```text
Cell(position)
  -> terrain reference
  -> ordered semantic placements
  -> collision/navigation metadata
  -> zone/house/interaction references
  -> compiler-derived indexes where appropriate
```

The ordered-placement requirement is important: Tibia-style presentation depends on ordering/depth semantics and must not be collapsed into an unordered set.

### 4.3 Stable identities and import provenance

Source numeric IDs should resolve through pinned mappings to stable Oteryn keys, for example:

```text
legacy item id
    -> revisioned migration mapping
    -> oteryn:item.* / package:* stable key
```

Legacy IDs should remain available only as provenance/evidence where useful for diagnostics and re-import.

### 4.4 Exact source revision lock

The future importer should bind exact input revisions/digests. Candidate manifest inputs include:

- OTBM path/revision/digest;
- item catalog revision/digest;
- appearance/asset catalog revision/digest;
- creature/spawn/NPC/script corpus revision;
- importer version;
- semantic mapping-profile revision;
- compiler/canonicalization revision;
- source/provenance/license metadata.

If an OTBM item ID cannot be resolved unambiguously against the pinned catalog, import should fail or produce an explicit blocking diagnostic rather than substitute another revision silently.

### 4.5 Chunked native runtime representation

The canonical authored model should compile into indexed chunks suitable for bounded server loading, client streaming, patching and cache invalidation. Exact chunk dimensions and vertical packing remain benchmark decisions.

Do not freeze 32x32, 64x64 or another size without evidence from:

- server locality;
- client streaming;
- pathfinding;
- map-editor behavior;
- bundle size;
- patch granularity;
- static/dynamic rendering cost.

## 5. Sprites, presentation and multi-tile objects

### 5.1 Sprites are not the problem

Sprite-based 2D remains a valid target visual technology for Oteryn. The modernization target is **coupling and runtime delivery**, not replacing sprites merely because the source technology is old.

Candidate client pipeline:

```text
semantic object/content key
        -> PresentationDefinition
        -> compiled presentation handle
        -> sprite regions / frames / effects
        -> texture atlas(es)
        -> GPU batching / specialized 2D renderer
```

Server/domain logic should never need a legacy sprite number to understand gameplay behavior.

### 5.2 One sprite fragment must not automatically equal one gameplay object

The source map contains recognizable objects assembled from neighboring fragments, such as:

- a fountain composed from four quarter-elements;
- a large tree composed from multiple trunk/crown elements;
- roofs, statues, ruins and larger decorations split across multiple tile pieces.

Oteryn should preserve the option to represent these as one semantic object/structure with one presentation made from multiple visual fragments.

Candidate separation:

```text
WorldObject / Structure / Placement
  -> semantic identity
  -> gameplay footprint
  -> collision footprint
  -> interaction footprint
  -> PresentationDefinition

PresentationDefinition
  -> visual bounds
  -> anchor/origin
  -> ordered layers/fragments
  -> animation/directions
  -> asset references
```

The exact names/types are TBD.

### 5.3 Different footprints are first-class concepts

Do not make the permanent invariant:

```text
visual footprint == gameplay footprint == collision footprint == interaction footprint
```

Examples:

- large tree: large visible crown, small blocking trunk;
- fountain: 2x2 visual and likely 2x2 physical footprint;
- roof: large visual extent, potentially no independent collision;
- large decorative structure: visual fragments can overlap many cells while gameplay interaction remains anchored to a smaller area.

### 5.4 Composite recognition must be explicit

The importer must not merge arbitrary neighboring items because they appear related.

Candidate solution: revisioned `LegacyCompositeMapping`/`CompositePattern` data that describes exact recognized source layouts. Example:

```text
stone fountain pattern
  NW -> legacy fragment A
  NE -> legacy fragment B
  SW -> legacy fragment C
  SE -> legacy fragment D
```

If a complete recognized pattern exists, import may create one semantic object plus one multi-fragment presentation. If the pattern is incomplete, conflicting or unknown, preserve a safe lower-level representation and emit a diagnostic rather than guessing.

### 5.5 Presentation can evolve independently

A logical fountain imported today as four sprite fragments could later render from one larger image, a differently packed atlas, or a new animation without changing the canonical gameplay object, map topology or server logic.

This decoupling is a core reason to keep presentation separate from world/content semantics.

## 6. Native item/content model

### 6.1 Definition versus instance

Preserve a strict distinction:

```text
ItemDefinition
  = what kind of thing this is

ItemInstance
  = one concrete durable/runtime item and its mutable state/location
```

Examples of instance state can include:

- durable identity;
- quantity/charges;
- durability/decay state;
- custom name or augmentations;
- owner/location;
- container contents;
- other mutable runtime values.

The definition should remain shared content, not copied per instance.

### 6.2 Prefer composition/capabilities over one rigid item class enum

A weapon, food item, potion, backpack, ring or rune should not require unrelated special-case hierarchies merely because legacy code organizes them separately.

Candidate definition composition:

```text
ItemDefinition
  + Equipable
  + Weapon
  + Protection/Armor
  + Container
  + Consumable
  + UseEffect
  + Ammunition
  + Stackable
  + Currency
  + LightSource
  + Decay
  + InteractionKey
  + transfer/trade/quest policies
  + presentation
```

This allows valid combinations such as an equipable charged light source or an item that is both consumable and ability-triggering.

Exact capability names and ownership remain TBD.

### 6.3 Candidate examples

Weapon:

```text
item:great_sword
  equipable -> hand slots / handedness
  weapon -> class / attack profile / requirements / range
```

Armor:

```text
item:plate_armor
  equipable -> torso
  protection -> armor/resistance profile
```

Potion:

```text
item:health_potion
  consumable
  use-effect -> restore-health effect
  targeting/requirements
```

Food:

```text
item:ham
  consumable
  use-effect -> regeneration/satiation effect
```

Other mappings to consider: ammunition, runes, rings, containers, keys, currency, quest items, tools, decay/light items and items with scripted interaction semantics.

### 6.4 Legacy script behavior should become native semantics where possible

Do not make permanent production item behavior depend on `item -> legacy Lua file` by default.

Preferred migration:

```text
legacy action/script
  -> classified semantic behavior
  -> native Interaction / Ability / Effect / policy
```

Importer outcomes should explicitly distinguish automatic semantic conversion from cases that need a native rule or remain unsupported.

## 7. Creatures and ordinary monsters

### 7.1 Definition versus runtime instance

Preserve the same separation as items:

```text
CreatureDefinition
  -> stats
  -> behavior reference
  -> abilities/effects
  -> loot
  -> XP/reward definitions
  -> presentation
  -> bestiary metadata

CreatureInstance
  -> concrete runtime identity/state/position/target/HP/etc.
```

### 7.2 Verified Demon fixture candidate

`data-global/monster/demons/demon.lua` is a useful representative fixture because it is mostly declarative and contains, in one definition, health/stat/bestiary/targeting/summon/voice/loot/attack/defense/element/immunity data.

Verified values from the supplied corpus include:

- experience: 6000;
- health/max health: 8200;
- speed: 128;
- armor: 44;
- charm points: 50;
- mitigation: 1.74 in this Crystal source revision.

The file also contains `lookType`/presentation linkage, attacks, resistances and loot. This makes it a strong future import/parity fixture.

The important architectural rule is that the canonical `creature:demon` identity must not be inseparable from the legacy `lookType`, sprite number or one historical tuning revision.

### 7.3 Versioned truth, not internet-name truth

External sites can be valuable verification/reference oracles, but a name such as `Demon` must not imply one timeless authoritative stat set.

Owner-supplied comparison references discussed during this checkpoint include:

- `https://tibiopedia.pl/monsters/Demon`
- `https://tibia.fandom.com/wiki/Demon`

Observed differences between external presentations and the supplied Crystal revision reinforce the need for:

- exact source revisions;
- canonical Oteryn definition revisions;
- machine-readable parity reports;
- no automatic overwrite of Oteryn values from an external wiki.

A future parity tool could compare fields such as HP, XP, armor, speed, mitigation, damage ranges, resistances, loot and presentation metadata and classify exact matches versus revision/custom differences.

## 8. Bosses and encounters

### 8.1 A boss creature is not the whole encounter

Do not encode a boss as merely `Monster + more HP` or force all encounter orchestration into `CreatureDefinition`.

Candidate separation:

```text
CreatureDefinition
  -> boss actor itself

EncounterDefinition
  -> arena/scope
  -> triggers/eligibility
  -> boss/add participants
  -> phases
  -> mechanics
  -> spawn/anchor policy
  -> completion/failure/reset
  -> uniqueness/multichannel policy
  -> reward eligibility/policy
```

This aligns with ADR-0005's accepted separation of semantic geography and encounter placement concepts (`EncounterZone`, `RaidCell`, `RaidAnchor`).

### 8.2 Verified evidence: Morgaroth is already split across systems

The supplied corpus contains:

- `data-global/monster/raids/morgaroth.lua` — creature/boss definition;
- `data-global/raids/liberty_bay/morgaroth.xml` — raid announcements and spawn timing/location;
- `data-global/scripts/spells/monster/morgaroth_summon.lua` — custom summon behavior.

Therefore a boss importer that reads only `monster/*.lua` cannot reconstruct the full encounter.

### 8.3 Verified evidence: Soul War mechanics are shared procedural behavior

The supplied corpus contains:

- `data-global/monster/quests/soul_war/goshnars_cruelty.lua` (156 lines);
- `data-global/scripts/quests/soul_war/soul_war_mechanics.lua` (1,103 lines).

The boss definition references events while significant encounter mechanics and death/progression behavior live in shared quest script code. Semantic extraction therefore has to trace event/script relationships rather than assume one file equals one boss.

### 8.4 Verified evidence: Ferumbras behavior is distributed

The supplied corpus contains:

- `monster/quests/ferumbras_ascension/bosses/ferumbras_mortal_shell.lua` (184 lines);
- `scripts/spells/monster/ferumbras_electrify.lua` (40 lines);
- `scripts/spells/monster/ferumbras_soulfire.lua` (42 lines);
- `scripts/quests/ferumbras_ascension/creaturescripts_bosses_kill.lua` (150 lines);
- additional Ferumbras-specific death handling.

This is another representative fixture for future encounter-semantic extraction.

### 8.5 Multichannel boss policy must be explicit

Future encounter schema needs a deliberate distinction between candidates such as:

- channel-local repeatable encounters;
- channel-local encounter with shared eligibility/cooldown;
- world-scoped unique encounter;
- event-policy-defined uniqueness.

The current CONTENT VSL already explores multiplicity vocabulary, but this note does not freeze its final encounter schema.

### 8.6 Boss rewards are broader than corpse loot

A boss encounter may produce multiple reward channels:

- ordinary loot table;
- personal reward eligibility;
- quest progression/rewards;
- achievements/bestiary progress;
- world-event rewards;
- cooldown/participation state.

Do not force all of these into one `CreatureDefinition.loot` field.

## 9. Legacy Semantic Graph: candidate intermediate target

The supplied corpus demonstrates that source semantics are distributed across:

```text
OTBM
XML sidecars
monster Lua
NPC Lua
spell Lua
quest Lua
CreatureEvents
raid XML
global events
storage/KV conventions
appearance/item catalogs outside this archive
```

Therefore the future import architecture should be considered as a **corpus importer**, not only an OTBM parser.

Candidate intermediate flow:

```text
OTBM parser        XML parsers        Lua/static analysis/import adapters
      \                |                /
       \               |               /
        -> revisioned Legacy Semantic Graph
                    -> normalization/mapping
                    -> conversion diagnostics
                    -> Canonical Oteryn World/Content
```

The intermediate graph should preserve enough source provenance to explain exactly why a canonical object, behavior or relationship was produced.

## 10. Conversion diagnostics and zero-silent-loss principle

A full-world/content migration should produce a machine-readable report. Candidate outcome classes:

- converted without material loss;
- converted using an explicit deterministic mapping;
- preserved in safe lower-level form with warning;
- unsupported / needs native rule;
- invalid/rejected;
- missing content/asset reference;
- conflicting legacy mapping;
- ambiguous script/event ownership;
- external parity difference / source-revision difference.

Unknown critical semantics must not disappear silently.

A future acceptance target for representative/full imports should include explicit coverage for:

- all map cells/tiles parsed within limits;
- all legacy item IDs resolved or reported;
- houses/towns/teleports/zones mapped or reported;
- creature/spawn/NPC references resolved or reported;
- composite-object pattern recognition deterministic and auditable;
- all script/event references classified;
- no unknown critical attributes silently dropped;
- deterministic output for identical pinned inputs and importer versions.

## 11. Server/client projection boundary

The canonical content model may contain information that must not be exposed to the client.

Candidate server-authoritative data includes:

- hidden quest/interaction conditions;
- loot probabilities and reward authority;
- AI/behavior internals;
- spawn eligibility and event policy;
- secret trigger data;
- authoritative formulas and durable-value rules.

Candidate client-safe data includes:

- visible terrain/objects;
- display names/descriptions permitted by product rules;
- presentation/animation/effect handles;
- visible creature/item properties;
- geometry and collision information intentionally exposed for rendering/prediction.

The current CONTENT work already validates the concept of separate server/client projections, but the full production schema remains future work.

## 12. Revisit triggers

This checkpoint must be actively revisited, rather than passively treated as accepted design, when any of the following work is allocated:

- full native world schema beyond the current VSL evidence model;
- OTBM/Crystal import pipeline;
- item/content registry production model;
- creature/monster import and production creature model;
- boss/raid/encounter architecture;
- Oteryn World Project or World Bundle implementation;
- production presentation/asset pipeline;
- native map renderer;
- Oteryn Studio world/content/asset tooling;
- migration parity/audit tooling.

At that point the executor/coordinator should explicitly inspect this checkpoint and Issue #64 before freezing the affected schema or implementation contract.

## 13. Required work before freezing these ideas

Before promoting candidate details from this note into accepted ADR/schema/contracts:

1. pin exact Crystal source revisions and the missing matching item/appearance catalogs;
2. build a representative fixture corpus covering ordinary items, weapons, armor, food, potions, containers, runes/ammunition, static decorations, multi-tile objects, houses, teleports, ordinary monsters, spawns, NPCs, simple bosses and complex scripted bosses;
3. prototype a bounded Legacy Intermediate/Semantic Graph;
4. prototype at least one full-region/full-subset import from the real corpus;
5. render representative imported world chunks with correct ordering and multi-cell presentation;
6. benchmark chunking, bundle shape, atlas/presentation handles and loading/patching strategy;
7. build conversion-loss diagnostics and prove zero silent dropping of critical semantics;
8. define how script semantics are classified and which cases require manual native rules;
9. validate exact server/client information boundaries;
10. run legal/provenance review for all source code/data/assets intended for redistribution;
11. use external sites only as comparison evidence, not unversioned truth authority;
12. promote only verified decisions into accepted architecture/contracts.

## 14. Explicit non-goals now

This checkpoint does **not**:

- allocate implementation;
- authorize changes to active Wave-1 work;
- define the final canonical world schema;
- define the final item capability schema;
- define the final creature/encounter schema;
- choose the final `.omap`/`.owb` encoding;
- choose a final chunk dimension;
- choose a final atlas/renderer representation;
- claim every multi-part legacy visual should become one object;
- authorize a permanent legacy Lua compatibility runtime;
- authorize redistribution of legacy/proprietary assets;
- make Tibiopedia, TibiaWiki/Fandom, Crystal, RME or another external project an Oteryn truth authority.

The purpose is to ensure these design observations are not forgotten and are deliberately re-examined when the relevant production work begins.
