# Full Tibia content storage structure and hierarchy audit — G0

- Repository: `Oteryn/Oteryn-Game`
- Protected baseline: `main@c07240d50473b8697cbe10641028cd0e7eb2d1e4`
- Gate: `FULL_CONTENT_STORAGE_STRUCTURE_AND_HIERARCHY_AUDIT`
- Machine-readable companion: `docs/agents/evidence/OTV2-20260923-full-content-storage-structure-audit.json`
- Result: **G0 architecture is sufficient for G1; no schema revision is justified.**
- Bulk source import performed: **NO**
- Hard exclusions retained: **Kalkulatory / Narzędzie do nasycania / Dostawca**

## Evidence basis

PROVEN on the protected baseline:

- PR #803 is merged as `c07240d50473b8697cbe10641028cd0e7eb2d1e4`; its retained Item census is 6,918 discovered/fetched pages, 5,475 `INFOBOX_ITEM`, 7 parse errors, 1,436 no-base-infobox pages and 83 distinct infobox fields.
- PR #805 remains a path-disjoint lifecycle-only closeout and is not taken over by this task.
- `apps/game-server/src/content/project/v2.rs` blob `e1b1488e3b27911add2b64891255f6942fe4ecfb` contains the current protected family vocabulary and stable placement model.
- `docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md` blob `b891e50a89aeed8bf80a39c251291f85cf5081eb` is the accepted v2 source-profile decision.
- The retained wiki-wide schema coverage manifest reports 19 domain groups, 49 structured concept records, `unclassified=0` and no remaining material schema concept gap after the protected v2 candidate.
- Real donor measurement proves a source stream of 18,997,668 tiles, but the retained real-batch measurement exercises only 2,048 cells / 2,463 source occurrences. Therefore it is not evidence that the canonical single `worlds/world.json` document is safe at full-world placement scale.

## A. What we already have

### Definitions

The current protected family vocabulary is adequate. Existing executable/reference families remain:

`Terrain`, `Presentation`, `LocalObject`, `Item`, `Creature`, `Ability`, `Effect`, `Formula`, `Loot`, `Behavior`.

WorldProject/v2 adds or retains declarative/source identities for:

`WorldObject`, `Area`, `Document`, `Achievement`, `Outfit`, `Mount`, `Charm`, `NPC`, `Dialogue`, `Service`, `Interaction`, `Quest`, `Transition`, `House`, `Encounter`.

No additional top-level family is required for Bestiary, Bosstiary, runes, libraries, shop, world changes, daily quests or Tibiadrome. Those surfaces map to existing definitions plus typed relationships and/or external runtime/durable owners.

### Relationships

Typed family-specific references already model the required graph:

- Item -> Ability / Interaction / Document;
- NPC -> Dialogue / Service;
- Creature -> Loot / Ability;
- Quest -> prerequisites / rewards / Encounter / Interaction;
- Area -> parent Area;
- House -> Area;
- Presentation -> Asset;
- WorldObject -> Area / Interaction / Transition / Document;
- Encounter -> Area / Interaction and related Creature/Quest evidence.

A universal generic relationship store is **not required for G1**. Relationships stay with the semantic owner instead of becoming an untyped catch-all graph.

### Placements / occurrences

`ProjectV2Placement` already separates placement identity from definition identity and retains:

- stable placement key;
- exact map revision;
- world;
- typed definition revision;
- optional Area;
- optional Document;
- optional parent placement;
- coordinate frame and `x/y/floor`;
- explicit presentation `plane/order`.

This covers ground, walls, borders, roofs, vegetation, rocks, water/lava, doors, stairs, bridges, furniture, decoration, transitions and other exact map occurrences without minting a reusable definition per coordinate.

### Mutable state boundary

Current quest progress, house owner/ACL/rent state, world-event phase, active imbues, stack quantity, container contents, charge/timer state, creature state and NPC dialogue session remain outside static WorldProject source. This boundary is correct and must not be weakened during source census.

## B. What is missing

### Concrete mandatory gap

**Tooling namespace for new G1+ work.**

`tools/reference-world-corridor-census/` is now a mixed flat set containing Item, Creature, NPC, Loot, Ability, Interaction and other census utilities, while its README still describes only the original thin Phase-A corridor consumer. Continuing the full programme there would create the exact unstructured script pile the G0 contract is meant to prevent.

The minimal repair is:

