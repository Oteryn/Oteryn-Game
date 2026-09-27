# Oteryn Full Game Content & Ruleset Tree v1

- Date: 2026-09-25
- Status: CANDIDATE
- Task: `OTV2-20260925-full-game-content-ruleset-tree-v1`
- Parent control plane: #162
- Programme: KAN-16
- Admission main: `2389c6671000b8b0efe341540a62e303e307ad15`
- Machine contract: `docs/agents/evidence/OTV2-20260925-full-game-content-ruleset-tree-v1.json`
- Item detail companion: PR #903 (`OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1` candidate)
- Creature detail companion: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1` candidate

## 1. Decision

Use one human-readable **game-data tree** with explicit semantic ownership instead of
treating every TibiaWiki surface as one undifferentiated `content` blob.

The tree has five distinct concerns:

1. **Content** — static authored identities, definitions, relationships and world occurrences.
2. **Rulesets** — reusable gameplay/progression/economy rules.
3. **Durable/runtime state** — current mutable player/item/world/service state; never static content.
4. **Commercial** — Store/premium/purchase truth owned by Oteryn Platform.
5. **Imports/provenance** — external source coordinates and reimport evidence; never gameplay identity.

The protected wiki-wide coverage already closed all 19 discovered structured domain groups
with `unclassified=0`. This successor tree makes those owners readable in the filesystem
and adds explicit owner placement for current gameplay/commercial system surfaces.

This document does **not** move current `content/world/**` yet. Migration is a separate
qualified slice so the existing WorldProject/v2 corpus remains readable until exact
semantic round-trip is proven.

## 2. Target Oteryn-Game authoring tree

```text
Oteryn-Game/
│
├── content/
│   ├── project.json
│   ├── manifest.json
│   ├── content.lock.json
│   │
│   ├── items/
│   │   ├── definitions/
│   │   ├── taxonomy/
│   │   └── relations/
│   │
│   ├── loot/
│   │
│   ├── creatures/
│   │   ├── definitions/
│   │   ├── bestiary/
│   │   ├── bosstiary/
│   │   └── familiars/
│   │
│   ├── npcs/
│   │   └── definitions/
│   │
│   ├── dialogues/
│   │
│   ├── services/
│   │   ├── trade/
│   │   ├── travel/
│   │   ├── bank/
│   │   ├── blessings/
│   │   ├── tasks/
│   │   └── crafting/
│   │
│   ├── abilities/
│   │   ├── definitions/
│   │   ├── effects/
│   │   └── formulas/
│   │
│   ├── quests/
│   │   ├── definitions/
│   │   ├── missions/
│   │   ├── objectives/
│   │   └── rewards/
│   │
│   ├── achievements/
│   ├── charms/
│   │
│   ├── cosmetics/
│   │   ├── outfits/
│   │   └── mounts/
│   │
│   ├── documents/
│   ├── interactions/
│   ├── behaviors/
│   │
│   ├── encounters/
│   │   ├── bosses/
│   │   ├── raids/
│   │   ├── world-changes/
│   │   ├── mini-world-changes/
│   │   ├── world-quests/
│   │   ├── arenas/
│   │   └── tibiadrome/
│   │
│   ├── world/
│   │   ├── worlds/
│   │   ├── areas/
│   │   │   ├── cities/
│   │   │   ├── islands/
│   │   │   ├── regions/
│   │   │   ├── hunting-places/
│   │   │   └── streets/
│   │   ├── terrain/
│   │   ├── objects/
│   │   ├── transitions/
│   │   └── placements/
│   │
│   ├── houses/
│   │
│   ├── presentations/
│   │   ├── definitions/
│   │   └── bindings/
│   │
│   └── assets/
│       └── catalog/
│
├── rulesets/
│   ├── character/
│   │   ├── vocations/
│   │   ├── skills/
│   │   ├── experience/
│   │   ├── stamina/
│   │   ├── soul/
│   │   ├── death/
│   │   └── blessings/
│   │
│   ├── combat/
│   │   ├── damage/
│   │   ├── conditions/
│   │   └── cooldowns/
│   │
│   ├── party/
│   │   └── shared-experience/
│   │
│   ├── pvp/
│   │   ├── skulls/
│   │   ├── wars/
│   │   └── arena/
│   │
│   ├── social/
│   │   └── marriage/
│   │
│   ├── entitlements/
│   │   └── premium-gates/
│   │
│   ├── items/
│   │   ├── imbuements/
│   │   ├── exaltation-forge/
│   │   ├── enchanting/
│   │   ├── doomforging/
│   │   ├── charging-system/
│   │   ├── crystal-shield/
│   │   └── umbral-creation/
│   │
│   ├── progression/
│   │   ├── bestiary/
│   │   ├── bosstiary/
│   │   ├── charms/
│   │   ├── prey/
│   │   ├── wheel-of-destiny/
│   │   ├── gem-atelier/
│   │   ├── weapon-proficiency/
│   │   └── loyalty/
│   │
│   ├── encounters/
│   │   ├── tibiadrome/
│   │   ├── hazard-system/
│   │   ├── soulpit/
│   │   ├── holy-shrines/
│   │   └── taints/
│   │       ├── bakragore/
│   │       └── goshnar/
│   │
│   └── economy/
│       ├── currencies/
│       ├── npc-trade/
│       ├── crafting-upgrades/
│       └── market/
│
└── imports/
    ├── official/
    ├── tibiawiki/
    ├── legacy/
    ├── crystalserver/
    └── canary/
```

## 3. Content ownership

### Items

`content/items/definitions/` owns what an Item **is**.

Examples include:

- display identity and aliases;
- taxonomy;
- physical/stack/equipment/weapon/protection/modifier properties;
- charges/duration/container capabilities;
- readable/fluid/use relations;
- imbuement slot capability;
- Forge classification/max tier;
- static Weapon Proficiency capability/augment declarations;
- presentation/document/Ability/Interaction references.

It does not own current stack count, remaining charges, active imbuements, current Forge
tier, current enchantment or container contents. Those are ItemInstance/durable state.

The detailed Item field model is owned by the Item Master Schema companion.

### Creatures, loot, Bestiary and Bosstiary

```text
content/creatures/definitions/
    what the Creature is

content/loot/
    reusable loot tables

content/creatures/bestiary/
    static difficulty / kill thresholds / charm-point facts

content/creatures/bosstiary/
    static boss classification / thresholds / points
```

Current player kill counters, unlocks and Bosstiary reward eligibility are durable
Character progression, not files in these directories.

### NPC, Dialogue and Service

NPC identity is separate from business/service definitions:

```text
NPC -> Dialogue
NPC -> Service
Service -> Item/currency/Interaction
```

Prices are forward Service offers. Item files must not contain authoritative reverse
`sold_by` / `bought_by` arrays.

### Ability / Effect / Formula

```text
Ability -> Effect -> Formula
```

Runes reuse Item + Ability + Interaction rather than becoming a duplicate Rune
definition family.

### Quests

Static quest graph belongs to `content/quests/`. Current character mission state,
completion flags, cooldowns and reward-claim state remain Character/Reward durability.

### World / Areas / placements

Area identity covers cities, islands, regions, hunting places and other reusable authored
locations. World placements are occurrences, not reusable definition identities.

The current monolithic physical world-record layout is **not** declared full-world
scalable by this tree. Before bulk map migration the existing measured placement-scale
gate still applies.

## 4. Ruleset ownership

### Imbuements

```text
Item definition:
    imbuement slot capability

rulesets/items/imbuements/:
    family
    tier
    effect
    material requirements
    eligible item rules
    cost
    duration

ItemInstance:
    active imbuements
    remaining duration
```

### Exaltation Forge

```text
Item definition:
    Forge classification
    max tier

rulesets/items/exaltation-forge/:
    fusion rules
    transfer rules
    costs
    success/failure policy
    tier effects

ItemInstance:
    current tier
```

### Prey

```text
rulesets/progression/prey/:
    slot rules
    roll rules
    bonus types
    reroll rules
    costs

Character:
    current prey slots
    selected creature
    active bonus
    bonus level
    rerolls/current state
```

### Wheel of Destiny and Gem Atelier

```text
rulesets/progression/wheel-of-destiny/
    wheel graph
    vocation layouts
    perks
    point rules

rulesets/progression/gem-atelier/
    gem tiers
    mod vocabulary
    reveal/bind rules
    slot rules
    resonance/mod rules

content/items/definitions/
    physical/unrevealed gem Item definitions

Character:
    wheel allocations
    revealed/bound gems
    selected gem slots/mod state
```

### Weapon Proficiency

Static Item capability/perk declarations remain associated with Item definitions.
XP, unlocks, selected perks and ranks are Character progression state governed by
`rulesets/progression/weapon-proficiency/`.

### Bestiary / Bosstiary / Charms

Static identities/facts live in content; progress/unlock/assignment logic lives under
`rulesets/progression/`; the current player's counters and assignments are durable state.

### Modern encounter/item systems

Current structured system surfaces receive explicit homes:

```text
Doomforging        -> rulesets/items/doomforging/
Charging System    -> rulesets/items/charging-system/
Crystal Shield     -> rulesets/items/crystal-shield/
Enchanting         -> rulesets/items/enchanting/
Umbral Creation    -> rulesets/items/umbral-creation/

Hazard System      -> rulesets/encounters/hazard-system/
Soulpit            -> rulesets/encounters/soulpit/
Holy Shrines       -> rulesets/encounters/holy-shrines/
Bakragore's Taints -> rulesets/encounters/taints/bakragore/
Goshnar's Taints   -> rulesets/encounters/taints/goshnar/
Tibiadrome         -> content/encounters/tibiadrome/
                      + rulesets/encounters/tibiadrome/
```

## 5. Commercial / Store boundary

The **Store is not Game static content authority**.

Logical Platform-owned commercial tree:

```text
Oteryn-Platform/
└── commercial/
    ├── store/
    │   ├── categories/
    │   ├── products/
    │   ├── offers/
    │   ├── prices/
    │   └── availability/
    ├── premium/
    ├── entitlements/
    ├── purchases/
    └── receipts/
```

This is a logical ownership surface only; this Game document does not select the exact
Platform filesystem.

Game may define `rulesets/entitlements/premium-gates/` describing what an entitlement
permits inside gameplay. Platform owns payment truth, commercial price, purchase record
and grant authority.

## 6. Mutable/durable state — intentionally not authoring content

```text
ItemInstance
├── stack quantity
├── remaining charges
├── durability
├── active imbuements
├── remaining imbue duration
├── current Forge tier
├── current enchantment
├── container contents/custody
└── player-written document body/writer/timestamp

Character
├── quest progress/completions/cooldowns
├── achievement unlocks
├── Bestiary/Bosstiary counters
├── Charm points/unlocks/assignments
├── Prey slots/rolls
├── Wheel allocations
├── Gem Atelier revealed/bound gems/selections
├── Weapon Proficiency XP/unlocks/perks/ranks
├── stamina
├── soul
├── blessings
└── selected/unlocked cosmetics as owned by Character/Account contracts

World / Channel
├── active raid/event phase
├── World Change state
├── wave/timers
├── live NPC conversation/schedules
└── runtime encounter state

House
├── owner
├── rent payment state
├── ACL/access lists
└── inventory/custody

Market
├── orders
├── history
├── personal prices
└── settlement state

Platform Account / Commercial
├── premium/commercial entitlements
├── Store purchases
└── receipts
```

## 7. Current Wiki/system coverage

The machine contract binds all protected wiki-wide domain groups:

```text
items
documents
geography
creatures
bosses
familiars
npc
quests
abilities
achievements
charms
cosmetics
events
houses
player_systems
economy
presentation
provenance
wiki
```

It also explicitly assigns current selected gameplay/commercial surfaces including:

```text
Store / Market / Outfitter
Raids / Invasions
Marriage
PvP Arena / Skull System / Guild War System
Loyalty
Imbuements / Exaltation Forge
Prey
Wheel of Destiny / Gem Atelier
Bestiary / Bosstiary / Charms
Weapon Proficiency
Stamina / Soul / Blessings
World Changes / Mini World Changes / World Quests
Tibiadrome
Hazard System
Soulpit
Doomforging
Charging System
Crystal Shield
Enchanting
Holy Shrines
Bakragore's Taints
Goshnar's Taints
Umbral Creation
Hunting Tasks / Daily Tasks
Premium gating
```

The Wiki client/help surface is explicitly classified as product/UI documentation rather
than Game content.

## 8. Imports and provenance

`imports/` is never runtime gameplay identity.

```text
imports/official/
imports/tibiawiki/
imports/legacy/
imports/crystalserver/
imports/canary/
```

Every source may retain exact repository/page/revision/digest/mapping coordinates, but:

```text
source id != canonical Oteryn gameplay id
```

Current source hierarchy and Reference evidence policy remain unchanged. OTS sources are
migration/hypothesis inputs, not independent proof of Global behavior.

## 9. Explicit exclusions

The following Wiki surfaces are not game-data families merely because Wiki exposes them:

- community/editorial/tutorial/news prose;
- server-list metadata;
- calculators;
- Wiki Imbuement Tool UI itself (the underlying Imbuement system is mapped);
- provider/vendor/helper pages.

Long-form copyrighted Wiki/Tibia prose and artwork are not bulk-copied.

## 10. Physical migration sequence

The target tree does not authorize an in-place mass move. Use this sequence:

```text
tree/ownership contract
        ↓
Item Master Schema detail contract
        ↓
compatibility reader/writer
        ↓
Items + Mounts mechanical migration
        ↓
Creature/Loot/NPC/Ability/Quest population
        ↓
full-world placement scale measurement
        ↓
world/placement physical migration
        ↓
all consumers switched to successor
        ↓
retire old monolithic locators
```

Every migration phase must preserve canonical identity, source/provenance, semantic
round-trip and required server/client projections. Old `content/world/**` cannot be
deleted merely because the new directories exist.

## 11. Acceptance

The companion validator requires:

- all 19 protected domain groups exactly assigned;
- all 39 selected current system surfaces exactly assigned;
- all target paths unique;
- every Game tree node names a semantic owner and repository;
- Store/commercial owner is Platform;
- mutable/durable state scopes are explicit;
- `unassigned_domains=0`;
- `unassigned_systems=0`.

A future Wiki surface can extend this contract only by receiving an explicit semantic
owner/disposition; no source category automatically creates a new runtime subsystem.
