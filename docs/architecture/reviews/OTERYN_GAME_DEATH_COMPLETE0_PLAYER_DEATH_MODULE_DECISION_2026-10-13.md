# DEATH-COMPLETE-0 player death module decision

- Decision: `DEATH-COMPLETE0-PLAYER-DEATH-MODULE-V1`
- Status: **CANDIDATE**. Acceptance requires exact-head validation, independent review and protected
  integration. The owner direction below is recorded; its decision number is assigned by the control plane.
- Role: client-track architect, delegated by the control plane (#1622)
- Lifts: the D1022 waiver of DEATH-3 (`OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md`,
  Resolution 2026-10-13, §4.4 "Delivery gap")
- Builds on: the Reference first player death decision (D58-D68), DEATH-0 receipt decision (§3.1,
  §3.3, §3.4), PARTY-PVP-0 §9-§10, DUR-03 contract §15 and §39, D3 corpse decision
- Admission baseline: `main@59f9fb12`
- Runtime, registry, migration, protocol and production authority: **NONE**. Each batch below needs its
  own allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

The owner wants the whole player death module deployed as one coherent unit, not in separate slices
(2026-10-13: "nie rozdrabiaj sie tylko caly modul smierci ma byc wdrozony"). What is the module, in
what minimum number of batches does it land, and which rules does each batch use so that the death
receipt, the registries and the migrations change once?

## 2. Owner decisions

| # | Decision | Source |
|---|---|---|
| DC-1 | The D1022 waiver is lifted. Item loss, the Amulet of Loss, lost-item set, player body, penalties, respawn and PvP death are delivered as one module. | owner, 2026-10-13, relayed by the control plane |
| DC-2 | Blessing purchase (DEATH-4) is in the first release of the module (batch E). | owner delegated the choice of defaults to the control plane, answer `1a` |
| DC-3 | The Amulet of Loss is delivered as a content item with selection and consumption only; purchase through an NPC is later. | same, `2a` |
| DC-4 | A lost container moves with its contents under the limits of §4.4. | same, `3a` |
| DC-5 | Client observation of death (wire, batch F) is not required for the first playable loop; it follows the first client end-to-end proof. | same, `4a` |

DC-2 to DC-5 were chosen by the control plane under the owner's delegation. Acceptance of this decision
confirms them.

## 3. Facts

**PROVEN** (`main@59f9fb12`)

- `0016_character_death_receipts.sql` already has `amulet_of_loss_item_id`, `lost_item_ids UUID[]` and the
  death cell columns. Two named checks hold them empty: `game_character_death_receipts_no_amulet_until_death3`
  and `game_character_death_receipts_no_lost_items_until_death3` ("DEATH-3's migration replaces these").
  There is no column for PvP fields yet.
- `durability/character_death.rs` writes the receipt and rejects a non-empty set with `23514`; the
  blessing set and `game_character_pending_respawns` exist; the XP/blessing/respawn transaction, the
  trigger, the temple respawn and the restart safety (#2004, #2006) are merged.
- DUR-03 native one-item transactions touch at most 2 ItemInstances (`DUR03-RL-01`) and expand no
  container (`DUR03-RL-05` = 0). Burn sinks admitted today are `DECAY_RETIRE`, `FeeBurnCause`, the
  conjure/proficiency/timed shapes. The timed burn retires a direct container entry only. No burn of an
  equipped item exists.
- Migration `0011` binds container entries to the character's own container slot by a foreign key, so a
  container located on Ground cannot hold entries today. Backpack placement depth is 1.
- No migration inserts a blessing; every death records an empty blessing set.
- Open lanes that touch the same files: #2032 (PVP-1, migration 0081), #2040 (NPC-TRADE-1, 0082),
  #2039 (ITEM-USE-1, 0083).

**UNKNOWN**

- The Global low-level no-loss exemption after the 2025-10-21 update (level 8 per D65 or 6 on the
  mainland): re-read at the target date by batch C.
- The Ground per-cell item limit, if any, and the dropped-item decay policy.
- The blessing price table at the target date (batch E reads it).
- The Amulet of Loss item key (Content).

## 4. Decision

### 4.1 Scope: one module

The module is: the Character death outcome (merged), the item workflow (drops and amulet), the player
body, respawn gated on the workflow, PvP death, blessing state and purchase. It is accepted and enabled
as a whole. Batches C and E may merge separately, but the owner acceptance of the module (and any
production exposure) is recorded once, after C, D and E have landed and been reviewed (F is outside, DC-5).
Without E every death is fully penalised, so exposing C alone is not accepted.

### 4.2 One receipt change

The death receipt changes once, in the migration of batch C:

- the two `*_until_death3` checks are replaced by: `lost_item_ids` has no duplicates and at most 10
  entries (the ten equipment slots including the backpack); an amulet selection implies an empty
  lost-item set; a non-empty lost-item set implies no amulet;
- columns for PARTY-PVP-0 §10.3 (`pvp_death`, `skull_at_death`, `twist_of_fate_used`,
  `adventurer_applied`, `unfair_fight_milli`), `NOT NULL` with the PvE values as defaults, typed as that
  section defines them, and covered by `command_binding`. Batch D fills them and changes no schema.

The revision chain, the pending respawn table and the XP receipt table are unchanged. A lost container's
contents are not listed in the set: they travel with the container.

### 4.3 Selection rules (batch C, computed in the Character transaction)

- **Exemptions.** No selection and no amulet use for a character at or below level 8 (D65), without a
  vocation, or at or below the level of a verified Global exemption read at the target date.
- **Amulet of Loss.** Selected when worn, the character has fewer than 5 regular blessings, the death is
  not PvP and the character is not exempt. It is the necklace slot item with the content key of the
  amulet. When selected, the lost-item set is empty. When not selected (5 or more blessings, exempt, PvP
  skull rule of PARTY-PVP-0 §10), it stays equipped.
- **Draw.** Otherwise each equipped item and the backpack is drawn independently against the D62 ladder
  in parts per thousand: backpack 1000/700/450/250/100/0 and equipped item 100/70/45/25/10/0 for 0/1/2/3/4
  and 5 or more regular blessings. The draw for slot `s` is the first 16 bits (modulo 1000) of a
  domain-separated SHA-256 over the death occurrence, the policy digest and the slot index; the item is
  lost when the draw is below the threshold. No other source of randomness is used. The slot order is
  fixed in the death policy.
- Everything above is recorded in the receipt and `command_binding`; a retry returns the first result.

### 4.4 DUR-03 primitives (batch B)

- **Death drop.** A new typed cause `PlayerDeathDropCause { death_occurrence, item_instance }` for a
  TRANSFER of one item from `CharacterInventory` (equipment slot or backpack slot) to Ground at the
  receipt's death cell. Idempotent per cause. TRANSFER rows `DUR03-RL-01`..`RL-08` are unchanged for
  other shapes; this shape registers its own rows with max and max+1 tests.
- **Container with contents.** A container moves as one item. Its contents are carried by re-parenting
  the container: the entries stay keyed to the container's instance, not to a character slot, and the
  contents are neither enumerated nor split. This needs the storage change to `0011`'s foreign key so a
  Ground-located container keeps its entries. The ceiling is the existing backpack placement depth of 1
  (`DUR03-RL-05` stays 0 for the other shapes; the death drop row is registered with depth 1 if the
  implementation proves it must enumerate). A bigger need returns for decision; no limit is widened
  silently. The implementation must never split a container.
- **Amulet destroy.** A new typed sink `PlayerDeathAmuletCause { death_occurrence, item_instance }`: a BURN
  of one equipped necklace item, with the typed sink, the cause and the before/after lines of DUR-03 §15.
  Idempotent per cause. A silent delete is not a sink.
- Each operation is its own one-item transaction. They are not one atomic transaction; the receipt is
  authoritative (death decision §4.4, unchanged).
- The proto and event registries (`native_one_item_transaction`, event foundation) and
  `RESOURCE_LIMITS_REGISTRY.json` change only in this batch, each under its single-writer lease.

### 4.5 Item workflow, body and respawn gate (batch C)

- After the Character commit the Channel owner drives the workflow: one drop per lost item and, when
  selected, the amulet destroy, in receipt order, each idempotent per (occurrence, item).
- Dropped items land on Ground at the death cell by the existing Ground placement rule. The player body is
  a runtime projection: a lootable association with those ground items, no new corpse Item (the D3 corpse
  Item stays for creatures). Any player takes items through the ordinary pickup path. A player corpse
  sprite and the dropped-item decay are not decided here (§6).
- **Gate.** The pending-respawn consumption (placement at the temple, on respawn or at the next
  admission) requires every (occurrence, item) cause of the receipt to have a committed result. After a
  restart or generation change the recovering owner reads the receipt, resumes every outstanding cause,
  and only then places the actor.
- **Failure.** A terminal reject of a cause (the item is not where the receipt says) is an invariant
  violation: the character stays death-pending, nothing is skipped, and the case escalates to the
  operator. No automatic repair that drops or recreates an item.
- **Reconnect.** A logout during the workflow is allowed; the next admission resumes it before placement.

### 4.6 PvP death (batch D)

PARTY-PVP-0 §10 governs: kill classification, red/black skull loss rules, Twist of Fate, Adventurer's
Blessing and the black-skull respawn with 40 HP and 0 mana. The receipt carries the fields (§4.2). The
respawn vitals follow the receipt's `skull_at_death` at placement; `game_character_pending_respawns` does
not change. The unfair-fight value is recorded as 0 until parity evidence exists (PARTY-PVP-0), and the
Adventurer's Blessing forfeiture is written by the same transaction. Batch D depends on #2032 and on
PVP-RT-1b.

### 4.7 Blessing state and purchase (batch E)

- A purchase is a composed workflow keyed by a purchase occurrence (UUIDv7): (1) the NPC service
  validates eligibility (the 3 Premium blessings need current Premium at purchase, D67), the level price
  and the blessing not already held; (2) payment through a DUR-03 burn with a new typed sink
  `BlessingPurchaseCause` (BANK-FEE0 gold rules, no silent deletion); (3) one Character transaction
  inserts the blessing with provenance and a grant receipt keyed by the purchase occurrence. It does not
  advance `CharacterRevision` (DEATH-0 §3.3).
- Order and recovery: the grant is written only after the payment committed. A committed payment without
  a grant is completed on retry. A grant without payment never exists.
- The price table and the Twist of Fate acquisition are read at the target date; Twist of Fate is a
  PARTY-PVP-0 matter (`twist_of_fate_used`), not purchased here.

### 4.8 Registry and migration discipline

- `PROTOCOL_OTERYN_V1_REGISTRY.json`: none of A-E touches it. Only F (a death observation) does, as its own
  single registry PR, after the first client end-to-end proof.
- Event, proto and resource registries: batch B only, after #2039 and #2040 merge.
- Migrations: one for B (storage change of container entries), one for C (receipt), one for E (grant
  receipt); D needs none (§4.2). Numbers are leased by the control plane after 0083; one migration PR at a
  time, in merge order.

## 5. Delivery: minimum batches

| Batch | Scope | Worker | Depends on |
|---|---|---|---|
| A. DEATH-COMPLETE-0 | this decision (documents only) | architect | none |
| B. DEATH-DUR-1 | §4.4 primitives, storage change, registries | **hard** (persistence, durable value) | A; #2039, #2040 merged |
| C. DEATH-3 | §4.2, §4.3, §4.5 receipt, selection, workflow, body, gate | **hard** (persistence, fencing) | A, B |
| D. PVP-DEATH-1 | §4.6 | **hard** (persistence, authority) | C, #2032, PVP-RT-1b |
| E. DEATH-4 | §4.7 | **hard** (durable value) for the grant and payment; the NPC dialog may be an ordinary slice | A, #2040, NPC-CMD-1b-write |
| F. DEATH-WIRE-2 (optional) | client observation of death and respawn | hard (wire) | first client e2e proof, outside the module |

D and E run in parallel after their dependencies. No batch splits a rule across two PRs: B owns the
primitives, C owns every use of them.

## 6. Rejected options and not decided

- **Split the receipt per batch** (items now, PvP later). Rejected: the receipt would change twice.
- **Mint a player corpse Item as a D3 container.** Rejected: it needs a new Ground-to-corpse transfer and
  a second location kind for no playable gain; ground items and a projection suffice (§4.4 of the death
  decision).
- **Skip an item that cannot be dropped.** Rejected: it changes the committed outcome and hides a defect.
- **Raise `DUR03-RL-05` globally.** Rejected: only the death drop registers its own rows.
- Not decided: the player corpse sprite, the dropped-item decay, Death Redemption (D64), charms, the
  Newhaven exemption value, the blessing prices, the wire of death (F).

## 7. Decision test

- **Must decide now:** YES. The owner released the module; B-E cannot be allocated without the shared
  rules (one receipt change, the container rule, the gate).
- **Minimum sufficient:** reuse the existing receipt columns, one new cause per item operation, no corpse
  Item, one migration per storage concern.
- **Superseding evidence:** measured limits from batch B; the Global exemption at the target date; a
  decision on the corpse sprite.

## 8. Handback

```yaml
result: CANDIDATE
owner_decisions: [DC-1, DC-2, DC-3, DC-4, DC-5]   # numbers assigned by the control plane
amends:
  - docs/architecture/reviews/OTERYN_GAME_REFERENCE_FIRST_PLAYER_DEATH_DECISION_2026-09-28.md   # waiver lifted
  - docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md   # §3.1 one receipt change
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_DEATH_COMPLETE0_PLAYER_DEATH_MODULE_DECISION_2026-10-13.md
resource_values_changed: false   # registered by batch B
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: false   # B-E need allocation after acceptance
required_fresh_allocation: true
required_independent_review: "exact-head independent review of the document (receipt, DUR-03 shapes, workflow gate, scope)"
implementation_lanes: [DEATH-DUR-1, DEATH-3, PVP-DEATH-1, DEATH-4]
required_revalidation:
  - "B: a death drop moves one item once per cause; a container keeps its contents and is never split; the amulet burn has a typed sink; replay, conflict and stale fence write nothing; max and max+1 for each registered row"
  - "C: the ladder per blessing count 0-5 at levels 1, 8, 9, 24 and 500; amulet selection and no-selection cases; restart mid-workflow resumes every cause once; no placement before all causes commit; no duplication"
  - "D: a PvP death writes the receipt fields; black skull respawn 40 HP and 0 mana; Twist of Fate and Adventurer's Blessing per PARTY-PVP-0 §10"
  - "E: payment before grant; retry completes a paid purchase; Premium blessing without Premium is refused"
remaining_unknowns: [Newhaven exemption, Ground cell limit and decay, blessing prices, amulet item key, container enumeration need]
next_action: "#1622 routes the independent review, then allocates B after #2039 and #2040."
```
