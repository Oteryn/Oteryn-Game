# Oteryn WorldProject Source Profile v2

Status: IMPLEMENTED SOURCE/AUTHORING CONTRACT

Profile ID: `OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v2`

This profile is the canonical declarative authoring successor to
`OTERYN_WORLD_PROJECT_SOURCE_PROFILE/v1`. V1 remains supported and unchanged.
V2 does not create a second executable gameplay model: already accepted executable
families continue to reuse the existing Reference model, while authoring families
without an accepted runtime family remain declarative and fail closed if a caller
attempts to treat them as executable Reference content.

## 1. Purpose

WorldProject/v2 provides one bounded source carrier for the full intended Oteryn
Content/World authoring surface:

- existing executable Reference records (Ability, Effect, Formula, Item, Creature,
  Terrain, LocalObject, Loot, Presentation, Behavior);
- NPC, Dialogue, Service and Interaction declarations;
- Quest, House and Encounter declarations;
- authored world placements and transitions;
- presentation appearance bindings, aliases and tags;
- asset, provenance and editor roles;
- modern Item authoring extensions that do not fit the accepted v1 Item artifact
  without changing its runtime semantics.

The project is declarative source. Runtime legality, persistence, mutable item state,
economy transactions and authoritative gameplay mutation remain owned by their
accepted domain contracts.

## 2. Compatibility and authority

V2 preserves these v1 invariants:

- stable namespaced keys are semantic identity; paths are not identity;
- external numeric IDs, wiki page IDs, sprite IDs and donor names are provenance,
  never canonical identity;
- unknown/conflict values are not coerced to zero/false;
- client-safe projection is allowlisted and non-authoritative;
- executable runtime definitions are not inferred from declarative records;
- imports/evidence do not automatically become gameplay truth.

A v2 project may embed an existing `ProjectReferenceRecord` for a family already
accepted by the Reference linker. That record is the sole executable representation
for that accepted family. V2 metadata and declarations may point at it, but do not
silently widen its runtime semantics.

## 3. Project roles

A v2 manifest may assign these typed roles. Locators are organizational and may be
sharded without changing object identity.

```text
project.json
manifest.json
content.lock.json

definitions/
  reference/        # accepted executable ProjectReferenceRecord records
  items/            # v2 Item authoring overlays bound to exact Item keys
  npcs/
  dialogues/
  services/
  interactions/
  quests/
  houses/
  encounters/
  proficiencies/
  augments/

worlds/
  placements/
  transitions/

presentations/
assets/
provenance/
editor/
imports/
metadata/
```

Role families are closed in the v2 schema. Unknown roles fail closed.

## 4. Definition/runtime split

```text
WorldProject/v2 declarative definition
        |
        +-- executable Reference record (only accepted families)
        |
        +-- authoring metadata / relationship graph / source evidence
             |
             +-- later typed linker/runtime owner may consume it
             +-- otherwise remains declarative
```

Examples:

- NPC prices are Service offers, not Item fields.
- `droppedby` is reverse discovery evidence for Creature -> Loot relationships,
  not an Item field.
- an Item `on use` behavior is a typed Interaction binding; the Interaction may
  bind to an accepted Ability/Effect or a separately accepted native rule.
- mutable Forge tier is ItemInstance/DUR state; the Item definition stores only
  immutable Forge eligibility/profile metadata.
- proficiency XP, unlocked/selected perks and ranks are character/progression
  state. WorldProject stores the immutable weapon proficiency profile and perk
  definitions only.

## 5. TibiaWiki Item audit

The current TibiaWiki `Infobox_Item` vocabulary includes:

```text
name, skillboost, flavortext,
itemclass, primarytype, secondarytype, tertiarytype,
weight, value, npcvalue, npcprice,
droppedby, droppedRaidby, droppedEventby, buyfrom, sellto,
implemented, removed, notes, attrib,
stackable, enchantable, enchanted,
edible, regenseconds,
writable, writechars,
levelrequired, vocrequired,
attack, defense, defensemod,
elementattack, elemental_bond, resist,
hands, type, range, armor, charges, readable, duration,
destructible, damage, damagetype, mana, hit, volume
```

TibiaWiki/Cyclopedia coverage also exposes contemporary systems outside the
historic infobox vocabulary:

- Exaltation Forge `Classification` and `Max. Tier`;
- Weapon Proficiency levels and per-level perk trees;
- Summer Update 2026 Perk Shaping, Refine (through rank 10), Reshape, Clear and
  Lunar Ascension;
- spell-specific proficiency augments;
- rune spell/required-magic-level relationships;
- Elemental Bond on Monk weapons.