- keep all existing protected tools at their current paths;
- create `tools/content-census/` as the destination for **new** full-content tooling;
- organize new tooling by family only when that family receives its first real tool;
- create shared `common/` only when a second consumer proves shared logic;
- do not perform cosmetic mass moves.

### Concrete scale unknown, not yet a schema gap

The published v2 physical layout uses one canonical `worlds/world.json` document. Existing evidence does **not** measure canonical v2 serialization/read/write/peak-memory/diff behavior at millions of placements.

Disposition: **DEFER_UNTIL_MEASURED**.

This does not block G1 definition/source discovery. It **does** become a mandatory measurement gate before bulk full-world placement reconciliation or any decision to version/chunk the v2 physical world-record layout.

### Not missing

There is no evidence-backed need for:

- `ItemV2`;
- `TerrainV2`;
- `CreatureV2`;
- `WorldObjectV2`;
- `ProjectV3`;
- a duplicate Rune family;
- a duplicate Bestiary/Bosstiary family;
- a second NPC/Quest/House runtime model;
- a universal fuzzy matcher.

## C. Directory / structure changes

### Mandatory

1. Add `tools/content-census/` as the new G1+ tooling namespace.
2. Retain this human-readable audit plus the machine-readable matrix under `docs/agents/evidence/`.
3. Before bulk world-placement reconciliation, add a canonical WorldProject/v2 full-world scale measurement covering bytes, writer time, reader time, peak memory, file count and one-placement diff/update cost.

### Recommended

- family-oriented stages for new tooling: `discovery -> classification -> crosswalk -> verification -> evidence/tests`;
- keep compact provenance/digest manifests in Git and large reproducible source outputs as bounded CI/scratch artifacts;
- use family-specific crosswalk signals and states; do not share one universal fuzzy scorer across unrelated families.

### Cosmetic / not worth doing

- mass-moving current `tools/reference-world-corridor-census/*` scripts;
- reorganizing the eight published WorldProject/v2 role locators merely for aesthetic directory symmetry;
- splitting definitions into many physical files before measured scale/review pressure exists.

## D. Scale findings

| Area | Verified evidence | G0 disposition |
|---|---|---|
| Item/source definitions | 38,157 prior protected source identities; 6,918 wiki-first Item pages | Current definition/source model is sufficient for G1 |
| Editable-format spike | largest synthetic fixture: 16,384 cells; chunked JSON tree 2,272,665 bytes | Useful format evidence, not canonical v2 full-world proof |
| Real donor map | 18,997,668 source tiles in retained stream | Proves full-world magnitude exists |
| Real measured batch | 2,048 cells; 2,463 source occurrences; 183 source definitions | Too small to prove `worlds/world.json` full-world safety |
| Canonical v2 full placements | not measured | **DEFER_UNTIL_MEASURED** |

Therefore:

**Keep the v2 schema and published physical contract for G1. Do not claim the current single world-record document is full-world scalable until measured.**

## E. TibiaWiki navigation coverage

Every required top-level surface has a non-`UNRESOLVED` disposition:

