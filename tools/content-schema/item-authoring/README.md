# Item authoring schema candidate v4

The 2026-10-01 continuation validates profile/taxonomy class consistency while retaining
the admitted ammunition, consumable and fluid-container class variants. Common capability
absence remains a warning under the authoring contract; a warning-free profile is not proof
of complete source coverage. The engine converter retains nested imbuement family ceilings
as `allowed_family_max_tiers` (`family`, `max_tier`), distinct from exact allowed tier pairs.
Missing children remain unknown; malformed or duplicate family ceilings block conversion.
These are source-backed authoring observations, not native runtime imbuement admission.
`test_engine_items.py` also runs the nested-limit regression tests used by CI.

This package turns the Item Master Schema v1 census into an executable authoring
contract. It validates one portable Item definition, not a placed map object and not a
mutable item instance.

Architecture and boundaries:
[`docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`](../../../docs/architecture/OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md).

| File | Purpose |
|---|---|
| `build_formal_schema.py` | Single source for the schemas, profile catalog and templates. |
| `item.schema.json` | One capability-composed portable Item schema for every family. |
| `item-dependencies.schema.json` | Exact definitions, presentations, assets and admitted proficiency crosswalks required by the Item. |
| `item-import-readiness.schema.json` | Per-source-field mapping/disposition ledger. |
| `real-source-evidence.schema.json` | Closed evidence record for the six real examples, including exact source pins and typed field links. |
| `profile-catalog.json` | The 22 census profiles, their Wiki families and expected capabilities. |
| `wiki-field-dispositions.json` | Exact 71-field Wiki disposition registry generated from the protected census. |
| `fandom-field-dispositions.json` | Exact 84-row historical Fandom migration-table registry; corroboration only. |
| `wiki-real-item-field-supplement.json` | Exact 14-field delta observed on six pinned current TibiaWiki BR item pages. |
| `fandom-real-item-field-supplement.json` | Exact 5-field delta observed on the same six pinned Fandom item pages. |
| `canary-field-dispositions.json` | Exhaustive pinned Canary ledger: 143 parser keys plus root, nested, appearance and reverse-relation inputs. |
| `crystal-field-dispositions.json` | Exhaustive pinned Crystal ledger: 143 parser keys plus root, nested, appearance and reverse-relation inputs. |
| `source_field_catalogs.py` | Generated-ledger source of truth, aliases, defects and owner routing. |
| `proficiency_profiles.py` | Pinned 15.30 client, Canary and Crystal proficiency sources; the admitted client crosswalks are the committed `content/proficiencies/` definitions. |
| `real_item_examples.py` / `real-source-examples.json` | Six generated, validated real-item examples with source evidence and explicit blockers. |
| `templates/*.json` | Twenty-three valid starting points covering all 22 family profiles; distance weapons and ammunition share a profile. Values are synthetic examples, not source facts or runtime admission. |
| `validate_item.py` | Structural, semantic, exact-reference and import-readiness validation. |
| `verify_formal_schema.py` | Focused positive/negative contract checks and deterministic fixtures. |
| `engine_items.py` | Converts one pinned Crystal/Canary `items.xml` + `appearances.dat` into candidate Item bundles: identity allocator, family_profile/taxonomy rules, field mapping, appearance/Presentation binding, and the `delivery_task_eligible` decision from the `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` authoring rule. Digest-verifies every input artifact first (text artifacts after CRLF->LF normalization, `appearances.dat` as exact raw bytes); a missing pinned artifact is a hard error. |
| `delivery-task-overrides.json` | Per-Item exceptions to the Delivery Task adoption rule (`{key: {eligible, reason}}`), strictly validated; starts empty. |
| `owner-item-family-decisions.json` | Owner-reviewed `family_profile` decisions (task #15, 2026-09-28) for ids every automated classifier left unresolved; strictly validated, applied as the last classifier. |
| `population_census.py` | Runs `engine_items` over an engine's full item universe, validates every emitted bundle (the real `delivery_task_eligible` decision, not a proposal), and writes one deterministic outcome census (counters, top blockers/validator errors, per-raw-field coverage, Delivery Task decision/observation/crystal-list counts, and a `routed_non_item` owner/reason breakdown). `--self-check` runs required engine-specific assertions for both engines; `--check` diffs an in-memory regeneration against the committed file instead of writing. |
| `test_engine_items.py` | Fixture-checkout tests for `engine_items`/`population_census`: LF/CRLF digest portability, per-engine Delivery Task pool parsing, the Crystal-list adoption rule and its per-item overrides, plus direct synthetic-`sources` tests (real identity index and disposition catalogs, fabricated `items`/`appearances`) for every implemented field family and value route. Run with `python test_engine_items.py`. |
| `samples/population-crystal-ff7ede5.json`, `samples/population-canary-47dfd51f.json` | Committed census outputs for the two pinned engine revisions. |
| `lower_promotion_packet.py` | Lowers every zero-validator-error Crystal Item bundle's authored values for the 9 field paths (`presentation.name`, `weapon.attack`/`defense`/`extra_defense`/`range_cells`/`hit_chance`, `protection.armor`, `charges.count`, `container.capacity`) that `apps/game-server/src/content/cw2_b1_import.rs`'s `decode_item_semantic_promotion_value` accepts into a candidate `OTERYN_ITEM_SEMANTIC_PROMOTION_LOWERING/v1` packet, in that decoder's exact typed representation. `--self-check` checks named Crystal ids (Magic Sword `3288`, a container, a charges item); `--check` diffs an in-memory regeneration against the committed file instead of writing. See "Item semantic-promotion lowering" below. |
| `test_lower_promotion_packet.py` | No-network fixture tests for `lower_promotion_packet.py`'s encode/decode mirror of the Rust decoder and its `build_packet`/`validate_packet` wiring, against the same kind of synthetic `sources` `test_engine_items.py` uses. Run with `python test_lower_promotion_packet.py`. |
| `samples/promotion-crystal-ff7ede5.json` | Committed candidate lowering packet for the pinned Crystal revision. |
| `lower_wiki_stats_packet.py` / `test_lower_wiki_stats_packet.py` | ITEM-SEM-2b-1: lowers the pinned TibiaWiki stat snapshot (`imports/tibiawiki/facts/items-stats.json`) into the `OTERYN_ITEM_STATS_PROMOTION/v2` packet `docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json` that `apps/game-server/src/content/item_stats_promotion.rs` applies: weapon attack/defense/extra defense/range/type/elements, armor, imbuement slots and weight (hundredths of an ounce) for content Items. A value is promoted only when every page listing the id agrees; conflicts and malformed values are reported. ITEM-SEM-2b-2: one compact equip pattern per wiki slot. The slot fixes the hands. A two-handed weapon reserves the shield slot. A two-handed distance weapon, a shield and a spellbook reserve the `oteryn:equipment-group.non_quiver_left_hand` group of `domain/equipment.rs`. Level (not `0`) and base vocations (`None` = unrestricted) are written outside the Extra slot. ITEM-SEM-2b-3: `without` is the vocation `NONE` (A13 key `none`); ammunition without a slot takes the Extra slot; use requirements (runes, ammunition, the Extra slot and slotless Items: level and `mlrequired` not `0`, vocations) go to `use_requirements` with `enforcement_mode: ON_USE`; i3450 is held under D310 (its definition is sealed by the reward stack proof). The same run writes the record `docs/agents/evidence/OTV2-20261003-item-equipment-requirements-v1.json` with the written counts and every held row. `--check` diffs a regeneration of both files; the test runs fixtures and that drift check. |

The wiki packet also supplies native `equipment.patterns` when an explicit wiki slot
and all present requirements agree. Known two-hand occupancy reserves the shield slot;
missing requirements remain `UNKNOWN`. Rune use requirements and ammunition do not
become equip requirements. Unsupported vocations and contradictory hand claims hold
the pattern. The main backpack retains its separate starter admission. This records
qualified content facts without changing materialization or transfer permissions.
| `lower_timed_item_packet.py` / `test_lower_timed_item_packet.py` | TIMED-CONTENT-1: lowers timed-item facts for rings, amulets and necklaces, boots, torches and lamps into the `OTERYN_ITEM_TIMED_PROMOTION/v1` packet `docs/agents/evidence/OTV2-20261003-timed-item-facts-v1.json` that `apps/game-server/src/content/item_timed_promotion.rs` applies: `charges.count`, `temporal.duration_ms`/`consumption_mode`/`stop_duration_while_unequipped` and `transform.use|equip|unequip|decay`. TibiaWiki `charges`/`duration` first, Canary `items.xml` (OTS_HYPOTHESIS_ONLY) as fallback and the only source of transform pairs, `stopduration` and `decayTo`; each row names its evidence class. `ON_EQUIP` is an equip-paired active form, `CONTINUOUS` a lit torch or lamp, never `ON_USE`. Limits TIMEDITEM0-RL-01 (65,535 charges) and RL-02 (604,800,000 ms); a stackable or mode-undetermined item, a wiki conflict and `showcharges` (no model field) are reported, never a row. `--check` diffs a regeneration; the test runs fixtures and that drift check. Run `python test_lower_timed_item_packet.py`. |
| `lower_equip_abilities_packet.py` / `test_lower_equip_abilities_packet.py` | EQUIP-CONTENT-1: writes the `OTERYN_EQUIP_ABILITIES/v1` record `docs/agents/evidence/OTV2-20261003-equip-abilities-v1.json` that `apps/game-server/src/content/item_abilities.rs` applies last in the materializer. The six EQUIP-0 §3.1 abilities are a derived view over `skill_modifiers` and `protection`; there is no codec or schema change. TibiaWiki (the committed stats packet) comes first. Where every wiki page of an Item is silent on a group, a Canary `items.xml` fallback row (D384 pin, OTS_HYPOTHESIS_ONLY, top-level attributes only) fills that Unknown leaf. Canary speed is in displayed units, 1:1 with the wiki; a disagreement fails the run. A Canary group with a key outside the mapped abilities (mantra, elemental bond, gain/ticks, mana shield, invisibility…) is held. The record lists every source per Item and the `timed` flag (`charges.count` or `temporal.duration`) that Rust re-derives. STAT_BOOST and LIGHT have no source in either input. A separate tool keeps the stats packet bytes unchanged. `--check` diffs a regeneration. Run `python test_lower_equip_abilities_packet.py`, which also works under `python -m unittest <path>`. |
| `item_weapon_proficiency.py` / `test_item_weapon_proficiency.py` | ITEM-PROF-1: digest-checks the committed 15.30 client proficiency staging (`imports/cipsoft-staticdata/proficiencies`, `weapon-proficiency-bindings`) against the pinned client files and writes `samples/item-weapon-proficiency-15-30-7fea90ec.json`: 443 profiles (raw perks, mastery level), 666 bindings keyed `oteryn:item.tibia.i<id>` and the owner-accepted D199 perk enum mapping (`perk_mapping`, raw values kept). The threshold class is per binding, not per profile: bolt ammunition (Crystal ff7ede5 `ammotype`, from the committed cw2-b1 identity catalog) is crossbow (D198, also in shared bow/crossbow profiles); a ranged binding without ammotype uses TibiaWiki `secondarytype` (Crossbows = crossbow, Bows = standard); Knight only for sword/axe/club weapons whose vocation includes Knight (D200); other weapons are standard (fist included, D197). ITEM-PROF-1b: the vocation is labelled per binding: TibiaWiki `vocrequired` from `imports/tibiawiki/facts/items-stats.json` read by numeric id (PROVEN), else the Crystal ff7ede5 `items.xml` weapon `vocation` attribute staged by `--stage-crystal ITEMS_XML` into `imports/crystalserver/facts/items-weapon-vocations.json` (digest-pinned; a weapon node without it is `unrestricted`; DERIVED), else UNKNOWN with a reason. A binding without evidence stays `unknown` (37 crossbow / 158 knight / 470 standard / 1 unknown). `--check` diffs a regeneration. The optional `proficiency.client_binding` Item field carries `client_proficiency_id` and `threshold_class` (standard, knight, crossbow, unknown). |
| `donor_census.py` | Censuses the ids a Crystal donor revision (an upstream checkout past the pinned `ff7ede5`) adds over the pinned engine: family-profile classification only, no Oteryn identity minted; since B1b/B2 each minted id's committed epoch-2 key is read (`registry_key`) so the wiki-evidence join applies, a held id keeps the provisional `donor:` key. Reuses `engine_items`'s classification helpers directly, in `convert_item`'s own priority order; never forks or modifies `engine_items.py`. See "Donor census" below. |
| `test_donor_census.py` | No-network fixture tests for `donor_census.py`: `donor_key`'s provisional format, its classification priority order, and the donor/base id set-difference. Run with `python test_donor_census.py`. |
| `samples/donor-census-crystal-summer-update-00ce02a5.json` | The B1a census of the 412 ids the `summer-update` donor revision adds over pinned Crystal. Frozen: it is the B1b epoch-2 input, pinned by exact bytes in `cw2_b1_import.rs` and the binding generator, and is never regenerated. |
| `samples/donor-census-crystal-summer-update-00ce02a5-keyed.json` | The live donor census (`donor_census.py` default output since B2): rows carry each id's committed key, and wiki evidence joins under it. |
| `client_appearance_census.py` / `test_client_appearance_census.py` | Task B3: censuses every 15.30 client appearance id that no pinned `items.xml` (Crystal, donor, Canary) defines. By default it reads `content/assets/files/appearances-2dfa943b….dat`, the file from the owner-staged 15.30 assets (#1251–#1253), pinned by size and sha256. CI runs `--check` against the pinned `items.xml` files, fetched at their revisions and digest-verified. It records the appearance facts, the routing Oteryn's own rules decide from the appearance alone, and a provisional `client:` key. No identity is minted. `--membership-out PATH` instead writes only the id-only membership manifest `OTERYN_CLIENT_APPEARANCE_MEMBERSHIP/v1` (needs just `--appearances`, no engine sources). |
| `samples/client-appearance-census-15-30-2dfa943b.json` | Committed B3 census: 9,310 undefined ids (1,306 in the 15.30 range). 8,766 are routed (Terrain ground/border 2,896, WorldObject immovable 5,569, corpse 301), 247 are pickupable candidates and 297 are unclassified. |

The profiles are guidance inside one schema. Missing a common capability produces a
warning; optional capabilities preserve the wider census union without warning noise.
This avoids making an exceptional but valid combination impossible. Unknown
fields fail closed, so map placement, collision, pathing, source numeric IDs and mutable
instance state cannot silently enter an Item definition.

```text
pip install -r requirements.txt -r requirements-dev.txt  # dev: ruff==0.16.1
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python validate_item.py synthetic-valid-item.json synthetic-valid-dependencies.json --manifest synthetic-valid-import-readiness.json
```

CI: `.github/workflows/item-authoring-schema.yml` runs these steps plus
`test_engine_items.py`, `test_lower_promotion_packet.py`, `test_lower_wiki_stats_packet.py`, the Item Master census and
Ruff on every PR touching the package. `population_census.py --check` and
`lower_promotion_packet.py --check`/`--self-check` need the pinned Crystal/Canary
checkouts, so neither is a CI step; each stays a local check run by hand before a
whole-population claim is made.

Engine population census (pinned Crystal/Canary checkouts, digests verified before read
on both LF and CRLF checkouts; Crystal also needs its existing delivery list, Canary
needs the `weeklyItems` table in `data/modules/scripts/taskboard/settings.lua`, plus a
`--rule-source` Crystal checkout for the Delivery Task rule below):

```text
python population_census.py --engine crystal --source /path/to/crystalserver --self-check
python population_census.py --engine canary --source /path/to/canary --rule-source /path/to/crystalserver --self-check
python population_census.py --engine crystal --source /path/to/crystalserver --check
python population_census.py --engine canary --source /path/to/canary --rule-source /path/to/crystalserver --check
python engine_items.py --engine crystal --source /path/to/crystalserver --id 3288
python engine_items.py --engine canary --source /path/to/canary --rule-source /path/to/crystalserver --id 3031
python test_engine_items.py
```

`population_census.py` is evidence tooling: it proves what the pinned engine sources
convert to under this schema today, not a corpus migration or Game truth. It never
commits per-item rows, only counters and capped examples in `samples/`.

### Item semantic-promotion lowering (v1, not yet wired)

`apps/game-server/src/content/cw2_b1_import.rs` already decodes one hand-compiled Item
semantic-promotion packet
(`docs/agents/evidence/OTV2-20260923-content-world-item-semantic-promotion.json`,
`OTERYN_ITEM_SEMANTIC_PROMOTION/v1`) into the existing Reference Item family for 9
field paths: `presentation.name`, `weapon.attack`/`defense`/`extra_defense`/
`range_cells`/`hit_chance`, `protection.armor`, `charges.count`,
`container.capacity`. `lower_promotion_packet.py` proves this package's own authored
Item data can systematically feed that same decoder, at population scale, rather than
by hand: it runs `engine_items`/`validate_item` over the whole pinned Crystal
population exactly like `population_census.py`, keeps only Items whose bundle
validates with zero errors, and re-encodes their authored values for those 9 field
paths in that decoder's exact typed representation (`TEXT`, `SIGNED_POINTS`, `CELLS`,
canonical-reduced `RATIONAL_PERCENT`, `COUNT_U32`, `CAPACITY_U16`) — a value this
package's own schema allows but the Rust decoder's narrower bounds do not is skipped,
counted, and never smuggled in as a wrong or clamped value. Every row is then checked
again by `validate_packet`/`validate_row`, a fail-closed Python restatement of the
Rust decoder's own rules (shape, kind, source/typed equality, bounds, ordering,
uniqueness, count partition), before the packet is written.

