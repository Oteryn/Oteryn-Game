# EQUIP-0 Equipment effects, timed and charged items

- Decision: `EQUIP0-EQUIPMENT-EFFECTS-TIMED-AND-CHARGED-ITEMS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  persistence and determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698, item 2, TIMED-ITEM-0, widened):
  WORLD-INTERACTION-0 left carried torches "with the timed-item decision that also covers soft boots
  and rings"; SKILLS-0 left "skill boosts from equipment"; CONDITIONS-0 left equipment-granted
  conditions; no decision makes worn equipment give its protections, skills, regeneration, mana
  shield or invisibility.
- Builds on: GAME-ITEM-01 §4.2, §4.4, §4.8 and §6.4 (charge and temporal capabilities, the
  deterministic modifier plan), DUR-03 §15-§17 (burn, transform, `STATE_MUTATION`), A12 (one
  definition per Tibia ID), ITEM-MOVE-WIRE-1 §4 (equip, requirements, Premium), IMBUE-FORGE-0 §4-§5
  (the checkpoint model and the effect stages this decision reuses), CONDITIONS-0 (families and
  speed), WORLD-INTERACTION-0 §11.4 (item light), GAME-ABILITY-01 whole gate §10 (typed
  contributions), NPC-0 (service shapes), RANGED-0 §3.3 (the Extra slot), owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| EQUIP-CONTENT-1 | content lane | typed ability rows on Item definitions (§3.1), `equip_to`/`unequip_to`/`expire_to` targets, `time_mode` and duration, charges; from TibiaWiki first, Canary `items.xml` as fallback | ITEM-SEM-2b |
| EQUIP-RT-1 | hard (combat), combat and determinism review | the equipment owner's active set and the Reference modifier plan (§3); effects at their stages (§4); recomputation triggers | ITEM-MOVE-2a; COND-1; SPEED-1 |
| EQUIP-TIME-1 | hard (persistence), persistence review | equip and unequip transforms (§5.1); duration budgets with checkpoints and expiry (§5.2); `ItemLifeCause` (§7) | EQUIP-RT-1; DUR-03 amendment accepted |
| EQUIP-CHARGE-1 | hard (persistence), persistence and combat review | the charge reserve (§6); expiry at zero charges | EQUIP-TIME-1 |
| EQUIP-REPAIR-1 | impl, economy review | NPC repair of worn soft boots (§5.4) | EQUIP-TIME-1; GOLD-FEE-2 |
| EQUIP-PARITY-1 | impl | fixtures: protection sums, skill and speed values, ring and boot durations, charge counts | EQUIP-RT-1 |

No new command or capability. Effects show through existing domains: skills and stats (A13 and
SKILLS-0 views), speed (VIS-2), conditions (CONDITIONS-0 icons), light (VIS-2 `light`); remaining
time and charges are item sub-state in domain 9 (ITEM-MOVE-WIRE-1).

Later, each with its own decision: exercise weapons and offline training (owner answer 2a),
invisibility from equipment (with the invisibility family), torches burning on the Ground, the Trap
item, item durability that is not time or charges (none in the Reference base), set bonuses
(none in Tibia).

## 1. Question

What does a worn item do for its wearer, and how do rings, soft boots, torches and charged amulets
run out without losing or creating value?

## 2. Facts

**PROVEN**

- GAME-ITEM-01 §6.4: every item modifier resolves to a typed definition with target, phase,
  priority and parameters, and every consuming ruleset publishes one deterministic evaluation plan.
  §4.2: charges are a typed bounded value, not stack quantity. §4.4: temporal items state "durable
  absolute deadline" or "authoritative active-time budget" semantics; FND-03 owns the clock.
- DUR-03 §17 classes `STATE_MUTATION` ("same lifecycle changes legal typed state under explicit
  cause/rule") and `TRANSFORM`; §16.1 requires each transform to choose `PRESERVE_INSTANCE` or
  `REPLACE_INSTANCE`.
- A12 (D146): each Tibia ID is its own definition key `oteryn:item.tibia.i<id>`, so a ring and its
  active form are two definitions.
- IMBUE-FORGE-0 §4.3: imbuement time is kept in memory, checkpointed every 60 s and at logout,
  transfer and every DUR-03 transaction touching the item; a crash returns at most one interval to
  the player ("No item or gold is created"); a failed checkpoint suspends the effects. §5: protections
  are "absorb percent, summed with other absorbs" at GAME-ABILITY-01 §10; skill boosts are a derived
  read, not a build-state write.
- ITEM-MOVE-WIRE-1 §4: level and vocation are checked at equip; an item stays equipped when they
  drop; Premium benefits stop when Premium ends.
- CONDITIONS-0 §4.1: effective speed adds "worn-equipment speed (from the item definitions, summed by
  the equipment owner)". WORLD-INTERACTION-0 §11.4: creature light is the higher of the `LIGHT`
  condition and the brightest equipped item.
- SKILLS-0: "offline training, exercise weapons, skill boosts from equipment and imbuements" are
  later decisions.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- Abilities apply when an item is equipped in its own slot and its requirements hold
  (`movement.cpp:508-636`): invisibility and mana shield as slot-keyed conditions, speed, condition
  suppressions, a slot-keyed regeneration condition (`healthgain`/`healthticks`,
  `managain`/`manaticks`), flat skill and stat modifiers, percent stats; `transformEquipTo` on
  equip; the reverse on unequip.
- Protections (`player.cpp:3913-3967`): after armor and defence, for each enabled slot in slot
  order, imbuement absorbs then the item's `absorbpercent` (plus `fieldabsorbpercent` against
  fields); each absorbing item loses one charge if it has charges.
- Duration (`item.cpp:3635-3641`): while decaying the item holds an absolute timestamp; with
  `stopduration` the remaining time is stored when it stops (unequipped). Zero charges or zero time
  turn the item into its `decayTo` or remove it (`game.cpp` `transformItem`).
- Examples: time ring `transformequipto`, `stopduration`; might ring 20 charges, 20% absorb in seven
  elements; stone skin amulet 5 charges, 80% physical and death; soft boots `healthgain 3`,
  `managain 12`, `healthticks 6000`, duration 14,400 s, decaying to worn soft boots.

## 3. The equipment owner (EQUIP-RT-1)

### 3.1 Typed abilities

Item definitions carry a closed set of typed abilities (GAME-ITEM-01 §4.8, no attribute bag):

| Ability | Parameters | Consumer |
|---|---|---|
| `SKILL_BOOST` | skill (the seven weapon skills, magic level), flat amount | derived skill read (SKILLS-0, A13) |
| `STAT_BOOST` | max health or max mana, flat or percent | derived vitals maxima |
| `SPEED` | flat amount | CONDITIONS-0 §4.1 equipment term |
| `PROTECTION` | element (11 combat types), percent; optional `field_only` | GAME-ABILITY-01 §10 incoming contribution |
| `REGENERATION` | health and mana gain, interval | the `EquipmentRegeneration` condition family (§4) |
| `MANA_SHIELD` | none | the existing family, sourced by the slot (§4) |
| `SUPPRESS` | condition family | CONDITIONS-0 admission (immunity while worn) |
| `LIGHT` | level, colour | WORLD-INTERACTION-0 §11.4 |

### 3.2 When an item is active

- An item is **active** when it sits in the slot its definition names, it met its requirements at
  equip (ITEM-MOVE-WIRE-1 §4), its Premium requirement holds now, and it is not out of time or
  charges. An item in the Extra slot (RANGED-0 §3.3) contributes only `LIGHT` (the manual's torch).
- The channel runtime's equipment owner keeps each actor's active set and recomputes it on equip,
  unequip, transform, expiry, charge reserve changes and Premium changes. It writes nothing durable.

### 3.3 The Reference evaluation plan (GAME-ITEM-01 §6.4)

- Contributions are collected in slot order (head, necklace, container, armor, right hand, left
  hand, legs, feet, ring, extra), then definition key, then ability order on the definition.
- Flat skill, stat and speed boosts sum. Percent stat boosts apply to the base value, then flats
  add.
- **Protections sum** per element across items, imbuements (IMBUE-FORGE-0 §5) and other sources
  at the GAME-ABILITY-01 §10 incoming stage, after armor and defence, and the sum is capped at 100.
  Canary applies them one slot after another (multiplicatively); the sum is IMBUE-FORGE-0's
  accepted rule and stays `PARITY_PENDING` until EQUIP-PARITY-1 checks it against TibiaWiki.

## 4. Effects (EQUIP-RT-1)

- Skills, stats, speed, protections and light are derived reads; nothing is written to A13 or the
  build state.
- `REGENERATION` and `MANA_SHIELD` are CONDITIONS-0 instances whose conflict key carries the source
  `Equipment {slot}` (Canary keys them by slot apart from spell conditions), so an equipment mana
  shield and a spell's coexist; applied when the item becomes active and removed when it stops,
  with no duration of their own. `EQUIPMENT_REGENERATION` is a new family beside
  `FOOD_REGENERATION`, so both run together. Amended: CONDITIONS-0 §3 (this PR).
- Invisibility from equipment waits for the invisibility family (CREATURE-AI-0 names it a later
  CONDITIONS family); until then such items grant nothing else.
- `SUPPRESS` makes CONDITIONS-0 refuse admission of that family while the item is active, and
  removes an existing instance when it becomes active (Canary `conditionSuppressions`).

## 5. Timed items (EQUIP-TIME-1)

### 5.1 Equip and unequip forms

- A definition may name `equip_to` and `unequip_to` (Canary `transformEquipTo`/
  `transformDeEquipTo`). Equipping a time ring makes it its active definition; unequipping makes it
  the inactive one. The change is a `TRANSFORM` line with `PRESERVE_INSTANCE` inside the same
  equip or unequip transaction (ITEM-MOVE-2a), carrying the time and charge state over.

### 5.2 Time

- `time_mode` on the definition: `WHILE_EQUIPPED` (rings, soft boots: an active-time budget that
  runs only while the item is active and the character is in the world) or `WHILE_HELD` (a lit
  torch: a budget that runs while the item is in the character's equipment or own trees and the
  character is in the world). Nothing runs offline, in a depot or in a mailbox (Tibia). A lit torch
  left on the Ground does not burn down (Tibia's does; `PARITY_PENDING`, deferred: Ground timers
  have no item writer yet).
- State: `remaining_ms` as typed temporal state (GAME-ITEM-01 §4.4, active-time budget), set from
  the definition's duration at MINT.
- **Accounting** follows IMBUE-FORGE-0 §4.3 exactly, with its item-only composition (§4.1 there): memory holds the exact value; a checkpoint
  `STATE_MUTATION` writes it after each 60 s of running (`EQUIP0-RL-01`), at logout, channel
  transfer, reconnect loss and inside every DUR-03 transaction touching the item; a crash returns at
  most one interval to the player; a failed checkpoint suspends the item's effects and its running
  until a checkpoint commits. Checkpoints are batched with imbuement checkpoints of the same
  character in the same bounded item-only transactions (`IMBFORGE0-RL-13`).
- **Expiry** at 0: effects stop at once; one audited transaction under `ItemLifeCause::Expire`
  either transforms the item into its `expire_to` definition (`PRESERVE_INSTANCE`; soft boots
  become worn soft boots) or burns it (a ring, a burnt-out torch). Until it commits, the item is
  inactive.

### 5.3 Lighting a torch

Using an unlit torch (a whole, non-stackable item) makes it its lit definition: one whole-instance
`TRANSFORM` (`PRESERVE_INSTANCE`) under ITEM-USE-0 as a new variant `ItemUseCause::Transform` keyed by
the CommandRef, in ITEM-USE-0's item slot with its reservation and cooldown rules. Using a lit torch
puts it out the same way. The lit torch is a `WHILE_HELD` item that gives `LIGHT` from any slot, the
Extra slot included. Amended: ITEM-USE-0 §4.2.

### 5.4 Repairing soft boots (EQUIP-REPAIR-1)

An NPC that offers the repair (content, TibiaWiki) turns worn soft boots into fresh ones for its
price: in one transaction a `TRANSFORM` (`PRESERVE_INSTANCE`, `remaining_ms` reset to the
definition's duration) under `ItemLifeCause::Repair {npc, offer}` and the fee under
`FeeBurnCause::Repair` (coins first, then the bank, as the gold fee amendment). Refused with nothing
written when the boots or the money are missing.

## 6. Charges (EQUIP-CHARGE-1)

- `charges` as typed charge state (GAME-ITEM-01 §4.2), set at MINT. An active `PROTECTION` item with
  charges spends one charge on each hit it reduces (Canary); charged items with other abilities
  spend per their content rule.
- **Charges are value; time is not.** A crash may return time (IMBUE-FORGE-0) but must never return
  charges. So charges are spent from a **reserve**:
  - when an item becomes active, or its reserve falls to 1, the runtime commits a
    `STATE_MUTATION` that moves `min(4, charges)` (`EQUIP0-RL-02`) from `charges` to
    `reserved_charges` on the item, under `ItemLifeCause::ChargeReserve`;
  - a hit spends from the runtime reserve; with an empty reserve (the refill still in flight) the
    item gives no protection for that hit (fail closed);
  - when the item stops being active, the unspent reserve returns to `charges` in the same
    transaction that stops it (the unequip or move carries the owner lane's unspent count, as
    IMBUE-FORGE-0 writes time inside moves), or by one `STATE_MUTATION` at logout and transfer;
  - a reserve commit and a move of the item serialize on the item row and `character_root` (rule 4);
    the reserve commit requires the item still active in the same slot and loses otherwise, with
    nothing written;
  - after a crash, `reserved_charges` found on a recovered item are spent: the player loses at most
    4 charges per item, never gains one.
- At 0 charges and an empty reserve: expiry as §5.2 (`expire_to` or burn).

## 7. DUR-03 (amended in this PR)

A closed cause `ItemLifeCause {Checkpoint, Expire, ChargeReserve, ChargeReturn, Repair}`, and the
`ItemUseCause::Transform` variant:

| Shape | Lines | Cause |
|---|---|---|
| Equip or unequip form | `TRANSFORM`, `PRESERVE_INSTANCE`, in the move transaction | the move's own cause |
| Checkpoint | `STATE_MUTATION` of `remaining_ms` down, no event | `Checkpoint` |
| Expire | `TRANSFORM` (`PRESERVE_INSTANCE`) to `expire_to`, or one BURN | `Expire`, audited |
| Reserve, return | `STATE_MUTATION` between `charges` and `reserved_charges` | `ChargeReserve`, `ChargeReturn` |
| Repair | `TRANSFORM` (`PRESERVE_INSTANCE`) plus the fee BURN | `Repair`; `FeeBurnCause::Repair` |
| Light a torch | `TRANSFORM` (`PRESERVE_INSTANCE`) | `ItemUseCause::Transform` |

- **Fence and key.** Server-originated writes (checkpoint, expire, reserve, return outside a move)
  use the composition decision's server-originated variant (STARTER-BACKPACK-0): the character's
  current admitted session's `CurrentCharacterItemFence`, no CommandRef, keyed by `(WorldId,
  ChannelId, scope ownership generation, runtime actor id, actor generation, ItemInstanceId, life
  sequence, CharacterId)`; repair and lighting are player commands keyed by their CommandRef (rule
  2). Rule 1: no `CharacterRevision` advance. Rule 4 lock order. Amended: the composition decision.
- **Supersession.** For these shapes only, the §39.1 exclusions of burn, transform and (for repair)
  multiple touched items within the gold fee shape; nested containers for items in the character's
  own trees. Every other §39 obligation is unchanged.

### 7.1 Rows (values fixed here, registered by each child)

| Row | Value |
|---|---|
| `DUR03-RL-01-ITEMLIFE` touched items | 1; a checkpoint batch up to `IMBFORGE0-RL-13` (10), shared with imbuement checkpoints; repair 1 plus the gold fee rows |
| `DUR03-RL-02-ITEMLIFE` lines | 1 per touched item |
| `DUR03-RL-06-ITEMLIFE` participants / work units | 1 / 2 per item |
| `EQUIP0-RL-01` checkpoint interval | 60 s of running |
| `EQUIP0-RL-02` charge reserve | 4 |

## 8. Rejected options

- **Abilities as conditions with durations.** Canary keys them by slot with no duration; a timed
  condition would outlive an unequip.
- **Writing derived skills to the build state.** SKILLS-0 and IMBUE-FORGE-0 keep boosts as derived
  reads; a durable write per equip adds nothing.
- **A runtime charge counter flushed later.** A crash would return spent charges, which are bought
  value.
- **Multiplicative protections now.** IMBUE-FORGE-0 accepted the sum; one rule for every source,
  checked by parity fixtures.
- **Time running offline.** Tibia stops worn items while logged out.

## 9. Architect rulings (owner rule 5905825574)

- **R1. Protection stacking.** a) Sum with other absorbs, capped at 100, `PARITY_PENDING`
  (recommended: one rule with IMBUE-FORGE-0); b) Canary's per-slot multiplication. **Ruled a).**
- **R2. Time accounting.** a) IMBUE-FORGE-0's checkpoint model (recommended: accepted, a crash
  returns at most 60 s); b) a write per tick. **Ruled a).**
- **R3. Charges.** a) A durable reserve of 4, a crash loses at most 4 (recommended: never creates
  value); b) a write per spent charge before the hit (a database wait inside a hit). **Ruled a).**
- **R4. Active-form transforms.** a) `PRESERVE_INSTANCE` between the two A12 definitions
  (recommended: the same ring, one lineage); b) `REPLACE_INSTANCE`. **Ruled a).**

## 10. Owner questions

None. Every choice is a Tibia-parity application or a bound under DUR-03 §28 and owner rule
5905825574.

## 11. Decision test

- **Must decide now:** YES. Without it no armor protects against elements, no ring or amulet works,
  and skill boosts, soft boots and torches do nothing.
- **Minimum sufficient:** one closed ability set, one plan, the accepted imbuement time model, one
  charge reserve, one cause.
- **Superseding evidence:** TibiaWiki or an official source showing multiplicative protections; a
  measured checkpoint load above the DUR-03 §28 envelope.
- **Deliberately not decided:** exercise weapons, the Trap item, Wheel and proficiency bonuses
  (their own decisions), imbuement effects (IMBUE-FORGE-0).

## 12. Before-freeze checklist

1. **Contract amendments:** DUR-03 §15, §39.1 and §39.3 (`ItemLifeCause`, `ItemUseCause::Transform`);
   the composition decision (server-originated item life writes); ITEM-USE-0 §4.2 (`Transform`);
   CONDITIONS-0 §3 (`EquipmentRegeneration`, equipment-sourced instances, `SUPPRESS`); SKILLS-0
   (skill boosts point here); WORLD-INTERACTION-0 §11.4 (carried torches point here). All applied in
   this PR.
2. **Serialization:** item-only writes under the item writer's fence and the `character_root` lock;
   a checkpoint racing a move is superseded by the move's own write of the same value (IMBUE-FORGE-0
   rule); a reserve commit racing an unequip loses and the unequip returns nothing it did not see.
3. **Restart:** `remaining_ms`, `charges` and `reserved_charges` are durable; recovery spends the
   reserve and resumes time from the last checkpoint.
4. **Typed references:** definitions are A12 keys; condition sources are (actor, slot); causes are
   typed per §7.
5. **Wire:** no new message; item sub-state carries time and charges in domain 9.
6. **Split work:** each shape is one item-only transaction; effects follow committed state only for
   charges (reserve) and in-memory state for time (checkpointed).
