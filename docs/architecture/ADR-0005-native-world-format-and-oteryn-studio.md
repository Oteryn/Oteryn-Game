# ADR-0005: Native Oteryn world format, integrated Studio and legacy conversion boundary

- Status: Accepted
- Date: 2026-08-05
- Decision owners: Oteryn project owner and Oteryn v2 architecture programme
- Coordination ID: `OTV2-NATIVE-WORLD-CONTENT`
- Related: ADR-0001, ADR-0002, `FOUNDATION_DECISION_BACKLOG.md`, `OTHERYN_REFERENCE_MIGRATION_PLAN.md`

## Context

Oteryn v2 is a greenfield native Rust client/server stack. The world and content architecture must not inherit OTBM, OTB, legacy client IDs or the internal constraints of historical OpenTibia editors as its canonical model.

The owner explicitly chose a clean native design instead of a hybrid or an "OTBM 2" evolution. Existing maps, items, appearances, sprites and related content remain valuable migration sources, but legacy formats are adapters at the boundary rather than foundations of the target runtime.

The owner also identified a gameplay-design problem visible in area/subarea-based dynamic encounters: a logical subarea may be appropriate for discovery, naming and progression while being too large or geometrically unsafe as the execution boundary for a raid. This ADR therefore separates player-facing geography, technical streaming partitions and precise encounter-placement units.

The observation about contemporary Tibia area/subarea and Echo-Raid-like behavior is owner-supplied design context. Oteryn does not copy that implementation or treat it as an external contract.

## Decision

### 1. Build a native format from zero

Oteryn will define its own versioned world/content model and will not use OTBM as the canonical editable or runtime format.

The architecture has three distinct representations:

```text
Legacy inputs and external packages
        |
        v
Bounded importers and Legacy Intermediate Representation
        |
        v
Canonical Oteryn World/Content Model
        |
        +--> editable Oteryn World Project
        |
        +--> deterministic compiler and validation
                  |
                  v
           Oteryn World Bundle
                  |
                  v
           client/server runtime
```

The exact file extensions remain implementation details until the format contract is frozen. The working names are:

- **Oteryn World Project** — editable source representation, commonly referred to as `.omap` during design;
- **Oteryn World Bundle** — compiled runtime representation, commonly referred to as `.owb` during design.

The names may change without changing this ADR. The separation between editable source, canonical model and compiled runtime bundle is mandatory.

### 2. Editable project requirements

The editable project is optimized for authoring, review, migration and source control rather than direct runtime loading.

It must support:

- independent, versioned manifests;
- regions and lazily loaded chunks;
- partial and atomic saves;
- deterministic ordering and serialization;
- small diffs and practical Git review;
- parallel editing with bounded conflict scope;
- stable references between maps, items, creatures, scripts and assets;
- migration history and provenance metadata;
- preservation of editor-only metadata without exposing it to runtime;
- recovery journals, autosave and pre-migration backups;
- validation of missing, unknown and conflicting references;
- forward-compatible optional fields and fail-closed critical fields.

A possible source layout is:

```text
world-project/
  manifest
  world/
    regions/
    chunks/
    areas/
    subareas/
    zones/
    encounters/
    houses/
    spawns/
    prefabs/
  content/
    items/
    creatures/
    npcs/
    spells/
    quests/
    interactions/
  assets/
    appearances/
    sprites/
    animations/
    effects/
    missiles/
    sounds/
    music/
  scripts/
  metadata/
    editor-state/
    migrations/
    provenance/
```

This layout is illustrative. The implementation contract may use directories, a container, or both, provided it preserves the accepted properties.

### 3. Compiled runtime bundle requirements

The runtime bundle is read-only, deterministic and optimized for secure loading and streaming.

It must support:

- indexed chunk access;
- bounded decompression and allocation;
- per-section or per-chunk checksums;
- manifest checksum and content revision;
- optional signing for release distribution;
- deterministic output for identical source, compiler and configuration;
- explicit minimum reader/runtime versions;
- independent client-safe and server-authoritative sections where required;
- partial download or patching without accepting unverified fragments;
- fast loading without editor metadata or source-control concerns;
- rejection of corrupt, oversized, unsupported or incompatible content.

At minimum, the bundle metadata distinguishes:

