# Item catalogue audit and map handover — 2026-10-01

This audit records gaps in item classification, semantics and map integration. It includes the
completed map import in PR #1170; importing the base map is no longer remaining work.

## Evidence boundary

- Classification: **PROVEN** means directly observed code/data; **DERIVED** means an inference;
  **UNKNOWN** means insufficient evidence; **CONFLICT** means retained sources disagree.
- Original item audit: Game commit `c3fce15f4e3b3f9f02a2b43e02f71a451473e8e0`.
- Map/runtime reassessment: PR #1170 head `675926489484315b9edc506339f17180890f198a`,
  base `35c645cbbe671bb51280a23adfc5dcd3faf6e9e6`.
- PR #1170 **merged** at 2026-10-01 09:39:15 UTC, commit
  `fafc51efd71e9f0e23e7725b75c1e0e86fe5f281`. Its map capture is identical to the reviewed head.
- The two commits between the original item audit and the reassessment base change NPC
  content/tooling and MAIL/BAGS documents, not the audited item shards, map catalogues or runtime.
- This is retained audit evidence and a proposed work order, not architecture acceptance,
  task allocation, production readiness, or authority to merge another PR.
- Four subagents audited catalogues, schemas, stats and runtime. Three revisited the map scope.
  The primary agent reproduced selected findings and checked exact-revision GitHub data.
- Live lifecycle facts must be refreshed before implementation. A historical task marked
  completed does not prove runtime delivery.

## Inventory and family schemas

**PROVEN:** 34,031 unique Item definitions in 69 shards: 33,971 source-bound and 60
appearance-only. Shared catalogues are `content/items/definitions/`,
`content/items/taxonomy/` and `content/items/relations/`.

There are **22 Item profiles**, deliberately represented by one composed authoring schema:
`tools/content-schema/item-authoring/item.schema.json` and `profile-catalog.json`.

| Profile | Navigation scope |
|---|---|
| equipment_armor | Helmets, armor, legs, boots |
| equipment_offhand | Shields, spellbooks, jewelry, extra slot |
| container_equipment | Equipped containers, including quivers |
| weapon_melee | Melee weapons |
| weapon_distance | Distance weapons and ammunition |
| weapon_magic | Wands and rods |
| rune | Runes |
| document | Documents |
| container | Containers |
| decoration | Decorations |
| event_collectible | Event collectibles |
| progression_material | Progression materials |
| transformation_item | Transformation items |
| quest_item | Quest items |
| material_valuable | Valuable materials |
| trash | Trash |
| key | Keys |
| light_source | Light sources |
| tool | Tools |
| food | Food |
| fluid | Fluids |
| plant | Plants |

Thirteen templates cover twelve profiles. Missing a dedicated template does not mean a family
schema is absent. Profile membership and capability completeness are separate questions.

## Item audit findings

| ID | Classification | Finding and consequence |
|---|---|---|
| I1 | PROVEN | Taxonomy covers 164/34,031 definitions: 153 have recognized profiles and 11 `Armas de Arremesso` rows have a null profile. 33,867 definitions lack taxonomy; only four of the 22 profiles are represented. This is incomplete classification, not proof that the other families have no items. |
| I2 | PROVEN | Family minimum capabilities are warnings, not a completeness gate. All 22 bare profiles validate. A fabricated melee weapon with food taxonomy and inconsistent equipment also passes; current validation cannot certify a complete family import. |
| I3 | PROVEN | All seven `rulesets/items/*` systems are index-only `READY_UNPOPULATED`: charging-system, crystal-shield, doomforging, enchanting, exaltation-forge, imbuements, umbral-creation. |
| I4 | PROVEN | There are 147 relation-source rows and 221 edges, all `CAPABILITY_GOVERNED_BY`. Of 656 items with known positive imbuement slots, only 74 have these relations: 582 are missing them. The tree generator iterates Wave 1 authoring for relations. |
| I5 | PROVEN | Source-supported semantics remain unpromoted: positive level 1,046, vocation 864, hands 758, equipment slot 1,805, attribute boosts 531 and resistances 430; also unambiguous stackability 4,929, duration 190 and capacity 36. These sets overlap and must not be summed as unique items. |
| I6 | PROVEN | The current 2b-1 packet's 10,502 fields on 6,524 items match materialized content. It does not complete requirements, modifiers or the remaining model gaps. Appearance-only items were not silently promoted. |
| I7 | PROVEN | The external parser discards nested imbuement restrictions: 620 Crystal item IDs / 2,725 children and 580 Canary IDs / 2,563 children. Slot counts survive, but family/tier restrictions do not. Preserve and corroborate these before admission. |
| I8 | CONFLICT | Twenty-seven capacity disagreements between Fandom and content match Crystal; 26 also match Canary. Glooth Spear stackability differs between retained BR and Fandom revisions. These are source conflicts, not established bad imports. |
| I9 | UNKNOWN | The retained stats snapshot does not cover 21,136 of the 33,971 source-bound items. Lack of evidence does not establish what stats are required for those items. |
| I10 | PROVEN | Authoring is wider than `ReferenceItemSemantics`. Light and forge classification are runtime model gaps; some corresponding authoring structures already exist. Do not describe them as absent from the entire project. |

