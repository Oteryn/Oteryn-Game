# Oteryn Item Authoring Formal Schema v1

- Date: 2026-09-27
- Status: CANDIDATE; authoring/evidence contract only
- Parent master: `OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md`
- Task: `OTV2-20260926-item-authoring-formal-schema-v1`
- Parent control plane: #162
- Programme: KAN-16 / #504
- Current authoring candidate: v4 (Delivery Task eligibility, canonical identity binding and
  validator hardening; v3 frozen SHA superseded)
- Package: `tools/content-schema/item-authoring/`

## 1. Decision

There is one formal portable Item schema composed from typed capabilities. Different
families use different templates and profile guidance; they do not fork into mutually
incompatible schemas.

This preserves valid combinations such as:

- a weapon that emits light;
- an accessory with charges and duration;
- a readable quest item;
- a quiver that is both equipment and a constrained container;
- a decoration kit that transforms into another Item definition.

`family_profile` is required to record the normal semantic family. The profile catalog
preserves the census union as `expected_capabilities`, splits it into common and
optional capabilities, and warns only when common capabilities are absent. It does not
reject an evidence-backed exception merely because a Wiki category is broad.

## 2. JSON unit

One `item.json` contains only stable, portable definition facts:

```json
{
  "identity": {
    "key": "oteryn:item.registry.i00003167",
    "revision": "definition-r1"
  },
  "display_name": "Magic Sword",
  "family_profile": "weapon_melee",
  "delivery_task_eligible": false,
  "presentation": {
    "appearance_binding": {
      "family": "Presentation",
      "key": "oteryn:presentation.crystal.item.3288",
      "revision": "ff7ede593c69d4c658b382c97443e8155926924a.appearance-v1"
    }
  },
  "taxonomy": {
    "item_class": "weapon",
    "primary": "sword",
    "tags": []
  },
  "physical": {
    "weight": {"value": "42.00", "unit": "oz"},
    "movable": true,
    "pickupable": true
  },
  "equipment": {
    "slot": "right_hand",
    "hands": 1,
    "reserved_slots": [],
    "groups": []
  },
  "weapon": {
    "weapon_type": "sword",
    "attack": 48,
    "defense": 35,
    "consumption_mode": "none"
  },
  "imbuement": {
    "slot_count": 2,
    "allowed_family_tiers": [],
    "excluded_families": []
  }
}
```

Every definition reference carries `{family,key,revision}`. Generic effect, sound and
color assets continue to use Oteryn namespaced keys. Appearance uses an exact
`PresentationRef`. `item-dependencies.json` closes those references and carries the
pinned appearance record (source tuple, appearance ID, geometry and ordered sprite
IDs) without embedding Ability, Effect, Interaction, Document or ItemInstance payloads
in the Item.

The presentation record may bind an exact raster `sprite_atlas` by key, revision and
SHA-256. Canary and Crystal provide appearance metadata and sprite numbers but not the
matching raster client atlas, so the six real-source examples deliberately omit this
binding and remain runtime/rendering blocked. A sprite number without that versioned
atlas is not a graphic.

`physical.weight.value` is an exact decimal string with exactly two fractional digits,
so `42.00 oz` corresponds one-to-one to the engine `items.xml` weight `4200`
(centi-ounces) and every value has a single spelling. The Item Master Schema chose this
shape; the runtime unit remains the separate `TYPED_WEIGHT_UNIT` blocker.

`item-dependencies`, `item-import-readiness` and `real-source-evidence` reference the
shared definitions in `item.schema.json` by `$id` instead of copying them, as the
Monster authoring package does.

## 3. Source-derived field delta

The formalization retains the Item Master Schema groups and adds fields exposed by the
bounded Canary, Crystal and Wiki comparison.

### Canary and Crystal

Pinned revisions:

- `opentibiabr/canary@47dfd51f45280a59a1d3e50ba7edd573d7234446`
- `zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a`

Both sources compose item definitions from appearance metadata, `items.xml` and
optional Lua behavior. Their broad in-memory `ItemType` is not copied as one Oteryn
record because it mixes portable definition facts with placed-object facts, mutable
instance state and external relationships.

Every Oteryn Item additionally requires one author-owned
`delivery_task_eligible` boolean. `true` admits that definition to a future Delivery
Task candidate pool; `false` explicitly excludes it. Missing, `null`, numeric and
string values are invalid, so absence cannot be confused with a deliberate decision.
This is only a stable eligibility input. Task selection, quantities, weekly rotation,
delivery from inventory/stash/depot, rewards, reset cadence and per-character progress
belong to the future task/ruleset implementation and are not Item fields.

The pinned engines already demonstrate that this separation is viable: Canary keeps
its weekly Item pool in Task Board settings and Crystal keeps a delivery-item list in
a task-specific Lua module, while their task systems own assignment and delivery
behavior. Oteryn normalizes only the reusable per-definition membership decision into
Item authoring. The boolean is not a claim that Oteryn runtime already consumes it and
is not, by itself, proof of current Global Tibia pool membership or quantity rules.

Fields admitted by this comparison include:

- `requirements.min_magic_level` and explicit `enforcement_mode`;
- presentation grammar plus appearance/effect/projectile/attack bindings;
- weapon attack/hit modifiers, damage range/type, consumption, break chance and chain;
- protection scope (`direct`, `field`, `all`) and condition suppressions;
- typed capacity, regeneration, critical, leech, magic, boolean/flat/percent
  mana-shield, perfect-shot, cleave and flat/percent reflection modifiers;
- fluid content/container/default distinctions;
- readable distance and write policy;
- container content kind/acceptance constraints;
- proficiency binding and lifecycle wrap/unwrap transforms;
- consumable `consume_count`.

The exhaustive v2 audit additionally types source facts that the first candidate had
only sampled:

- independent display flags for stack count, duration, attributes, client expiry timer
  and client wear counter; `showcharges` remains charge display semantics;
- signed source `presentation.display_weight`, while gameplay `physical.weight` remains
  nonnegative;
- signed movement-speed points, invisibility, numeric mantra points/damage types and a
  closed elemental-bond damage type;
- `dual_wielding`, premium-only requirements and Crystal's multiplicative level/magic
  shortfall rule (one failed check = 1/2 damage; two = 1/4);
- distinct launcher, ammunition, thrown-missile, shield and spellbook authoring kinds;
- Crystal chain disable/override/default inheritance without collapsing it into
  Oteryn-native targeting geometry;
- typed wrapping state and `destroy` lifecycle transforms.

The generated engine ledgers classify exactly 143 unique registered XML keys per
engine (`movable` is duplicated in both source registries), plus item-root fields,
14 nested definition fields and appearance-loaded inputs. Only Crystal loads reverse
`bags.xml` relations (`src/items/items.cpp#L324-L339`); Canary has no `bags.xml` at the
pinned revision, so its ledger does not list those fields. Each ledger pins the SHA-256
of every engine artifact a manifest may cite.
Canary-only `proficiency` and Crystal-only `meleeattackeffect` remain explicit. Six
Canary and four Crystal registered keys are pinned source defects and have no formal
Item destination; they are never silently promoted into executable truth.

Canary does not expose a separate light radius at the pinned appearance revision, so
`light.radius_cells` is optional evidence rather than a condition of `emits=true`.
Typed proficiency authoring reuses the existing WorldProject/v2 augment shape and adds
the missing weapon-proficiency distinctions: ordered selection slots, a typed direct
perk value, signed millisecond modifiers, skill-scaled auto-attack/spell-healing
targets and the weapon/shield modifier target. Free-form augment strings are not
admitted.