| Surface | Disposition | Oteryn owner(s) |
|---|---|---|
| Rzeczy | DUPLICATE_OVERLAP | Item, Document, WorldObject, LocalObject, Presentation |
| Bestiariusz | MAPPED_TO_OTERYN_FAMILY | Creature |
| Uroki | MAPPED_TO_OTERYN_FAMILY | Charm |
| Mapa | RELATIONSHIP_SYSTEM | Terrain, WorldObject, Area, Transition, WorldPlacement, Presentation |
| Domy | RELATIONSHIP_SYSTEM | House, Area, WorldPlacement |
| Ratusze | DUPLICATE_OVERLAP | Area, WorldObject, NPC, Service |
| Bossiary | RELATIONSHIP_SYSTEM | Creature, Encounter |
| Sklep | DUPLICATE_OVERLAP | Item, Outfit, Mount, Presentation; commercial facts stay external |
| Magiczne Archiwum | DUPLICATE_OVERLAP | Ability, Effect, Formula, Item |
| Osiągnięcia | MAPPED_TO_OTERYN_FAMILY | Achievement |
| Biblioteki | RELATIONSHIP_SYSTEM | Document, Area, WorldPlacement, WorldObject, Item |
| Miasta | MAPPED_TO_OTERYN_FAMILY | Area |
| Zabudowania | RELATIONSHIP_SYSTEM | Area, WorldObject, House, WorldPlacement |
| Stworzenia | MAPPED_TO_OTERYN_FAMILY | Creature |
| Tereny łowieckie | RELATIONSHIP_SYSTEM | Area, Creature, Encounter, WorldPlacement |
| Zaklęcia | RELATIONSHIP_SYSTEM | Ability, Effect, Formula |
| Zmiany w Mini Świecie | MAPPED_TO_OTERYN_FAMILY | Encounter |
| Mocowania | MAPPED_TO_OTERYN_FAMILY | Mount |
| NPC-e | RELATIONSHIP_SYSTEM | NPC, Dialogue, Service, Interaction, WorldPlacement |
| Obiekty | DUPLICATE_OVERLAP | Terrain, WorldObject, LocalObject, Item, Transition, Presentation, Asset |
| Stroje | MAPPED_TO_OTERYN_FAMILY | Outfit |
| Zadania | RELATIONSHIP_SYSTEM | Quest, Interaction, NPC, Area, Encounter |
| Runy | RELATIONSHIP_SYSTEM | Item, Ability, Interaction |
| Serwery | SOURCE_NAVIGATION_ONLY | only reusable game-content evidence, never server-list/tool metadata |
| Codzienne zadania | RELATIONSHIP_SYSTEM | Quest, Interaction, Encounter |
| Tibiadrom | RELATIONSHIP_SYSTEM | Encounter, Creature, Area, Item, Achievement |
| Aktualizacje | PROVENANCE_ONLY | provenance/change evidence only |
| Zmiany świata | MAPPED_TO_OTERYN_FAMILY | Encounter |
| Zadania światowe | RELATIONSHIP_SYSTEM | Quest, Encounter, Interaction |

Final `UNRESOLVED = 0`.

The three hard exclusions are intentionally absent from this matrix.

## F. Canonical storage matrix

| Kind | Canonical storage |
|---|---|
| Executable/reference definitions | `definitions/reference.json` |
| Declarative definitions and source-only overlays | `definitions/declarations.json` |
| World identity and placement occurrences | `worlds/world.json` |
| Presentation -> Asset relations | `presentations/bindings.json` |
| Assets | `assets/catalog.json` |
| Import/reimport evidence | `provenance/imports.json` |
| Exact source identities/revisions/digests | `provenance/sources.json` |
| Noncanonical editor aliases/tags/notes | `editor/author.json` |
| Retained audit/census evidence | `docs/agents/evidence/` |
| Existing predecessor census tools | keep in `tools/reference-world-corridor-census/` |
| New full-content programme tools | `tools/content-census/<family>/` on demand |

## G. Next slices

1. **G1.0 — CONTENT_CENSUS_TOOLING_BOOTSTRAP**: land the first real discovery/common implementation under `tools/content-census/`; no mass migration.
2. **G1 — FULL_CONTENT_SOURCE_DISCOVERY_AND_FAMILY_CENSUS**: bounded source-universe-first discovery across navigation, categories, subcategories, templates, infoboxes, structured lists and overlaps. Explicit source-shape and family states are mandatory.
3. **G2 — GLOBAL_SOURCE_OVERLAP_AND_DEDUPLICATION**: deduplicate source identities/aliases before aggregate totals.
4. **G3 — CONTENT_FAMILY_CLASSIFICATION_CLOSURE**: close unresolved family classification to zero or a documented blocker.
5. **G4+ — PER-FAMILY IDENTITY CROSSWALKS** using family-specific signals.
6. **Before WORLD_PLACEMENT_RECONCILIATION**: run canonical v2 full-world placement scale measurement; only measured evidence may justify a physical-format/version change.

## G0 acceptance

- all required navigation surfaces inventoried: **PASS**
- hard exclusions absent: **PASS**
- every source surface has an owner/disposition: **PASS**
- every family has canonical storage: **PASS**
- definitions/relationships/placements/runtime separated: **PASS**
- v2 physical scale evaluated: **PASS, with one explicit deferred measurement**
- tooling/evidence/test organization evaluated: **PASS**
- actual representation gaps identified: **PASS**
- unnecessary schema redesign rejected: **PASS**
- minimal target hierarchy produced: **PASS**
- next slices defined: **PASS**
- bulk source corpus imported: **NO**

**G0 candidate verdict: READY TO PROCEED TO G1 after exact-head repository qualification.**
