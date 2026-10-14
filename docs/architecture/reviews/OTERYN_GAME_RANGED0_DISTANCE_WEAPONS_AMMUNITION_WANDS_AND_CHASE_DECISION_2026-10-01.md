# RANGED-0 Distance weapons, ammunition, wands and chase

- Decision: `RANGED0-DISTANCE-WEAPONS-AMMUNITION-WANDS-AND-CHASE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  persistence, movement and protocol) and protected integration. Amended 2026-10-14 after the
  four-lane review of 2026-10-14 (all four lanes failed; every blocker is closed in §14 and the
  sections it names); the amended head needs the four lanes again.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698, item 1); ATTACK-0 left distance and
  throwing weapons, ammunition, wands and rods and chase movement to their own decision
  (ATTACK-0 brief and §8).
- Builds on: ATTACK-0 (attack target, `AutoAttack` timer, fight modes, domain 10), GAME-ABILITY-01
  (sole damage authority), DUR-03 §7, §12, §15, §23, §25, §39 (reservation, split, burn, retry,
  the one-item shapes), the composition decision (rules 2-4 and the STARTER-BACKPACK-0
  server-originated variant), RUNE-USE-0 §5-§8 (burn before effect, PREPARE and PRIMARY COMMIT,
  one slot, ambiguous commits), ITEM-MOVE-WIRE-1 §4-§6 and BAGS-0 §3-§6 (equipment, Ground drops,
  containers), SKILLS-0 §2 and §3.4 (distance tries), A13 §4.5 (magic-level training), CONDITIONS-0 §4
  (pacing), CREATURE-AI-0 §5 (paths and steps), VSL-MOVE-01 §3, §5, §8 (movement authority),
  SPELL-PRESENT-0 §5 (projectiles), PARTY-PVP-0 §7 (PvP legality and damage), owner rule
  5905825574 (Tibia parity).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| RANGED-CONTENT-1 | content lane | weapon rows on Item definitions (§7), lowered from the formal schema's mapped `weapontype` and TibiaWiki first, Canary `items.xml` as fallback; quivers | ITEM-SEM-2b |
| QUIVER-1 | impl, persistence review | the quiver tree in the left hand and the Extra slot (§3) | BAGS-1; ITEM-MOVE-2a; BAGS-WIRE-1 |
| RANGED-1 | hard (combat, persistence), combat and persistence review | distance, throwing and wand swings (§4-§5); the weapon-use slot, reservation and `WeaponUseCause` shapes (§6); wand mana | ATTACK-1; QUIVER-1; RANGED-CONTENT-1; ITEM-MOVE-2b; the §6.4 aggregate registered |
| CHASE-1 | **GATED, not allocatable** (hard, movement), movement and determinism review | server-driven chase (§8). Chase stays client-driven (#1942) until the §8 gates hold | an owner decision superseding #1942; CREATURE-MOVE-1 path profile in code; the §8 pacing and wire gates |
| RANGED-PARITY-1 | impl | fixtures of §5 against TibiaPal and Canary; break rates over 10,000 seeded draws | RANGED-1 |

No new command, state domain or capability. Ranged swings reuse `ATTACK_TARGET_INTENT`,
`FIGHT_MODES_INTENT` and domain 10. Quiver use needs `CONTAINER_TREE_V1` (§3.1). Chase stays
client-driven (§8): this decision adds no server movement and no chase wire. **Projectiles:** the
SPELL-PRESENT-0 presentation stream has no protocol registry row yet
(`spell_presentation_candidate.rs`), so RANGED-1 emits no missile until that stream is registered
by its own protocol-reviewed child; until then weapons swing without a projectile (§7). This
decision registers nothing on the wire.

Later, each with its own decision: the `FOLLOW` command, diagonal chase steps (with diagonal
Movement), the Wheel and quiver Perfect Shot values, exercise weapons (owner answer 2a).

## 1. Question

How does a paladin shoot, how does ammunition run out without duplicating or losing items, how do
sorcerers and druids attack with wands and rods, and who moves a player in chase mode?

## 2. Facts

**PROVEN**

- ATTACK-0 §4: one target and one `AutoAttack` timer per actor, 2,000 ms, `DEADLINE_STATE`; RNG
  purposes closed to `hit_chance` (reserved for distance), `damage_draw`, `defence_draw`,
  `armor_draw`; "a distance weapon, throwing weapon, wand or rod in the hand is treated as no
  weapon for auto-attack until its own decision". §3: `chase` "has no effect in the first slice".
- The Tibia manual (`docs/reference/tibia-manual/combat.md` §5.3.6): the melee cadence; ammunition
  and weapons below their minimum level "can't be used"; hit chance falls with distance; a
  point-blank penalty; bows and crossbows are two-handed; thrown weapons can vanish.
  `interface.md` §3.4.1: the shield slot holds a shield, spellbook or quiver (paladin); paladins keep
  the quiver with a two-handed bow; the old ammunition slot is the free-form Extra Slot.
- `apps/game-server/src/domain/equipment.rs`: the right hand is the weapon hand; the quiver is a
  left-hand item and the only one allowed beside a two-handed distance weapon.
- DUR-03: §7.1 reservation makes a reserved unit unavailable to competing moves; §12 split into a
  planned identity (§11.3); §15 closed burn causes; RUNE-USE-0 §5 burn before effect; §39.1
  excludes burn, multiple touched items, quantity redistribution and nested containers outside
  admitted shapes; §28 ceilings must be fixed before implementation.
- Composition decision rule 2 binds a cause to a CharacterId and a fence; rule 3 proves ingress by a
  pending CommandRef; the STARTER-BACKPACK-0 amendment admits a server-originated variant fenced by
  the admitted session's `CurrentCharacterItemFence` without a CommandRef.
- ITEM-MOVE-WIRE-1 §5: Ground drops (10 items per tile `ITEMMOVE1-RL-01`, 20,000 per channel
  `ITEMMOVE1-RL-02`, house tiles `BLOCKED`, the per-tile row lock and counter); §6.2 the §32 scope
  fence on every Ground write; §6.1 supersessions; §7.2 the extended lock order; §4 refuses containers and unknown equipment semantics in the nine slots. BAGS-0
  §3: trees of depth 8 and 500 items, `GAMEITEM01-REACHABLE-ITEMS` 509; §4.1 entries keyed by
  parent item; §6 "the quiver waits"; capability `CONTAINER_TREE_V1`.
- SKILLS-0 §2 and §3.4: distance 2 tries per unblocked hit, 1 per blocked, a miss repeats the
  previous block state (Canary). A13 §4.5: "each cast's mana cost" accumulates as `mana_spent` in the
  live session, committed as build receipts (that wand mana counts the same is `DERIVED`).
- CONDITIONS-0 §4.3: a player has one pacing clock with a one-step buffer. VSL-MOVE-01 §3: only the
  Movement owner commits a step; §5: a movement occurrence is a source occurrence plus a semantic
  kind; §8: a path needs "its own accepted implementation profile". CREATURE-AI-0 §5: the creature
  step timer, the path profile, the window budget (R3), and the floor, zone and door bans of §5.2.
- PARTY-PVP-0 §7: player targets, `PARTYPVP0-RL-11` 50% PvP damage (100% on a black skull) after the
  damage draw.
- SPELL-PRESENT-0 §5: projectiles are emitted per Effect with a `projectile_asset_binding`.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- Ammunition comes only from the quiver: the first quiver entry whose `ammo_type` matches the bow
  and whose level the player meets (`player.cpp:405-451`); a distance weapon without usable
  ammunition makes no attack at all (`player.cpp` `doAttacking`).
- Range: same floor, Chebyshev distance at most the shoot range (`weapons.cpp:135-147`).
- Hit chance (`weapons.cpp:729-830`): Canary text reads an item `hit_chance` as overriding the
  table; this repository does not follow that reading (§5, ruling R6); the cap is
  `max_hit_chance`, or 90 for ammunition weapons, or 75 for throwing weapons; a table by distance
  1-7 and capped skill (cap 75: `min(skill,74)+1` at 1 and 5, `min(skill,28)*2.40+8` at 2,
  `min(skill,45)*1.55+6` at 3, `min(skill,58)*1.25+3` at 4, `min(skill,90)*0.80+3` at 6,
  `min(skill,104)*0.70+2` at 7; caps 90 and 100 have their own rows; other caps are §5's rule);
  one draw `chance >= uniform(1,100)` hits. A miss lands on the target tile or, beyond adjacent
  range, on a random walkable tile of the 3x3 around it (`:837-859`).
- Damage (`weapons.cpp:899-940`): attack = bow + ammunition (+ ammunition element);
  `max = round(0.09 x attackFactor x distanceSkill x attack + level / 5)`, `min = level / 5`
  (halved against players, quartered with an element); `normal_random(min, max x
  distDamageMultiplier)`; physical and element split by attack share; blocked by armor, not by
  shield.
- Use (`weapons.cpp:335-390`): `break_chance` first (`uniform(1,100) <= break_chance` removes one
  unit); else `removecount` removes one unit and `move` moves one unit to the landing tile, merging
  into a matching stack there. Throwing weapons are distance weapons without an `ammoType`.
- Wands and rods: `normal_random(min, max)` of their element, no hit roll, not blocked by armor or
  shield, mana per shot (`weapons.cpp:344-351, 968-983`).
- Chase follows the attacked creature, re-paths when it moves, waits 2,000 ms after a failed path,
  full search with target distance 1 (`player.cpp:6139-6210`). Reference only: this repository
  keeps chase client-driven (§8).

## 3. Equipment (QUIVER-1)

### 3.1 The quiver

- A quiver is an Item with `container` semantics and the content flag `quiver`. A paladin (content
  vocation) may equip it in the **left hand**, beside a two-handed distance weapon or alone.
- It is the only container admitted in a slot other than the container slot. Its tree has depth 1:
  it admits only direct entries with `weapon_kind = AMMUNITION`, which are never containers
  (`QUIVER_ONLY_AMMUNITION`, refused as `SLOT_MISMATCH` under `CONTAINER_TREE_V1`). Entries are
  keyed to the quiver as their parent item (BAGS-0 §4.1). It is one of the character's own trees
  (BAGS-0 §6): reach, views, invalidation and moves are those of the main backpack tree.
- Equipping or unequipping a non-empty quiver is a BAGS-0 tree move (`DUR03-*-TREE-MOVE` rows); a
  swap where either side is a non-empty tree is refused (`SLOT_MISMATCH`); an empty quiver uses
  ITEM-MOVE-WIRE-1's one-item and swap shapes.
- `GAMEITEM01-REACHABLE-ITEMS` becomes 529 (the main backpack tree, the nine slots and at most 20
  quiver entries).
- **Capability.** Only a session that negotiated `CONTAINER_TREE_V1` may equip a quiver, move it or
  shoot from it. A session without it is refused `SLOT_MISMATCH` on equip, cannot move a quiver
  tree that is equipped, and its swings draw no ammunition (no attack).
- Amended: ITEM-MOVE-WIRE-1 §4 and BAGS-0 §3 and §6 (this PR).

### 3.2 Where ammunition comes from

A weapon with an `ammo_type` shoots the **first direct quiver entry**, in display order, whose
`ammo_type` matches, whose level and vocation requirements the character meets, and that is not
reserved or unspendable (§6). No such entry: no attack.

### 3.3 The Extra slot

The ninth slot keeps its semantic key (`ammo`, GAME-ITEM-01 §6.1) and becomes the manual's Extra
slot: it admits any whole item without `container` semantics, whatever its equipment semantics,
with no level, vocation or Premium check. An item there grants no equipment effect (EQUIP-0 decides
the one exception, light) and is never shot. Amended: ITEM-MOVE-WIRE-1 §4.

## 4. Swings (RANGED-1)

- **One timer, one ability.** Distance, throwing and wand swings are variants of ATTACK-0's
  `AutoAttack`: the same timer, interval, `DEADLINE_STATE` catch-up and in-fight deadline, and the
  same `AutoAttack` ability invocation through GAME-ABILITY-01 (`combat/attack/target.rs` `Swing`).
  There is no separate `WeaponSwing` origin. The right-hand weapon
  decides the variant: `DISTANCE` (ammunition per §3.2), `THROWING` (the hand stack), `WAND`;
  anything else stays melee or fist.
- **Validity** replaces ATTACK-0 §4's adjacency for these variants: same floor, Chebyshev distance
  at most `range_tiles` (content rejects values above 7), line of sight by the check RUNE-USE-0 §8
  step 3 uses, neither actor in a protection zone, re-entry protection respected, PvP legality by
  PARTY-PVP-0 §7. Out of range or sight, the swing waits.
- **Requirements.** Level, vocation and (wands) magic level of the weapon and the ammunition are
  checked at PREPARE; below them the weapon "can't be used" (the manual): no attack. A quiver's
  vocation and level gate is ITEM-SEM's equip check driven by the content flag; QUIVER-1 enforces
  only the container rules of §3.1 and ammunition selection (§3.2) re-checks the entry's own
  requirements. Canary's
  half or zero damage for `wieldunproperly` items is a `PARITY_PENDING` note. A wand needs
  `mana_per_shot` unheld mana; with less it does not swing.
- **The slot.** An actor has at most one weapon use between PREPARE and its outcome. A deadline that
  falls due while it is pending makes no attack; the next deadline counts from that moment (a
  stall, never a backlog). The slot is separate from RUNE-USE-0's rune slot and ITEM-USE-0's item
  slot.

### 4.1 PREPARE (channel owner, at the due deadline)

1. Validity and requirements; select the shot entry (§3.2) or the hand stack.
2. **Draws**, on ATTACK-0 §4's swing occurrence stream (which charms also use) and frozen:
   `hit_chance` (distance and throwing), then `damage_draw`, then `armor_draw` (distance damage is
   blocked by armor; `defence_draw` is unused), then for a miss beyond adjacent range
   `miss_landing`, then for `on_use = DROP` `break`. Wands draw only `damage_draw`. Amended:
   ATTACK-0 §4's closed purposes gain `miss_landing` and `break`; RANGED-1 adds the variants
   `MissLanding` and `Break` to `SwingRngPurpose` (`combat/attack/target.rs`, which today holds
   `HitChance`, `DamageDraw`, `DefenceDraw`, `ArmorDraw`).
3. **Freeze**: target identity, landing tile, damage magnitudes, and the consequence by `on_use`
   (§6.1), decided in this order: (a) a landing tile that is not walkable or is a house tile fixes
   a burn; (b) otherwise a merge receiver (§6.1) is selected and reserved first, and a merge
   consumes no tile or channel limit, so a tile at its limit still takes it; (c) only without a
   merge receiver is the new-item path checked, and a tile or channel limit reached (as the
   database would refuse) fixes a burn. Canary throws with `FLAG_NOLIMIT`; the limits are this
   repository's protection rows (`ITEMMOVE1-RL-01`, `-RL-02`), so a burn at a full tile is a
   recorded deviation, limited to tiles at that limit.
4. Wands: take a mana hold of `mana_per_shot` (RUNE-USE-0 §7 hold rules).
5. Reserve the shot unit under DUR-03 §7.1, **allocate the TransactionId** (a UUIDv7, DUR-03 §20:
   exactly one, before the first commit attempt) and freeze it with the draws, then send the
   transaction. Wands send none.

A refusal in step 1 makes no attack and spends nothing.

### 4.2 Commit and PRIMARY COMMIT

- **Known commit.** In the owner lane: the caster must still be the same runtime actor in the
  channel; the target must be alive, on the same floor, not in a protection zone, and still PvP
  legal. Then the frozen effect runs through GAME-ABILITY-01 as the `AutoAttack` ability: damage on a
  hit, none on a miss. A target that fails the recheck is not hit; the unit is spent.
- **Known abort.** DUR-03 §25 retries the same transaction while it can; a terminal refusal
  releases the reservation (and a wand's mana hold), and the swing has no effect (nothing spent).
- **Ambiguous commit.** After `RANGED0-RL-02` (2,000 ms, as `ITEMUSE0-RL-04`) the slot is freed; the
  shot unit stays reserved and unspendable until reconciliation reads the receipt; a commit known
  only then applies no late effect.
- **Wands** have no transaction: PRIMARY COMMIT follows PREPARE in the same owner turn, and the
  mana hold settles there.
- **Afterwards (ruling R7):** skill tries to SKILLS-0 by its rule (2 unblocked, 1 blocked, a miss
  repeats the previous block state); wand mana to A13 §4.5's `mana_spent`; the in-fight deadline
  refreshes. These are live-session accumulators applied **once**, in the owner lane, when the
  outcome is first known committed. They have no durable key of their own: a crash between the
  commit and the accumulation loses that swing's tries or mana_spent (a bounded loss of at most
  one swing's progress, never a duplication). A replay or a reconciliation read returns the first
  outcome and **credits nothing**. A later durable key from SKILLS-0 or A13 supersedes this.
- **Channel transfer and logout** wait for the slot's outcome, within the same bound.
- **No ammunition.** A `DISTANCE` swing with no usable entry (§3.2), or from a session without
  `CONTAINER_TREE_V1`, makes no attack, spends nothing, keeps the target and waits like an
  out-of-range swing (a stall, not a cleared target). No client text is defined (no new wire);
  a "no ammunition" message is `PARITY_PENDING` and a later protocol decision.
- **Hooks.** Charm hooks that key on an `AutoAttack` hit (`combat/charm_effects.rs`) fire for a
  `DISTANCE` or `THROWING` hit by the melee rules and do not fire for a miss; none fires for a
  `WAND` swing (elemental, no hit roll; `PARITY_PENDING`). Proficiency effects
  (`content/project/v2/proficiency`) take no part in a distance, throwing or wand swing until their
  own decision. RANGED-1 audits `charm_effects.rs` against this rule.
- **Creatures** keep CREATURE-AI-0 §4.4: their ranged attacks are think entries, not this timer.

## 5. Formulas (RANGED-1)

- **Hit chance (ruling R6).** One meaning for each field:
  - `max_hit_chance` selects the cap (default 90 for `DISTANCE` with ammunition, 75 for
    `THROWING`); `hit_chance` on the bow, the ammunition and a throwing item is a **signed relative
    modifier in percentage points, added** (never an override; the pinned source carries values
    such as -20, 1, 3, 7). Content bounds it to [-100, 100].
  - `chance = min(cap, table(cap row, distance, min(distance skill, row ceiling)) + bow modifier +
    ammunition modifier)`, clamped to [0, 100]; one draw `chance >= uniform(1, 100)` hits.
  - **Cap rows.** The table defines rows for caps 75, 90 and 100. Any other cap `c` (the packet
    ships 76, 80, 86 and 96) uses the row of the smallest defined cap `>= c` and is then limited
    by `min(c, ...)` above. `PARITY_PENDING`.
  - The point-blank penalty (distance 1 is not exempt from the table) is `PARITY_PENDING`.
- **Distance and throwing damage:** the Canary formula of §2 with ATTACK-0's fight-mode factor
  (it applies to distance and throwing; `PARITY_PENDING`) and the vocation's distance multiplier,
  with the level term of ATTACK-0 §5 (the official 13.05 curve, equal to `level / 5` up to level
  500) in place of Canary's `level / 5`, as `player_expression` trees for `spell/formula.rs` (ATTACK-0
  §5). Blocked by armor only. Element ammunition splits physical and element by attack share;
  resistances apply in GAME-ABILITY-01 mitigation.
- **Wands and rods:** `normal_random(damage_min, damage_max)` of the wand's element, applied through
  GAME-ABILITY-01 mitigation like any elemental damage; no armor, shield or hit roll.
- **Against players:** Canary's lower minimum is part of the draw; PARTY-PVP-0's
  `PARTYPVP0-RL-11` then applies once to the drawn damage. Both are Canary behaviour
  (`PARITY_PENDING`); RANGED-PARITY-1 checks the combined result.
- Every value is `PARITY_PENDING` until RANGED-PARITY-1 matches TibiaPal (official, then TibiaPal,
  then Canary, as ATTACK-0 §5).

## 6. Item consequences (RANGED-1; DUR-03 and composition amendments)

### 6.1 Shapes

The content `on_use` decides:

- **`CONSUME`** (ammunition, and throwing items that always vanish): one BURN of one unit of the shot
  quiver entry or the right-hand stack, which keeps its identity or retires at zero (§11.1, §11.5),
  hit or miss.
- **`DROP`** (spears and other recoverable thrown weapons): the frozen break draw decides. Broken,
  or a drop refused at PREPARE: the same one-unit BURN. Not broken: one unit goes to the landing
  tile's Ground, as in Canary:
  - **Merge** when the tile's **top Ground item** (first row of the existing read order, §6.1.2) is a
    compatible stack with room and not reserved by another transaction: a stackable non-corpse item,
    same definition key and revision, equal state apart from quantity (B3 §4.4), quantity below the
    stack maximum (D82). A corpse, a container or anything else on top means no merge (Canary merges
    only into the top item). The unit moves into it by a
    DUR-03 §13 quantity transfer: the receiver keeps its identity and grows by one; the hand stack
    shrinks by one, or retires at zero (§11.5). No new Ground item, so the tile and channel limits are
    not consumed. This is a Ground-custody quantity transfer of the §6.4 aggregate; it is **not**
    the inventory `FullMerge` (`item_transfer.rs`, main-backpack only), and no cross-custody merge
    is admitted anywhere else.
  - **New item** otherwise: a §12 split into a planned identity (§11.3), or the whole item when it is
    the last unit, at the top of the tile (the next `stack_ordinal`), under ITEM-MOVE-WIRE-1 §5's Ground
    rules (tile and channel limits with the tile row lock and counter, house tiles refused).
  - Merge or new item is chosen at PREPARE and frozen; a merge receiver is reserved under DUR-03 §7.1
    at PREPARE, so no pickup, move or second throw can take it meanwhile: a second thrower that finds
    the top stack reserved has no merge receiver and takes the new-item path under step 3 (c), so on
    a tile at its limit its unit burns, as on any full tile (never a free shot). At commit, after
    the item rows in ItemInstanceId order, the tile row is locked `FOR SHARE` (rule 4's order) and the
    receiver must still be live, on that tile, the top Ground item, compatible and with room;
    otherwise the commit is refused under the same TransactionId (§23) and the swing has no effect
    (the unit stays). A merge takes no tile-limit or channel-counter row.
  - Every Ground write takes ITEM-MOVE-WIRE-1 §6.2's §32 scope fence; D191 reset retirement applies;
    reach and line of sight do not apply.
- **`NONE`** and wands: no transaction.

### 6.1.1 Database deltas (RANGED-1)

- a BURN at zero retires the item (lifecycle RETIRED, the DUR-03 retirement row) and removes the
  quiver entry or the right-hand slot row; the receipt records the retirement. No row is deleted
  without its RETIRED item row and receipt;
- Ground insertion by split with a planned identity, and by whole TRANSFER from the right-hand slot
  under `WeaponUseCause`;
- a quantity change of a live dropped Ground stack by a §13 merge, with the source's retirement at
  zero, and the merge receiver's reservation;
- no new column and no migration: the Ground order is the existing `stack_ordinal` (§6.1.2);
- the weapon receipt keyed by the TransactionId (§6.2), with the frozen consequence;
- reservation of the shot entry or hand stack under §7.1.

### 6.1.2 Ground stacking order (uses the existing owner-stamped `stack_ordinal`)

- `game_item_ground_locations.stack_ordinal` already exists (migration 0034): `BIGINT UNIQUE`,
  stamped by the `game_item_ground_stack_stamp` trigger from `game_item_ground_stack_ordinal_seq`
  under the `game_item_ground_owner_lock` advisory lock on every real Ground insertion, never
  assigned by a writer. This decision adds **no column, no sequence, no default and no migration**
  (a column default of `nextval()` would be evaluated per row and bypass the 0034 owner lock).
- **Top item.** The read path is `ORDER BY stack_ordinal DESC NULLS FIRST`
  (`spell_item_transaction.rs`): the first row of that order is the tile's top Ground item. A legacy
  row has `stack_ordinal NULL` ("legacy stack order is unknown", migration 0034) and therefore sorts
  **first**. When the first row has `stack_ordinal NULL` the top is ambiguous and **no merge
  happens** (the unit takes the new-item path). A merge never orders by ItemInstanceId (DUR-03 §13).
- The new-item path inserts a Ground location row; the trigger stamps the next `stack_ordinal`, so
  the new item is the top. A merge inserts no row.
- Map-authored LocalObjects are below every Ground root. Only a tree's root has a Ground row.
- The client's in-tile order and the 256-entity cutoff (MOVE-RL-11 §4.3) are unchanged by this
  decision; RANGED-1 does not edit them. Later Ground decisions reuse the same read order.

### 6.2 Cause, transaction identity and fence

- **Cause:** the closed `WeaponUseCause {Ammunition, Throwing}`.
- **Transaction identity (ruling R8).** The TransactionId is a UUIDv7 allocated at PREPARE (§4.1
  step 5) before the first commit attempt (DUR-03 §20) and frozen with the swing; every retry of the
  same swing (§4.2, DUR-03 §25) reuses it. It is **not derived** from any tuple. The durable
  receipt is unique on it. Within a live owner a replay of the swing returns the first outcome.
  Across a restart nothing replays: an uncommitted swing has no effect and the next swing is a new
  logical intent with a new TransactionId (a commit that completed but was never observed costs the
  player that unit without damage, never a duplicate; reconciliation reads the receipt by the
  TransactionId only while the owner lives).
- **Provenance** (audit only, not an identity): `(WorldId, ChannelId, scope ownership generation,
  runtime actor id, CharacterId)` plus the fence below. No "actor generation" or swing sequence is
  relied on for uniqueness, so no durable high-water mark is needed.
- **Fence:** the composition decision's server-originated variant (as STARTER-BACKPACK-0): the
  actor's current admitted session's `CurrentCharacterItemFence` (GameSession, lease and scope
  generations), its CharacterId and WorldId equal to the key's, no CommandRef; plus the §32 scope
  fence for a Ground drop. Lock order: rule 4 as extended by ITEM-MOVE-WIRE-1 §7.2 and BAGS-0 §4.2
  (`character_root`, item rows in ItemInstanceId order, the container-slot row, then the Ground tile
  row and the channel counter). Composition rule 1 (no `CharacterRevision` advance for an item-only
  transaction) applies to `CharacterInventory` custody only; a Ground drop is world custody and the
  receipt records the custody it touched. Amended: composition decision (specified here, §13).
- **Audit:** one weapon-use event per consequence, in the §6.4 aggregate.

### 6.3 Rows (values fixed here, registered by RANGED-1)

These are the caps of the §6.4 named aggregate. The one-item caps (`DUR03-RL-01`, `-02`, `-06`,
`-07` and the ceilings in `dur03_native_one_item_audit.rs`) are **unchanged**.

| Row | Value |
|---|---|
| `DUR03-RL-01-WEAPON` touched items | burn 1; split drop 2 (source and new item); whole drop 1; merge 2 (source and receiver) |
| `DUR03-RL-02-WEAPON` location lines; quantity changes | burn 1 removal at zero, else 0 location and 1 quantity change; split 1 location, 2 quantity changes; whole drop 2 location; merge 0 location (1 removal when the source retires), 2 quantity changes |
| `DUR03-RL-06-WEAPON` participants / effect work units | one participant per touched item: burn 1 / 3 (participant, removal, retirement at zero); split 2 / 4; whole drop 1 / 3; merge 2 / 5 (source participant, decrement, retirement at zero; receiver participant, increment); with max and max+1 boundary tests |
| `DUR03-RL-07-WEAPON` envelope and payload | the one-item caps; RANGED-1 proves the split and the merge receiver (up to 256 B, as B3 §4.5) within them |
| `RANGED0-RL-01` weapon transactions in flight per channel | 256; above it a due swing makes no attack (a stall) |
| `RANGED0-RL-02` ambiguous commit bound | 2,000 ms |

### 6.4 The `WeaponUseAggregateV1` amendment (DUR-03 §15, §39.1, §39.3; B3-style)

This is the amendment text. It activates on acceptance of this decision and supplies no runtime
authority until RANGED-1 is allocated.

- **Why an aggregate.** The drop shapes touch two items and two participants (split: source and new
  item; merge: source and receiver). That exceeds the one-item caps that DUR-03 §39.1 fixes, so the
  shapes cannot be a one-item transaction. They form a named B3-style aggregate with its own caps
  (§6.3), as B3 did for its shapes (B3 §4.4-§4.5).
- **DUR-03 §15** gains the closed burn causes `WeaponUseCause::Ammunition` and `::Throwing`.
- **DUR-03 §39.1** (exclusions of burn, multiple touched items, quantity redistribution) gains one
  exception: `WeaponUseAggregateV1`, and only its four closed shapes: `BURN_ONE`,
  `DROP_SPLIT_NEW`, `DROP_WHOLE` and `DROP_MERGE_GROUND`, over the touched custodies named in §6.1.
  Nested containers stay excluded except the depth-1 quiver entry (§3.1).
- **DUR-03 §39.3** gains the binding of the aggregate: the cause key and fence of §6.2, the lock
  order of §6.2, the receipt keyed by the TransactionId, the RETIRED row at zero, and the server-
  originated variant without a CommandRef.
- **Registered by RANGED-1:** the §6.3 rows with max and max+1 boundary tests; the receipt and audit
  tables; no migration for the Ground order (§6.1.2).
- Until accepted and implemented nothing here changes `dur03_native_one_item_audit.rs` or the
  registered one-item rows.

## 7. Content (RANGED-CONTENT-1)

- On Item definitions: `weapon_kind` (`DISTANCE`, `THROWING`, `AMMUNITION`, `WAND`) lowered from
  the formal schema's mapped `weapontype` (`OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`), with
  `ammo_type`, `range_tiles` (1-7), `hit_chance`, `max_hit_chance`, `break_chance_pct`, `on_use`
  (`CONSUME`, `DROP`, `NONE`), `mana_per_shot`, wand `damage_min`/`damage_max` and `element`, the
  `missile` appearance key, level, magic level and vocation; quivers with the `quiver` flag. Sources
  and a `PARITY_PENDING` flag where TibiaWiki and Canary differ.
- **Presentation:** a weapon projectile is a `missile` event from the shooter to the landing tile
  with the shot item's (or wand's) `missile` key, emitted at PRIMARY COMMIT, hit or miss, then the
  §5 hit rows for committed damage. It rides the SPELL-PRESENT-0 stream **once that stream is
  registered** (it has no protocol registry row today); until then no missile is emitted. The wire
  `u16` missile id is the id the stream's `projectile_asset_binding` maps from the content
  `missile` key; this decision defines no second mapping, content supplies only the key, and a key
  with no binding emits no projectile (never a refusal of the swing). Amended: SPELL-PRESENT-0 §5.

## 8. Chase (client-driven; CHASE-1 gated)

- **Decision.** Chase stays **client-driven** (LOOP-COMBAT-MODES-1: "chase stays client-driven
  (#1942)"; `attack/tests.rs` asserts the same). The server adds no chase movement, no path, no step
  timer and no wire. The `chase` fight mode keeps ATTACK-0 §3's "no effect on the server". The
  VSL-MOVE-01 rule that a client route is never authority is unchanged: the client sends ordinary
  step requests, each validated by the Movement owner. ATTACK-WIRE-1's chase toggle semantics are
  those of #1942, not this decision.
- **CHASE-1 is GATED and not allocatable.** A server-driven chase may return only as its own
  decision, after all of:
  1. an owner decision superseding #1942;
  2. the CREATURE-MOVE-1 path profile exists in code (today `ai_think.rs` and `path_proposal.rs` do
     not carry it; VSL-MOVE-01 §8 requires "its own accepted implementation profile");
  3. a **per-actor** pacing clock: today the player's pacing clock is connection-local
     (`connection.rs`, `pacing.rs`), so server steps and client steps cannot be ordered. That
     decision must fix the actor-level clock and the order of buffered steps (client intent wins,
     one pending step);
  4. a protocol decision for what the client sees of a server-chosen path (re-emit and
     correction), since "wire: none new" cannot be shown for server movement.
- Rejected here and kept as research for that future decision: Canary's server chase (§2).

## 9. Rejected options

- **Ammunition from the Extra slot.** The manual made it free-form and Canary reads only the quiver.
- **A runtime ammunition counter flushed later.** A crash would duplicate arrows (DUR-03 §4).
- **Burning a batch of shots ahead.** Unused units would need a refund MINT; one unit per swing is
  the accepted rune shape.
- **Always burning thrown weapons.** Spears that land on the ground are Tibia; the split, merge and
  Ground shapes exist.
- **Never merging on the ground.** Owner answer 1a (#162 5930136984): faithful, as Tibia.
- **Choosing the merge receiver by ItemInstanceId.** DUR-03 §13 forbids it; the Ground `stack_ordinal` is
  Tibia's top item.
- **Server-driven chase now.** It contradicts the owner decision of #1942 and needs an actor-level
  pacing clock, the CREATURE-MOVE-1 path profile and a path wire that do not exist (§8).
- **A separate ranged timer.** It would allow two swings per interval.

## 10. Architect rulings (owner rule 5905825574)

- **R1. Commit order.** a) Burn before the effect, RUNE-USE-0's PREPARE and PRIMARY COMMIT
  (recommended: no duplication or free shot after a crash); b) effect first, reconcile later.
  **Ruled a).**
- **R2. Thrown weapons.** a) Tibia: break chance, else drop one unit, merging into a compatible top
  stack (recommended; owner answer 1a); b) always burn. **Ruled a).**
- **R3. Chase steps.** a) Server-driven through the Movement owner on CREATURE-AI-0's profile with
  player differences; b) client-driven, as the owner decided in #1942 (recommended: no server
  movement without the §8 gates). **Ruled b)** (supersedes the first ruling a)).
- **R4. Overload.** a) No attack for a due deadline above 256 weapon transactions in flight per
  channel (recommended: bounded, no queue, as WORLD-INTERACTION-0 R5); b) a queue. **Ruled a).**
- **R5. Requirements.** a) The manual: below the level the weapon cannot be used (recommended: an
  official source governs); b) Canary's half or zero damage. **Ruled a).**

- **R6. Hit chance.** a) Item `hit_chance` overrides the table (Canary text); b) signed relative
  points added to the table, capped by `max_hit_chance` (recommended: the pinned data holds
  negative values that an override cannot express, and the content lowering already treats it as
  relative). **Ruled b).** Caps without a defined row use the next higher row, limited to the cap.
- **R7. Post-commit accumulators.** a) A new durable key per swing for skill tries and wand mana;
  b) once-per-outcome live accumulation with a bounded loss on a crash and no duplication
  (recommended: no new durable surface; SKILLS-0 and A13 own those accumulators). **Ruled b).**
- **R8. Transaction identity.** a) A TransactionId derived from a swing tuple; b) an allocated
  UUIDv7 frozen at PREPARE, reused by retries (recommended: DUR-03 §20 and the UUIDv7 audit check
  require an allocated identity; no durable counter is needed). **Ruled b).**

## 11. Owner questions

None. Every choice is a Tibia-parity application or a bound under DUR-03 §28 and owner rule
5905825574.

## 12. Decision test

- **Must decide now:** YES. Without it paladins, sorcerers and druids cannot auto-attack.
- **Minimum sufficient:** no new wire; one burn cause with the accepted rune pattern; one container
  admission; the existing Ground `stack_ordinal`; no server chase.
- **Superseding evidence:** an official formula, TibiaPal disagreeing with Canary, or a measured
  database cost of one transaction per shot above the DUR-03 §28 envelope.
- **Deliberately not decided:** `FOLLOW`, diagonal steps, Perfect Shot values, exercise weapons,
  creature ranged attacks (CREATURE-AI-0).

## 13. Before-freeze checklist

1. **Contract amendments**, specified in this decision and active on its acceptance (not edited
   into the contracts by this PR): DUR-03 §15, §39.1 and §39.3 (§6.4 `WeaponUseAggregateV1`); the
   composition decision (server-originated swing variant, rule 1 scope, §6.2); ITEM-MOVE-WIRE-1 §4
   (quiver, Extra slot) and §5 and §6.2 (no schema change; Ground order is the existing
   `stack_ordinal`); BAGS-0 §3, §6 and §10 (quiver tree, reachable items); ATTACK-0 §4 (variants,
   RNG purposes); SPELL-PRESENT-0 §5 (weapon projectiles, after its stream is registered).
   **Withdrawn from the first draft:** ATTACK-0 §3 (chase), CONDITIONS-0 §4.3, CREATURE-AI-0 §7 and
   MOVE-RL-11 §4.3 (no server chase, no ordinal change).
2. **Serialization:** one weapon-use slot per actor; the shot unit is reserved under DUR-03 §7.1, so a
   concurrent move of that entry is refused; rule 4's lock order; the Ground tile row lock and
   counter for drops; the merge receiver is reserved at PREPARE and rechecked at commit; a known abort
   releases, an ambiguous commit keeps the unit unspendable.
3. **Restart:** the TransactionId is allocated at PREPARE and receipts are unique on it; a committed
   receipt replays only while the owner lives; an uncommitted swing has no effect; chase, target and
   modes are runtime; skill tries and wand mana follow R7.
4. **Typed references:** weapons and ammunition are A12 keys; the shot entry is its ItemInstanceId
   with its parent quiver; provenance is fully typed (§6.2).
5. **Wire:** none new; the quiver is gated by `CONTAINER_TREE_V1`; no chase wire; missiles wait for
   the SPELL-PRESENT-0 stream registration and its u16 binding (§7). A session without
   `CONTAINER_TREE_V1` that logs in with a quiver equipped sees it as an item with no tree and its
   swings draw no ammunition (§3.1); the Extra slot keeps the semantic key `ammo` so older clients
   render it as before.
6. **Split work:** one swing, one transaction committed before the effect, one PRIMARY COMMIT.

## 14. Review amendments (2026-10-14)

| Lane | Blocker | Closed in |
|---|---|---|
| Movement | connection-local pacing clock, unordered buffered steps | §8 gate 3 (no server chase now) |
| Movement | server chase contradicts #1942 | §8, R3 now b), CHASE-1 gated |
| Movement | CREATURE-MOVE-1 profile not in code | §8 gate 2 |
| Protocol | chase conflict in wire comments | §8 (client-driven, nothing changes) |
| Protocol | no client-visible server path | §8 gate 4 |
| Protocol | SPELL-PRESENT-0 stream unregistered | brief note and §7 |
| Protocol | missile u16 has no mapping | §7 |
| Combat | hit_chance override versus add | §5, R6 |
| Combat | cap table covers 75, 90, 100 only | §5 cap rows |
| Combat | `WeaponSwing` versus `AutoAttack`, hooks, RNG purposes | §4, §4.1 |
| Persistence | `ground_ordinal` duplicates `stack_ordinal`, legacy order | §6.1.2 |
| Persistence | `ADD COLUMN DEFAULT nextval()` | §6.1.2 (no column) |
| Persistence | derived TransactionId fails UUIDv7 | §4.1 step 5, §6.2, R8 |
| Persistence | swing-key uniqueness across restarts | §6.2 |
| Persistence | skill tries and mana_spent without a durable key | §4.2, R7 |
| Persistence | DUR-03 amendment text absent, caps exceeded | §6.3, §6.4 |

Non-blocking items taken: PARITY_PENDING point-blank (§5), 13.05 level curve (§5), fight-mode
factor on distance (§5), wand damage through GAME-ABILITY-01 (§5), range plus line of sight in place
of `MELEE_RANGE` (§4), quiver vocation gate belongs to ITEM-SEM (§4), RETIRED row plus receipt on
quiver-entry deletion (§6.1.1), Rule 1 scope (§6.2), no cross-custody merge (§6.1), no-ammo
disposition (§4.2), non-cap-14 quiver rendering and the slot-9 compatibility note (§13.5).