Evidence entry points: `tools/content-migration/world_project_v2_to_tree.py`,
`tools/content-schema/item-authoring/{validate_item.py,engine_items.py}`,
`apps/game-server/src/content/{reference_playable.rs,project.rs,item_stats_promotion.rs}`,
and the rooted Item definitions/taxonomy/relations.

Known spell mana/damage/rune requirements and food effects require their owning Ability/Interaction
path; do not assume every source property belongs on an Item record.

## Terrain and WorldObject already delivered

| Domain | Catalogue | Records / shards | Schema |
|---|---|---:|---|
| Terrain | `content/world/terrain/` | 8,548 / 18 | `tools/content-schema/world-object-authoring/terrain.schema.json` |
| WorldObject | `content/world/objects/` | 12,782 / 26 | `tools/content-schema/world-object-authoring/world-object.schema.json` |

**PROVEN:** Both catalogues are populated. Their schemas, generators, validators and separate
`routed-item-pointer.schema.json` exist. They should not be reimplemented.

Terrain kinds: border 3,761; ground 2,197; wall 2,166; roof 234; field 125; unknown 65.
WorldObject kinds: object 5,654; corpse 3,358; decoration 2,784; door 728; bed 192;
teleport 49; ladder 17. The schema supports `container_fixture`, with zero current records.

The importer intentionally keeps absent sparse XML facts UNKNOWN. Walkability/sight are known
for the existing catalogue corpus. Unknown projectile blocking, door permissions, or field facts
need evidence and consumer requirements, not guessed defaults. Ninety of 125 field records
lack a field type; 65 Terrain records lack a known kind.

## What merged PR #1170 closes

**PROVEN:** Base map import, B3 codec/tooling, placements, 60 island records and the World record
are delivered in `main`. The exact capture reports:

- 1,208 regions, 31,119 sectors, 19,373,519 tiles and 24,983,331 map-element occurrences.
- 25,984 palette entries: 20,029 canonical Item keys and 5,955 provisional donor keys.
- 24,223,058 occurrences use canonical Item keys; 760,273 use provisional keys.
- Direct Terrain/WorldObject palette keys are zero by the accepted resolution order.
  Of the canonical map IDs, 15,275 already have Terrain/WorldObject catalogue records.
  Zero direct family keys does not mean the map lacks those families.
- The PR description's 5,995 provisional count is stale; the capture and README say 5,955.
- Import carries 872 teleport attributes matching Transition records. It excludes 1,583
  destination attributes with recorded reasons (1,577 unset, one outside the map, five missing
  destination tiles), preserving the objects themselves.

The WorldBundle compiler and its verified reader already exist in
`tools/world-bundle-compiler/`. The compiler resolves catalogue ownership using inverse
`provenance.item_pointer` when Item `routed_to` is absent. LocalObject state machines and the
`WORLD_OBJECT_OVERLAY` protocol already exist. None of these should be planned from zero.

## Actual remaining map work

