# RANGED-0 Distance weapons, ammunition, wands and chase

- Decision: `RANGED0-DISTANCE-WEAPONS-AMMUNITION-WANDS-AND-CHASE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  persistence, movement and protocol) and protected integration.
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
| RANGED-1 | hard (combat, persistence), combat and persistence review | distance, throwing and wand swings (§4-§5); the weapon-use slot, reservation and `WeaponUseCause` shapes (§6); wand mana | ATTACK-1; QUIVER-1; RANGED-CONTENT-1; ITEM-MOVE-2b; DUR-03 amendment accepted |
| CHASE-1 | hard (movement), movement and determinism review | chase: server-driven steps toward the attack target through the Movement owner (§8) | ATTACK-1; SPEED-1; CREATURE-MOVE-1 |
| RANGED-PARITY-1 | impl | fixtures of §5 against TibiaPal and Canary; break rates over 10,000 seeded draws | RANGED-1 |

No new command, state domain or capability. Ranged swings reuse `ATTACK_TARGET_INTENT`,
`FIGHT_MODES_INTENT` and domain 10; projectiles reuse SPELL-PRESENT-0's presentation stream. Quiver
use needs `CONTAINER_TREE_V1` (§3.1). ATTACK-WIRE-1 ships the client's chase toggle disabled
(always `STAND`) until CHASE-1 lands, so no released client gains server-driven movement unasked.

Later, each with its own decision: the `FOLLOW` command, diagonal chase steps (with diagonal
Movement), the Wheel and quiver Perfect Shot values, exercise weapons (owner answer 2a).

## 1. Question

How does a paladin shoot, how does ammunition run out without duplicating or losing items, how do
sorcerers and druids attack with wands and rods, and how does chase mode move a player?

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
- Hit chance (`weapons.cpp:729-830`): an item `hit_chance` overrides the table; else the cap is
  `max_hit_chance`, or 90 for ammunition weapons, or 75 for throwing weapons; a table by distance
  1-7 and capped skill (cap 75: `min(skill,74)+1` at 1 and 5, `min(skill,28)*2.40+8` at 2,
  `min(skill,45)*1.55+6` at 3, `min(skill,58)*1.25+3` at 4, `min(skill,90)*0.80+3` at 6,
  `min(skill,104)*0.70+2` at 7; caps 90 and 100 have their own rows); the bow's `hit_chance` adds;
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
  full search with target distance 1 (`player.cpp:6139-6210`).

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

- **One timer.** Distance, throwing and wand swings are variants of ATTACK-0's `AutoAttack`, with
  its interval, `DEADLINE_STATE` catch-up, charm hooks and in-fight deadline. The right-hand weapon
  decides the variant: `DISTANCE` (ammunition per §3.2), `THROWING` (the hand stack), `WAND`;
  anything else stays melee or fist.
- **Validity** replaces ATTACK-0 §4's adjacency for these variants: same floor, Chebyshev distance
  at most `range_tiles` (content rejects values above 7), line of sight by the check RUNE-USE-0 §8
  step 3 uses, neither actor in a protection zone, re-entry protection respected, PvP legality by
  PARTY-PVP-0 §7. Out of range or sight, the swing waits.
- **Requirements.** Level, vocation and (wands) magic level of the weapon and the ammunition are
  checked at PREPARE; below them the weapon "can't be used" (the manual): no attack. Canary's
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
   ATTACK-0 §4's closed purposes gain `miss_landing` and `break`.
3. **Freeze**: target identity, landing tile, damage magnitudes, and the consequence by `on_use`
   (§6.1). A tile that cannot take the drop at PREPARE (not walkable, house,
   tile or channel limit reached, as the database would refuse) fixes a burn now.
4. Wands: take a mana hold of `mana_per_shot` (RUNE-USE-0 §7 hold rules).
5. Reserve the shot unit under DUR-03 §7.1 and send the transaction. Wands send none.

A refusal in step 1 makes no attack and spends nothing.

### 4.2 Commit and PRIMARY COMMIT

- **Known commit.** In the owner lane: the caster must still be the same runtime actor in the
  channel; the target must be alive, on the same floor, not in a protection zone, and still PvP
  legal. Then the frozen effect runs through GAME-ABILITY-01 with origin `WeaponSwing`: damage on a
  hit, none on a miss. A target that fails the recheck is not hit; the unit is spent.
- **Known abort.** DUR-03 §25 retries the same transaction while it can; a terminal refusal
  releases the reservation (and a wand's mana hold), and the swing has no effect (nothing spent).
- **Ambiguous commit.** After `RANGED0-RL-02` (2,000 ms, as `ITEMUSE0-RL-04`) the slot is freed; the
  shot unit stays reserved and unspendable until reconciliation reads the receipt; a commit known
  only then applies no late effect.
- **Wands** have no transaction: PRIMARY COMMIT follows PREPARE in the same owner turn, and the
  mana hold settles there.
- **Afterwards:** skill tries to SKILLS-0 by its rule (2 unblocked, 1 blocked, a miss repeats the
  previous block state); wand mana to A13 §4.5's `mana_spent`; the in-fight deadline refreshes.
- **Channel transfer and logout** wait for the slot's outcome, within the same bound.
- **Creatures** keep CREATURE-AI-0 §4.4: their ranged attacks are think entries, not this timer.

## 5. Formulas (RANGED-1)

- **Hit chance:** the Canary table of §2 for caps 75, 90 and 100 by Chebyshev distance 1-7 and the
  live distance skill; an item `hit_chance` overrides; the bow's adds.
- **Distance and throwing damage:** the Canary formula of §2 with ATTACK-0's fight-mode factor and
  the vocation's distance multiplier, as `player_expression` trees for `spell/formula.rs` (ATTACK-0
  §5). Blocked by armor only. Element ammunition splits physical and element by attack share;
  resistances apply in GAME-ABILITY-01 mitigation.
- **Wands and rods:** `normal_random(damage_min, damage_max)` of the wand's element; no armor,
  shield or hit roll.
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
  - **Merge** when the tile's **top Ground item** (the highest Ground ordinal, §6.1.2) is a
    compatible stack with room and not reserved by another transaction: a stackable non-corpse item,
    same definition key and revision, equal state apart from quantity (B3 §4.4), quantity below the
    stack maximum (D82). A corpse, a container or anything else on top means no merge (Canary merges
    only into the top item). The unit moves into it by a
    DUR-03 §13 quantity transfer: the receiver keeps its identity and grows by one; the hand stack
    shrinks by one, or retires at zero (§11.5). No new Ground item, so the tile and channel limits are
    not consumed.
  - **New item** otherwise: a §12 split into a planned identity (§11.3), or the whole item when it is
    the last unit, at the top of the tile (the next Ground ordinal), under ITEM-MOVE-WIRE-1 §5's Ground
    rules (tile and channel limits with the tile row lock and counter, house tiles refused).
  - Merge or new item is chosen at PREPARE and frozen; a merge receiver is reserved under DUR-03 §7.1
    at PREPARE, so no pickup, move or second throw can take it meanwhile: a second thrower that finds
    the top stack reserved takes the new-item path (a spear is never burnt for it). At commit, after
    the item rows in ItemInstanceId order, the tile row is locked `FOR SHARE` (rule 4's order) and the
    receiver must still be live, on that tile, the top Ground item, compatible and with room;
    otherwise the commit is refused under the same TransactionId (§23) and the swing has no effect
    (the unit stays). A merge takes no tile-limit or channel-counter row.
  - Every Ground write takes ITEM-MOVE-WIRE-1 §6.2's §32 scope fence; D191 reset retirement applies;
    reach and line of sight do not apply.
- **`NONE`** and wands: no transaction.

### 6.1.1 Database deltas (RANGED-1)

- deletion of a quiver entry or the right-hand slot row by a BURN at zero;
- Ground insertion by split with a planned identity, and by whole TRANSFER from the right-hand slot
  under `WeaponUseCause`;
- a quantity change of a live dropped Ground stack by a §13 merge, with the source's retirement at
  zero, and the merge receiver's reservation;
- the Ground ordinal of §6.1.2;
- the weapon receipt keyed by the swing key (§6.2), with the frozen consequence;
- reservation of the shot entry or hand stack under §7.1.

### 6.1.2 Ground stacking order (amends ITEM-MOVE-WIRE-1 §5 and §6.2)

- Every Ground location row (`game_item_ground_locations`) gets a `ground_ordinal`, `NOT NULL`,
  assigned by the database from one sequence at insert (a column default), so every Ground writer
  (drops, splits, Ground moves, corpse and loot MINTs, map-item materialization, death drops, tree
  drops) assigns it without code changes; a later insert always has a higher ordinal; it is never
  reused or changed (the rows are immutable). Only a tree's root has a Ground location row, so only
  roots carry it. A merge inserts no row. A channel restart projects the stored ordinals.
- **Migration** (RANGED-1): the column is added with existing rows (corpses and loot) at ordinal 0,
  below every new row; on a tile whose highest ordinal is 0 the top is ambiguous and no merge happens.
- The **top Ground item** of a tile is its live Ground root with the highest ordinal, the item Tibia
  shows on top. Map-authored LocalObjects are below every Ground root. The ordinal never orders by
  ItemInstanceId (DUR-03 §13: "UUID/client list ordering never selects survivor/receiver").
- The client shows the items of one tile in `ground_ordinal` order, top last. Amended: MOVE-RL-11
  §4.3.
- This is the Ground stacking order later Ground decisions reuse (for example a top-item pickup).

### 6.2 Cause, key and fence

- **Cause:** the closed `WeaponUseCause {Ammunition, Throwing}`.
- **Key** (fully typed, unique across restarts): `(WorldId, ChannelId, scope ownership generation,
  runtime actor id, actor generation, swing sequence, CharacterId)`. The swing sequence is
  monotonic within the actor generation. The TransactionId derives from it; the RNG stays on ATTACK-0
  §4's swing occurrence.
- **Fence:** the composition decision's server-originated variant (as STARTER-BACKPACK-0): the
  actor's current admitted session's `CurrentCharacterItemFence` (GameSession, lease and scope
  generations), its CharacterId and WorldId equal to the key's, no CommandRef; plus the §32 scope
  fence for a Ground drop. Lock order: rule 4 as extended by ITEM-MOVE-WIRE-1 §7.2 and BAGS-0 §4.2
  (`character_root`, item rows in ItemInstanceId order, the container-slot row, then the Ground tile
  row and the channel counter). A replay of the key returns the
  first outcome. Amended: composition decision (this PR).
- **Audit:** one `OneItemTransactionV1` event per consequence.

### 6.3 Rows (values fixed here, registered by RANGED-1)

| Row | Value |
|---|---|
| `DUR03-RL-01-WEAPON` touched items | burn 1; split drop 2 (source and new item); whole drop 1; merge 2 (source and receiver) |
| `DUR03-RL-02-WEAPON` location lines; quantity changes | burn 1 removal at zero, else 0 location and 1 quantity change; split 1 location, 2 quantity changes; whole drop 2 location; merge 0 location (1 removal when the source retires), 2 quantity changes |
| `DUR03-RL-06-WEAPON` participants / effect work units | one participant per touched item: burn 1 / 3 (participant, removal, retirement at zero); split 2 / 4; whole drop 1 / 3; merge 2 / 5 (source participant, decrement, retirement at zero; receiver participant, increment); with max and max+1 boundary tests |
| `DUR03-RL-07-WEAPON` envelope and payload | the one-item caps; RANGED-1 proves the split and the merge receiver (up to 256 B, as B3 §4.5) within them |
| `RANGED0-RL-01` weapon transactions in flight per channel | 256; above it a due swing makes no attack (a stall) |
| `RANGED0-RL-02` ambiguous commit bound | 2,000 ms |
| `RANGED0-RL-03` player chase searches per owner window per channel | 64 (§8) |

## 7. Content (RANGED-CONTENT-1)

- On Item definitions: `weapon_kind` (`DISTANCE`, `THROWING`, `AMMUNITION`, `WAND`) lowered from
  the formal schema's mapped `weapontype` (`OTERYN_ITEM_AUTHORING_FORMAL_SCHEMA_V1.md`), with
  `ammo_type`, `range_tiles` (1-7), `hit_chance`, `max_hit_chance`, `break_chance_pct`, `on_use`
  (`CONSUME`, `DROP`, `NONE`), `mana_per_shot`, wand `damage_min`/`damage_max` and `element`, the
  `missile` appearance key, level, magic level and vocation; quivers with the `quiver` flag. Sources
  and a `PARITY_PENDING` flag where TibiaWiki and Canary differ.
- **Presentation:** a weapon projectile is a `missile` event from the shooter to the landing tile
  with the shot item's (or wand's) `missile` key, emitted at PRIMARY COMMIT, hit or miss, then the
  §5 hit rows for committed damage. Amended: SPELL-PRESENT-0 §5.

## 8. Chase (CHASE-1)

- **Effect.** With `chase = CHASE` and a target, the player's actor steps toward the nearest tile at
  Chebyshev distance 1 from the target, for every weapon (Canary target distance 1). With `STAND` it
  never moves on its own. Amended: ATTACK-0 §3.
- **Path profile** (the profile VSL-MOVE-01 §8 asks for): CREATURE-AI-0 §5.3's costs, tie order,
  `AI01-PATH-SEARCH-WORK` and `AI01-ROUTE-STEPS`, a 12-tile search (Canary searches fully,
  `PARITY_PENDING`). Floor-change and teleport tiles are never path tiles (Canary refuses them to
  every creature when pathing), nor are protection-zone tiles while the player holds a target
  (entering one ends the attack, ATTACK-0 §4). The one player difference: no harmful-field cost
  (Canary applies it to monsters only). The Movement owner's step rules decide each step. At most one search per player per 1,000 ms; a failed search
  waits 2,000 ms (Canary). Player searches have their own row in the writer's window budget,
  `RANGED0-RL-03` (64 searches per window per channel), served in actor-id order after creature
  searches; over it, the search waits for the next window. Amended: CREATURE-AI-0 §7.
- **Steps.** A chase step is one Movement owner step of the player's `ExactActorRef`, revalidated
  like a client step. It uses the player's single pacing clock and one-step buffer (CONDITIONS-0
  §4.3): a chase step is a step request from a server source. The chase step timer is due when the
  pacing clock frees, with at most one pending. Its movement occurrence (VSL-MOVE-01
  §5) is `(chase step timer, actor, chase step sequence)`, kind `CHASE`. A refused step drops the
  path. Amended: CONDITIONS-0 §4.3.
- **Client intent wins.** A client step request cancels the pending chase step and the path; chase
  re-paths at its next 1,000 ms check if the target and chase mode still hold (`PARITY_PENDING`).
- **Ends** when the target is cleared or changes, chase is set to `STAND`, the target leaves the
  floor or perception, or the player is under re-entry protection. A chase never changes floors.
- **Runtime only.** Nothing durable; cleared on reconnect with the target (ATTACK-0 §3).

## 9. Rejected options

- **Ammunition from the Extra slot.** The manual made it free-form and Canary reads only the quiver.
- **A runtime ammunition counter flushed later.** A crash would duplicate arrows (DUR-03 §4).
- **Burning a batch of shots ahead.** Unused units would need a refund MINT; one unit per swing is
  the accepted rune shape.
- **Always burning thrown weapons.** Spears that land on the ground are Tibia; the split, merge and
  Ground shapes exist.
- **Never merging on the ground.** Owner answer 1a (#162 5930136984): faithful, as Tibia.
- **Choosing the merge receiver by ItemInstanceId.** DUR-03 §13 forbids it; the Ground ordinal is
  Tibia's top item.
- **Client-driven chase.** VSL-MOVE-01 forbids a client route as authority; Tibia chases on the
  server.
- **A separate ranged timer.** It would allow two swings per interval.

## 10. Architect rulings (owner rule 5905825574)

- **R1. Commit order.** a) Burn before the effect, RUNE-USE-0's PREPARE and PRIMARY COMMIT
  (recommended: no duplication or free shot after a crash); b) effect first, reconcile later.
  **Ruled a).**
- **R2. Thrown weapons.** a) Tibia: break chance, else drop one unit, merging into a compatible top
  stack (recommended; owner answer 1a); b) always burn. **Ruled a).**
- **R3. Chase steps.** a) Server-driven through the Movement owner on CREATURE-AI-0's profile with
  player differences (recommended); b) client-driven. **Ruled a).**
- **R4. Overload.** a) No attack for a due deadline above 256 weapon transactions in flight per
  channel (recommended: bounded, no queue, as WORLD-INTERACTION-0 R5); b) a queue. **Ruled a).**
- **R5. Requirements.** a) The manual: below the level the weapon cannot be used (recommended: an
  official source governs); b) Canary's half or zero damage. **Ruled a).**

## 11. Owner questions

None. Every choice is a Tibia-parity application or a bound under DUR-03 §28 and owner rule
5905825574.

## 12. Decision test

- **Must decide now:** YES. Without it paladins, sorcerers and druids cannot auto-attack, and
  knights cannot chase.
- **Minimum sufficient:** no new wire; one burn cause with the accepted rune pattern; one container
  admission; one path profile reused for players.
- **Superseding evidence:** an official formula, TibiaPal disagreeing with Canary, or a measured
  database cost of one transaction per shot above the DUR-03 §28 envelope.
- **Deliberately not decided:** `FOLLOW`, diagonal steps, Perfect Shot values, exercise weapons,
  creature ranged attacks (CREATURE-AI-0).

## 13. Before-freeze checklist

1. **Contract amendments**, all applied in this PR: DUR-03 §15, §39.1 and §39.3 (`WeaponUseCause`
   shapes and supersessions); the composition decision (server-originated swing variant);
   ITEM-MOVE-WIRE-1 §4 (quiver, Extra slot), §5 and §6.2 (Ground ordinal); MOVE-RL-11 §4.3 (in-tile
   order); BAGS-0 §3, §6 and §10 (quiver tree, reachable items);
   ATTACK-0 §3 (chase) and §4 (variants, RNG purposes); CONDITIONS-0 §4.3 (chase steps);
   CREATURE-AI-0 §7 (player chase searches); SPELL-PRESENT-0 §5 (weapon projectiles).
2. **Serialization:** one weapon-use slot per actor; the shot unit is reserved under DUR-03 §7.1, so a
   concurrent move of that entry is refused; rule 4's lock order; the Ground tile row lock and
   counter for drops; the merge receiver is reserved at PREPARE and rechecked at commit; a known abort
   releases, an ambiguous commit keeps the unit unspendable.
3. **Restart:** the cause key includes the scope ownership and actor generations; a committed
   receipt replays; an uncommitted swing has no effect; chase, target and modes are runtime.
4. **Typed references:** weapons and ammunition are A12 keys; the shot entry is its ItemInstanceId
   with its parent quiver; the swing key is fully typed (§6.2).
5. **Wire:** none new; the quiver is gated by `CONTAINER_TREE_V1`; the chase toggle ships disabled
   until CHASE-1.
6. **Split work:** one swing, one transaction committed before the effect, one PRIMARY COMMIT.
