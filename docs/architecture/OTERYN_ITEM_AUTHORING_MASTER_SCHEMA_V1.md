# Oteryn Item Authoring Master Schema v1

- Date: 2026-09-25
- Status: CANDIDATE / schema-census complete; no runtime or WorldProject storage supersession by this document
- Task: `OTV2-20260925-tibiawiki-item-master-schema-v1`
- Parent control plane: #162
- Programme: KAN-16 / #504
- Admission main: `2389c6671000b8b0efe341540a62e303e307ad15`
- Machine evidence: `docs/agents/evidence/OTV2-20260925-tibiawiki-item-master-field-census-v1.json`

## 1. Outcome

Oteryn Item authoring uses one **superset Item schema composed from typed capabilities**.
It does not create one incompatible schema per Wiki category and it does not use XML,
client, server or MediaWiki numeric identifiers as gameplay identity.

A family profile answers which capability groups are normally applicable. It does not
erase valid exceptional combinations. An item may therefore be a weapon and a light
source, an accessory and a timed item, a quest item and a readable document, etc.

The current TibiaWiki `Infobox Item` source exposes 71 top-level parameters:
67 content/source parameters plus 4 template controls. The current `Predefinição:Itens`
navigation exposes 50 principal Item families. The accompanying census assigns every
parameter and every family; `unassigned_fields=0` and `unassigned_families=0`.

## 2. Identity

Canonical identity is Oteryn-owned and stable:

```text
item:<stable-oteryn-key>
```

A human-readable slug may be used when unique and intentionally selected, but display
name is not identity and a later rename must not break references.

These values are **never** canonical Item identity:

- TibiaWiki page id or revision id;
- XML/server numeric item id;
- client appearance id;
- file path;
- source row order;
- display name by itself.

Source identifiers may be retained only in importer/provenance data for reimport,
diagnostics and audit.

## 3. Master Item shape

The authoring model is intentionally readable. Unknown/absent capabilities are omitted;
the source ingestion/evidence layer may separately retain UNKNOWN/CONFLICT states.

```text
Item
├── identity
│   ├── id
│   ├── name
│   └── aliases[]
├── presentation
│   ├── flavor_text
│   ├── sounds[]
│   └── variants[]
├── taxonomy
│   ├── item_class
│   ├── primary
│   ├── secondary
│   ├── tertiary
│   └── tags[]
├── physical
│   ├── weight
│   ├── movable
│   └── pickupable
├── stack
│   ├── stackable
│   └── max_count
├── requirements
│   ├── min_level
│   ├── vocations[]
│   └── context
├── equipment
│   ├── slot
│   ├── hands
│   ├── reserved_slots[]
│   └── groups[]
├── weapon
│   ├── weapon_type
│   ├── ammunition
│   ├── attack
│   ├── defense
│   ├── extra_defense
│   ├── range_cells
│   ├── hit_chance
│   ├── max_hit_chance
│   └── elemental_attack[]
├── protection
│   ├── armor
│   └── resistances[]
├── modifiers
│   ├── skill_boost[]
│   ├── mantra
│   ├── elemental_bond
│   └── misc[]
├── charges
│   └── count
├── temporal
│   ├── duration
│   ├── consumption_mode
│   └── decay_target
├── container
│   └── capacity
├── imbuement
│   ├── slot_count
│   ├── allowed_family_tiers[]
│   └── excluded_families[]
├── forge
│   ├── classification
│   └── max_tier
├── proficiency
│   ├── levels[]
│   ├── perks[]
│   ├── shaping
│   └── augments[]
├── consumable
│   ├── edible
│   └── regeneration_seconds
├── fluid
│   └── fluid_type
├── readable
│   ├── readable
│   ├── writable
│   ├── max_characters
│   └── document_binding
├── light
│   ├── emits
│   ├── color
│   ├── intensity
│   ├── radius
│   ├── when_equipped
│   └── toggleable
├── bed
│   └── sleepable
├── use
│   ├── usable
│   ├── use_with
│   ├── interactions[]
│   ├── ability
│   ├── damage
│   ├── damage_type
│   └── mana_cost
├── lifecycle
│   ├── enchantable
│   ├── enchanted_variant
│   ├── destructible
│   └── transforms[]
├── trade
│   └── marketable
├── source_observations
│   └── attributes_text
└── editor
    ├── notes
    └── estimated_value
```

The model reuses the existing protected Item owners whenever they already exist:
`ReferenceItemSemantics` remains the executable owner for presentation/classification,
physical/stack/equipment/weapon/protection/modifiers/charges/temporal/container/
imbuement/use-transform/trade/fluid/readable semantics. WorldProject/v2 authoring
already owns taxonomy, Forge, proficiency/augments, consumable facts, use observations
and lifecycle/source-lifecycle facts.

