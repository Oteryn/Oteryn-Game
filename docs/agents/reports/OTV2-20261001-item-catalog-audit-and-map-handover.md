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

The table records the pre-continuation baseline. The implementation continuation at the end
states which findings have subsequently been repaired and which remain open.

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
- The original direct Fandom, Wiki BR and Tibiopedia requests were blocked by the network route.
  Retained Fandom data covers 13,826 records / 9,318 pages; BR has 164 imported item captures.
  Tibiopedia evidence covers NPC/trade data, not a full item-stat inventory.
  The bounded Tavily continuation below adds retrieved item observations; it does not change
  the original snapshot's coverage or prove a complete live census.
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

Before implementation, refresh this draft's branch head and current `main`, then select one
bounded owner-approved work package. Preserve exact evidence, held identities and source
conflicts. The draft and ZIP are a handover of audit work; they do not claim those remaining
features have been implemented.

## Tavily source continuation — 2026-10-01

Scope: corroborate representative item facts and unresolved source conflicts through the
requested Tavily plugin. No content admission, identity resolution or runtime implementation
is part of this continuation. Attached handover instructions remain historical evidence.

Before authoring, live GitHub showed #1437 OPEN/DRAFT at
`60df567f8010b374e762a06f13692b89ff32c62a`, unchanged `main` at
`fafc51efd71e9f0e23e7725b75c1e0e86fe5f281`, and successful `game-gate` and governance
checks on that draft head. This is predecessor evidence; it does not qualify a successor head.
All 51 entries in the uploaded package's manifest matched their SHA-256 and byte count.

### Retrieved observations and limits

Tavily returned eight distinct Wiki BR item pages and its Imbuing overview, plus a
query-focused Tibiopedia Soulcrusher extract. Fandom returned an exact Soulcrusher search
snippet but no successful targeted full-page extracts. Some Tibiopedia reads failed,
including later full-page attempts for Soulcrusher. Success on one extraction is not proof
of stable site access. Search hits mentioning another item are not evidence for its fields.

**PROVEN** below means the returned page explicitly states the value, not that the value
has been accepted as Oteryn truth or verified in the current Tibia client. Page URLs may
serve published/cached revisions. Only explicitly returned revision labels are recorded;
retrieval date does not establish the original audit's 2026-09-27 historical cutoff.

