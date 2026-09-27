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
Sword (page `5810`) is `oteryn:item.registry.i00003167`. The other five pages have no
binding yet, so their example keys are provisional and they carry the
`canonical_item_identity_not_bound` blocker until the Item identity binding covers them. Magic Sword now carries its admitted exact Proficiency
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