| Work package | Concrete remainder |
|---|---|
| Catalogue identity closure | Qualify 5,949 actual client appearances with no Terrain/WorldObject records, plus six exceptions. Basic classes are 740 ground, 1,534 border, 2,614 blocking and 1,061 decoration; appearance class alone does not prove full ownership or behavior. |
| Exceptions | ID 99 is absent from both XML and client appearances. ID 2141 is a reserved sprite / placeholder exclusion. IDs 35500, 53380, 54613 and 54614 are alias-gate holds. Do not mint them or invent behavior simply to make the unresolved count zero. |
| Source-pin reconciliation | The map uses Crystal `00ce02a5`, catalogues `ff7ede5`. There are 35 canonical map Items without WO/Terrain records that are route candidates under the newer source: 28 Terrain and seven WorldObject. These require qualification separately from the 5,955 provisional references. This does not establish that the entire catalogue is obsolete. |
| WO-2b | Add catalogue-backed typed payloads/lowering, WorldObject family admission and formal Item/Rust `routed_to`. `DefinitionFamily::Terrain` already exists. All 34,031 Item definitions currently omit `routed_to`; existing compiler inverse routing is functional. |
| Typed relations | Replace 505 WorldObject `source_item_id` placeholders with qualified typed targets: 86 rotation, 192 bed parts, 145 male transforms, 82 female transforms. Fluid-source names also need canonical references where a consumer requires them. |
| WO-3 | Add an optional revision-bound presentation reference to LocalObject states. Dynamic states, collision, variants and absent-state handling already exist. |
| MAP-LOAD-1 | Integrate the existing bundle reader into server boot; build a compact indexed base shared by channels, verify revision/compatibility and measure budgets. Server boot still activates `native_entry_room`. |
| MAP-OVERLAY-1 | Integrate base map hide/add with origin-bound pickup MINT, Ground recovery/re-hide and planned-world-reset retirement. Existing LocalObject overlay alone does not implement this item-map path. |
| MAP-WIRE-1 | Implement viewport snapshots of tiles, base map items and Ground, content generation and atomic pickup/hide visibility. Existing WORLD_OBJECT_OVERLAY does not carry this full state. |
| MAP-CUTOVER-1 | Boot the full world from the bundle and qualify the first reset and release gates, retaining the entry room as a fixture. |
| Map completion | Seven minimap-derived draft areas and 28 unresolved entrances remain; the Edron cave is intentionally sealed. Drafts and provisional keys cannot enter a production bundle under the current gate. |
| Gameplay semantics | Bind admitted doors, beds, fields, teleport and tool behavior through Interaction/Ability. Static catalogue fields alone do not implement those mechanics. |

Evidence entry points: `docs/architecture/ADR-0021-world-map-runtime-loading.md`,
`tools/world-bundle-compiler/src/{resolve.rs,compile.rs,bundle.rs}`,
`apps/game-server/src/{node/serve.rs,world_runtime.rs,content/reference_playable.rs,content/project.rs}`,
and `tools/content-schema/world-object-authoring/README.md`.

## Source qualification and client assets

- Crystal summer-update: `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
- Catalogue source Crystal: `ff7ede593c69d4c658b382c97443e8155926924a`.
- Canary main examined: `04b83b512114bfd888000d6e1433ed8ecaec7c5b`.
- OTS data is **OtsHypothesisOnly**, not an automatic authority over contradictory wiki evidence.
- Live Fandom, Wiki BR and Tibiopedia requests were blocked by the network route.
  Retained Fandom data covers 13,826 records / 9,318 pages; BR has 164 imported item captures.
  Tibiopedia evidence covers NPC/trade data, not a full item-stat inventory.
- The supplied 15.33 asset manifest differs from admitted 15.30. The repo's binary
  appearances/catalog/proficiencies were verified against 15.30.
  The 15.33 appearances binary is unverified; the uploaded ZIP exceeds the file-download limit.
  Manifest entry counts are not item counts.
- No identity baseline, asset pin, source conflict or content semantics was changed by this audit.

## Validation and practical next step

Focused audit validation passed, including the WorldObject suite (21,583 checks) and selected
schema/parser reproductions. The map PR head had seven observed successful workflow runs,
including World Metadata and Merge gate; integration is now confirmed by PR metadata and the
protected-main commit. This audit did not run a new full-map compile or server/client E2E.

This draft changes retained documentation only. Its own exact-head CI is separate from
PR #1170's evidence; passing import CI is not full-world gameplay qualification.

Proposed order (not allocation): catalogue/identity qualification and WO-2b/WO-3; server
load; map overlay and wire integration; full-world cutover; production data closure.
Independent item work remains: taxonomy, imbuement restrictions/relations, equipment requirements,
modifiers and the admitted model gaps.

The next session should open this draft and the ZIP's `README_CONTINUATION.md`, refresh its
branch head and current `main`, then select one bounded owner-approved work package before
implementation. Preserve exact evidence, held identities and source conflicts. The draft and ZIP
are a handover of audit work; they do not claim those remaining features have been implemented.