TibiaWiki is structured reference evidence only. V2 defines destinations; it does
not promote those values by itself.

## 6. Item field routing

### 6.1 Existing Reference Item semantics

These map to the existing accepted Item model and do not require a new v2 runtime
field:

| Source concept | Canonical destination |
|---|---|
| name | Item presentation.name |
| flavortext / Cyclopedia description | Item presentation.description |
| weight | Item physical.weight |
| stackable | Item stack.stackable |
| levelrequired / vocrequired | equipment pattern requirements |
| hands / body slot | equipment patterns |
| attack / defense / defensemod | weapon |
| elementattack | weapon.elemental |
| elemental_bond | skill modifier ElementalBond |
| hit / range | weapon hit/range |
| armor / protection / resist | protection |
| skillboost | skill modifiers |
| charges | charges |
| duration | temporal |
| volume | container.capacity |
| readable / writable / writechars | readable/writeable |
| marketable | trade restrictions |
| imbuement slots/families | imbuement |

The TibiaWiki collector may need widening for fields that already have a canonical
destination; that is an importer concern, not a new Item model.

### 6.2 V2 Item authoring overlay

WorldProject/v2 adds a declarative Item overlay bound to an exact existing Item
identity:

```text
ItemAuthoringV2
  item: exact Item ref

  presentation:
    appearance: optional Asset ref
    aliases[]
    tags[]
    taxonomy[]          # source/editor taxonomy, incl. primary/secondary/tertiary

  forge:
    classification      # immutable definition metadata
    max_tier            # immutable definition metadata
    effect_binding?     # typed declarative Effect/Formula binding
                        # current tier is NOT stored here

  proficiency_profile?  # typed ProficiencyDefinition ref

  augment_bindings[]    # typed AugmentDefinition refs

  on_use_interactions[] # typed InteractionDefinition refs

  use_requirements:
    required_magic_level?

  lifecycle:
    enchantable?
    destructible?
    enchant_interactions[]
    destroy_interactions[]

  source_lifecycle:
    implemented?
    removed?
```

`enchanted` is deliberately not a mutable boolean on ItemDefinition. If the
source distinguishes base and enchanted forms, represent the forms as exact
definitions/states and bind the transition through a typed Interaction/transform.

## 7. Exaltation Forge

Immutable definition metadata belongs in Item authoring:

```text
ItemForgeProfileV2
  classification: 1..4
  max_tier: bounded positive tier
  effect_binding?: Effect/Formula ref
```

The current tier of a concrete value-bearing item is mutable ItemInstance state
and remains outside WorldProject.

V2 does not hard-code a Global-Tibia tier table as Oteryn truth. A Reference import
may retain the observed class/max pair with provenance. Oteryn may later declare a
different authored policy explicitly.

## 8. Weapon Proficiency and Perk Shaping

A proficiency profile is its own reusable declarative definition:

```text
ProficiencyDefinitionV2
  identity
  weapon_item: Item ref
  levels[]
    level
    perks[] -> Augment refs
  shaping?
    max_rank
    replace_slots
    refine_enabled
    reshape_enabled
    clear_enabled
    lunar_ascension_enabled
    cost_service? -> Service ref
```

An augment is typed and may target an ability, an auto-attack/rune family or
another accepted target domain:

```text
AugmentDefinitionV2
  identity
  target:
    ability -> Ability ref
    auto_attack
    offensive_rune
    creature_class <stable authored key>
    generic <closed authored target key>
  effect -> Effect ref
  rank_values[] -> exact rational/signed values
```

Spell-specific proficiency augments therefore bind to the exact Ability identity;
they are not free-form strings or scripts.

Mutable proficiency progress, selected shaped perks and their current ranks are
not WorldProject definition state.

## 9. Item use, consumables, foods, catalysts, tools and quest items

V2 does not add an arbitrary `on_use_script` field.

Items bind to typed Interaction definitions. An Interaction has a closed trigger
and typed execution binding:

```text
InteractionDefinitionV2
  identity
  trigger:
    USE | USE_WITH | READ | WRITE | ROTATE | WRAP | UNWRAP |
    EQUIP | DEEQUIP | PICKUP | MOVE | STEP_IN | STEP_OUT |
    ADD_ITEM | REMOVE_ITEM | OPEN | CLOSE | OPEN_CONTAINER |
    SLEEP | TELEPORT | FLOOR_TRANSITION | HANG | NPC_TALK |
    SERVICE | QUEST_TRIGGER
  execution:
    Ability ref
    Effect ref
    Service ref
    Quest ref
    Transform Item -> Item
    NativeRule key
```

