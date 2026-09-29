# A10 corpse container DUR-03 amendment decision

- Decision: `A10-CORPSE-CONTAINER-DUR03-V1`
- Status: **CANDIDATE, no new owner decision (§2)**. Acceptance requires exact-head validation,
  independent review (DUR-03 value and persistence change) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Profile: `Oteryn Reference` (Global Tibia at 2026-09-27, D33)
- Answers: architect ruling A10, #162 comment 5882274851, to the Combat escalation #162 comment
  5881365903 (corpse as a container)
- Amends: DUR-03 contract §39.1, §39.2 and §39.3 (corpse association wording, one destination
  kind, one burn/retire shape); B3 decision §4.1 (destinations)
- Owner behaviour already decided: D121 (corpses and loot as Global), with D109 and D118 as context
- Admission baseline: `main@005550da`
- Runtime, registry, migration, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

Combat D3 needs loot inside a corpse container, a 10 second loot window and corpse decay (D111,
D112, D113). DUR-03 §39.1-§39.3 mint loot only into typed Ground and treat the corpse only as
provenance. B3 D80 lists the backpack as the only destination. Where does loot live while it is in
a corpse, and how does a corpse end?

## 2. Owner decisions

No new owner decision. D121 already fixes the behaviour: corpses and loot follow Global, the top
damage character or its party has loot rights for 10 seconds, the corpse is a container, and decay
is set per creature. D109 and D118 are context and are not restated here. The texts of D109, D111,
D112, D113, D118 and D121 are taken from #162 as cited in the ruling; this record does not copy them.

## 3. Facts

**PROVEN** (main `main@005550da`)

- B3 §4.1: the destinations are the equipped main backpack and direct entries of it. The entry is
  `Container { parent, entry = placement ordinal }`, a u64 that takes the highest live ordinal plus
  one, computed under a row lock, with display newest first. Placement depth is 1. "Nothing else is
  a destination in this slice." RL-05 is 0. B3 §4.2: `current_entry_count < definition_capacity`.
- DUR-03 §39.1 and §39.2: MINT establishes the item in typed `Ground`. Corpse association is
  "provenance/projection only, never a competing item location" (§39.3). §39.1 excludes burn,
  nested containers and additional custody families. §39.3 admits, by amendment, the B3 shapes and
  the reward-claim MINT into a backpack entry.
- DUR-03 §5.2: `Container { parent_item_instance_id, entry }` is a defined location family.
- DUR-03 §11.4: a retired ItemInstanceId is never reused. §15: burn names item, quantity, a typed
  sink and cause, and the retirement result; silent deletion is not a sink.
- Migration `0011_item_transfer_backpack.sql`: `game_item_container_entries` is bound by a foreign
  key to the character's own container slot. It cannot hold a corpse parent as written.
  `DUR03-RL-08` allows 3 work units per logical transaction.
- VSL Combat rows (`VSL-COMBAT-01-RESOURCE-ROWS-V1`): `COMBAT01-ITEMS-PER-CORPSE` = 16, direct root
  only; `COMBAT01-CORPSES-PER-SCOPE` = 64; the loot plan has at most 16 entries, one one-item MINT
  each (D77).
- WO-0 (D93): a corpse is a routed Item. Decay target, decay duration and container capacity are
  Item-owned facts on the routed Item record; a corpse definition holds no loot.
- D52: a loot MINT descends from a committed `CreatureDeathOccurrenceRef`. A MINT committed before
  the death's ownership generation ended keeps its result. One not committed by then is
  terminally not minted.
- Evidence `OTV2-20260912-reference-combat-death-corpse-loot-chain`: the 10 second rule is loot
  authorization plus corpse immovability. It is not a durable `corpse_owner` lease (CORPSE-01,
  CORPSE-02).

**UNKNOWN**

- Rat corpse container capacity and decay duration (Content).
- The numeric Item key of the rat corpse.
- The exact tick or rounding of the 10 second edge (CORPSE-03).
- Whether the corpse MINT and its loot MINTs share one physical DUR-03 commit or are sequenced.
  This record requires only the order in §4.1.
- Retire work units and the evidence payload of a 17-item retire.

## 4. Decision

### 4.1 Corpse and loot MINT

- The corpse is minted as the routed corpse Item (WO-0), located on Ground, in the same death
  workflow as the loot: one committed death occurrence, one D52 cause. The corpse commits before
  any child, because a child's parent must be live.
- Each loot entry is a MINT whose location is `Container { parent = corpse item instance, entry }`.
  It has no Ground custody. This is a new destination kind for MINT only.
- Direct root only, depth 1. At most `COMBAT01-ITEMS-PER-CORPSE` (16) entries per corpse. An entry
  never becomes a parent inside the corpse.
- The entry ordinal follows B3 §4.1: highest live ordinal plus one, no renumbering, newest first.
  The serialization point is the parent corpse item row, not `character_root`.
- Capacity check before insert: `current_entry_count < corpse container capacity`. The effective
  limit is the lower of the capacity and 16. A failed check rejects, as the row says. Nothing
  falls to Ground.
- The corpse and each child keep separate ItemInstanceIds, separate one-item MINT transactions and
  the D77 per-entry cause. Retry and restart follow D52.

### 4.2 Take from the corpse (D112)

- Taking is a TRANSFER of a corpse entry into a main-backpack entry, under D81-D83: free-entry
  check, stack maximum 100, merge and top-up with the same shapes and rows as B3 §4.4. Only the
  source location differs (`Container` of the corpse, not Ground).
- The loot-rights guard is a runtime authorization checked at admission time: the top-damage
  principal or its party for 10 seconds, anyone afterwards. It is not durable state. No
  `corpse_owner` column, lease or row is added.