Canary and Crystal both carry the full Magic Sword profile at source pair `238`/`3`.
At the pinned revisions, Canary `data/items/proficiencies.json` and Crystal
`data/json/proficiencies.json` are byte-identical (SHA-256
`1a915dffd9265cd1c18d39e55da7ede691b2e58add534bc186238ae028a73f22`), including the
same profile name, three ordered levels and six perk records. The numeric ID remains a
source-local identity. It binds `oteryn:proficiency.weapon.sword.magic-sword` only
through two exact `dependencies.proficiency_crosswalks` entries, one for each engine.
The validator requires both source identities, the admitted canonical target and exact
inline payload; repeating an invented ref in `dependencies.definitions` does not pass.

The Item owns only the static proficiency offer: profile binding, levels, one-per-level
selection policy, ordered choices, typed targets and values. Character XP, unlocked
levels and active choices are mutable player state and remain outside Item authoring.
Global XP curves, catalyst awards, protection-zone change rules, persistence and
runtime effect application belong to the Proficiency/player/ruleset implementation.
This candidate does not claim that those runtime owners already consume this schema.

Community documentation is corroboration, not executable authority. The pinned
[Fandom Magic Sword revision `1147223`](https://tibia.fandom.com/index.php?title=Magic_Sword&oldid=1147223)
renders the same six labels and values as the engine profile (rendered HTML captured
2026-09-27, SHA-256
`a1e3b417e08493c5e5de849046b65adcdce304c93c095efd230c03972276b28e`). The TibiaWiki
BR [gameplay guide](https://www.tibiawiki.com.br/index.php?stableid=442799&title=Manual%3AJogabilidade)
independently states that a level offers one to three choices and only one perk per
proficiency level may be active. The engine JSON remains the exact
numeric profile source; the Wiki pages supply human-readable semantic corroboration.

Canary's Market appearance flag proves `marketable=true`, not general
`tradeable=true`. Candidate v3 therefore makes these two facts independently optional;
market category or market vocation restrictions require explicit `marketable=true`.

The protected `ReferenceItemSemantics` model is also preserved where the earlier
human-readable master view was intentionally flatter: equipment may use multiple typed
patterns, temporal authoring retains the stop-duration flag, and trade distinguishes
general tradeability from market listing plus vocation restrictions.

`forge.max_tier` remains because the protected Oteryn WorldProject/v2 and Item Master
evidence already model it per Item. It is not inferred from Canary's global tier table;
an importer without independent item-level evidence must leave the Forge capability
unmapped rather than copy a global maximum onto every Item.

Evidence locations:

- Canary ItemType:
  `https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/items.hpp#L258-L381`
- Canary parser registry:
  `https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/functions/item/item_parse.hpp#L19-L164`
- Canary appearances schema:
  `https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/protobuf/appearances.proto#L124-L192`
- Crystal load order:
  `https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/src/items/items.cpp#L137-L275`
- Crystal parser registry:
  `https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/src/items/functions/item/item_parse.hpp#L27-L173`
- Crystal ItemType:
  `https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/src/items/items.hpp#L268-L391`
- Canary Task Board Item settings:
  `https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/modules/scripts/taskboard/settings.lua`
- Canary Task Board catalog/rules:
  `https://github.com/opentibiabr/canary/tree/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/modules/scripts/taskboard`
- Crystal Delivery Task Item list:
  `https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/data/scripts/lib/task_board_delivery_items.lua`
- Crystal weekly task implementation:
  `https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/src/io/ioweeklytasks.cpp`

### Wiki BR and Fandom

The exact TibiaWiki BR infobox inventory remains the source-field census authority:

- `https://www.tibiawiki.com.br/index.php?stableid=424807&title=Predefini%C3%A7%C3%A3o%3AInfobox_Item`

Fandom family/project material is corroborating taxonomy and authoring-shape evidence,
not runtime authority. The registry is explicitly the 84-row historical migration table
at revision `1035268` (`2023-08-12T17:54:38Z`), not a claim of a current Fandom census:

- `https://tibia.fandom.com/wiki/TibiaWiki:Projects/Merge_Items_and_Objects`

The base registries remain exactly 71 BR fields and 84 historical Fandom rows. They are
not widened to pretend that one template revision describes all current item pages.
Two bounded overlays capture only the delta observed on exact revisions of Magic
Sword, Demon Armor, Backpack, Red Apple, Sudden Death Rune and Vial: 14 BR fields and
5 Fandom fields. Each overlay stores page ID, revision ID/timestamp, capture timestamp,
MediaWiki SHA-1, UTF-8 wikitext SHA-256 and the complete sorted raw-parameter inventory. Tests require
`observed_fields - base_fields - supplement_fields == empty` and reject unobserved
supplement fields.

The comparison confirms the need for `requirements.min_magic_level`, explicit readable
write policy and container content constraints. It also confirms that community value,
drops, NPC offers, quest membership and source numeric IDs must not become intrinsic
portable Item truth.

The TibiaWiki BR `modificadores` parameter maps to the typed `modifiers` aggregate.
Editor notes alone cannot satisfy it, and any unparsed modifier clause blocks import
readiness. Fandom `actualname`, `fansite` and `imbuements` remain explicitly unresolved
at the pinned historical revision.

The six generated examples use only real source identities (2854, 2874, 3155, 3288,
3388 and 3585), exact pinned Wiki revisions and the identical ordered sprite sequences
verified in Canary and Crystal. Source IDs stay in evidence/crosswalk data, never as
the canonical Item identity. An example keeps the canonical Item key that
`imports/tibiawiki/bindings/items.json` binds to its pinned TibiaWiki BR page: Magic
Sword (page `5810`) is `oteryn:item.registry.i00003167`. All six examples are now
bound through the Crystal item-id binding (PR #989, fixed by #996), and their keys carry
exact Proficiency references verified against both engines. Magic Sword now carries its admitted exact Proficiency
and Ability references; other unadmitted Ability, sound, interaction and raster-atlas
dependencies remain explicit blockers instead of guessed fields.
All six examples set `delivery_task_eligible=false` as an explicit Oteryn authoring
decision recorded in `non_source_defaults`; it is deliberately not presented as a
Wiki- or engine-proven parity fact.
Their closed evidence records partition every ordinary authored scalar leaf into
either a source-evidenced value or an explicit non-source default. Exact
profile-bound proficiency leaves form a separate closed partition: the validator
matches the entire static payload and both source crosswalks against the admitted
profile registry. The validator cross-checks
each source proof against its effective catalog route, normalized value and exact raw
observation, then verifies the bundle against the Item, both pinned engine artifacts,
both pinned Wiki pages and the complete blocker set; the examples are not merely
standalone schema-valid Item objects. The catalog route must equal the scalar leaf or
be its JSON-pointer ancestor, while a pinned raw-to-typed transform must reproduce the
leaf value for every observation. This rejects both unrelated proofs and correlated
edits that make Item/evidence copies agree while departing from the pinned raw fact.
The complete extracted raw-value matrix for each example is additionally bound by an
independently pinned canonical SHA-256, so changing source observations, evidence and
Item together still fails closed.
Non-source leaves must match an admitted
destination/state/value normalization and cannot act as a generic escape hatch.

## 4. Boundary table

| Source concept | Formal owner |
|---|---|
| Stable portable stats/capabilities | `item.json` |
| Exact referenced definitions/assets | `item-dependencies.json` |
| Source URL/revision/digest and per-field disposition | import-readiness manifest |
| Stable admission to a future Delivery Task pool | `item.delivery_task_eligible` |
| Task pool construction, quantities, assignment, delivery, rewards and reset/player state | task/ruleset owner |
| Collision, walk/path/projectile blocking, floor change | Terrain / placed WorldObject |
| Hang/rotate/placement orientation | placed WorldObject / Interaction |
| Fluid source tile/cask and sleepable bed placement | Terrain / WorldObject / Interaction |
| Current count, subtype, charges, text, owner, container contents | ItemInstance / durability |
| Rune/potion executed behavior | Ability / Effect / Interaction |
| Lock opened by a key | lock/Interaction relation |
| Creature drops, NPC offers, quest membership | reverse relation owner |
| Community/editor estimate | editor metadata only |

Unknown root or capability fields are rejected. The schema therefore cannot accept
these foreign concepts as an accidental generic attribute bag.

This formal boundary narrows the earlier master census candidate for `bed.sleepable`
and fluid sources: those facts describe a placed bed, cask or terrain source and route
to WorldObject/Terrain/Interaction. Portable fluid contents and containers remain Item
capabilities.

### Relation to the Monster authoring Item projection

`monster-authoring` carries a verification projection of Items (`$defs/item`) for
corpse and loot checks. `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` states it is "not Item
authority" and that `ItemRef` must resolve to the canonical Item catalogue before
admission. Where the two differ, this schema follows
the executable Game model and the projection must follow it:

| Fact | Item authoring (this schema) | Monster projection today | Game model |
|---|---|---|---|
| weight | `{value:"42.00",unit:"oz"}` | `weight_centioz: 4200` | unit blocked (`TYPED_WEIGHT_UNIT`) |
| collision | not an Item fact | required `collision.*` | `CellDefinition.collision`, placement `CollisionFootprint` |
| fluid source | Terrain/WorldObject | `fluid.fluid_source` | Terrain/WorldObject |
| appearance | `PresentationRef {family,key,revision}` | bare `asset_binding` key | `ProjectV2DefinitionRef` to Presentation |
| percentages | lowest-terms ratio | (loot chances stay 0..100 numbers → ppm) | `ReferenceRationalPercent`; loot `probability_ppm` |

The weight values convert exactly (`weight_centioz = value × 100` for `oz`). Aligning
the projection is a separate Monster-package change.

## 5. Profiles and templates

The profile catalog preserves all 22 semantic profiles and all 50 navigation families
from the Item Master census. Thirteen templates cover materially different JSON shapes:

1. generic portable item;
2. armor;
3. accessory;
4. melee weapon;
5. distance launcher;
6. ammunition;
7. magic weapon;
8. rune;
9. container;
10. food;
11. fluid;
12. document;
13. portable decoration kit.

Quest items, tools, materials, keys, event collectibles, plants, light sources and
transformation items start from the nearest template and add only evidenced capability
groups. Light remains a cross-cutting capability rather than a separate incompatible
Item schema.

## 5a. Engine population census

`engine_items.py` converts every pinned Crystal and Canary `items.xml` +
`appearances.dat` entry into a candidate Item bundle and `population_census.py`
validates all of them (`samples/population-*.json`; counters and capped examples only,
no per-item rows). Canonical keys come from the explicit Crystal identity bindings
`imports/crystalserver/bindings/items.json` (`ots/item_server_id`, #989 and #996; e.g.
Crystal `3288` → `oteryn:item.registry.i00003167`, `3031` →
`oteryn:item.currency.gold_coin`). Both engines share that id space; a Canary id without a
binding stays `identity_not_in_b1_catalog`. All six real examples are bound, and where a
TibiaWiki BR binding also exists it must agree.

| | Crystal `ff7ede5` | Canary `47dfd51` |
|---|---|---|
| entries | 38,157 | 37,527 |
| validator errors | 0 | 0 |
| valid, only the sprite atlas pending | 11,216 | 10,879 |
| valid, other blockers | 902 | 877 |
| routed to a non-Item owner | 25,728 | 25,265 |
| not converted (no family, or no identity binding) | 311 | 506 |
| Delivery Task eligible | 433 | 401 |

Entries that are not portable Items are counted as `routed_non_item` with an owner and
reason, not as failures (Crystal counts): WorldObject `immovable_unclassified` 8,450,
`appearance_placeholder_slot` 4,338, `corpse` 3,344, `corpse_decoration` 14,
`primarytype_world_object` 981; Terrain `primarytype_world_object` 5,138,
`ground_or_border` 3,443; Fluid `fluid_type_without_appearance` 20 (§5d). An immovable
entry that resolves an Item family stays an Item with `physical.movable = false`.
`corpse_decoration` (owner decision 2026-09-28, §5c) is identical on Canary (14); so are
`fluid_type_without_appearance` (20) and the 38-item increase in
`appearance_placeholder_slot` (§5d).

`PRIMARYTYPE_PROFILE` also admits two case-fold/plural aliases of existing entries
(`decorations`, `lamps`); neither occurs in either engine's `items.xml`, so they change no
engine-attribute result. `tools (objects)` is deliberately not an alias of `tools`: the
TibiaWiki "(Objects)" suffix names a map-object category, and the engine rows carrying it
(niche, buoy, buoy line, parasol, ...) stay routed to WorldObject/Terrain, not portable tools.

Every catalog-mapped field is converted and value-dependent routes are applied as pinned.
`weapontype` `ammunition` is mapped; `ammo` and `rod` are pinned no-effect because both
engines reject them. Writability comes only from `items.xml`; appearance write flags prove
readability only. Two exact engine defaults are admitted: Forge `max_tier` by
classification (1→1, 2→2, 3→3, 4→10, from `data/scripts/systems/item_tiers.lua`,
identical in both engines) and mantra damage types (energy, fire, earth, ice). An Item
with proficiency `238` cites both engines' admitted crosswalks.

Remaining blockers (Crystal): `sprite_atlas_not_admitted` on every converted Item (no
admitted sprite atlas yet); `family_profile_unresolved` 311 (no structural signal and no
admitted wiki evidence, engine wrap-target, owner name-rule, fluid-type or
late-placeholder evidence either; editorial backlog — see §5d); other
proficiency ids 642, `augments` 83 and `runespellname` 36 (need an Ability identity
crosswalk); `flags.forceuse` 34 (loaded but unused by both engines); and small
data-quality residuals such as `stopduration` without decay or a container without
`containersize`. Canary's own `family_profile_unresolved` is 270, plus 236
`identity_not_in_b1_catalog` (no Crystal allocator key at all, so the wiki fallback below
is never consulted for those; 270 + 236 = 506).

`family_profile_basis` distinguishes how each converted Item's `family_profile` was
decided: `engine_attribute` (the engine's own `primarytype`/`weapontype`/slot/etc., as
`classify_family_profile` always decides first), `wiki_evidence_fallback` (§5b below,
applied only when the engine carried no such signal at all), `engine_wrap_target` or
`owner_name_rule` (§5c below, both lower-priority than the wiki fallback). Crystal: 10,679
`engine_attribute`, 1,342 `wiki_evidence_fallback`, 85 `engine_wrap_target`, 12
`owner_name_rule`. Canary: 10,357 `engine_attribute`, 1,302 `wiki_evidence_fallback`, 85
`engine_wrap_target`, 12 `owner_name_rule`. `clothing accessories` is never an admitted
engine-attribute `primarytype` value (old rag and ivory comb both carry it natively but
it names no one real family on its own; see §5b), so both fall to the wiki fallback
instead of `engine_attribute`.

`delivery_task_eligible` follows the owner-approved authoring rule
`ADOPT_CRYSTAL_DELIVERY_LIST@ff7ede5`: an Item is eligible iff its Crystal id is on the
digest-pinned Crystal delivery list at `ff7ede5`, unless
`tools/content-schema/item-authoring/delivery-task-overrides.json` records a per-Item
exception with a reason (strictly validated, currently empty). Canary runs read that same
list through a required `--rule-source` Crystal checkout. Each engine's own pool stays a
separate observation and never decides. The list has 436 unique ids; at Crystal 433 are
now converted and eligible (384 before the wiki fallback recovered 49 delivery-list
members that were previously stuck `family_profile_unresolved`), 2 are in `items.xml` but
still not converted, and 43848 is absent from `items.xml`. Text artifacts are digested as
Git blob bytes (CRLF normalized to LF); `appearances.dat` is digested raw, so LF and CRLF
checkouts give identical censuses. `--check` fails on any drift from the committed census.
`population_census.py --check` needs the pinned Crystal/Canary checkouts, so it is run
locally, not by repository CI.

## 5b. Wiki-evidence family fallback (English TibiaWiki)

`tools/content-census/item_wiki_family_capture.py` is a manual, network-using capture
tool (not run by repository CI) that resolves every `family_profile_unresolved` engine
Item's own numeric id or name against English TibiaWiki (`tibia.fandom.com`), recording
which join produced the evidence as an auditable `match_basis: "itemid" | "title"` on
every record and in the emitted `family_profile_evidence`.

The exact-id join (`match_basis: "itemid"`) runs first and is authoritative: every
main-namespace page embedding `{{Infobox Object` (`list=embeddedin&eititle=Template:
Infobox Object` — 9,980 pages as captured) is fetched once and indexed by the integer(s)
in its own `| itemid = ...` field (comma-separated lists included; 13,906 distinct ids
indexed). When one or more pages list the engine's own numeric id, only those pages are
ever considered for it — one page resolves through the admitted fields as usual, 2+ pages
must all agree on the exact same profile — and if the id-matched pages exist but do not
resolve (or disagree), the item stays unresolved with no fallback to name matching: exact
id evidence, once it has an opinion at all, is never overridden by a name guess. Only when
no page lists the id at all does the item fall back to the pre-existing name-based join
(`match_basis: "title"`).

That name-based join looks up the engine's own name against an exact, case-insensitive
index of every main-namespace title (`list=allpages`, `apnamespace=0`, `apfilterredir=all`,
walked to completion — 29,012 titles on the wiki as captured). A name's candidate titles
are every title whose lower case equals the name itself, or the name plus a `" (item)"`/
`" (object)"` disambiguating suffix (TibiaWiki's own convention for an item that shares a
bare name with an NPC or other page); content is fetched with `redirects=1`. That is the
full extent of the matching: exact case-insensitivity, redirects and the two admitted
suffixes, never a capitalisation guess. Each candidate is parsed for `{{Infobox Object`/
`{{Infobox Item` `primarytype`/`objectclass` fields, or, for a `{{Disambig}}` page,
resolved through every linked candidate page (read from both `[[wikilinks]]` and the
positional entries of an `{{ItemList ...}}` template, Fandom's own alternative to linking
each variant, `key=value` parameters skipped either way) instead; a page with neither is
not evidence (e.g. an NPC page sharing an item's bare name) and is ignored. A match is
only ever committed when `engine_items.resolve_wiki_family_value` — the admitted mapping,
`PRIMARYTYPE_PROFILE` folded to lower case plus a small dedicated
`WIKI_OBJECTCLASS_PROFILE` for the `objectclass` fallback, plus `WIKI_STATUS_PROFILE` for
the `status` fallback — resolves it to exactly one profile; when a name's candidates
include two or more pages carrying admitted evidence (directly, or through a
disambiguation page's own linked candidates), every one must independently resolve and
all must agree on the exact same profile, recorded with the existing disambiguation shape
("several pages, all agree"), regardless of which join produced them. Broad buckets
(`others`, `other items`, `household items`, `tools and other equipment`, `utilities`,
`plants, animal products, food and drink`, `other objects`) and the owner-decision-pending
values `fireworks` and `clothing accessories` never resolve as `primarytype`/`objectclass`.
`blessing charms` resolves to `progression_material` by owner decision (2026-09-28): the
charms are single-use items that grant one blessing (Crystal
`data/libs/systems/blessing.lua` `Blessings.All[*].charm`: 10341-10345, 25360, 25361); the
blessing effect itself is runtime behaviour.

An infobox `status` field is a third, lowest-priority admitted field: consulted only once
neither `primarytype` nor `objectclass` has resolved (a non-empty, unadmitted value in
either never blocks it), and admitting only `status = event` -> `event_collectible`. By
owner decision (2026-09-28): TibiaWiki's own "Clothing Accessories" `primarytype` names no
one real family (green piece of cloth and ivory comb are filed there only as a *secondary*
type, their real family being the `primarytype = Creature Products` -> `material_valuable`
each also carries; old rag carries no such secondary evidence and is instead TibiaWiki's
own `status = event` 20th-anniversary drop, resolving to `event_collectible` through the
new field instead). Both old rag (Crystal/Canary `24415`) and ivory comb (`32773`) carry
`primarytype = clothing accessories` as their only native engine attribute, so neither
resolves from `engine_attribute` at all; each depends entirely on its own wiki fallback
record.

The snapshot (`imports/tibiawiki/facts/items-family-fallback.json`, batch
`g5-item-family-fallback-tibiawiki-r1`) holds only page/revision identity, digests and the
one or two field observations each record needed — never wikitext bodies or images — and
is strictly loaded and re-validated (`engine_items.load_wiki_family_fallback`, mirroring
`load_delivery_overrides`'s fail-closed style: duplicate-key rejection, unknown-key
rejection, a recomputed-and-compared `snapshot_sha256`, every `registry_key` required in
the identity index, a `match_basis` required on every record and exactly `itemid` or
`title`, every mapped value required to resolve through the admitted mapping) before
`convert_item` ever applies it. It is applied only after `classify_family_profile` and the
non-item/immovable routing have already found no family, and only when the unresolved
item's own lower-cased engine name is one the snapshot actually matched (regardless of
which join produced the record: `matched_names` is always the engine's own lower-cased
name), and only to an entry that has an `appearances.dat` object (the fluid-kind name rows
1-20 and other appearance-less rows are not physical Items and stay unresolved);
engine-attribute classification always wins. A hit sets `family_profile_basis:
"wiki_evidence_fallback"` and a `family_profile_evidence` citation (`match_basis`, matched
field/value, wiki title/page id/revision id/content digest, or the full candidate list for
a disambiguation) on the converted Item. Of 3,563 previously-unresolved (engine, id) pairs
(1,822 Crystal + 1,741 Canary) across 1,060 unique engine names, the exact-id join matched
1,565 registry keys by their own numeric id (1,378 resolved directly, 2 as an
agreeing-candidate disambiguation, 185 left unresolved with no name fallback because the
id-matched pages did not resolve or disagreed), and the remaining 108 names went through
the pre-existing name-based join, resolving 35 more. The snapshot holds 1,415 registry-key
records total (1,380 `match_basis: "itemid"`, 35 `match_basis: "title"`; 1,412 direct, 3
disambiguation), recovering 1,342 Crystal and 1,302 Canary Items (2,644 total, up from 877
Crystal / 837 Canary before the exact-id join); the remainder stays
`family_profile_unresolved`, still fail-closed. Ten registry keys that already had a
title-based record changed profile once exact id evidence disagreed with it (id evidence
always wins), and four keys that used to resolve by title (`cm token`, `glowworms` x2,
`empty bucket`) lost their record entirely because their own id-matched pages exist but do
not resolve — both are the intended, owner-specified behaviour of exact id evidence
overriding a same-page name match. Old rag (Crystal/Canary `24415`) and ivory comb
(`32773`) both now resolve through the exact-id join (`match_basis: "itemid"`) rather
than a name match, to the same `status = event` -> `event_collectible` and `primarytype =
Creature Products` -> `material_valuable` evidence as before (§5a).
The title index itself is not pinned in the schema (it is provenance, not correctness --
every record's own wiki page/revision identity is what the loader verifies); its count and
digest are recorded in the capture tool's uncommitted report only.

`collect_unresolved` (the function that decides which ids still need a wiki lookup) must
probe using only the classifiers that outrank the wiki fallback -- engine attributes and
`immovable_non_item_route` -- never wrap-target inheritance, the dead-item rules, or
either of the two §5d last-resort routes, all of which rank *below* the wiki fallback in
`convert_item`. `sources["skip_post_wiki_fallback_routes"]` enforces this: without it, an
id independently resolvable by one of those lower-priority rules would be wrongly excluded
from the lookup, permanently dropping its real (and, in production, still-winning) wiki
evidence on the next recapture. This was verified empirically while adding the §5d rules:
the fixed probe reproduces the exact same 3,563 unresolved pairs / 1,565 id-matched keys /
1,415 committed records as the original capture, byte-for-byte identical evidence for
every shared key.

## 5c. Engine wrap-target inheritance and corpse-like "dead ..." items

Two further owner decisions (2026-09-28), both applied only once every existing
classifier, `immovable_non_item_route` and the §5b wiki-evidence fallback have already
failed to resolve a family -- lowest priority, so no already-resolved item is ever
affected -- and in this order:

**Engine wrap-target inheritance** (`family_profile_basis: "engine_wrap_target"`). Many
`family_profile_unresolved` items are house furniture (chairs, a forge, a workbench,
lamps) whose own `items.xml` carries `wrapableto="T"`, almost always the generic
decoration-kit id `23398` (`primarytype = furniture` -> `decoration`). When the
unresolved item's own `wrapableto` names an id `T` whose own `primarytype` resolves
through the existing `PRIMARYTYPE_PROFILE` -- exactly one hop: never chained through T's
own `wrapableto`, and never through T's own wiki fallback -- the item takes that profile,
recorded with evidence `{wrap_target_id, wrap_target_primarytype}`. When T is absent from
`items.xml`, or T's own `primarytype` does not resolve, the item stays unresolved. This
resolves 85 Items identically in both engines (all -> `decoration`; wrap targets `23398`
and, for the "dragon pinata" pair, `23473`, both `primarytype = furniture`): chairs
(ornate, dwarven stone, heart, artist, sculptor, kitchen), lamps (little big flower,
turquoise/purple flower, sea-devil wall, scales wall, opulent floor), a forge, workbench,
wooden stool, grinding wheel, pair of bellows, water bucket/seafood bucket, artist shelf,
wallcupboard, glowworms, luminescent fungi, supreme mana cask/keg, light/torch of change,
and dragon pinata.

**Corpse-like "dead ..." items.** An appearance-flagged corpse
(`flags.corpse`/`flags.player_corpse`) is already routed to WorldObject/corpse by
`non_item_route`, before family classification even starts. An engine name starting with
`"dead "` (case-folded) that carries neither flag and still resolves no family is either a
non-take-able map/quest decoration corpse or a take-able carcass. Without `flags.take` it
is routed non-Item: owner `WorldObject`, reason `corpse_decoration` (14 items, identical
in both engines: dead dragon/bear/cyclops/goblin x2 each, dead lava, and five named quest
corpses -- dead Doctor Perhaps, dead Dirtbeard, dead Evil Mastermind, dead Monstor, dead
Mephiles). With `flags.take`, the exact lower-cased name is looked up in the small,
explicit `DEAD_CREATURE_PROFILE` owner table
(`family_profile_basis: "owner_name_rule"`, evidence
`{rule: "take_able_dead_creature", name}`); every take-able "dead ..." name found in
either pinned engine is an ordinary animal/creature carcass, so the table maps all 7 of
them (dead troll/rat/snake/spider/wolf/rabbit/frog, 12 items total) to
`material_valuable` -- none is a unique/named quest character, since every named corpse in
either engine is one of the five `corpse_decoration` entries above (no `flags.take`). A
take-able `"dead ..."` name absent from the table stays `family_profile_unresolved` (fail
closed) rather than guessed.

Both rules recover the exact same 111 items (85 + 14 + 12) in Crystal and Canary alike:
`family_profile_unresolved` 480 -> 369 (Crystal), 439 -> 328 (Canary); `routed_non_item`
25,656 -> 25,670 (Crystal), 25,193 -> 25,207 (Canary); no other outcome or blocker count
changes.

## 5d. Fluid types and late placeholder names

Two more owner decisions (2026-09-28), each applied only once every rule above --
including §5c -- has already failed to resolve a family for this exact item, so neither
can ever reroute an already-resolved or already-routed id. Verified by diffing every id's
full outcome against the pre-task baseline (the committed §5c state): exactly 58 ids
change in each engine, all from `family_profile_unresolved` to one of these two routes;
every other id's outcome (resolved, routed, or still unresolved) is byte-identical.

(A wiki-evidence `world_object` routing -- itemid-matched pages agreeing on a
`WORLD_OBJECT_PRIMARYTYPES` `primarytype` instead of an admitted Item profile -- was
investigated and then removed: it never routed a single item in the current universe
(playable-first doctrine: no speculative machinery for zero observed cases). The
id-matched-but-otherwise-unresolved pages' own `primarytype`/`objectclass` values, kept
here for reference only since nothing acts on them: `primarytype` empty 57 (remains of a
crude dream, sacramental wine, gnomish lava fishing rod), `primarytype = others` 23
(etcher, special bat coffin, "the tale of joran and yvette - part i"), `objectclass =
other items` 15 (some mortal essence, cleansed sanity, cask), `objectclass = household
items` 3 (dragon pinata, pantibian amphora), `objectclass = tools and other equipment` 3
(spying eye, sulphur blossom lamp), `primarytype = contest prizes` 1 (cm token),
`primarytype = tools (objects)` 1 (root vegetable dish), `primarytype = valuables` 1 (cm
token). None is a `WORLD_OBJECT_PRIMARYTYPES` key.)

**(b) Fluid types with no appearance object** (reason `fluid_type_without_appearance`,
owner `Fluid`). Both pinned engines' `data/items/items.xml` ids 1-20 name the server's
own `Fluids_t` enum values (`FLUID_WATER`=1 .. `FLUID_CHOCOLATE`=20; `FLUID_NONE`=0 has no
items.xml row), verified identical in both engines' own upstream source --
`src/utils/utils_definitions.hpp` (Canary `research_clones/canary-full` lines 413-433,
Crystal `research_clones/crystal-full` lines 423-443; the pinned data-only checkouts this
converter reads carry no engine source tree of their own) -- and carry no
`appearances.dat` object in either engine. `FLUID_TYPE_NAMES` names exactly this list
(water, wine, beer, mud, blood, slime, oil, urine, milk, manafluid, lifefluid, lemonade,
rum, fruit juice, coconut milk, mead, tea, ink, candyfluid, chocolate); an items.xml
record with one of these exact names and no appearance routes non-Item to `Fluid`. Every
other appearance-less `family_profile_unresolved` name stays unresolved: `arena
leaderboard`, `bridge`, `buttress`, `cracked wall`, `earth`, `flooded sand`, `glowing
obsidian pipes`, `hive structure`, `hivecomb floor`, `ice`, `insectoid meatball`, the five
"miniature of a ..." names, `obsidian pipes`, `pepper grass`, `sandy rock pile`, `sewer
pipe`, `skull pile`, `slimy growth`, `small stream`, `stone pavement`,
`time-particle extractor`, `trapped water`, `waterway`, and `weapon of mayhem` (see (d)
below) -- all 20 ids resolve identically in Crystal and Canary.

**(c) Late placeholder names** (reason `appearance_placeholder_slot`, owner
`WorldObject` -- same as the early `PLACEHOLDER_APPEARANCE_NAMES` check in
`non_item_route`). `LATE_PLACEHOLDER_APPEARANCE_NAMES` (`old tibia item`, `unknown item`,
`unknow`, `event item`, `unknown corpse`) route the same way, but only once every other
classifier has already failed -- deliberately never added to the early check, and never
gated on "no other items.xml attribute" the way the early check is, because some ids
carrying these very names are genuinely resolved by engine attributes or by wiki evidence
(id-matched, independent of name) and must never be rerouted. Routed ids (identical in
both engines): `event item` 2 (31964, 32006), `old tibia item` 19 (24504, 24505, 24657-
24664, 24696, 24766-24771, 24774, 24775), `unknow` 6 (32577, 32592, 32637, 32695, 32696,
32706), `unknown corpse` 1 (4023), `unknown item` 10 (9117, 12078, 12259, 12261, 12303,
17345, 27952, 27953, 28185, 28186) -- 38 total.

**(d) Investigation: "weapon of mayhem" (46 Crystal items; resolved by §5e below).**
Crystal `items.xml` carries two disjoint id ranges under the server's generic overcharge
description ("This weapon is overcharged", no `weapontype`/`primarytype` attribute of
their own): `23223-23342` named `weapon of carving`, and `23577-23667` named `weapon of
mayhem`. TibiaWiki's per-weapon "X of Mayhem" pages (Slayer of Mayhem, Bow of Mayhem
(Charged), etc.) carry `itemid` values `23224`, `23226`, `23228`, `23229`, `23239` --
inside the `weapon of carving` range, not the `weapon of mayhem` one; the exact-id join
already resolves all of those correctly (`primarytype = Sword Weapons` -> `weapon_melee`),
which is why "weapon of carving" is fully resolved today. A full-text search of the whole
wiki for every id in `23577-23667` returns zero hits: no page anywhere declares any of
those ids by itemid. But the client's own `appearances.dat` carries CipSoft's real
per-id names for this range (id 23580 "slayer of mayhem", 23595 "bow of mayhem", etc.) --
different from the blanket items.xml label -- and TibiaWiki does document these under
those real per-weapon names, just not by itemid; see §5e, which resolves 45 of the 46 by
that appearance name instead. Of the 91 ids in the range, 45 already resolve via
`flags.clothes.slot` (`equipment_offhand`); of the remaining 46, 45 resolve via §5e and
the last one has no `appearances.dat` object at all and routes via §5h.

Together (b)+(c) recover 58 items identically in both engines. §5e-5h below extend the
same branch, applied in order after (b)/(c): 225 ids change per engine in the final
state (20 fluid + 38 late placeholder + 45 appearance-title + 17 actualname + 105
no-client-appearance), all from the pre-task `family_profile_unresolved` baseline of 369
(Crystal) / 328 (Canary) down to 144 (Crystal) / 103 (Canary); every other outcome is
byte-identical. See `docs/agents/tasks/active/OTV2-20260928-item-nonitem-routing.md` for
the full changed-id proof.

## 5e. Appearance-name wiki title join (`match_basis: "appearance_title"`)

For a key still unresolved after both the exact-id and title joins (§5b), whose own
`appearances.dat` object carries a non-empty `name` that differs (case-insensitively)
from its items.xml name, the capture tool looks that appearance name up in the same exact
case-insensitive title index the `title` join already builds (same `(Item)`/`(Object)`
suffix and disambiguation handling). This exists because a blanket items.xml range label
can hide real, individually wiki-documented per-id names: Crystal/Canary ids
`23577-23667` all share the single items.xml name `weapon of mayhem`, but the client's own
`appearances.dat` carries CipSoft's real per-id names (23580 "slayer of mayhem", 23595
"bow of mayhem", 23638 "wand of remedy", 23660 "rod of carving", ...), and TibiaWiki
documents these under those real names. Records carry `match_basis: "appearance_title"`
and `matched_names` = the appearance name (lower-cased); `convert_item`'s name gate checks
the item's own appearance name instead of its items.xml name for these records.

Resolves 45 of the 46 remaining "weapon of mayhem" ids (§5d) identically in both engines,
to `weapon_melee` or `weapon_distance` depending on the matched page's own weapon type;
the 46th has no `appearances.dat` object and routes via §5h instead. Owner decision
(2026-09-28): these 10.94 Carving/Mayhem/Remedy weapons carry TibiaWiki `status =
unavailable` (merged into "of Destruction" in the 2017 Winter Update) and are kept as
ordinary Items with their real weapon family -- `status = unavailable` is never used to
exclude or specially route an item (see §5f).

## 5f. Availability from TibiaWiki `status`

Every wiki-matched record (any `match_basis`) also carries the matched page's own
`status` infobox field as an optional top-level Item `availability` field: `{"status":
<value>, "evidence": {"source": "tibiawiki", "page_id", "revision_id", "wiki_title",
"match_basis"}}`. TibiaWiki's `Infobox Object` template forwards `status` verbatim to
`Status Messagebox`, whose own `{{#if:{{{1|}}}|...}}` renders nothing when the field is
absent or empty -- confirmed from the live template source, not assumed -- so there is no
"active" default to fall back to: an absent/empty `status`, or an item with no wiki record
at all, carries no `availability` field at all (absence means "not asserted", never
"available"). A disambiguation record only carries `availability` when every candidate
page agrees on `status`. The admitted enum is exactly the five distinct values TibiaWiki's
own `Status Messagebox` switch recognizes (`deprecated`, `ts-only`, `event`,
`unobtainable`, `unavailable`); an unadmitted value fails closed in the loader. Observed
in the recapture (both engines, identical): `unavailable` 105 (includes all 45 §5e
Carving/Mayhem/Remedy weapons), `event` 5, `unobtainable` 2, `ts-only` 1 -- `deprecated`
was not observed. Availability is purely additive: it is never consulted by
`classify_family_profile`, routing, or any other outcome, and the changed-id invariant
proof covers only the (b)/(c)/(e)/(g)/(h) routing changes above.

## 5g. Exact `actualname` join (`match_basis: "actualname"`)

Every TibiaWiki `Infobox Object` page also carries its own `actualname` field -- the
exact in-game object name, which often differs from the page title (page "Water
(Liquid)" `actualname` "vial of water", page "Mosaic" `actualname` "mosaic"). The
exact-id join (§5b) already fetches all ~9,980 Infobox Object pages, so a second index is
built from their own `actualname` field (lower-cased, trimmed, comma/semicolon lists
split) at no extra network cost. For a key still unresolved after the itemid, title and
appearance-title joins, the item's own items.xml name (and, if different, its appearance
name) is looked up in that index; it resolves only when every page sharing that exact
`actualname` agrees through the existing primarytype/objectclass/status order -- one page
resolves directly, two or more must all agree, any disagreement or unresolvable page
leaves the item unresolved with no further fallback. `matched_names` records whichever of
the item's own names (items.xml or appearance) produced the match.

Because this is a looser, name-string join rather than an exact id, a generic engine name
already correctly handled by an *engine-data-driven* rule can coincidentally collide with
an unrelated wiki page's `actualname` -- found empirically while implementing this join:
"dead rat" (already routed by §5c's `DEAD_CREATURE_PROFILE` table to `material_valuable`)
and "dead goblin" (already routed by §5c's corpse-decoration rule to
`WorldObject`/`corpse_decoration`) both also happen to be the literal `actualname` of
unrelated Oramond-quest-specific pages ("Dead Rat (Oramond)", three "Dead Goblin (...)
Quest" pages), which would have silently overridden the already-correct §5c answer.
`convert_item` therefore consults `appearance_title`/`actualname` evidence (§5e/§5g) only
*after* wrap-target inheritance and both dead-item rules (§5c) have already failed to
resolve or route the item; the exact `itemid`/`title` bases (§5b) are unaffected and still
rank above §5c, as before this change (`fallback_entry_matches_name`'s
`HIGH_CONFIDENCE_MATCH_BASES` vs. `NAME_JOINED_MATCH_BASES` in `engine_items.py`). Both
"dead rat"/"dead goblin" keep their original §5c outcome; the changed-id invariant proof
(both engines, zero unsafe diffs) covers this directly.

Resolves 17 items identically in both engines: "tic-tac-toe token" (2 ids, via
disambiguation over "Tic-Tac-Toe Token (O)"/"(X)", `primarytype = Game Tokens` ->
`event_collectible`) and "stone" (15 ids, via disambiguation over nine "Stone (...)"
size/location variant pages, `primarytype = Rocks`/`Metals` -> `material_valuable`).

## 5h. No-client-appearance last resort (`no_client_appearance`)

Once every rule above -- both wiki-evidence tiers (§5b, §5e/§5g), wrap-target and
dead-item (§5c) -- has failed, an item with no `appearances.dat` object at all (the
fluid-type (b) and late-placeholder (c) routes are checked first and keep their own
reasons) routes `routed_non_item` owner `WorldObject`, reason `no_client_appearance`: it
has no client-visible presence anywhere in the pinned 15.30 universe, so it cannot be a
placed, renderable Item. These are id gaps inside a blanket items.xml range where only
some ids also got a client appearance -- e.g. Crystal `bridge` (71 of 103 ids have an
appearance and already route Terrain/WorldObject via other rules; the appearance-less 32
stay); the same pattern recurs for `hive structure` (20), `stone pavement` (12), `slimy
growth` (12), `arena leaderboard`, `obsidian pipes`, and others. Resolves 105 items
identically in both engines. An item with a resolvable wiki record, or any
`appearances.dat` object at all, is never affected.

## 5i. Owner leftover-family decision table (task #15, 2026-09-28)

After §5b-§5h, exactly 144 Crystal / 103 Canary ids stayed `family_profile_unresolved`.
The owner manually reviewed every one (per-item wiki lookup, client-flag inspection and
a proposed profile or an explicit `UNSURE`; the raw review evidence itself is not
committed, only its accepted conclusions below) and produced a per-item family
decision. `tools/content-schema/item-authoring/owner-item-family-decisions.json`
(schema `OTERYN_ITEM_OWNER_FAMILY_DECISIONS/v1`, loaded by
`engine_items.load_owner_family_decisions`, modelled on `load_delivery_overrides`'s
fail-closed style) commits the non-`UNSURE` rows, keyed by Oteryn registry key: `name`
(the exact lower-cased engine name, a guard against a same-key coincidence), `profile`
(one of the 22 admitted profiles), `reason`, and `source` (`wiki_url` or `null` plus a
`facts` summary). 121 of the review's 123 non-`UNSURE` ids were still unresolved on
this branch (2, "tic-tac-toe token", had already resolved via the §5g actualname join
by the time of this review and are correctly excluded); all 121 map to a real registry
key with a matching engine name, verified before committing the table.

The table is the LAST classifier: `convert_item` only ever consults it once every
higher-priority rule -- engine attributes, both wiki-evidence tiers (§5b/§5e/§5g),
wrap-target inheritance and the dead-item rules (§5c) -- has already failed, gated by
`skip_post_wiki_fallback_routes` the same way as every other post-wiki-fallback rule (a
per-item owner decision still ranks below real wiki evidence for the capture tool's
"does this id need a lookup" probe). A hit sets `family_profile_basis:
"owner_name_rule"` with evidence `{rule: "owner_leftover_review_2026_09_28", name}` --
the same basis the dead-item owner table (§5c) uses, distinguished by `rule`.

Resolves 121 Crystal / 80 Canary items -- Canary has fewer real ids in the CW2 B1
allocator's shared id space for this exact set (41 Crystal-only ids have no Canary
counterpart at all). By profile: Crystal `quest_item` 37, `decoration` 28, `document`
15, `tool` 9, `trash` 5, `transformation_item` 5, `light_source` 4, `material_valuable`
4, `plant` 3, `progression_material` 3, `event_collectible` 3, `fluid` 3, `food` 2;
Canary `quest_item` 33, `decoration` 18, `document` 3, `tool` 6, `trash` 5,
`transformation_item` 2, `light_source` 4, `plant` 3, `progression_material` 3,
`event_collectible` 3 (`material_valuable`/`fluid`/`food` have zero Canary-side ids in
this set).

## 5j. Empty client object last resort (task #15, 2026-09-28)

Once every rule above -- including the §5i owner table -- has failed, an item that DOES
have an `appearances.dat` object, but whose `flags` dict is completely empty (not even
`take`/`usable`; `decode_flags`'s own "presence == key in dict" contract means an empty
dict is a proven absence of every flag, never a decode gap), routes `routed_non_item`
owner `WorldObject`, reason `appearance_placeholder_slot` -- the same reason the early
`PLACEHOLDER_APPEARANCE_NAMES` and late `LATE_PLACEHOLDER_APPEARANCE_NAMES` checks use:
a flag-less client object is the same kind of unauthored placeholder slot, just not
identifiable by name. Verified against the pinned Crystal/Canary appearances: energy
barrier (25799), skull stone (10134-10139), tentugly (39003), towel (20889) and wilds
monsters outfit (19125-19128) all decode to `flags: {}` exactly -- 13 ids, identical in
both engines. An id in that name set that turns out to carry any real flag is left
alone by this rule (none of the 13 did).

## 5k. Remaining gap (task #15, 2026-09-28)

10 ids (identical in both engines) stay `family_profile_unresolved` -- the owner's own
`UNSURE` verdict, no wiki page or only a wiki page with no distinguishing fact, and a
non-empty but non-diagnostic set of client flags: `aligned opticording sphere` (19392),
`arena certificate` (23547), `blackened hand mirror` (36876), `cask` (34078),
`eye-shaped frame` (36707), `frost cannon` (9132), `remains of a crude dream` (20129,
20131), `the ashes of a device` (21212), `unknow item` (32267). Each stays fail-closed
editorial backlog rather than guessed; recovering any of them needs either new wiki
evidence or a further owner decision, not a rule change.

Together, §5i-§5j recover 134 Crystal / 93 Canary items (121+13 / 80+13):
`family_profile_unresolved` 144 -> 10 (both engines); `routed_non_item` 25,833 ->
25,846 (Crystal), 25,370 -> 25,383 (Canary); `fully_resolved` 11,278 -> 11,399
(Crystal), 10,941 -> 11,021 (Canary). No other outcome or blocker count changes --
proven by diffing every item id's full outcome against the pre-task baseline (PR #1105,
`bddc0616`), both engines: exactly 134 (Crystal) / 93 (Canary) ids change, every one
previously `family_profile_unresolved`, zero unsafe diffs. See
`docs/agents/tasks/archive/OTV2-20260928-item-owner-leftover-table.md` (merged as PR
#1109) for the full changed-id proof.

## 5l. Donor census: upstream is a source of facts, never Oteryn truth (task B1a, 2026-09-28)

Upstream `zimbadev/crystalserver` (Crystal's own upstream) periodically adds new ids to
its own `items.xml`/`appearances.dat` between the Oteryn-pinned revision (`ff7ede5`)
and a later one. The owner's rule: an upstream donor checkout is only a source of
FACTS -- its `items.xml` attributes and `appearances.dat` client data -- never an
Oteryn classification. Oteryn always decides `family_profile` with its own rules,
applied fresh to the donor's own facts, never by trusting or copying an upstream
label. This principle governs every future donor-ingestion step, not only this one.

`tools/content-schema/item-authoring/donor_census.py` censuses the ids a donor
revision adds over the pinned Crystal engine (the first such donor: `summer-update`
@ `00ce02a5`, 412 new ids -- 244 in the new 15.30 appearance range `52977-55117`, 168
older ids simply absent from the pinned 15.25-era revision). It reuses
`engine_items.py`'s already-factored classification helpers directly (`non_item_route`,
`classify_family_profile`, `immovable_non_item_route`, `resolve_wrap_target_profile`,
`resolve_dead_item_route_or_profile`, the fluid/late-placeholder/no-appearance/
empty-object last-resort routes, `fallback_entry_matches_name`) in the exact priority
order `convert_item` uses -- `engine_items.py` itself is not forked or modified.

**No identity is minted.** These ids have no CW2 B1 allocator key (`build_identity_index`
never resolves one for them) and none is created here: identity allocation for donor
ids is a separate, reviewed Content/World step (B1b). Rows are keyed by a provisional,
clearly non-canonical string -- `donor:crystalserver@00ce02a5:item/<id>` -- never
`oteryn:item.registry.*`. Because the committed wiki-evidence snapshot and the owner
leftover-family table (§5b, §5i) are both keyed by Oteryn registry key, neither can
ever match a donor-only id; the join is still attempted, exactly where
`convert_item` would attempt it, for structural fidelity, but is guaranteed to return
zero matches by construction, not by omission. Recapturing the wiki for these ids,
once B1b assigns them identity, is B2's job; this tool never touches the network.
Delivery-task eligibility, field mapping and Presentation binding are all out of scope
too (all need identity); this is a family-classification census only.

**Never re-classifies an existing id.** `donor_census.py` computes only the id
set-difference (donor items.xml minus the pinned base items.xml); an id present in
both is never touched, and a wrap-target lookup that happens to resolve through a
shared id always prefers the pinned base's own record over a re-fetched donor copy.
Proven by keeping both pinned population censuses (`samples/population-{crystal,
canary}-*.json`) byte-identical -- verified via `population_census.py --check` for
both engines, unaffected because `engine_items.py` is untouched by this task.

The committed output, `samples/donor-census-crystal-summer-update-00ce02a5.json`
(schema `OTERYN_ITEM_DONOR_CENSUS/v1`, deterministic, `--check`/`--self-check` like
`population_census.py`), classifies all 412 new ids: 261 resolved (257 by engine
attribute, 4 by wrap-target inheritance -- `decoration`, 127; `material_valuable`, 78;
`document`, 24; `weapon_melee`, 14; `equipment_armor`, 5; `weapon_magic`/
`weapon_distance`, 4 each; `plant`/`light_source`, 2 each; `food`, 1), 138 routed
non-Item (`WorldObject:corpse` 80, `WorldObject:immovable_unclassified` 24,
`Terrain:ground_or_border` 16, `Terrain:primarytype_world_object` 14,
`WorldObject:primarytype_world_object` 4), and 13 `family_profile_unresolved` for lack
of any evidence this task is scoped to supply (no wiki evidence possible without
identity; no engine attribute, routing rule or owner-table entry applies): `sample of
bluish tide veil` (53692), `sample of bluish whisper reed` (53693), `lunar ascension
orb` (53695), `empty crystal flask` (53696), `shell gauge` (53783), `key` (54262),
`moonsilver crystals` (54267), `auric moon sigil` (54480), `crystal flask with blue
lava` (54564), `crystal flask with blessed blue lava` (54566), `skewered fish` (54638),
`scraps of a radiant attire` (54640), `cloud in a bottle` (54651) -- B2's wiki
recapture, once these ids have identity (B1b), is expected to resolve some of these.

**Follow-ups, not implemented here:** B1b (Content/World, reviewed separately) mints
CW2 B1 allocator identity for the 412 donor ids that should become real Oteryn Items;
B2 recaptures wiki evidence for those ids once identified; B3 (out of this task's
scope entirely) folds the donor facts into the pinned engine revision once B1b/B2
land, so a future `population_census.py` run covers them natively.

## 6. Validation and non-claims

The schema validator checks:

- closed JSON shape and explicit units;
- required boolean Delivery Task eligibility and exact evidence/default binding;
- exact dependency and asset closure;
- exact Presentation dependency closure, geometry/sprite-count consistency and
  preservation of ordered duplicate sprite IDs;
- pinned proficiency source crosswalks for every `profile_binding`;
- membership of every proficiency crosswalk and exact target in the pinned admitted
  source-to-target index; Magic Sword `238`/`3` requires both exact Canary and Crystal
  sources and exact equality with the admitted three-level payload;
- unique contiguous proficiency selection slots, bounded selection counts and typed
  values for every selectable perk;
- canonical rational values and bounded percentages;
- range/order and equipment-hand invariants;
- capability uniqueness and direct self-reference rejection;
- source-field disposition and mapped JSON Pointer resolution;
- exact equality between each declared source-field inventory and its dispositions;
- the protected 71-field Wiki disposition registry, including reverse/external owners;
- the exact 84-row historical Fandom registry and both 143-key engine registries;
- the exact 14-field BR and 5-field Fandom real-page supplements over twelve pinned
  page revisions without modifying the base catalogs;
- exact allowed formal JSON Pointer patterns for every mappable source field;
- pinned source identity/revision and explicit registered-but-ineffective keys;
- exact engine field origin (`items.xml`, `appearances.dat` or Crystal `bags.xml`) and
  the pinned SHA-256 of each cited engine artifact;
- RFC 3339 `date-time` values (`rfc3339-validator` is a required dependency; the
  validator refuses to run without it) and duplicate JSON object keys;
- canonical two-decimal weights, signed percentages within -100..100 and equipment
  slot reservations (never the own slot; the other hand only when two-handed);
- value-dependent owner routing and normalized destination values for type, event,
  weapon action/kind, signed weight and inverse movement flags;
- timezone-qualified capture timestamps, per-capture SHA-256 and the pinned Fandom
  revision SHA-1 as separate provenance facts;
- exact leaf-level real-example Item/evidence/default partition (every authored leaf,
  including presentation leaves other than the appearance binding and empty
  containers), type-exact normalized values, effective source-field routing, engine and
  Wiki pins, canonical Item key of the bound TibiaWiki page, Presentation
  geometry/sprites and blocker sets;
- fail-closed `unresolved_semantics`, `unsupported_source_field` and `conflict` states.

This candidate does not:

- migrate the 38,157 Item corpus;
- supersede WorldProject/v2 serialization;
- activate runtime Item behavior;
- claim Tibia Global parity;
- make Wiki text executable;
- authorize production or Merge Queue changes.

Runtime lowering and corpus migration require separate accepted slices with exact-head
evidence.