- `project_format_version`;
- `world_schema_version`;
- `content_revision`;
- `compiler_version`;
- required runtime/protocol capabilities;
- source/provenance summary;
- checksums and signature metadata.

The runtime must not deserialize unstable Rust memory layouts or treat an implementation-specific serializer as a permanent public format without a separately accepted schema contract.

### 4. One integrated application: Oteryn Studio

Oteryn will build one project-owned authoring application, **Oteryn Studio**, instead of making separate legacy tools the canonical workflow.

Oteryn Studio contains coherent modules sharing the same content registry, command history, validation engine and preview pipeline:

1. **World Editor**
   - terrain, floors, tiles and static objects;
   - regions and technical chunks;
   - areas, subareas and semantic zones;
   - houses, towns, teleports and waypoints;
   - spawn definitions and encounter definitions;
   - instances and reusable prefabs;
   - collision, navigation and environmental metadata.

2. **Asset Editor**
   - sprites and sprite sheets/atlases;
   - appearances and outfits;
   - animations, effects and missiles;
   - sounds, music and asset metadata;
   - import, inspection and transformation pipelines with provenance.

3. **Content Editor**
   - items and item behavior metadata;
   - monsters, NPCs and loot;
   - spells, professions and interactions;
   - quests, events and content relationships.

4. **Validation and diagnostics**
   - broken references and missing assets;
   - invalid teleports, spawns and encounter anchors;
   - inaccessible or isolated areas;
   - conflicting zone rules;
   - invalid content IDs or migration mappings;
   - unsafe chunk size, density or bundle limits;
   - conversion loss and unsupported legacy semantics.

5. **Preview and simulation support**
   - map rendering aligned with the client rendering contract;
   - animation, effect, lighting and audio preview;
   - collision and pathfinding overlays;
   - encounter eligibility visualization;
   - bounded test execution of a selected region when runtime support exists.

The target desktop direction is a Rust core with a Tauri-based shell and a modern web UI such as Svelte. The final viewport technology must be selected through a prototype and benchmarks. A large tile world must not be implemented as an unbounded DOM grid; the map viewport requires a GPU-capable or otherwise specialized renderer.

### 5. Stable content identity

Canonical content identity uses stable namespaced keys, for example:

```text
oteryn:item.currency.crystal_coin
oteryn:creature.dragon
package.example:item.custom_blade
```

Legacy numeric IDs and compact runtime IDs are mappings, not canonical identity.

The compiler may assign compact numeric runtime IDs for performance. Those IDs are scoped to a compiled content revision and must never be assumed stable across arbitrary builds unless the content contract explicitly guarantees it.

The Content Registry owns:

- stable keys;
- package and namespace ownership;
- dependency resolution;
- legacy ID mappings;
- collision detection;
- deprecation and migration aliases;
- client-visible versus server-only metadata;
- conversion provenance.

Map data refers to stable content keys in source and resolved identifiers in the compiled bundle.

### 6. Static world source versus dynamic state

The world project and runtime bundle contain static or authored definitions, including:

- terrain and static object placement;
- collision and navigation metadata;
- areas, subareas and zones;
- house topology and authored definitions;
- spawn and encounter definitions;
- triggers, prefabs and script references;
- environmental, audio and visual profiles.

Authoritative mutable gameplay state belongs to the game runtime and PostgreSQL ownership defined by the persistence architecture, including:

- characters and progression;
- dynamic item ownership and placement;
- player-modified house contents;
- quest and world-state progress;
- market, depot, bank and inventory state;
- active encounter instances and cooldowns where durability is required.

A running server must not rewrite the canonical authored map project as its persistence mechanism.

### 7. Spatial model: semantic, technical and encounter layers

Oteryn uses separate, composable spatial concepts.

#### Player-facing and semantic geography

- `Area` — large named or progression-oriented geography.
- `Subarea` — a logical part of an area used for discovery, labels, achievements, ambient profiles, broad gameplay policy or analytics.
- `Zone` — an overlapping semantic/rules boundary such as protection, house, no-logout, quest, weather, audio or PvP policy.

Areas and subareas are not required to align to chunk boundaries. Zones may overlap when their precedence and conflict rules are explicit.

#### Technical partitioning

- `Region` — a technical grouping used for ownership, packaging, tooling, streaming or operational policy.
- `Chunk` — the bounded load/save/stream/cache unit.

