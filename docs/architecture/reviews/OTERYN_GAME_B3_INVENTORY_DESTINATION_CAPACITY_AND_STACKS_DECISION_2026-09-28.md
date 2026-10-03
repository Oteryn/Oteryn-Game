# B3 inventory destination, capacity and stacks decision

- Decision: `B3-INVENTORY-DESTINATION-CAPACITY-STACKS-V1`
- Status: **CANDIDATE with owner decisions D80-D83 taken (§2)**. Acceptance requires exact-head
  validation, independent review and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Profile: `Oteryn Reference` (Global Tibia at 2026-09-27, D33)
- Answers: the #513 gate B3 ("legal Content/Item/loot/inventory product admission") that keeps
  DUR-03 TRANSFER closed (Combat waiting item 3, #162 5874268146)
- Amends: DUR-03 contract §39.1 and §39.3 (TRANSFER destination scope), within the location
  families DUR-03 §5.2 already defines
- Owner decisions posted: #162 comment 5874405992
- Admission baseline: `main@e6141b414e0cb20fa050ba451869a8ede52a5b53`
- Runtime, registry, migration, protocol and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

DUR-03 TRANSFER stays closed until a legal inventory destination, capacity and placement rules
are accepted (DUR-03 §39.2, §39.3). GAME-ITEM-01 leaves stack maxima, slot vocabulary and
capacity `PARITY_PENDING_EVIDENCE` and forbids implementation without absolute ceilings (§7.1).
Where does a picked-up item go, what limits apply, and do stacks merge?

## 2. Owner decisions

| # | Decision | Owner choice (2026-09-28) |
|---|---|---|
| D80 | A picked-up item goes directly into the character's equipped main backpack, as in Global. One level only; placing into a bag inside the backpack comes later. | "Do plecaka, jak Global" |
| D81 | The first TRANSFER checks free backpack entries only. Weight capacity follows once item weights are confirmed. | "Najpierw miejsca, waga potem" |
| D82 | Stack maximum 100, or the value a definition proves; hard safety ceiling 100. | "100, wyjątki z definicji" |
| D83 | Merging into an existing stack is part of the first TRANSFER admission. | "Od razu" |

## 3. Facts

**PROVEN** (main `e6141b414e0cb20fa050ba451869a8ede52a5b53`)

- DUR-03 §5.2 already defines the location families `CharacterEquipment` and
  `Container { parent_item_instance_id, entry }`. §39.1 and §39.3 admit only Ground → direct-root
  `CharacterInventory` and no nested containers.
- DUR-03 §13: a quantity transfer from A to compatible B keeps B's id, decreases A, retires A at
  zero (§11.5), conserves units, and never picks the receiver by UUID or client order. §10:
  capacity, type and nesting constraints validate before commit.
- DUR-03 rows (`RESOURCE_LIMITS_REGISTRY.json`, decision D50/D51): RL-01 = 1 touched item,
  RL-02 = 2 location lines, RL-03 = 0 value lines, RL-05 = 0 container expansion, RL-06 = 1
  participant / 3 work units, RL-07 envelope 9,216 B / payload 7,936 B; a TRANSFER with a 128 B
  `typed_position` has a 7,565 B payload.
- GAME-ITEM-01 §4.1: `1 <= quantity <= definition stack maximum <= absolute safety ceiling`; §6
  and §7.1: direct entries, nesting depth and reachable graph size need absolute ceilings.
- Content: the backpack definition has a known container capacity of 20
  (`content/items/definitions`, `oteryn:item.registry.i00002752`). Many definitions have an
  unknown stack class; gold coin `max_stack` = 100 was not admitted for lack of a decision.
- tibia.com manual (Oteryn notes, capture 2026-09-28, `docs/reference/tibia-manual/`):
  - §interface 3.4.1: fixed body slots; the Container Slot holds bags and backpacks; items go only
    in their designated slot type;
  - §interface 3.4.4: picking up an item over capacity produces an error;
  - §characters 5.1.9: capacity gain per level Druid 10, Knight 25, Monk 25, Paladin 20,
    Sorcerer 10;
  - §characters 5.1.11: a character who lost the backpack receives a fresh empty bag afterwards.
- tibia.com manual snapshot `imports/official/tibia-com/2026-09-28-160207Z` (#1129):
  - `controls.4-1-basic-controls.10`: a new item placed in a container appears in the first slot
    unless the client's manual sort mode is on;
  - `products.6-3-available-products.15`: coins are converted from "a stack of 100".
- The reward chest decision (D41) already checks "the main backpack needs at least one free slot".
- Migration `0010_item_mint_ground.sql` has Ground as the only location table; TRANSFER has no
  code.
- The Character/item composition decision: an inventory-only DUR-03 transaction does not advance
  `CharacterRevision` and serializes on the `character_root` row lock.

**CONFLICT** (not used here): item weights (gold coin 10 / 1 / 0.01 oz across sources); base
capacity changed by the 2025 Newhaven update (GAME-CHAR-01 Stage B evidence).

**UNKNOWN:** Global behaviour when a stack only partly fits and the backpack has no free entry.

## 4. Decision

### 4.1 Destinations (D80)

- **Main backpack:** the container item in the character's `CharacterEquipment` slot `container` (Global's "Container Slot" for bags and backpacks).
- **Backpack entry:** `Container { parent = main backpack, entry = placement ordinal }`. The
  ordinal is a u64, unique among the backpack's live entries: a placement takes the highest live
  ordinal plus one, computed under the `character_root` row lock, so no counter state is added.
  The `typed_position` is the parent `ItemInstanceId` and the ordinal (≤ 128 B).
- **Order:** the display order is newest first, so a new item appears in the first slot, as in
  Global (manual §controls 4.1.10). A placement never renumbers the other entries; the capacity is
  the number of entries.
- **Container slot:** an empty container item may move from Ground into the empty `container`
  slot only if its definition declares a complete equip pattern whose primary slot is `container`
  (GAME-ITEM-01 §6.2). Being a container is not enough. The backpack definition's equipment
  semantics are still `UNKNOWN` in content, so B3-1 needs that pattern proven for at least one
  container. This is how a character without a backpack gets one in this slice; a starter kit is a
  separate decision.
- Nothing else is a destination in this slice: other equipment slots, bags inside the backpack,
  Ground drops and depot are later decisions.
- No backpack equipped, or the destination is not the character's own main backpack: rejected,
  the item stays on Ground.

### 4.2 Capacity (D81)

- An entry-placing TRANSFER needs a free entry in the main backpack: before the insert,
  `current_entry_count < definition_capacity`. A full backpack rejects; the item stays on Ground and the player is told why.
- Weight capacity is **not** checked in this slice. This is a declared delivery gap against Global,
  closed by child B3-3 once item weights and the character capacity source are accepted.
- The moved item may be a container only if it is empty (RL-05 stays 0: no descendant expansion).

### 4.3 Stacks (D82)

- A stackable definition carries its stack maximum; if it is stackable with no proven value, the
  maximum is 100. The absolute ceiling is 100; a proven larger value waits for a new decision.
- An item with an unknown stack class is not admitted for pickup (fail closed). A non-stackable
  item has quantity 1.

### 4.4 Merge on pickup (D83)

On pickup of stackable Ground item A (quantity q), the receiver B is the compatible stack in the
main backpack with room that comes first in display order (the highest ordinal). Compatible means
the same definition key and revision, both stackable, and equal item state apart from quantity
(the quantities may differ). Then, in one DUR-03 transaction:

| Case | Effect | Touched items (participants) | Location lines | Quantity changes |
|---|---|---|---|---|
| No compatible B with room | plain TRANSFER of A to a new entry (new ordinal), if the backpack has a free entry; otherwise rejected | 1 | 2 | 0 |
| B.q + q ≤ max | full merge: B grows by q, A retires (§13, §11.5) | 2 | 1 | 2 |
| B.q + q > max, a free entry exists | top-up: B grows to max, A keeps its id with the remainder and moves to a new entry (new ordinal) | 2 | 2 | 2 |
| B.q + q > max, no free entry | rejected; nothing moves (partial pickup is `UNKNOWN` parity) | 0 | 0 | 0 |

**Amendment (pending on acceptance of STACK-0; `reviews/OTERYN_GAME_STACK0_PARTIAL_COUNTS_AND_STACK_MERGES_DECISION_2026-10-01.md` §4.2).** For sessions with `ITEM_STACK_COUNT_V1`, the last row becomes
Canary's partial fit: B grows to max, A keeps its identity with the remainder in place, and the
result is `PARTIAL`. Sessions without the capability keep the refusal.

Quantity changes are item state of the touched ItemInstances, recorded as their before and after
evidence; they are not value lines (RL-03 counts non-item value and account lines, DUR-03 §18).
Exact units are conserved (`SPLIT_MERGE_QUANTITY`). A retry of the same pickup command returns the
first result.

### 4.5 Resource rows

Registered by the B3-1 allocation under the registry single-writer lease, with max and max+1
tests; checked before allocation, never truncated.

| Row | Value | Note |
|---|---|---|
| `DUR03-RL-01` touched items | **2** for the merge and top-up shapes; 1 otherwise | amended |
| `DUR03-RL-06-PARTICIPANTS` | **2** for the merge and top-up shapes (A and B); 1 otherwise | amended |
| `DUR03-RL-02` location lines | 2 | unchanged |
| `DUR03-RL-03` value lines | 0 | unchanged; stack quantities are item state, not value lines |
| `DUR03-RL-05` container expansion | 0 | unchanged; reading the main backpack's entry count is not expansion |
| `DUR03-RL-06-EFFECT-WORK-UNITS` | **6** for the merge and top-up shapes; 3 otherwise | amended; a participant plus its effects: top-up A = 1 + Ground removal + entry placement + quantity = 4, B = 1 + quantity = 2; full merge A = 1 + Ground removal + retirement = 3, B = 2 |
| `DUR03-RL-07` envelope / payload | 9,216 B / 7,936 B | unchanged: the receiver adds at most 256 B (id 16 B, position ≤ 128 B, two quantities, tags), so the payload stays ≤ 7,821 B |
| `GAMEITEM01-STACK-QUANTITY-MAX` | 100 | new absolute ceiling (D82) |
| `GAMEITEM01-CONTAINER-ENTRIES-MAX` | 20 | new; a main backpack with a larger capacity waits for a new decision |
| `GAMEITEM01-PLACEMENT-DEPTH` | 1 | new (D80) |
| `GAMEITEM01-REACHABLE-ITEMS` | 21 per character (the main backpack and 20 entries) | new |

The registration proves the exact RL-07 worst case of the merge shapes; if it exceeded the
registered caps, the merge shapes would return for a new decision instead of widening a cap.

**Amendment (pending on acceptance of BAGS-0;
`reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` §3, §6).**
A moved container may have contents. `GAMEITEM01-PLACEMENT-DEPTH` becomes 8 (`BAGS0-RL-01`), a tree
holds at most 500 items (`BAGS0-RL-02`), `GAMEITEM01-REACHABLE-ITEMS` becomes 509, and `DUR03-RL-05`
is 8 for the tree shapes. The `MAIN_BACKPACK` destination keeps this decision's rule: direct entries
only, and a full main backpack is refused (no fall-through into nested bags).

### 4.6 DUR-03 amendment

This decision explicitly supersedes, for the shapes below only, these DUR-03 statements:

- §39.1: "multiple touched items", "quantity redistribution" and "nested containers" are
  unsupported. The merge and top-up shapes of §4.4 (exactly two same-definition stacks, DUR-03 §13)
  and placement into direct entries of the equipped main backpack are admitted.
- §39.1 and §39.3: TRANSFER goes only to direct-root `CharacterInventory`. The destinations of
  §4.1 are admitted.
- §39.3 decision test: "NO for the unresolved TRANSFER destination position, capacity and
  admission". B3 resolves destination, capacity and placement; TRANSFER is admissible once B3-1
  proves them.

Every other §39 obligation is unchanged (fences, cause, evidence, idempotency, current authority).
Pointer notes are added to §39.1 and §39.3.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| B3-1 | Equipment slot and container-entry location tables (migration), TRANSFER to the container slot and to backpack entries, the merge and top-up shapes, the resource rows, the pickup refusal reasons | DUR-03 C/MINT (done); the Character/item composition decision; a content definition proving a `container`-slot equip pattern |
| B3-2 | Combat pickup through GAME-INTERACTION into B3-1 | B3-1; Combat D |
| B3-3 | Weight capacity: item weights, the character capacity source, refusal on overweight | evidence for item weights and base capacity |
| Later | Bags inside the backpack (RL-05 > 0), other equipment slots, drop to Ground, starter backpack, DEATH-3 container drops and the fresh empty bag after a lost backpack (§characters 5.1.11) | their own decisions |

## 6. Rejected options

- **A flat inventory outside the backpack.** The owner chose Global's backpack (D80).
- **Weight now.** Item weights conflict across sources; checking them now would block TRANSFER on
  unrelated evidence (D81).
- **Merge later.** The owner chose merging in the first admission (D83); DUR-03 §13 already
  defines its identity rules.
- **Store the slot index.** Global inserts new items in the first slot, so a stored index would
  renumber every entry on each placement and touch up to 20 items; the ordinal keeps one touched
  item.
- **Charge stack quantities to RL-03.** RL-03 is the fail-closed non-item value line; widening it
  would admit a capability these shapes do not use.
- **Pick the receiver stack by UUID or client order.** DUR-03 §13 forbids it; the placement ordinal is
  a semantic order.

## 7. Decision test

- **Must decide now:** YES. TRANSFER, pickup and Combat's loot delivery wait on B3.
- **Minimum sufficient:** one backpack level, entry count only, one stack ceiling, merge within
  the existing DUR-03 §13 rules.
- **Superseding evidence:** accepted item weights and base capacity; proven stack maxima above
  100; containers larger than 20; the Global partial-pickup behaviour.
- **Deliberately not decided:** weight, nested bags, other equipment slots, drop, depot, starter
  kit, container drops on death.

## 8. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
source_escalation: "#513 gate B3; Combat waiting item 3 (#162 5874268146)"
owner_decisions: [D80, D81, D82, D83]
amends: docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md   # §39.1, §39.3 destination scope only
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_B3_INVENTORY_DESTINATION_CAPACITY_AND_STACKS_DECISION_2026-09-28.md
resource_values_changed: true   # registered by the B3-1 allocation
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # B3-1 may be allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (DUR-03 destinations, merge conservation, resource rows)"
implementation_lanes: [B3-1, B3-2, B3-3]
required_revalidation:
  - "B3-1: pickup places the item as the newest entry, shown first; with 20 entries the 21st is rejected and the item stays on Ground; no backpack rejects; an empty container with a container-slot equip pattern enters the empty container slot; a container without that pattern or with contents is rejected"
  - "B3-1 merge: full merge retires the source; top-up keeps the source id with the remainder in a free entry; no free entry rejects with nothing moved; units conserved; stacks with different quantities are compatible; receiver chosen as the first compatible stack in display order; a retry returns the first result"
  - "B3-1 rows: max and max+1 for each row (touched items 2, participants 2, work units 6, RL-03 still 0); the merge-shape RL-07 worst case within 7,936 B payload"
remaining_unknowns:
  - item weights and base capacity (B3-3)
  - Global partial pickup with a full backpack
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates B3-1."
```