The schema-census exposes additional authoring concepts that are not yet first-class
executable Item fields, especially typed light emission/toggle facts, sleepable beds,
generic usable/use-with facts and presentation-variant authoring. They remain
candidate-only until their owning implementation slice is accepted.

## 4. Complete TibiaWiki field disposition

The machine-readable census is normative for the source-to-authoring mapping. Its
disposition classes are:

- `ITEM_TYPED` — fits an existing typed Item semantic owner;
- `ITEM_AUTHORING` — static authoring data retained without granting execution;
- `RELATIONSHIP` — belongs on the other side of a typed relation, not duplicated as
  Item truth;
- `PRESENTATION_EDITOR` — display/editor metadata only;
- `PROVENANCE` — source lifecycle/audit metadata;
- `EXTERNAL_DOMAIN` — economy/commercial/system value owned outside Item;
- `SOURCE_TEXT_PRESERVE_AND_PARSE` — retain exact bounded source observation and
  promote recognised typed facts; never execute the text;
- `TEMPLATE_CONTROL` — MediaWiki rendering/query control, never content.

### Reverse relations

The following Wiki fields are useful discovery inputs but must not become intrinsic
Item arrays:

```text
droppedby       -> Creature/Loot -> Item
droppedRaidby   -> Encounter/Loot -> Item
droppedEventby  -> Encounter/Loot -> Item
buyfrom         -> NPC Service -> Item offer
sellto          -> NPC Service -> Item offer
npcprice        -> NPC Service sell-to-player price
npcvalue        -> NPC Service buy-from-player price
taskitem        -> Task/Objective -> Item
```

### External/non-Item values

`dromevalue`, `htaskvalue`, `storevalue` and `tournamentvalue` remain with
their economy/commercial owners. Wiki `value` is editor/community estimate only and
cannot be authoritative value-conservation state.

## 5. Free-form `attrib` is a required lossless seam

The Wiki template carries important facts only through `attrib`. Current examples
include light emission, item use, toggling, sleepable beds, transformations and
teleports. Therefore dropping `attrib` after parsing known fields would lose content.

Policy:

1. retain the bounded exact source observation in
   `source_observations.attributes_text`;
2. recognise independently testable clauses and promote them into typed authoring
   capabilities;
3. keep unparsed clauses visible and non-executable;
4. never let raw Wiki prose become runtime logic.

Known promotion targets include:

- `light.*` for light source/color/intensity/radius/equipped/toggle facts;
- `use.*` for usable/use-with/Interaction facts;
- `bed.sleepable`;
- `presentation.variants` and `lifecycle.transforms`;
- redundant confirmations of stack/hands/enchant/destructible/read/write/duration.

This closes the representability gap without inventing a generic free-form gameplay
attribute bag.

## 6. Family profiles

The current Wiki top-level families are assigned to 22 semantic profiles in the
machine census. Profiles describe expected capabilities, not separate schemas.

Examples:

```text
Capacetes / Botas / Armaduras / Calças
  -> equipment_armor

Machados / Clavas / Espadas / Punhos
  -> weapon_melee

Distância / Munição
  -> weapon_distance

Wands / Rods
  -> weapon_magic

Livros / Documentos e Papéis
  -> document

Fontes de Luz
  -> light_source

Comidas
  -> food

Líquidos
  -> fluid

Chaves
  -> key

Ferramentas / Ferramentas de Cozinha / Itens de Domar
  -> tool

Itens de Quest
  -> quest_item
```

Every family permits additional typed capabilities when actual evidence requires them.
Category membership never grants behavior.

## 7. Human-readable Item files

The master schema is independent of the final physical WorldProject migration, but the
intended human authoring surface is simple:

```json
{
  "id": "magic-sword",
  "name": "Magic Sword",
  "taxonomy": {
    "item_class": "equipment",
    "primary": "sword"
  },
  "physical": {
    "weight": {"value": "42.00", "unit": "oz"}
  },
  "equipment": {
    "slot": "weapon",
    "hands": 1
  },
  "weapon": {
    "weapon_type": "sword",
    "attack": 48,
    "defense": 35
  },
  "imbuement": {
    "slot_count": 2
  }
}
```

Importer coordinates such as a MediaWiki page id stay outside this file.

## 8. Acceptance and next step

This slice is complete when repository validation proves:

- exact expected current template parameter set is present;
- every parameter appears once in the mapping;
- no unknown disposition exists;
- every current top-level Wiki Item family is assigned exactly one semantic profile;
- `unassigned_fields=0`;
- `unassigned_families=0`.

This document does **not** itself migrate the existing 38,157 Item corpus or supersede
the published WorldProject/v2 physical contract.

After this candidate is protected, the next bounded design/implementation slice can
use the closed master schema to define the physical content-tree migration and then
mechanically transform the existing Item corpus without losing canonical Oteryn
identity or typed semantics.