The initial candidate chunk size is 32x32 or 64x64 tiles over a defined floor strategy. The final dimensions and vertical packing must be selected by benchmark against editor behavior, bundle size, server locality, pathfinding, client streaming and patch granularity.

A single subarea may span many chunks. A chunk may intersect multiple semantic boundaries. The compiler creates spatial indexes so runtime checks do not require expensive polygon evaluation for every operation.

#### Precise dynamic-encounter placement

- `EncounterZone` — the allowed semantic and policy scope for one class of dynamic encounter.
- `RaidCell` — a smaller validated placement unit within an encounter zone.
- `RaidAnchor` — a concrete preferred or fallback origin used to instantiate an encounter.

The resulting model is:

```text
World
  Area
    Subarea
      EncounterZone
        RaidCell
          RaidAnchor

World
  Region
    Chunk

World
  overlapping semantic Zones
```

These hierarchies are related by spatial indexes, not parented into one artificial tree.

### 8. Encounter safety and Echo-Raid-like mechanics

Dynamic raids or encounters must not spawn across an entire oversized subarea merely because that subarea is the discovery or naming unit.

An encounter definition may specify:

- allowed event types;
- permitted floors and geometry;
- minimum and maximum players;
- cooldown and maximum concurrent instances;
- allowed raid sizes;
- preferred anchor policy;
- minimum distance from entrances, exits, houses or protected zones;
- forbidden terrain, narrow passages or blocking tiles;
- quest, level, time, weather or world-state conditions;
- scaling and despawn policy;
- channel-local versus world-shared uniqueness policy.

During compilation or validation, Studio produces encounter eligibility data such as:

- reachable and not isolated;
- valid floor and collision state;
- enough contiguous space for a small, medium, large or boss encounter;
- no protection/house/no-spawn conflict;
- usable entry and exit paths;
- no critical corridor blockage;
- safe anchor and fallback anchors.

The bundle may contain precomputed masks or indexed candidate sets:

```text
eligible_for_small_encounter
eligible_for_medium_encounter
eligible_for_large_encounter
eligible_for_boss
```

Runtime selection performs bounded checks against current occupancy and state, then selects a validated cell/anchor. A kill-triggered encounter may use the death position as a hint but may move to the nearest valid anchor in the same allowed encounter scope.

This design allows a subarea to remain meaningful to players while dynamic mechanics operate on precise, safe geometry.

### 9. Legacy conversion boundary

OTBM, OTB, XML sidecars, appearances, sprites and Otheryn/Canary-era content are external inputs handled by bounded importers.

The import pipeline is:

```text
legacy files
  -> parser with size/depth/count limits
  -> Legacy Intermediate Representation
  -> semantic mapping and normalization
  -> conversion diagnostics
  -> canonical Oteryn project
```

The conversion report classifies at least:

- converted without material loss;
- converted with an explicit mapping;
- defaulted with warning;
- unsupported and preserved as opaque evidence where safe;
- rejected as invalid or unsafe;
- missing content/asset reference;
- conflicting legacy ID;
- information that cannot round-trip.

Import must be deterministic for the same pinned inputs and configuration. Malformed or hostile content requires negative tests, hard limits and fuzz/property coverage where practical.

OTBM export is optional and constrained. Oteryn must not promise lossless round-trip export for native features that OTBM cannot represent. Any exporter must clearly report omissions or fail rather than silently corrupting semantics.

### 10. Use of Remere's Map Editor and Beats Assets Editor

Remere's Map Editor and Beats Assets Editor are reference and migration sources, not canonical dependencies.

Approved uses include:

- studying workflows and expected editor operations;
- producing or opening legacy fixtures;
- comparing import/export behavior;
- serving as behavioral or compatibility oracles;
- identifying required map, item, appearance and asset features;
- informing independently designed Oteryn Studio UX.

Not approved without explicit legal and architecture review:

- copying code or UI components;
- copying icons, sprites or other assets;
- line-by-line reimplementation intended to evade license obligations;
- making either project a required runtime dependency;
- adopting a fork as the canonical Oteryn editor without superseding this ADR.

As observed during the 2026-08-05 design review, Beats Assets Editor uses a noncommercial/share-alike license and Remere's Map Editor presents licensing signals that require clarification before code reuse. Exact repository revisions and licenses must be pinned and independently reviewed before any reuse. Until then, implementation is clean-room/project-owned and uses only behavior, public format evidence and legally permitted fixtures.