- No TRANSFER shape moves a corpse in this slice. The corpse is therefore immovable during the
  window by admission. Moving it later is not decided.

### 4.3 Decay (D113)

- Decay is one retire transaction of the corpse and all its live children at the corpse Item's
  decay deadline. Retired items are never reused (§11.4).
- The shape is new to DUR-03 and is defined minimally here. Typed sink: corpse decay. Cause: the
  corpse instance id plus the decay facts and revision read from the corpse Item record. It is
  idempotent by corpse instance id: a retry returns the first result, and a retired corpse is a
  terminal result.
- A child already transferred out is no longer a child by location and is not touched. A
  transfer and a decay serialize on the corpse row: the first commit wins. A transfer after decay
  finds the source retired and is rejected.
- Evidence follows §39 and §15: each retired item has a before line (live, location, quantity)
  and an after line (retired), plus the sink and cause. If the measured payload or work units exceed
  the registered rows, the shape returns for decision. No cap is widened here.
- The runtime scope owner triggers decay. What happens to a pending decay across a restart, beyond
  D52, is not decided. The idempotent key lets a later decision retire a past-deadline corpse.

### 4.4 Resource rows

Registered by the implementing lane under the registry single-writer lease, each with max and
max+1 tests, checked before allocation, never truncated. Row ids are working labels.

| Row | Value | Note |
|---|---|---|
| corpse-parent container locations per corpse | 16 | equals `COMBAT01-ITEMS-PER-CORPSE`; new |
| retire touched items per transaction | 17 | 1 corpse plus 16 children; new; `DUR03-RL-01` for TRANSFER stays 2 |
| retire work units | **UNKNOWN** | the Durability lane measures it; `DUR03-RL-08` is 3 today |
| retire evidence payload | **UNKNOWN** | measured against `DUR03-RL-07` |
| `DUR03-RL-05` container expansion | 0 for MINT and TRANSFER | the retire shape names its own row above |

### 4.5 DUR-03 amendment

This decision supersedes, for the shapes above only:

- §39.1 and §39.2 (corpse "provenance only") and §39.3 "Corpse association is provenance/projection
  only": for corpse Items created by this path the corpse is the parent location of its loot.
- §39.1 exclusions of burn, nested containers and additional custody families: one retire shape
  (§4.3) and one corpse parent, depth 1.
- B3 §4.1 "nothing else is a destination": one extra kind, a corpse container, for loot MINT only.
  Players cannot place items into corpses in this slice.
- §39.1 "MINT establishes typed Ground" for loot children only. The corpse itself is still minted
  on Ground.

Every other §39 obligation is unchanged (fences, cause, evidence, idempotency, current authority).
Pointer notes are added to §39.1 and §39.3.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| Durability | Corpse-parent location table or an equivalent that keeps the 0011 character-slot invariants, corpse-parent MINT, TRANSFER from a corpse entry, the retire shape, the resource rows | this decision merged |
| Combat D3 | D111 corpse container in the death workflow, D113 decay trigger | Durability child; Content facts |
| Combat D112 | Loot-rights guard | none; may ship first, independent of this decision |
| Content | Rat corpse capacity and decay duration facts (numbers UNKNOWN) | none |

## 6. Rejected options

- **Loot stays on Ground with provenance only.** The corpse would not be a container, which breaks
  D121.
- **Nested containers inside corpses.** Depth above 1 needs container expansion (RL-05 above 0) and
  is a later decision.
- **A durable `corpse_owner` lease.** The evidence says the 10 second rule is authorization only.

## 7. Decision test

- **Must decide now:** YES. Combat D3 and the Durability schema wait on where corpse loot lives
  and how a corpse ends.
- **Minimum sufficient:** one parent kind, depth 1, the existing ordinal rule and TRANSFER rules,
  one retire shape.
- **Superseding evidence:** measured retire work and payload; Content capacity and decay facts; a
  decision on player deposit or deeper nesting.
- **Deliberately not decided:** player deposit into corpses, corpses of players (DEATH-3), decay
  durations (Content), loot persistence across restart beyond D52, moving a corpse.

## 8. Handback

```yaml
result: RESOLVED_WITHOUT_NEW_OWNER_DECISION
source_escalation: "#162 comment 5881365903; ruling 5882274851 (A10)"
owner_decisions: [D121]   # context: D109, D118; Combat items D111, D112, D113
amends: docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.1-§39.3, corpse container only
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_A10_CORPSE_CONTAINER_DUR03_AMENDMENT_DECISION_2026-09-29.md
resource_values_changed: true   # registered by the Durability allocation; two values UNKNOWN
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # the Durability child may be allocated after merge
required_fresh_allocation: true
required_independent_review: "exact-head independent review (DUR-03 corpse parent, retire shape, idempotency, resource rows)"
implementation_lanes: [Durability, Combat-D3, Content]
required_revalidation:
  - "MINT: corpse on Ground first, then up to 16 entries with parent = corpse; the 17th, or an entry at capacity, is rejected; ordinals are unique and newest first; retry and restart follow D52"
  - "TRANSFER: corpse entry to backpack under D81-D83; guard denies a non-principal inside 10 s and allows anyone after; no corpse_owner state exists"
  - "retire: corpse plus all live children in one transaction; retry returns the first result; a child already taken is untouched; transfer and decay race has one winner; retired ids are never reused; max and max+1 for each row"
remaining_unknowns:
  - rat corpse capacity, decay duration and numeric Item key
  - 10 second edge quantization (CORPSE-03)
  - corpse and loot MINT commit grouping
  - retire work units and payload
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates the Durability child."
```