This covers food/regeneration, proficiency catalysts, potions, tools and quest
objects without embedding runtime scripts in the Item record. The exact runtime
owner must still accept the referenced execution family before it can execute.

For runes/wands/rods, damage/damage type/mana/spell facts should bind to the
appropriate Ability/Effect/Formula definitions rather than being duplicated as an
independent Item damage engine.

## 10. NPC, Service and Loot relationship routing

TibiaWiki fields `value`, `npcvalue`, `npcprice`, `buyfrom`, `sellto` and
Market/NPC tables are not immutable Item combat semantics.

V2 routes them as:

```text
NpcDefinition
  -> ServiceDefinition
       -> offers[]
            item: Item ref
            direction: BUY_FROM_PLAYER | SELL_TO_PLAYER
            price/currency rule
```

`droppedby`, `droppedRaidby` and `droppedEventby` are discovery/provenance
signals for:

```text
CreatureDefinition -> LootDefinition -> Item ref
Encounter/Event      -> LootDefinition -> Item ref
```

Reverse lists may be generated for tooling/UI but are not authored Item truth.

## 11. NPC / Dialogue / Service

V2 declares stable definitions separately:

```text
NpcDefinition
  identity
  presentation?
  behavior?
  dialogues[]
  services[]

DialogueDefinition
  identity
  nodes[]
  edges[]

ServiceDefinition
  identity
  kind
  offers[]
  execution bindings
```

Dialogue text and service/economy behavior are separate. A shop update does not
rewrite NPC movement or dialogue identity.

## 12. Quest

V2 uses stable semantic quest identities, never raw legacy storage arithmetic:

```text
QuestDefinition
  identity
  stages[]
  objectives[]
  transitions[]
  rewards[]
```

Conditions and execution are typed references/bindings. Mutable quest progress is
durable character/account state, not WorldProject.

## 13. World placements and transitions

V2 declares authored placements separately from definitions:

```text
WorldPlacementV2
  placement_key
  definition ref
  world
  x / y / floor
  presentation/state hint where allowed

WorldTransitionV2
  transition_key
  from world/position
  to world/position
  interaction ref?
```

Moving a placement may preserve PlacementKey; copying creates a new PlacementKey.
Filesystem or shard movement never changes identity.

## 14. Houses and Encounters

House and Encounter are declarative families in v2. Their mutable/live state stays
with the owning runtime/durability domain.

```text
HouseDefinitionV2
  identity
  world/zone refs
  access-policy binding
  entry/exit refs

EncounterDefinitionV2
  identity
  participants/spawn refs
  trigger interactions
  loot/reward refs
  reset policy binding
```

## 15. Presentation, assets, aliases and tags

V2 closes the v1 Item presentation gaps:

- appearance bindings point to Asset records;
- aliases are bounded editor/search/localization aliases and never identity;
- tags/taxonomy are bounded authoring metadata and never runtime capabilities
  unless an explicit compiler mapping promotes them;
- client presentation remains a separate allowlisted projection.

## 16. Provenance and source lifecycle

V2 provenance can bind evidence to an exact entity and field selector:

```text
ProvenanceRecordV2
  entity ref
  field selector
  value digest
  source id
  source revision/page revision
  target cut
  observation classification
  observed implementation/removal window
```

TibiaWiki `implemented`/`removed` belongs here unless Oteryn explicitly authors
a gameplay availability rule.

Notes/spoilers are source/editor material, not authoritative gameplay semantics.

## 17. Explicitly excluded mutable state

WorldProject/v2 definitions MUST NOT contain:

- current Forge tier of a concrete item;
- active imbuements or remaining timers;
- concrete stack quantity, charges or durability;
- character proficiency XP/level/unlocked shaped perk selection/ranks;
- current container contents/custody/location;
- live quest progress;
- live NPC conversation state;
- live encounter state;
- transaction/retry ownership.

Those belong to ItemInstance, Character/Progression, Quest/DUR, runtime or other
accepted owners.

## 18. Acceptance invariants

The executable implementation must prove:

1. v1 profile IDs and parser/writer remain unchanged.
2. V2 record families are closed and unknown members fail closed.
3. Duplicate definition/placement/transition identities reject.
4. Typed references validate family compatibility.
5. An Item overlay references exactly one existing Item identity.
6. Item authoring contains no fields for current Forge tier or mutable proficiency
   progress.
7. NPC/shop/drop relationships cannot be serialized as Item semantic fields.
8. An unsupported declarative family cannot silently become executable Reference
   content.
9. Existing `ProjectReferenceRecord` remains the single runtime-lowerable record
   for accepted families.
10. Appearance/aliases/tags/taxonomy and modern Item profiles round-trip
    deterministically through the v2 source carrier.