Proprietary Tibia/CipSoft assets must not be committed without confirmed rights and provenance.

### 11. Proposed repository ownership

The target ownership shape, introduced only by accepted implementation tasks, is:

```text
apps/
  oteryn-studio/

crates/
  world-schema/
  world-project/
  world-compiler/
  world-bundle/
  world-validation/
  world-migrations/
  world-spatial/
  content-registry/
  asset-pipeline/
  editor-commands/
  editor-history/
  map-renderer/
  legacy-intermediate/
  otbm-import/
  otb-import/
  legacy-appearances-import/
  legacy-conversion-report/
```

Names are candidates until the Workspace and Dependency Contract is accepted. Dependency direction must keep canonical domain/schema crates independent from Tauri, UI widgets, SQL, network transports and renderer implementation.

## Robustness and security requirements

The implementation contracts must define and test:

- maximum dimensions, floors, entities, objects per tile and nesting depth;
- integer overflow and coordinate bounds;
- bounded strings, metadata and decompression ratios;
- duplicate stable keys and reference cycles;
- path traversal and archive extraction rules;
- atomic save and crash recovery;
- corruption detection and partial-recovery policy;
- deterministic compilation and reproducible fixtures;
- schema migration forward/rollback rules;
- signature and checksum verification;
- untrusted plugin/script/content capability boundaries;
- diagnostics that identify exact source path, object and conversion rule.

## Consequences

### Positive

- Oteryn is independent from legacy Tibia format constraints.
- The server and client can stream and cache bounded chunks.
- Large logical areas remain natural while raids and events use safe local placement units.
- Maps, items, sprites and related content share one registry and authoring workflow.
- Legacy migration remains possible without contaminating the native runtime model.
- Deterministic projects and bundles support CI, review, patching and reproducible releases.
- Stable namespaced keys remove reliance on fragile historical numeric IDs.

### Costs

- Oteryn must implement and maintain its own schema, compiler, editor and migration tooling.
- Conversion requires a fixture corpus and careful semantic mapping.
- A GPU-capable editor viewport and large-world UX require substantial engineering.
- Native-only features will not always export to OTBM.
- Legal/provenance review remains mandatory for external code, fixtures and assets.

## Rejected alternatives

1. **Use OTBM as the canonical format** — rejected because it freezes legacy constraints into a greenfield runtime.
2. **Create an extended hybrid OTBM** — rejected because it creates ambiguous compatibility, versioning and ownership boundaries.
3. **Fork Remere's Map Editor as the permanent editor** — rejected because of legacy architecture, coupling and unresolved reuse/licensing concerns.
4. **Use Beats Assets Editor as the whole world editor** — rejected because it is primarily an asset/content reference and carries incompatible reuse constraints for an unrestricted project-owned base.
5. **Use subarea as the raid execution boundary** — rejected because player-facing geography may be too large, inaccessible or unsafe for precise encounter placement.
6. **Use technical chunks as player-facing areas** — rejected because streaming and cache boundaries should be changeable without redefining game geography.
7. **Store live server state back into the authored map** — rejected because it mixes source content with authoritative durable gameplay state.

## Follow-up contracts

This ADR accepts the direction but does not freeze every encoding detail. Follow-up work must define:

1. Workspace and dependency contract, including final crate names.
2. World Project schema and migration contract.
3. World Bundle binary/index/signature contract.
4. Spatial geometry, overlap precedence and coordinate contract.
5. Chunk-size and floor-packing benchmark.
6. Content Registry/package/version contract.
7. OTBM/OTB/appearance importer contracts with pinned source revisions and fixtures.
8. Oteryn Studio renderer/UI prototype and performance acceptance.
9. Encounter compiler/runtime contract, including multichannel uniqueness and anti-hopping policy.
10. Scripting contract; this ADR does not select the scripting language.
11. Asset rights, provenance and release-signing policy.

## Acceptance invariant

Future implementations comply with this ADR only when:

> Oteryn authors and runs a project-owned native world/content model; OTBM and historical tools remain bounded migration/reference inputs; semantic geography is independent of technical chunks; and dynamic encounters execute within explicitly validated encounter zones, cells and anchors rather than across an oversized subarea.