```text
python lower_promotion_packet.py --source /path/to/crystalserver --self-check
python lower_promotion_packet.py --source /path/to/crystalserver --check
```

The committed output is `samples/promotion-crystal-ff7ede5.json`: 14,927 rows over
12,301 Items (`charges.count` 126, `container.capacity` 453, `presentation.name`
12,301, `protection.armor` 432, `weapon.attack` 621, `weapon.defense` 636,
`weapon.extra_defense` 160, `weapon.hit_chance` 56, `weapon.range_cells` 142;
~3.8 MiB). Its `schema`/`profile`/`status`/`next_action` are deliberately different
literal strings from the pinned Rust constants and from the wired packet's own values,
so this candidate can never be mistaken for, or silently accepted as, the wired one.
The #1048 Rust wiring pins an earlier copy (13,292 rows over 10,674 Items, before the wiki
family fallback and blessing charms added Items); re-pinning that copy belongs to the
promotion-pass assignment.

This is a v1 lowering **candidate**, not a Rust-side claim: wiring it into
`apps/game-server/src/content/cw2_b1_import.rs` (pinned byte-count/digest constants
for this packet, a bespoke apply function mirroring `apply_item_semantic_promotion`,
and a Rust integration test, per the existing packet's own pattern) is left to the
Content/World import role, since it is the one that owns and can requalify the target
Item family this would apply to.

### Value-dependent fields and non-Item routing

A `VALUE_DEPENDENT` catalog field (`disposition == "VALUE_DEPENDENT"`, top-level
`status` always `unresolved_semantics`) is resolved per observed value against that
field's own `source_value_routes`: a route whose own `status` is `mapped` is applied
exactly as pinned (`destination_value_equals`/`destination_array_contains`/
`destination_value_transform`) and counts as `mapped`/`converter_missing` like any other
field; a route whose own status is a routed status (e.g. `external_domain`) counts as
`routed`, never a blocker; a value with no route at all is a distinct
`unresolved:<field>=<value>` blocker, so the remaining tail stays diagnosable by value
instead of collapsing into one generic bucket. `flags.unmove` is one such field: **both**
its `true` and `false` routes map to `/item/physical/movable` (`boolean_not_source`); the
field itself does not route to a non-Item owner. An `flags.unmove=true` entry that *does*
resolve a `family_profile` (e.g. a heavy, non-pickupable weapon or container) still
converts as an ordinary Item with `physical.movable: false`.

Some ids are never Item candidates at all: `flags.corpse`/`flags.player_corpse`
(WorldObject/Interaction-owned corpse behavior), a reserved sprite-sheet placeholder
appearance name (`reserved sprite`, `deprecated item`, `empty sprite`, case-insensitive,
with no other `items.xml` attribute), and a `primarytype` naming a Terrain/WorldObject
fixture (`artificial tiles`, `natural tiles`, `fields`, `walls`, `constructions`,
`machines (objects)`, `machines`, `traps`). `convert_item` recognizes these before
attempting `family_profile` classification and reports them as `routed_non_item`
(`{"owner": ..., "reason": ...}`) with `converted: false` and no blockers; the census
counts them under the `routed_non_item` outcome, separate from `not_converted`.

Separately, once `family_profile` classification has already failed to resolve a family,
an `flags.unmove=true` entry (formal schema §5a: immovable map geometry — walls, ground,
borders, top decorations, doors/stairs/ramps/windows — with no Item family profile) is
also routed to `routed_non_item` instead of staying `family_profile_unresolved`: owner
`Terrain`/reason `ground_or_border` when `flags.bank`, `flags.fullbank` or `flags.clip` is
set, otherwise owner `WorldObject`/reason `immovable_unclassified`. Owner decisions WO-2c
(2026-09-30) refine this for two `type` values: a fixed carpet (`type="carpet"`) is owner
`WorldObject`/reason `fixed_carpet` even with `flags.clip` (18 Crystal ids), and an unbanked
magic field (`type="magicfield"`) is owner `Terrain`/reason `magic_field` (21). A carpet or
field that resolves an Item family (for example a placeable house carpet or a campfire)
stays an Item. An `unmove=false` (or
absent) entry with no resolved family keeps `family_profile_unresolved`, since that case
stays editorial backlog rather than a known non-Item owner decision.

Two more owner decisions (2026-09-28), applied lowest-priority and in this order, once
every existing classifier, `immovable_non_item_route` and the wiki-evidence fallback have
all already failed to resolve a family (so no already-resolved item is ever affected):
(1) **engine wrap-target inheritance** — an unresolved item's own `items.xml`
`wrapableto="T"` names another items.xml id `T` whose own `primarytype` resolves through
`PRIMARYTYPE_PROFILE` (one hop only: never chained through T's own `wrapableto`, never
through T's wiki fallback); the item takes that profile with
`family_profile_basis: "engine_wrap_target"` and evidence
`{wrap_target_id, wrap_target_primarytype}`. This resolves house furniture (chairs, a
forge, a workbench, lamps) wrapped into the generic decoration-kit id 23398
(`primarytype = furniture` -> `decoration`); an item whose wrap target does not itself
resolve stays unresolved. (2) **corpse-like "dead ..." items** — an appearance-flagged
corpse (`flags.corpse`/`flags.player_corpse`) is already routed to WorldObject/corpse by
the rule above, before family classification even starts. An engine name starting with
"dead " that carries neither flag and still has no resolved family is routed to
`routed_non_item` (`{"owner": "WorldObject", "reason": "corpse_decoration"}`) when it also
has no `flags.take` (a non-take-able map/quest decoration corpse); when it does have
`flags.take`, its exact lower-cased name is looked up in the small, explicit
`DEAD_CREATURE_PROFILE` owner table (`family_profile_basis: "owner_name_rule"`, evidence
`{rule: "take_able_dead_creature", name}`) and stays `family_profile_unresolved` (fail
closed) if the table does not name it.

Two further owner decisions (2026-09-28), applied lowest-priority still, once every rule
above (including the two just described) has already failed: (1) **fluid types with no
appearance** — an items.xml-only entry (both pinned engines' ids 1-20) whose exact
lower-cased name is a `Fluids_t` enum name (`FLUID_TYPE_NAMES`: water, wine, beer, mud,
blood, slime, oil, urine, milk, manafluid, lifefluid, lemonade, rum, fruit juice, coconut
milk, mead, tea, ink, candyfluid, chocolate — verified against
`src/utils/utils_definitions.hpp`'s `enum Fluids_t`, identical in both engines) and no
`appearances.dat` object routes `routed_non_item` owner `Fluid`, reason
`fluid_type_without_appearance`; any other appearance-less name stays
`family_profile_unresolved`. (2) **late placeholder names** — `old tibia item`,
`unknown item`, `unknow`, `event item`, `unknown corpse` (`LATE_PLACEHOLDER_APPEARANCE_NAMES`)
route the same way the early `PLACEHOLDER_APPEARANCE_NAMES` check does (owner
`WorldObject`, reason `appearance_placeholder_slot`), but only once every other
classifier has already failed — unlike the early check, this one never gates on "no
other items.xml attribute", because these particular names are also real wiki-resolvable
item names on some ids, and only running it last guarantees a wiki- or engine-resolved id
sharing one of these names is never rerouted.

The wiki-evidence capture tool's own `collect_unresolved` probe (which decides which ids
still need a wiki lookup) must ask that question using only classifiers that outrank the
wiki fallback -- never wrap-target inheritance, the dead-item rules, or the two rules
above, all of which rank *below* it -- or a recapture would silently drop real wiki
evidence for an id one of those lower-priority rules also resolves.
`sources["skip_post_wiki_fallback_routes"]` enforces this in that one probe path; it is
never set for a real conversion.

### Name-joined wiki evidence, availability, and a last-resort no-appearance route

Two more `match_basis` values extend the wiki-evidence fallback for a key the exact
`itemid`/`title` joins still miss (owner decisions 2026-09-28): **`appearance_title`**
looks up this item's own `appearances.dat` name (not its items.xml name, which can be a
blanket range label -- Crystal/Canary ids 23577-23667 all share the single items.xml name
`weapon of mayhem`, while the client's own per-id appearance names, e.g. `slayer of
mayhem`, `blade of mayhem`, are the real per-weapon titles) in the same exact-title index
the `title` basis already builds; **`actualname`** looks up this item's items.xml name *or*
appearance name in a second index built from every already-fetched Infobox Object page's
own `actualname` field (TibiaWiki's own asserted in-game object name, which often differs
from the page title, e.g. page "Water (Liquid)" `actualname` "vial of water"), resolving
only when every page sharing that literal `actualname` agrees through the existing
primarytype/objectclass/status order.

Both are looser, name-string joins, unlike the exact `itemid`/`title` bases: a generic
engine name (e.g. "dead rat", "dead goblin") can coincidentally be the literal
`actualname` of an unrelated quest-specific wiki page. `convert_item` therefore consults
`appearance_title`/`actualname` evidence only *after* wrap-target inheritance and the
dead-item rules have both already failed to resolve or route the item -- both of those
already resolve those generic names correctly from engine data alone, with no collision
risk, so a same-named-but-unrelated wiki page reached only through a name join can never
override them. The exact `itemid`/`title` bases are unaffected and still rank above
wrap-target/dead-item, as before. See `fallback_entry_matches_name`'s docstring and
`HIGH_CONFIDENCE_MATCH_BASES`/`NAME_JOINED_MATCH_BASES` in `engine_items.py`.

Every wiki-matched record (any `match_basis`) also carries the matched page's own
`status` infobox field, lower-cased/trimmed, as an optional top-level Item `availability`
field: `{"status": <one of "deprecated"/"ts-only"/"event"/"unobtainable"/"unavailable">,
"evidence": {"source": "tibiawiki", "page_id", "revision_id", "wiki_title",
"match_basis"}}`. TibiaWiki's `Infobox Object` forwards `status` to `Status Messagebox`,
which renders nothing when the field is absent or empty -- there is no confirmed "active"
default, so an absent/empty `status` field, or an item with no wiki record at all, carries
no `availability` field, never a guessed "active" one. A disambiguation record only ever
carries `availability` when every candidate page agrees on `status`. Availability is a
purely additive fact: it never affects `family_profile`, routing, or any existing outcome.
Owner decision: the 10.94 Carving/Mayhem/Remedy weapons (`status: unavailable`, merged
into "of Destruction" in the 2017 Winter Update) are kept as ordinary Items with their
real weapon family -- `status = unavailable` is recorded truthfully, never used to exclude
or specially route an item.

Once every rule above -- including both wiki-evidence tiers, wrap-target and dead-item --
has failed, an item with no `appearances.dat` object at all (never any client-visible
presence in the 15.30 universe: a map-authoring id gap inside a blanket items.xml range,
e.g. `bridge`, `hive structure`, `stone pavement`) routes `routed_non_item` owner
`WorldObject`, reason `no_client_appearance`; the fluid-type and late-placeholder routes
above stay checked first and keep their own reasons. An item with a resolvable wiki
record, or any `appearances.dat` object at all, is never affected.

### Owner leftover-family decision table and the empty-object last resort

After every rule above, 144 Crystal / 103 Canary ids stayed `family_profile_unresolved`.
The owner manually reviewed every one and produced a per-item family decision or an
explicit `UNSURE`; the non-`UNSURE` rows are committed as
`owner-item-family-decisions.json` (schema `OTERYN_ITEM_OWNER_FAMILY_DECISIONS/v1`,
keyed by canonical Item key -- the Tibia key since ITEM-ID-1b: `name`/`profile`/`reason`/`source`), loaded by
`engine_items.load_owner_family_decisions` -- a strict, fail-closed loader modelled on
`load_delivery_overrides` (unique keys, closed key sets, a known registry key, an
admitted profile, non-empty `reason`/`source.facts`). It is the LAST classifier: only
consulted once every higher-priority rule -- engine attributes, both wiki-evidence
tiers, wrap-target inheritance and the dead-item rules -- has already failed, gated by
`skip_post_wiki_fallback_routes` the same way as every other post-wiki-fallback rule. A
hit sets `family_profile_basis: "owner_name_rule"` with evidence
`{rule: "owner_leftover_review_2026_09_28", name}`. Resolves 121 Crystal / 80 Canary
items. ITEM-ID-1b added the 14 owner-approved Q13d rows that have an engine binding
(lists B and C of `docs/agents/evidence/OTV2-20260929-item-family-proposals-266.md`); the
register `docs/agents/evidence/OTV2-20260929-item-id-1b-q13d-family-decisions.json` holds
every Q13d row with its status, and the approved client-only rows wait for their record.
ITEM-Q13D-APPLY (owner decision D165, 2026-09-30) decided the 41 disputed rows: the three with an
engine binding (54262, 23547, 21212, all `quest_item`) joined this table, the client-only rows stay
`PENDING_MINT`, 44044 is `EXCLUDED`, and id 9132 (frost cannon) routes `WorldObject` with reason
`non_pickupable_blocking_prop` (`NON_PICKUPABLE_BLOCKING_PROP_IDS`).

Once even the owner table has failed, an item that DOES have an `appearances.dat`
object but whose `flags` dict is completely empty (not even `take`/`usable`) routes the
same way the placeholder-name checks do: `routed_non_item` owner `WorldObject`, reason
`appearance_placeholder_slot` -- a flag-less client object carries literally nothing to
anchor a family to, the same kind of unauthored placeholder slot, just not
name-identifiable. Resolves 13 items identically in both engines (energy barrier, skull
stone x6, tentugly, towel, wilds monsters outfit x4).

10 ids (identical in both engines) remain the owner's own `UNSURE` verdict -- no wiki
page, or a matched page with no distinguishing fact, and non-diagnostic client flags --
and stay `family_profile_unresolved`, fail-closed editorial backlog.

### Donor census: upstream is a source of facts, never Oteryn truth

Owner rule: an upstream donor checkout (a later revision of Crystal's own upstream,
`zimbadev/crystalserver`) is only a source of FACTS -- its own `items.xml` attributes
and `appearances.dat` client data for ids the pinned engine does not have -- never an
Oteryn classification. Oteryn always applies its own rules to those facts fresh, never
trusting or copying an upstream label. `donor_census.py --donor-source <donor
checkout> --base-source <pinned Crystal checkout>` censuses exactly the ids present in
the donor but absent from the pinned base (a plain items.xml id set-difference); an id
either checkout shares is never touched, so the pinned `population-*.json` censuses
stay byte-identical (verified by `population_census.py --check`, both engines,
unaffected since `engine_items.py` itself is untouched by this tool).

It reuses `engine_items`'s already-factored classification helpers directly --
`non_item_route`, `classify_family_profile`, `immovable_non_item_route`,
`resolve_wrap_target_profile`, `resolve_dead_item_route_or_profile`, the
fluid/late-placeholder/no-appearance/empty-object last-resort routes,
`fallback_entry_matches_name` -- in `convert_item`'s own priority order, but starting
after identity resolution instead of before it: these donor ids have no CW2 B1
allocator key at all, and this task never mints one (a separate, reviewed
Content/World step, B1b, does). Rows are keyed by a provisional, clearly
non-canonical string, `donor:crystalserver@<short-commit>:item/<id>`, never
`oteryn:item.registry.*`. Because the committed wiki-evidence snapshot and the owner
leftover-family table above are both keyed by Oteryn registry key, neither can ever
match a donor-only id; the join is attempted anyway, exactly where `convert_item`
would attempt it, and the committed census reports the resulting zero-match count
explicitly (`wiki_evidence_fallback_resolved`, `owner_leftover_table_resolved`) so
that "no wiki evidence" reads as a proven structural fact, not a skipped step.
Delivery-task eligibility, field mapping and Presentation binding are all out of
scope too (all need identity). The output,
`samples/donor-census-crystal-summer-update-00ce02a5.json` (schema
`OTERYN_ITEM_DONOR_CENSUS/v1`), is deterministic with the same `--check`/`--self-check`
contract as `population_census.py`. Recapturing the wiki for these ids once B1b
assigns them identity is B2's job; this tool never touches the network.

A field this converter cannot implement because the schema needs data neither pinned
engine's evidence supplies keeps a precise blocker rather than a guessed value:
`flags.upgradeclassification`/`upgradeclassification.upgrade_classification`
(`/item/forge` also requires `max_tier`, which only a Wiki infobox field supplies),
`augments` (the nested augment tree's target/value kind has no admitted crosswalk),
`mantra` (its `damage_types` are not derivable from the single `points` attribute),
`runespellname` (no admitted Ability identity crosswalk is wired into this package), and
`proficiency.proficiency_id` (the converter binds only Magic Sword/238, citing the pinned
client, Canary and Crystal crosswalks; every other id keeps
`converter_missing:proficiency.proficiency_id` plus a `proficiency_crosswalk_not_admitted:<id>`
blocker, and its weapon binding comes from `content/proficiencies/bindings.json` instead). `flags.forceuse` is `UNRESOLVED`
with no route at all and is intentionally left as `unresolved:flags.forceuse`.

Successful validation proves authoring shape and declared dependency closure only. It
does not prove runtime lowering, gameplay parity, corpus migration or production
activation.

Dependency closure is exact in both directions: every used reference/asset must be
declared and every declared entry must be used. Import readiness likewise requires one
disposition for every entry in each source's explicit field inventory. `source_profile`
selects one pinned registry; unknown fields and wrong engine revisions fail closed.
Every mapped row is restricted to an explicit formal JSON Pointer (including
array-index patterns where required). Registered-but-ineffective engine keys are
recorded as `pinned_no_effect`, and unresolved historical semantics remain
readiness-blocking rather than being guessed into gameplay truth.

`presentation.appearance_binding` is an exact `{family,key,revision}`
`PresentationRef`, never a raw client ID or an unversioned asset string. Its dependency
record preserves the pinned engine source, appearance ID, frame geometry and ordered
sprite IDs. Repeated sprite IDs are valid. A matching raster `sprite_atlas` is a
separate versioned artifact; without it the validator warns because source sprite
numbers alone are not renderable pixels. The real examples retain that warning instead
of inventing a client asset pack. Tests independently lock the complete geometry and
ordered sprite sequence of all six fixtures and reject inline pixel/blob payloads.

`tradeable` and `marketable` are independently optional facts. Market category or
market vocation restrictions require `marketable=true`; an engine Market flag does
not prove general player-to-player tradeability. Numeric source proficiency IDs remain
provenance until a pinned `proficiency_crosswalks` entry binds them to an admitted
exact `ProficiencyRef`.

**Crosswalk rule (amended by PROFICIENCY-0 §4.1, PROF-CONTENT-1d):**

- **Required source:** the pinned 15.30 client proficiency file (`content/assets/files/proficiencies-7fea90ec….json`,
  source profile `cipsoft_client_15_30_proficiencies_v1`) is CipSoft's own data and admits a
  definition on its own. `profile_binding` requires a client crosswalk. Its external id is the
  `ProficiencyId`, and its source version is the definition's `Version`.
- **Admitted targets:** exactly the committed `content/proficiencies/` definitions
  (`oteryn:proficiency.tibia.p<ProficiencyId>`).
- **Optional corroboration:** Canary `data/items/proficiencies.json` and Crystal
  `data/json/proficiencies.json` crosswalks. They are admitted only where pinned (Magic Sword
  `238`/`3`). The two files are byte-identical at the pinned revisions (SHA-256
  `1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22`).
- **No inline profile:** the Item carries no levels, perks or shaping. They belong to the
  definition (`tools/content-schema/proficiency-authoring`), so the Magic Sword Item carries
  only `profile_binding` `oteryn:proficiency.tibia.p238` and its `threshold_class` `standard`.
- **Threshold class:** `threshold_class` (standard, knight, crossbow) is required together with
  `profile_binding`, as in `content/proficiencies/bindings.json`; an `unknown` weapon gets no
  binding.
- **Rejected:** a binding without the client crosswalk, unknown IDs/versions or targets, wrong
  source artifacts, and inline `levels`/`shaping`.

Character-owned XP and active perk selections are deliberately not Item fields.

Every Item must declare `delivery_task_eligible` as a JSON boolean. `true` means the
definition may be considered by a future Delivery Task pool; `false` is an explicit
exclusion, not an unknown value. The Item does not carry requested quantities, weekly
rotation, task assignment, delivery state, rewards or reset data. Those concerns stay
with the future task/ruleset system, and this authoring field does not claim that a
runtime consumer already exists. The six real-source examples currently use `false`
as an explicit Oteryn author decision rather than presenting it as a Wiki-derived
Global Tibia fact; the converter's `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5` rule (below)
independently reaches the same `false` decision for all six.

### Delivery Task eligibility rule

`engine_items.convert_item` decides `delivery_task_eligible` for every converted Item
under the owner-approved authoring rule `ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5`
(`engine_items.RULE_ID`): an Item is eligible iff the Crystal id sharing its CW2 B1
allocator key is a member of the digest-pinned Crystal delivery list
(`data/scripts/lib/task_board_delivery_items.lua`) at
`ff7ede593c69d4c658b382c97443e8155926924a`, unless `delivery-task-overrides.json`
records an explicit per-Item exception. This is an Oteryn authoring decision applied to
Crystal-list evidence, not an engine fact.

Both engines already share one numeric item-id space through the CW2 B1 allocator (the
same `item_id` that resolves identity also resolves the rule), so a Crystal run reads the
list as its own pinned Delivery Task pool and a Canary run reads it through a required
`--rule-source <crystal checkout>` (see the CLI examples above); a Canary run started
without one is a hard error, never a silent "nothing is eligible". Each engine's *own*
Delivery Task pool (Crystal's delivery list; Canary's `weeklyItems` table) remains
separate upstream *observation* and never decides eligibility by itself — a Canary item
can be a `weeklyItems` member while the rule still finds it ineligible, because it is not
a Crystal-list member. The per-item conversion report keeps both apart:

```json
"delivery_task": {
  "observation": {"source": "canary_task_board_weekly_items", "member": true},
  "decision": {
    "rule": "ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5",
    "basis": "crystal_list_non_member",
    "eligible": false
  }
}
```

`basis` is `crystal_list_member`, `crystal_list_non_member`, or `override` (with a
`reason`) when `delivery-task-overrides.json` names that Item key. The overrides file is
`{"schema": "OTERYN_ITEM_DELIVERY_TASK_OVERRIDES/v1", "rule": "<RULE_ID>", "overrides":
{"<item key>": {"eligible": <bool>, "reason": "<non-empty string>"}}}`; it starts empty,
is validated strictly (unknown keys, wrong types, or an override for an Item key the CW2
B1 allocator never assigned all fail), and `--overrides <path>` on either CLI substitutes
it for testing. An Item with no CW2 B1 allocator key gets no decision at all: it keeps
the `identity_not_in_b1_catalog` and `delivery_task_decision_not_admitted` blockers and
never converts, so it never carries `delivery_task_eligible`.

The BR profile is pinned to stable source `424807`; the Fandom profile is pinned to
historical revision `1035268` and its revision SHA-1. Per-capture SHA-256 remains a
separate required digest. Value-dependent source fields carry `source_value` so the
validator can prove conditional owner routing and normalizations (including signed
weight, `unmove`/`immobile`, item type, equip events, weapon actions and weapon kinds).

The 71-field BR and 84-row historical Fandom base catalogs remain unchanged. The two
real-page supplements are bounded overlays: each stores six exact page IDs, revision
IDs, timestamps, MediaWiki SHA-1 values, UTF-8 wikitext SHA-256 values and complete
sorted raw-parameter inventories plus the timezone-qualified capture time. Coverage
tests require
`observed - base - supplement == empty`; unknown values still fail closed.

Every real example is validated as one bundle. `field_evidence` and
`non_source_defaults` exactly partition every authored scalar Item leaf. Each evidence
record links its typed value to exact raw Canary/Crystal/BR/Fandom observations and to
an allowed catalog destination; external, unresolved, approved-omission and fabricated
fields cannot prove an Item value. Non-source defaults are restricted to an admitted
destination/state/value registry, so they cannot be used to add an unrelated field.
The routed catalog destination must equal the evidenced leaf or be its JSON-pointer
ancestor, and every admitted observation must deterministically normalize from its raw
source value to that exact typed leaf. Correlated edits to the Item and evidence copy
therefore cannot conceal source-value drift. A separately pinned canonical SHA-256 for
each example's complete extracted `source_observations` matrix also rejects a correlated
edit of the raw observations, typed proof and Item value together.
The validator also proves the Item key, appearance ID, Presentation ref,
ordered sprite IDs, engine definition pins, Wiki page pins and unresolved blocker set.
Changing the Item value, the recorded source observation or their typed normalization
makes the bundle invalid. Engine manifest fields are additionally restricted to their actual origin:
`items.xml`, `appearances.dat` or (Crystal only) `bags.xml`, and each cited engine
artifact must carry its pinned SHA-256.

An example keeps the canonical Item key bound to its TibiaWiki BR page in
`imports/tibiawiki/bindings/items.json` (Magic Sword: `oteryn:item.registry.i00003167`).
Examples whose page has no binding yet keep a provisional key and the
`canonical_item_identity_not_bound` blocker.

Weights are exact decimals with two fractional digits (`42.00 oz` = engine weight
`4200`). The auxiliary schemas reference `item.schema.json` definitions by `$id`, like
the Monster package. The Monster package's Item projection still uses
`weight_centioz`, `collision` and a bare `asset_binding`; see the architecture document
for how it maps to this schema.

Leech source modifiers retain only observed chance/amount fields. A missing companion
remains unknown; an explicit source zero remains zero. The schema requires a resource
and at least one percentage. Partial source observations do not qualify complete runtime
modifiers. `test_engine_items.py` runs the partial-leech regression tests in CI.
