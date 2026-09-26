# Oteryn Item Authoring Formal Schema v1

- Date: 2026-09-26
- Status: CANDIDATE; authoring/evidence contract only
- Parent master: `OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md`
- Task: `OTV2-20260926-item-authoring-formal-schema-v1`
- Parent control plane: #162
- Programme: KAN-16 / #504
- Admission main: `e300c14102f0e78be147d380779cf1265e76a182`
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
    "key": "oteryn:item.weapon.sword.magic",
    "revision": "definition-r1"
  },
  "display_name": "Magic Sword",
  "family_profile": "weapon_melee",
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

Every definition reference carries `{family,key,revision}`. Asset bindings use an
Oteryn namespaced key. `item-dependencies.json` closes those exact references for local
validation without embedding Ability, Effect, Interaction, Document or ItemInstance
payloads in the Item.

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

Canary does not expose a separate light radius at the pinned appearance revision, so
`light.radius_cells` is optional evidence rather than a condition of `emits=true`.
Canary proficiency augments lower into the existing typed WorldProject/v2 augment
shape (target, optional Effect and ranked typed values); free-form augment strings are
not admitted.

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

### Wiki BR and Fandom

The exact TibiaWiki BR infobox inventory remains the source-field census authority:

- `https://www.tibiawiki.com.br/index.php?stableid=424807&title=Predefini%C3%A7%C3%A3o%3AInfobox_Item`

Fandom family/project material is corroborating taxonomy and authoring-shape evidence,
not runtime authority:

- `https://tibia.fandom.com/wiki/TibiaWiki:Projects/Merge_Items_and_Objects`

The comparison confirms the need for `requirements.min_magic_level`, explicit readable
write policy and container content constraints. It also confirms that community value,
drops, NPC offers, quest membership and source numeric IDs must not become intrinsic
portable Item truth.

## 4. Boundary table

| Source concept | Formal owner |
|---|---|
| Stable portable stats/capabilities | `item.json` |
| Exact referenced definitions/assets | `item-dependencies.json` |
| Source URL/revision/digest and per-field disposition | import-readiness manifest |
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
- exact dependency and asset closure;
- canonical rational values and bounded percentages;
- range/order and equipment-hand invariants;
- capability uniqueness and direct self-reference rejection;
- source-field disposition and mapped JSON Pointer resolution;
- exact equality between each declared source-field inventory and its dispositions;
- the protected 71-field Wiki disposition registry, including reverse/external owners;
- exact allowed formal JSON Pointer patterns for every mappable Wiki field;
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