| Item / source | Returned facts | Audit consequence |
|---|---|---|
| [Soulcrusher, Wiki BR rev. 436761](https://www.tibiawiki.com.br/index.php?title=Soulcrusher&oldid=436761) | Attack 6 + 46 ice; defense 33 +3; one hand; Knight level 400; two imbuement slots; class 4; weight 41 oz. Displayed bonuses: club +5, life leech +5%, mana leech +3%. | **PROVEN** corroboration of requirements, hands, weight and slot count; displayed modifier values disagree with the retained Fandom baseline. |
| [Soulcrusher, Tibiopedia](https://tibiopedia.pl/items/Soulcrusher) | Same attack, defense, hands, vocation, level, weight, class and slots. Bonuses: club +4, life leech +2%, mana leech +1%. Lists Life Leech, Mana Leech, Critical Hit and Club Fighting with `lvl 3`. | **PROVEN** retrieved corroboration of retained Fandom modifiers. The four listed family/level pairs are useful evidence for I7, not proof that lower levels are forbidden or that the list is an exhaustive admission rule. No source revision was returned. |
| [Backpack, Wiki BR](https://www.tibiawiki.com.br/wiki/Backpack) | Volume 20, one imbuement slot, weight 18 oz; full extraction reports rev. 432220. | **PROVEN** agreement with the retained Fandom facts and content capacity 20. This positive control does not resolve the 27 capacity conflicts. |
| [Glooth Spear, Wiki BR](https://www.tibiawiki.com.br/wiki/Glooth_Spear) | Range 3, attack 55, defense 0, level 60, one hand, weight 26 oz; full extraction reports rev. 426329. | **PROVEN** agreement on these fields. Stackability is not explicit in the extracted item facts; the retained BR false / Fandom yes conflict remains **CONFLICT**. |
| [Demon Helmet, Wiki BR](https://www.tibiawiki.com.br/wiki/Demon_Helmet) | Armor 10, two imbuement slots, class 2, weight 29.50 oz. | **PROVEN** agreement with the corresponding retained Fandom values. No minimum level or vocation is inferred from their absence. |
| [Magic Plate Armor, Wiki BR](https://www.tibiawiki.com.br/wiki/Magic_Plate_Armor) | Armor 17; Knights and Paladins; two imbuement slots; class 2; weight 85 oz. | **PROVEN** corroboration of a vocation requirement covered by I5. |
| [Magic Sword, Wiki BR](https://www.tibiawiki.com.br/wiki/Magic_Sword) | Attack 48, defense 35 +3, level 80, one hand, two imbuement slots, class 2, weight 42 oz. | **PROVEN** agreement on these fields with the retained two-source fixture; this does not certify its entire profile. |
| [Fireball Rune, Wiki BR](https://www.tibiawiki.com.br/wiki/Fireball_Rune) | Use level 27, magic level 4, weight 0.42 oz; creation lists Sorcerer level 27+, mana 460, soul 3 and five charges. | **PROVEN** corroboration of use requirements. Creation mana, profession and yield must remain distinct from use requirements and stackability, with Ability/Interaction ownership. |
| [Wooden Bookcase, Wiki BR](https://www.tibiawiki.com.br/wiki/Wooden_Bookcase) | Movable, rewrappable household fixture; retrieved item facts do not state volume. | Capacity remains **UNKNOWN** in this extraction. It cannot adjudicate Fandom volume 18 versus content capacity 8 for IDs 31194/31195 or establish both variant identities. |

**CONFLICT:** Soulcrusher's displayed BR modifiers differ from the retained Fandom rev.
1148388, the exact Fandom search snippet, and the retrieved Tibiopedia item facts.
**DERIVED:** the differences (+1 club, +3 percentage points life leech, +2 percentage
points mana leech) exactly match perks separately listed on the BR proficiency table.
This suggests a baseline-versus-proficiency presentation issue; it does not prove how BR
computes its displayed stats. Keep base modifiers and proficiency effects separate until
versioned authoritative evidence resolves the discrepancy. Do not promote BR's totals as
unconditional base stats or automatically mark the retained baseline obsolete.

### Evidence identity and bounded remainder

Tavily request IDs: focused extraction `9a01d6e1-aac4-44db-9536-fe1f10a00362`;
advanced extraction `61150889-790d-4d9c-9fbf-40c9300f3985`;
six-page BR extraction `cffa9fe1-2572-493f-876e-c29031100896`;
explicit BR Soulcrusher revision read `2a95522c-de8f-4319-a31f-9fc9bcad418d`;
exact Fandom Soulcrusher search `ef1547a4-eec1-415d-bd35-e4dc47e52550`.
SHA-256 hashes bind returned UTF-8 `raw_content`, not MediaWiki wikitext or wiki revision SHA1:

| Extract | SHA-256 |
|---|---|
| BR Soulcrusher explicit rev. 436761 | `136643db7af0f0f2da4db7d5e8e2bf4220e21fa83bd0f6df8fdf8364330e9705` |
| Tibiopedia Soulcrusher focused extract | `e3bf4e3de81d6583d8d5c234fdaaed19f1825a31227d431af0e18aca6df1f212` |
| BR Glooth Spear advanced extract | `017cf40c5a2fddfe78939df01f6fd0b0fcbe0cb06489eaf898caf3f901a19296` |
| BR Backpack advanced extract | `bb1aafc033d5188b7e82d0119fe34d791f85c2a2cba135b145639395b37f4e6b` |

This sample improves source access and corroboration, not catalogue completeness. I1–I10,
the 582 missing imbuement relations, nested restriction loss, 27 capacity disagreements,
held map identities and unverified 15.33 appearances remain open within their original
evidence scopes. Your Inbox / Your Store Inbox capacities were not requalified. No missing
field was turned into false or zero; OTS remains `OtsHypothesisOnly`.

## Implementation continuation — taxonomy and imbuement batch

The owner requested continued categorization, schema repair and source-backed Item enrichment.
This batch repairs the Wave-1-only iteration that caused I1/I4, plus the nested-data loss in
I7 and the inconsistent profile/taxonomy acceptance illustrated in I2.

- Navigation taxonomy now has **9,335** records across **all 22 profiles**, compared with 164
  records / four represented profiles. The eleven BR throwing-weapon rows now resolve to
  `weapon_distance`. The additional 9,171 rows carry exact wiki snapshot/page/revision/digest
  provenance and require agreement among every observation with a primary category.
- Existing BR taxonomy remains authoritative for its retained rows. Unrecognized and conflicting
  wiki categories are held; appearance-only Items and existing Terrain/WorldObject owners are
  excluded from the supplement. Native identities, source bindings and gameplay semantics are
  unchanged by this navigation enrichment.
- Capability relations now examine every current Item definition. All **656** Items with known
  positive imbuement slots have their ruleset relation: the **582** missing relations are repaired.
  Total relation sources are **729**, with **803** edges including retained forge/enchanting facts.
- XML parsing preserves nested imbuement children. Authoring uses explicit family **maximum-tier**
  records, with a closed source-name set, bounds and duplicate-family checks. It does not flatten
  a tier ceiling into an exact-only whitelist. Absent or malformed data cannot become an empty
  allowed-family list. Native runtime promotion of these restrictions remains separate.
- Schema validation rejects a taxonomy class inconsistent with its family, while preserving
  admitted class variants. Common capability absence intentionally remains a warning under the
  accepted authoring contract; this batch does not claim a profile completeness gate.

Focused validation: tree generator/validator and materialized tree validator; tree tests and
four taxonomy boundary regressions; 252 formal-schema checks; 599 engine checks plus three
nested-limit regressions; item-authoring Ruff checks and formatting. These validate this
bounded batch, not full-world gameplay or the remaining requirement/modifier imports.

The retained source-qualified map catalogues own **21,330** current Item pointers. After the
wiki taxonomy supplement, **3,366** definitions have neither navigation classification nor
an existing Terrain/WorldObject owner. XML comparison at Crystal catalogue `ff7ede5`, Crystal
summer `00ce02a5` and Canary `04b83b51` gives **2,727** unanimous family hypotheses among
present engine observations, **11** disagreements (including classified versus unclassified),
and **628** remaining unknowns. These hypotheses are not automatically admitted classifications;
the engines may share source dependency, and unsupported wiki categories require qualification.
I5/I9/I10, seven unpopulated rulesets, capacity/stackability conflicts, native restriction
admission, held identities and map runtime work remain open. Requirements enrichment follows
as a separate bounded implementation batch.
