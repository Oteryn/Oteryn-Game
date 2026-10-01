# RANGED-0 Distance weapons, ammunition, wands and chase

- Decision: `RANGED0-DISTANCE-WEAPONS-AMMUNITION-WANDS-AND-CHASE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  persistence and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the base-mechanics close-out plan (#162 5929069698, item 1); ATTACK-0 left distance and
  throwing weapons, ammunition, wands and rods and chase movement to their own decision
  (ATTACK-0 brief and §8).
- Builds on: ATTACK-0 (attack target, `AutoAttack` timer, fight modes, domain 10), GAME-ABILITY-01
  (sole damage authority), DUR-03 §12, §15 and §39.3 (split, burn and the one-item burn shapes),
  ITEM-USE-0 and RUNE-USE-0 (the one-unit burn committed before the effect), ITEM-MOVE-WIRE-1 and
  BAGS-0 (equipment and containers), SKILLS-0 (distance tries), A13 (magic level), CREATURE-AI-0
  §5 (paths and the step timer), VSL-MOVE-01 (sole movement authority), SPELL-PRESENT-0
  (projectiles), PARTY-PVP-0 (legality against players), owner rule 5905825574 (Tibia parity).
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| RANGED-CONTENT-1 | content lane | weapon rows on Item definitions: `weapon_kind` (`DISTANCE`, `THROWING`, `AMMUNITION`, `WAND`), `ammo_type`, `range_tiles`, `hit_chance`, `max_hit_chance`, `break_chance_pct`, `on_use` (`CONSUME`, `DROP`, `NONE`), `mana_per_shot`, wand `damage_min`/`damage_max` and `element`, `missile` appearance key, level and vocation; quivers as containers with the `quiver` flag; from TibiaWiki first, Canary `items.xml`/`weapons` as fallback (§7) | ITEM-SEM-2b |
| QUIVER-1 | impl, persistence review | the quiver in the left hand: a container with contents in the shield slot for paladins (§3), the BAGS-0 and ITEM-MOVE-WIRE-1 amendments; the Extra slot (§3.3) | BAGS-1; ITEM-MOVE-2a |
| RANGED-1 | hard (combat, persistence), combat and persistence review | distance, throwing and wand swings as `AutoAttack` variants (§4); hit chance and damage (§5); the `WeaponUseCause` burn and drop shapes (§6); mana for wands | ATTACK-1; QUIVER-1; RANGED-CONTENT-1; DUR-03 amendment accepted |
| CHASE-1 | hard (movement), movement and determinism review | chase mode: server-driven steps toward the attack target through the Movement owner (§8) | ATTACK-1; CREATURE-MOVE-1 (path service and step timer) |
| RANGED-PARITY-1 | impl | fixtures of §5 against TibiaPal and Canary: hit chance per distance and skill, damage bands, break rates over 10,000 seeded draws | RANGED-1 |

No new command, state domain or capability. Ranged swings reuse `ATTACK_TARGET_INTENT`,
`FIGHT_MODES_INTENT` (its `chase` field gains its effect) and domain 10; projectiles reuse
SPELL-PRESENT-0's `WORLD_PRESENTATION` domain. A quiver can be equipped only by a session that
negotiated the BAGS-0 container capability (§3.1).

Later, each with its own decision: the `FOLLOW` command (follow a creature without attacking),
diagonal chase steps (with diagonal Movement), the Wheel and quiver Perfect Shot values, exercise
weapons (offline-training decision, owner answer 2a).

## 1. Question

How does a paladin shoot, how does ammunition run out without duplicating or losing items, how do
sorcerers and druids attack with wands and rods, and how does chase mode move a player?

## 2. Facts

**PROVEN**

- ATTACK-0 §4: one target and one `AutoAttack` timer per actor, 2,000 ms, `DEADLINE_STATE`, the
  RNG purpose `hit_chance` reserved for distance; "a distance weapon, throwing weapon, wand or rod in
  the hand is treated as no weapon for auto-attack until its own decision". §3: `chase` is carried
  and has no effect.
- The Tibia manual (`docs/reference/tibia-manual/combat.md` §5.3.6): ranged attacks use the melee
  cadence; a minimum level per weapon and ammunition; hit chance falls with distance; a point-blank
  penalty at adjacent range; bows and crossbows are two-handed; thrown weapons can vanish.
- The Tibia manual (`interface.md` §3.4.1): the shield slot holds a shield, a spellbook or a quiver
  (paladin); paladins keep the quiver with a two-handed bow; the old ammunition slot is the free-form
  **Extra Slot** (for example a torch).
- `apps/game-server/src/domain/equipment.rs`: the right hand is the weapon hand; the quiver is a
  left-hand item and the only one allowed beside a two-handed distance weapon.
- DUR-03 §15 and §39.3: the admitted burn sinks are closed causes; RUNE-USE-0 made rune use "a
  caller-chosen one-unit BURN, not `DECAY_RETIRE`", committed before the effect. §12 admits a split
  of `x < q` units into a planned identity (§11.3). No cause admits ammunition or a thrown weapon.
- ITEM-MOVE-WIRE-1 §4 refuses every item with `container` semantics in the nine slots ("a quiver or
  a bag"); BAGS-0 §6 keeps that refusal ("the quiver waits").
- SKILLS-0 §3.4: distance gets 2 tries for an unblocked hit and 1 for a blocked one, fed by
  ATTACK-0 distance attacks.
- VSL-MOVE-01 §3: GAME-AI and path workers only propose; the Movement owner derives and commits
  every step. §8: "route/path semantics require their own accepted implementation profile".
- CREATURE-AI-0 §5.1 and §5.3: a step timer family (`DEADLINE_STATE`, one pending step), a path
  profile with a 12-tile chase search, and the writer's window budget (R3).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- Ammunition comes only from the quiver: the first quiver entry whose `ammo_type` matches the bow
  and whose level requirement the player meets (`player.cpp:405-451`). A distance weapon without
  usable ammunition makes no attack at all, not even a fist (`player.cpp` `doAttacking`).
- Range check: same floor and Chebyshev distance at most the weapon's shoot range
  (`weapons.cpp:135-147`); a level or magic level below the requirement gives 0 damage, or half for
  items flagged `wieldunproperly` (`:173-179`).
- Hit chance (`weapons.cpp:729-830`): an item `hit_chance` overrides the table; else the cap is
  `max_hit_chance`, or 90 for weapons that use ammunition, or 75 for throwing weapons; then a table
  by distance 1-7 and capped skill (for cap 75: `min(skill,74)+1` at 1 and 5, `min(skill,28)*2.40+8`
  at 2, `min(skill,45)*1.55+6` at 3, `min(skill,58)*1.25+3` at 4, `min(skill,90)*0.80+3` at 6,
  `min(skill,104)*0.70+2` at 7; for cap 90 and cap 100 their own rows). The bow's own
  `hit_chance` adds to the ammunition's; one draw `chance >= uniform(1,100)` hits.
- A miss lands on the target tile or, beyond adjacent range, on a random walkable tile around it
  (`weapons.cpp:837-859`).
- Damage (`weapons.cpp:899-940`): attack = bow attack + ammunition attack (+ ammunition element);
  `max = round(0.09 x attackFactor x distanceSkill x attack + level / 5)`, `min = level / 5`
  (halved against players, quartered with an element), `normal_random(min, max x
  distDamageMultiplier)`; physical and element split by attack share (`:282-295`); blocked by armor,
  not by shield (`:666-670`).
- Use (`weapons.cpp:335-390`): mana and soul debited; `break_chance` first (`uniform(1,100) <=
  break_chance` removes one unit); else the action: `removecount` removes one unit, `move` moves one
  unit to the landing tile.
- Wands and rods: `normal_random(min, max)` of the wand's element, no hit roll, not blocked by
  armor or shield, mana per shot spent as magic-level mana (`weapons.cpp:344-351, 968-983`).
- Chase: setting a target in chase mode follows it; following re-paths when the target moves; a
  failed follow waits 2,000 ms before the next try; the player path search is a full search with
  target distance 1 (`player.cpp:6139-6210`).

## 3. Equipment (QUIVER-1)

### 3.1 The quiver

- A quiver is an Item with `container` semantics and the content flag `quiver`. It may be equipped
  in the **left hand** by a paladin (content vocation), beside a two-handed distance weapon or
  alone (the existing `NonQuiverLeftHand` rule).
- It is the **only** container admitted in a slot other than the container slot. Its entries are
  keyed to it exactly as the main backpack's are to the container slot (BAGS-0 §3), and its tree is
  bounded by BAGS-0 §3; a quiver admits only items with `weapon_kind = AMMUNITION` (Canary
  `quiver` container restriction, `PARITY_PENDING`).
- Equipping, unequipping and moving items into or out of it use the BAGS-0 and ITEM-MOVE-WIRE-1
  shapes unchanged (a container with contents moves as a tree). A session without the BAGS-0
  container capability is refused `SLOT_MISMATCH` when it equips a quiver: it could not see the
  entries it shoots.
- Amended: ITEM-MOVE-WIRE-1 §4 and BAGS-0 §6 (this PR).

### 3.2 Where ammunition comes from

- A weapon with an `ammo_type` shoots the **first direct quiver entry**, in display order, whose
  `ammo_type` matches and whose level requirement the character meets (Canary). Nested containers
  inside a quiver are never searched.
- No usable ammunition: the swing makes no attack, not a fist attack, and reports nothing.

### 3.3 The Extra slot

The ninth slot keeps its semantic key (`ammo`, GAME-ITEM-01 §6.1) and becomes the manual's Extra
slot: it admits any whole item that is not a container. Nothing in it is ever shot. Amended:
ITEM-MOVE-WIRE-1 §4 (this PR).

## 4. Swings (RANGED-1)

- **One timer.** Distance, throwing and wand swings are variants of ATTACK-0's `AutoAttack`, with
  its interval, `DEADLINE_STATE` catch-up, occurrence key, charm hooks and in-fight deadline. Nothing
  else deals auto-attack damage.
- **Weapon selection.** The right-hand weapon decides the variant: `DISTANCE` (with ammunition per
  §3.2), `THROWING` (the hand stack itself), `WAND`; anything else stays ATTACK-0's melee or fist.
- **Validity** replaces ATTACK-0 §4's adjacency for these variants: same floor, Chebyshev distance
  at most `range_tiles`, line of sight by the GAME-ABILITY-01 projectile query, neither actor in a
  protection zone, re-entry protection respected. Out of range or sight, the swing waits.
- **Requirements.** Level, vocation and (wands) magic level are checked at the swing; below them the
  damage is 0, or half for `wield_unproperly` items (Canary). A wand needs `mana_per_shot` mana; with
  less it does not swing.
- **Order inside one swing** (one GAME-ABILITY-01 invocation):
  1. validity and requirements;
  2. for distance and throwing: the `hit_chance` draw (§5.1); for wands no draw;
  3. the item consequence (§6) is committed durably **before** the effect, as for runes;
  4. on commit success, the effect: damage on a hit, nothing on a miss; on a refused or ambiguous
     commit, no effect (DUR-03 §25 resolves the commit; the swing is spent);
  5. wands: the mana debit and its magic-level mana, through the same vitals path as an instant
     spell's mana cost;
  6. skill tries to SKILLS-0: distance 2 for an unblocked hit, 1 for a blocked one, 0 for a miss
     (`PARITY_PENDING`: Canary reuses the last block type on a miss); wands give none.
- **Attack interval after a stall** keeps ATTACK-0's rule; a pending commit does not delay the
  next deadline.
- **Creatures** keep CREATURE-AI-0 §4.4: their ranged attacks are think entries, not this timer.

## 5. Formulas (RANGED-1)

### 5.1 Hit chance

- The Canary table of §2 for caps 75, 90 and 100, by Chebyshev distance 1-7 and the live distance
  skill; an item `hit_chance` overrides the table; the bow's `hit_chance` adds to the ammunition's;
  PROFICIENCY-0's ranged hit chance bonus adds through its existing hook. One draw of RNG purpose
  `hit_chance` per swing.
- A miss lands on the target's tile, or beyond adjacent range on the first tile, in an order drawn
  with RNG purpose `miss_landing`, among the 3x3 around the target that has ground and is not a
  solid block (Canary).

### 5.2 Damage

- Distance and throwing: the Canary formula of §2, with the fight-mode factor of ATTACK-0 and the
  vocation's distance multiplier, authored as `player_expression` trees for the spell formula
  engine (`spell/formula.rs`), as ATTACK-0 §5 does for melee. Blocked by armor only.
- Element ammunition: the physical and element parts split by attack share (Canary); element
  resistances apply in GAME-ABILITY-01's mitigation stage.
- Wands and rods: `normal_random(damage_min, damage_max)` of the wand's element; no armor, shield or
  hit roll.
- Every value is `PARITY_PENDING` until RANGED-PARITY-1 matches TibiaPal (ATTACK-0 §5 precedence:
  official, then TibiaPal, then Canary).

## 6. Item consequences (RANGED-1; DUR-03 amendment)

- **Cause.** A new closed burn cause `WeaponUseCause {Ammunition, Throwing}`, keyed by the swing
  occurrence of ATTACK-0 §4 (runtime scope, attacker actor id and generation, swing sequence), not
  by a CommandRef: a swing is a timer occurrence. One swing has at most one consequence.
- **Ammunition** (`on_use = CONSUME`): one BURN of one unit of the shot quiver entry, which keeps its
  identity or retires at zero (DUR-03 §11.1, §11.5), hit or miss.
- **Throwing** (`on_use = DROP`): first the `break_chance_pct` draw (RNG purpose `break`); broken:
  one BURN of one unit of the hand stack; not broken: one unit moves to the landing tile's Ground
  (the target's tile on a hit, the §5.1 landing on a miss) as a §12 split into a planned identity
  (§11.3), or the whole item when it is the last unit, under the Ground drop rules of DUR-03 §8 and
  ADR-0021 D191 (it survives a crash and retires at the planned world reset). A full or refusing
  tile turns the drop into a BURN (no item is lost silently; the receipt records why).
- **Wands** consume nothing durable.
- **Audit.** One `OneItemTransactionV1` event per consequence with its own suffixed resource rows
  (`DUR03-RL-01-WEAPON`), committed before the effect.
- **Bound.** At most 256 weapon-use transactions in flight per channel (`RANGED0-RL-01`,
  registered by RANGED-1). A swing due while the bound is full makes no attack for that deadline,
  and the next deadline counts from it (a stall, never a backlog).
- Amended: DUR-03 §15 and §39.3 (this PR).

## 7. Content (RANGED-CONTENT-1)

- The weapon rows of the brief, on Item definitions, lowered from TibiaWiki (attack, range, hit
  chance, break chance, level, vocation, mana per shot, element, damage range) with Canary
  `items.xml` as fallback, each with its source and a `PARITY_PENDING` flag where the two differ.
- Quivers: the `quiver` flag, capacity and vocation.
- The missile appearance key is the Ability's `projectile_asset_binding` (SPELL-PRESENT-0); the
  combat presentation emits the projectile from the swing's commit, to the landing tile on a miss.

## 8. Chase (CHASE-1)

- **Effect.** With `chase = CHASE` and a target, the player's runtime actor follows the target: it
  steps toward the nearest tile at Chebyshev distance 1 from the target, for every weapon (Canary
  path target distance 1). With `STAND` it never moves on its own.
- **Path profile.** This is the accepted path profile VSL-MOVE-01 §8 asks for: CREATURE-AI-0 §5.3's
  profile (costs, tie order, `AI01-PATH-SEARCH-WORK`, `AI01-ROUTE-STEPS`, 12-tile search), run in
  the writer's window budget; at most one request per player per 1,000 ms; a failed search waits
  2,000 ms before the next (Canary).
- **Steps.** A player chase step is one Movement owner step of the player's `ExactActorRef`, on the
  CREATURE-AI-0 §5.1 step timer family with the player's own step duration (CONDITIONS-0 §4.2),
  revalidated exactly like a client step; its lineage is the CommandRef that set the target. The
  path is a proposal: the Movement owner commits each step or refuses it, and a refused step drops
  the path.
- **Client intent wins.** A client movement command cancels the pending chase step and the adopted
  path; chase re-paths at its next 1,000 ms check if the target and chase mode still hold
  (`PARITY_PENDING`).
- **Ends** when the target is cleared or changes, chase is set to `STAND`, the target leaves the
  floor or perception, or the player is under re-entry protection. Chase never changes floors
  (CREATURE-AI-0 R2 applies to players' chase too).
- **Runtime only.** Chase writes nothing durable and does not survive a reconnect (the target is
  cleared there, ATTACK-0 §3).

## 9. Rejected options

- **Ammunition from the Extra slot.** The manual made it free-form and Canary reads only the
  quiver.
- **A runtime ammunition counter flushed later.** A crash between flushes would duplicate arrows;
  DUR-03 §4 forbids value created by recovery.
- **A burn per batch of shots.** Unused reserved units would need a refund MINT and a cause; one
  unit per swing is the rune shape that is already accepted.
- **Always burning thrown weapons.** Spears that land on the ground are Tibia; the split and Ground
  drop shapes already exist.
- **Client-driven chase.** VSL-MOVE-01 forbids a client route as authority, and Tibia chases on the
  server.
- **A separate ranged timer.** ATTACK-0 has one auto-attack timer; a second would allow two swings
  per interval.

## 10. Architect rulings (owner rule 5905825574)

- **R1. Ammunition commit.** a) Commit the one-unit burn before the effect (recommended: the
  accepted rune shape, no duplication or free shot after a crash); b) commit after the effect and
  reconcile. **Ruled a).**
- **R2. Thrown weapons.** a) Tibia: break chance, else drop one unit on the landing tile
  (recommended); b) always burn. **Ruled a).**
- **R3. Chase steps.** a) Server-driven through the Movement owner with CREATURE-AI-0's path
  profile (recommended: Tibia, one path profile); b) client-driven. **Ruled a).**
- **R4. Overload.** a) No attack for that deadline above 256 in-flight weapon transactions per
  channel (recommended: bounded, no queue, as WORLD-INTERACTION-0 R5); b) an unbounded queue. **Ruled a).**
- **R5. The Extra slot.** a) Any whole non-container item (recommended: the manual, no nested
  location); b) any item including containers. **Ruled a).**

## 11. Owner questions

None. Every choice is a Tibia-parity application or a bound under DUR-03 §28 and owner rule
5905825574.

## 12. Decision test

- **Must decide now:** YES. Without it paladins, sorcerers and druids cannot auto-attack, and
  knights cannot chase.
- **Minimum sufficient:** no new wire; one burn cause; one container admission; one path profile
  reused for players.
- **Superseding evidence:** an official formula, TibiaPal disagreeing with Canary, or a measured
  database cost of one transaction per shot above the DUR-03 §28 envelope.
- **Deliberately not decided:** `FOLLOW`, diagonal steps, Perfect Shot values, exercise weapons,
  creature ranged attacks (CREATURE-AI-0), PvP legality (PARTY-PVP-0).

## 13. Before-freeze checklist

1. **Contract amendments:** DUR-03 §15 and §39.3 (`WeaponUseCause`); ITEM-MOVE-WIRE-1 §4 and
   BAGS-0 §6 (the quiver, the Extra slot); ATTACK-0 §4 (Weapon) points here. All applied in this PR.
2. **Serialization:** each swing is one occurrence of the attacker's timer inside the channel
   owner's tick; its durable consequence takes the item writer's `character_root` lock (DUR-03 §29); a
   losing concurrent move of the same entry makes the swing's commit fail, and the swing has no
   effect.
3. **Restart:** a committed burn or drop is durable and its receipt replays to the same result; an
   uncommitted swing has no effect. Chase, target and modes are runtime.
4. **Typed references:** weapons and ammunition are A12 keys; the shot entry is its ItemInstanceId
   with its container location; the swing occurrence is typed per ATTACK-0 §4.
5. **Wire:** none new; quiver equip gated by the BAGS-0 capability.
6. **Split work:** one swing, one invocation, one transaction committed before the effect.
