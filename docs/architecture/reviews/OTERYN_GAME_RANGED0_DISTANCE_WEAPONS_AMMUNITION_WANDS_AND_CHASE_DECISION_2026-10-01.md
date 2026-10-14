# RANGED-0 Distance weapons, ammunition, wands and chase

- Decision: `RANGED0-DISTANCE-WEAPONS-AMMUNITION-WANDS-AND-CHASE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  persistence, movement and protocol) and protected integration. Amended 2026-10-14 after the
  four-lane review of 2026-10-14 (all four lanes failed; every blocker is closed in §14 and the
  sections it names); the amended head needs the four lanes again. Round 2 (2026-10-14) closes the second-round
  blockers (§14). Round 3 (2026-10-14) records the owner decision that chase is server-driven (§8). Acceptance includes the R7 bounded loss of a crash-interrupted swing's skill tries or wand mana (§4.2, §10 R7); it is an explicit owner notice (§11), not a silent default.
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
| RANGED-1 | hard (combat, persistence), combat and persistence review | distance, throwing and wand swings (§4-§5); the weapon-use slot, reservation and `WeaponUseCause` shapes (§6); wand mana | ATTACK-1; QUIVER-1; RANGED-CONTENT-1; ITEM-MOVE-2b (RANGED-1 itself registers the §6.4 aggregate rows, receipt and audit tables; the missile also needs the SPELL-PRESENT-0 stream and its key to u16 table, §7, and the slot-9 key reconciliation is QUIVER-1's, §3.3) |
| CHASE-1 | hard (movement), movement and determinism review | server-driven chase (§8): the chase search budget and rows `RANGED0-RL-03` to `-07`, the player chase step source, the per-actor pacing order | ATTACK-1; the shared CREATURE-MOVE-1 §5.3 path search in code (one profile, RL-09 ordering, AI01-ROUTE-STEPS stays 128 as the route-step bound; `RANGED0-RL-05` is the 12-tile search radius); #2113 lands that shared search module (player chase first, creature reuse by CREATURE-MOVE-1), not a player-only copy; LOOP-COMBAT-MODES-1 (#2113) rebased on this §8. CHASE-1 itself moves the pacing clock to the actor (§8) |
| RANGED-PARITY-1 | impl | fixtures of §5 against TibiaPal and Canary; break rates over 10,000 seeded draws | RANGED-1 |

No new command, state domain or capability. Ranged swings reuse `ATTACK_TARGET_INTENT`,
`FIGHT_MODES_INTENT` and domain 10. Quiver use needs `CONTAINER_TREE_V1` (§3.1). Chase is
server-driven (§8, owner decision 2026-10-14): the client sends only the chase mode and the target,
as today, and no new wire is added. **Projectiles:** the
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
- Hit chance (`weapons.cpp:729-830`, read from Canary 04b83b5, not vendored; RANGED-CONTENT-1 verifies before freeze). Roles: a shot
  item's (ammunition or throwing item) own nonzero `hit_chance` **replaces** the table result with
  that absolute chance; the bow's `hit_chance` is **added**, and only when the weapon fires
  ammunition. The cap is `max_hit_chance` when set, else 90 for a weapon with an `ammoType` and 75
  otherwise. Rows exist for caps 75, 90 and 100 (distance 1 and 5 share a row; `s` is the distance
  skill):

  | Cap | d1 and d5 | d2 | d3 | d4 | d6 | d7 |
  |---|---|---|---|---|---|---|
  | 75 | `min(s,74)+1` | `trunc(min(s,28)*2.40)+8` | `trunc(min(s,45)*1.55)+6` | `trunc(min(s,58)*1.25)+3` | `trunc(min(s,90)*0.80)+3` | `trunc(min(s,104)*0.70)+2` |
  | 90 | `trunc(min(s,74)*1.20)+1` | `trunc(min(s,28)*3.20)` | `min(s,45)*2` | `trunc(min(s,58)*1.55)` | `min(s,90)` | `min(s,90)` |
  | 100 | `trunc(min(s,73)*1.35)+1` | `trunc(min(s,30)*3.20)+4` | `trunc(min(s,48)*2.05)+2` | `trunc(min(s,65)*1.50)+2` | `trunc(min(s,87)*1.20)-4` | `trunc(min(s,90)*1.10)+1` |

  Ceilings (skill at the row limit, d1/d2/d3/d4/d6/d7): cap 75: 75, 75, 75, 75, 75, 74; cap 90:
  89, 89, 90, 89, 90, 90; cap 100: 99, 100, 100, 99, 100, 100. Any cap other than 75, 90 and 100 (the pinned items data carries many, for example 40, 76, 80, 86, 87, 89, 91, 93, 94, 95 and 96) gives a **flat chance
  equal to the cap**, at every distance. A perfect shot gives chance 100 (out of scope). One draw
  `chance >= uniform(1,100)` hits. A miss lands on the target tile or, beyond adjacent range, on a
  random walkable tile of the 3x3 around it (`:837-859`).
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
  full search with target distance 1 (`player.cpp:6139-6210`). This repository adopts the server-driven chase with the bounds of §8.

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

The ninth slot is `EquipmentSlot::Ammo` in the domain (`crates/protocol-oteryn/src/item_view.rs:151`) with the
semantic key `EXTRA` (`item_view.rs:198`, pinned by `item_view_tests.rs:816`). GAME-ITEM-01 §6.1 has no `ammo`
key. This decision changes neither name, and QUIVER-1 reconciles the documentation to `EXTRA` (no wire change is made here). The slot becomes the manual's Extra slot: it admits any whole item without `container` semantics, whatever its equipment semantics,
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
  at most `range_tiles` (content rejects values above 7), line of sight by the swing's own query `OperationalCastFacts.line_of_sight_clear`
  (`ordinary_combat.rs:803`), checked on **every** swing (RUNE-USE-0 §8 checks line of sight only
  with `block_walls`; a swing does not inherit that condition), neither actor in a protection zone, re-entry protection respected, PvP legality by
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
   transaction. Wands send none. The reservation is runtime only (DUR-03 §7.1); the durable truth of
   the unit is its item row, and §6.2 defines the restart path.

A refusal in step 1 makes no attack and spends nothing.

### 4.2 Commit and PRIMARY COMMIT

- **Known commit.** In the owner lane: the caster must still be the same runtime actor in the
  channel; the target must be alive, on the same floor, not in a protection zone, and still PvP
  legal. Then the frozen effect runs through GAME-ABILITY-01 as the `AutoAttack` ability: damage on a
  hit, none on a miss. A target that fails the recheck is not hit; the unit is spent.
- **Known abort.** DUR-03 §25 retries the same transaction while it can; a terminal refusal
  releases the reservation (and a wand's mana hold), and the swing has no effect (nothing spent).
- **Ambiguous commit.** After `RANGED0-RL-02` (2,000 ms, as `ITEMUSE0-RL-04`) the slot is freed for every use **except another
  weapon swing**: the character's weapon swings stay refused (a stall, no backlog) while an ambiguous swing is
  unreconciled, so a character has at most one outstanding ambiguous TransactionId and no second one is
  allocated before the first receipt is read. The shot unit stays reserved and unspendable until reconciliation reads the receipt by the
  TransactionId (DUR-03 §7.3, §25); a commit known only then applies no late effect. If the owner
  is lost meanwhile the restart path of §6.2 applies.
- **Wands** have no transaction: PRIMARY COMMIT follows PREPARE in the same owner turn, and the
  mana hold settles there.
- **Afterwards (ruling R7):** skill tries to SKILLS-0 by its rule (2 unblocked, 1 blocked, a miss
  repeats the previous block state); wand mana to A13 §4.5's `mana_spent`; the in-fight deadline
  refreshes. These are live-session accumulators applied **once**, in the owner lane, when the
  outcome is first known committed. They have no durable key of their own: a crash between the
  commit and the accumulation loses that swing's tries or mana_spent (a bounded loss of at most
  one swing's progress, never a duplication; a bounded loss recorded as the §11 owner notice). A replay or a reconciliation read returns the first
  outcome and **credits nothing**. A later durable key from SKILLS-0 or A13 supersedes this.
- **Channel transfer and logout** wait for the slot's outcome, within the same bound.
- **No ammunition.** A `DISTANCE` swing with no usable entry (§3.2), or from a session without
  `CONTAINER_TREE_V1`, makes no attack, spends nothing, keeps the target and waits like an
  out-of-range swing (a stall, not a cleared target). RANGED-1 makes this check (a read of the quiver
  and the session capability) before `AttackState::poll_swing` advances `last_swing`, so the refusal
  does not consume the swing interval and a swing fires on the first poll after ammunition is
  available; a test covers a swing made available right after a refusal. No client text is defined (no new wire);
  a "no ammunition" message is `PARITY_PENDING` and a later protocol decision.
- **Hooks (wiring rule).** CHARM-0 answer 16: an auto-attack triggers charms only on its main
  target. `AttackDamageCalculation` is pre-commit (`apps/game-server/src/gameplay_transport/attack.rs:706`) and carries the critical charms (Low Blow, Savage Blow: `charm_effects.rs` maps `CriticalHitChance` and `CriticalExtraDamage` only to it). RANGED-1 raises it for a `DISTANCE`, `THROWING` or `WAND` swing **after the hit is selected (a wand always hits) and before its damage is frozen**, for the main target only; whether those critical charms apply to ranged or wand attacks in Tibia is `PARITY_PENDING`. `CommittedHit` (defined at `apps/game-server/src/combat/charm_effects.rs:507`; the call is `committed_hit` at `apps/game-server/src/gameplay_transport/attack.rs:774-789`) fires only on a frozen hit, which RANGED-1 adds as a hit gate, and runs the attack-hit and creature-killed effects from the committed outcome, **only when the main target takes the frozen damage**: a
  `DISTANCE` or `THROWING` hit, or a `WAND` swing's damage (a wand has no hit roll). **Neither hook
  fires on a distance or throwing miss** (no hit is selected). The source stays `CharmHitSource::CharacterAttack` for all
  three variants; no new variant is added. RANGED-1 tests a wand main-target attack that evaluates
  both hooks (critical and on-hit charms) and a miss that evaluates none. Whether a wand triggers charms in Tibia is
  `PARITY_PENDING` (Canary lacks the rule). Proficiency effects (`content/proficiencies/`)
  take no part in a distance, throwing or wand swing until their own decision. RANGED-1 audits
  `charm_effects.rs` against this rule.
- **Creatures** keep CREATURE-AI-0 §4.4: their ranged attacks are think entries, not this timer. Creature keep-distance steps
  are gated by CREATURE-MOVE-1; the local steps in `ai_think.rs:376-399` are pre-profile and are not
  extended by this decision.

## 5. Formulas (RANGED-1)

- **Hit chance (ruling R6).** One meaning per role, as Canary (§2):
  - **Ammunition and throwing item:** a nonzero `hit_chance` is an **absolute chance that replaces
    the table** (content bound 1..=100). Zero or absent means the table.
  - **Bow** (a `DISTANCE` weapon firing ammunition): `hit_chance` is a **signed relative modifier**
    added to the table result (content bound -100..=100).
  - `max_hit_chance` selects the cap (default 90 with an `ammo_type`, else 75). Caps 75, 90 and 100
    use the §2 rows. **Any cap other than 75, 90 and 100 is a flat chance equal to the cap**, at every
    distance, before the bow modifier.
  - Canary's proficiency `RANGED_HIT_CHANCE` (`weapons.cpp:831`, `UNKNOWN`: not readable at Canary 04b83b5 in this tree) is **excluded**: proficiency takes no part here.
  - An absolute chance is **not clamped** by `max_hit_chance`: `i25735` (`maxhitchance` 86, `hitchance` 90,
    `imports/canary/items-xml/items.xml:48885-48898`) rolls 90 (`OTS_HYPOTHESIS_ONLY`; RANGED-CONTENT-1 verifies
    against `weapons.cpp` at 04b83b5 before freeze; if Canary clamps it, `i25735` rolls 86).
  - `chance = clamp(base + bow modifier, 0, 100)` where `base` is the shot item's absolute chance
    if it has one, else the table (or flat) value; one draw `chance >= uniform(1, 100)` hits.
  - **RANGED-CONTENT-1 re-baseline:** any non-bow row whose `hit_chance` is not positive is held
    unclassified until a source classifies it (an absolute chance cannot be negative or zero).
    Hit chance 90 on `i25735` (CANARY, `OTS_HYPOTHESIS_ONLY`; source: the pinned Canary items data)
    is a **throwing** item (no `ammo_type`), consistent with a throwing absolute override.
  - The point-blank penalty (distance 1 is not exempt from the table) is `PARITY_PENDING`.
- **Distance and throwing damage:** the Canary formula of §2 with ATTACK-0's fight-mode factor
  (it applies to distance and throwing; `PARITY_PENDING`) and the vocation's distance multiplier,
  with the level term of ATTACK-0 §5 (the 13.05 curve, `PRIMARY_OFFICIAL` evidence known only through mirrors of the official news, `OTERYN_SPELL_AUTHORING_SCHEMA_V1.md:99-100, 109-110`; it equals `floor(level / 5)` only for level <= 500) in place of Canary's `level / 5`, as `player_expression` trees for `spell/formula.rs` (ATTACK-0
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
    is admitted for weapon use anywhere else (inventory `FullMerge` keeps its own rules).
  - **New item** otherwise: a §12 split into a planned identity (§11.3), or the whole item when it is
    the last unit, at the top of the tile (the next `stack_ordinal`, subject to §6.1.2), under ITEM-MOVE-WIRE-1 §5's Ground
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
- The new-item path inserts a Ground location row; the trigger stamps the next `stack_ordinal`,
  the highest among stamped rows. On a tile with no legacy NULL row the new item is the top; on a
  tile with a legacy NULL row the read order (`NULLS FIRST`) still puts a legacy row first, so the
  new item is **not** first in read order, and the next thrower finds an ambiguous top and takes the
  new-item path. The no-merge outcome is unchanged. A merge inserts no row.
- Map-authored LocalObjects are below every Ground root. Only a tree's root has a Ground row.
- The client's in-tile order and the 256-entity cutoff (MOVE-RL-11 §4.3) are unchanged by this
  decision; RANGED-1 does not edit them. Later Ground decisions reuse the same read order.

### 6.2 Cause, transaction identity and fence

- **Cause:** the closed `WeaponUseCause {Ammunition, Throwing}`.
- **Transaction identity (ruling R8).** The TransactionId is a UUIDv7 allocated at PREPARE (§4.1
  step 5) before the first commit attempt (DUR-03 §20) and frozen with the swing; every retry of the
  same swing (§4.2, DUR-03 §25) reuses it. It is **not derived** from any tuple. The durable
  receipt is unique on it. Within a live owner a replay of the swing returns the first outcome.
- **Restart and reconciliation (DUR-03 §20 "proven terminal", §7.2, §7.3, §25, §47).** The reservation of the shot unit is
  runtime only (§7.1) and dies with the owner; the unit is durably held by its item row, whose
  quantity changes only in the committed transaction. In this order: (a) the replacement owner's
  generation claim commits in the database under the `character_root` lock **before any receipt
  read**, and advances only the `CurrentCharacterItemFence` component that actually changed, never
  more than one on its own authority: (i) an eligible same-GameSession reconnect under a live
  `ChannelRuntime` advances only `connection_generation` (composition decision §3). The live owner keeps its
  weapon-use slot, so a still-pending TransactionId commits with the current `connection_generation`
  and no restart read is needed; a stale predecessor presenting an older `connection_generation` fails
  the fence; (ii) a GameSession or lease replacement (a controller replacement, a moved lease)
  advances the `character_lease_generation`/GameSession through the character's own admission
  claim, and the restart read below follows; (iii) a scope-owner replacement advances the scope
  ownership generation only through the scope-assignment authority (FND-03 §4.2), and the
  character claims under the new owner then follow (ii). A character recovery path never replaces
  channel authority. The receipt persists the whole fence (`connection_generation`, GameSession,
  lease and scope generation), and the restart read below is indexed by CharacterId and the
  replaced component. In cases (ii) and (iii) every old-generation commit of a burn or drop fails the
  fence, which checks all four components; (b) the old owner's TransactionIds are then terminal, because a commit validates the scope
  ownership generation and `CurrentCharacterItemFence` in the same transaction that writes the
  receipt (DUR-03 §7.2), so no commit of the old generation can land after the claim; (c) before the
  character's first weapon swing the replacement reads **the newest one receipt** by CharacterId and the old
  generation (a single indexed `LIMIT 1` read, ordered by the receipt's commit sequence): the owner allows no new
  weapon swing while one is unobserved or ambiguous (§4.2), so at most one TransactionId of the old
  generation can be unobserved, and every earlier receipt was observed and applied by the live owner. The read is
  bounded work (one row, one retry budget of DUR-03 §23, DUR-03 §28); if it fails or exhausts the budget the
  character's weapon swings stay refused (fail closed) and the read is retried, never skipped; (d) a committed receipt is applied as committed (the unit is spent, the landing item
  exists, no damage is re-run), and no receipt after the claim means proven non-committed (the unit
  is untouched); (e) only then is a new TransactionId allocated for a new swing, an intentionally
  new logical intent (§20, §25, §47). The receipt index on provenance is part of the RANGED-1 receipt
  table. A commit that completed unobserved costs the player that unit without damage, never a
  duplicate. Reconciliation by TransactionId while the owner lives is §4.2.
- **Provenance** (audit and the restart read above, not an identity): `(WorldId, ChannelId, scope ownership generation,
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
| `DUR03-RL-07-WEAPON-EVENTS` | 1 event per logical transaction |
| `DUR03-RL-07-WEAPON-PAYLOAD-BYTES` | 7,936 (the existing one-item `DUR03-RL-07-PAYLOAD-BYTES`, unchanged) |
| `DUR03-RL-07-WEAPON-ENVELOPE-BYTES` | 9,216 (the existing one-item envelope, unchanged) |
| Fit proof | RANGED-1 serializes the split and the merge worst cases (B3 §4.5 measured its two-participant worst case at 7,821 bytes under the same caps) and records them with max and max+1 tests; a larger cap needs a measured amendment, not a guess |
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
  order of §6.2, the receipt keyed by the TransactionId and indexed by provenance (the restart read of §6.2), the
  RETIRED row at zero, and the server-originated variant without a CommandRef. No "actor
  generation" or swing sequence key exists.
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

## 8. Chase (server-driven, owner decision 2026-10-14)

- **Decision.** Chase is **server-driven**, as in Canary and Tibia. The owner decision of
  2026-10-14 supersedes the earlier "chase stays client-driven" position of #1942 (LOOP-COMBAT-MODES-1
  is to be rebased on this section by #2113; its task file is not flipped yet). The client sends only the chase mode and the target (ATTACK-WIRE-1's
  existing intents); it does not path. With `chase` set and a live target out of reach, the server
  walks the player toward the target. `chase` loses ATTACK-0 §3's "no effect on the server" for the
  player; `stand` keeps it. `balanced` is a fight mode, independent of the chase mode, and does not disable chase.
- **Search bounds** (rows registered by CHASE-1; values fixed here):

  | Row | Value |
  |---|---|
  | `RANGED0-RL-03` chase searches per window per channel | 64 per 50 ms, served oldest-waiting request first, then actor id (so every player's request is served within a bounded number of windows), inside CREATURE-AI-0's `RL-09` work-unit budget, with 16 of the 64 searches (16,384 units) reserved for chase so a full creature queue cannot starve it (a request over budget waits for the next window) |
  | `RANGED0-RL-04` expansions per search | 1,024 (A*, one work unit per expansion) |
  | `RANGED0-RL-05` search radius around the chaser (Chebyshev) | 12 tiles (not a route-length cap: AI01-ROUTE-STEPS stays 128, so a detour inside the radius may take more than 12 steps) |
  | `RANGED0-RL-06` searches per player | 1 per second |
  | `RANGED0-RL-07` backoff after a failed search | 2 s (the player holds position and keeps target and mode) |

  `RANGED0-RL-04` (1,024 expansions) is the same bound as the accepted creature search (`AI01-PATH-SEARCH-WORK` 1,024, CREATURE-MOVE-1 §5.3, CREATURE-AI-0 §5.3): the shared search takes one cap and the 512-node figure in CREATURE-AI-0 §5 is Canary reference evidence (`astarnodes.hpp:37`), not an Oteryn bound.

  The path uses the CREATURE-MOVE-1 profile with player differences (no creature bans that do not
  apply to players; the floor, zone and door rules of the Movement owner apply to every step; protection-zone tiles are never entered or crossed by a chase step, so a target reachable only through a protection zone is "cannot be reached" and starts the backoff), a goal
  of adjacency for melee and of in-range and in-sight for distance and throwing weapons and wands,
  and it re-searches when the target moves, within the rows above. A search never fails because of
  the budget. **The chase ends, and schedules no further search or backoff, when the target is on
  another floor** (chase steps cannot change floors); the chaser keeps the target and mode like an
  out-of-range swing, as ATTACK-0 §4 clears a target only when it is dead or gone. On a floor change
  the chase holds: the route resets and the target is kept, with no search or backoff until the target
  is back on the floor. A floor-change
  test is CHASE-1 scope.
- **Movement authority.** VSL-MOVE-01 §3 holds: only the Movement owner commits a step. A chase step
  is a movement occurrence whose source is the server (VSL-MOVE-01 §5), validated by the same rules
  as a client step; a refused step drops the path and starts the backoff.
- **Pacing rule.** CONDITIONS-0 §4.3's one player pacing clock is today connection-local
  (`connection.rs`, `pacing.rs`). CHASE-1 moves it to the actor, so a client step and a chase step
  share one clock with a one-step buffer: **a client step always wins**; a pending chase step is
  dropped when a client step arrives; any manual step cancels the current path (the mode stays).
  Server steps never queue behind each other.
- **What the client sees.** Chase steps are ordinary committed movement events for the chaser; no
  path and no new wire are sent. Rejection and position correction use the existing movement
  correction. The Protocol lane confirms there is no new field (CHASE-1 review).
- **CHASE-1 depends on** the shared CREATURE-MOVE-1 §5.3 path search in code (one profile, RL-09
  ordering, AI01-ROUTE-STEPS stays 128 as the route-step bound and `RANGED0-RL-05` is the 12-tile search radius; today `ai_think.rs` and
  `path_proposal.rs` do not carry it; VSL-MOVE-01 §8 requires "its own accepted implementation
  profile") and the actor-level clock above. #2113 lands that shared search module (player chase
  first, creature reuse by CREATURE-MOVE-1), not a player-only copy.
- Wand and distance chase uses the swing's range and line of sight (§4) as the goal; `FOLLOW` and
  diagonal steps stay later decisions (diagonal Movement).

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
- **Client-driven chase.** The first amended draft kept it (#1942); the owner decision of
  2026-10-14 chose server-driven chase (§8).
- **A separate ranged timer.** It would allow two swings per interval.

## 10. Architect rulings (owner rule 5905825574)

- **R1. Commit order.** a) Burn before the effect, RUNE-USE-0's PREPARE and PRIMARY COMMIT
  (recommended: no duplication or free shot after a crash); b) effect first, reconcile later.
  **Ruled a).**
- **R2. Thrown weapons.** a) Tibia: break chance, else drop one unit, merging into a compatible top
  stack (recommended; owner answer 1a); b) always burn. **Ruled a).**
- **R3. Chase steps.** a) Server-driven through the Movement owner on CREATURE-AI-0's profile with
  player differences and the owner's bounds; b) client-driven (#1942). **Ruled a)** by the owner
  decision of 2026-10-14 (the first draft ruled a), the first amendment b)).
- **R4. Overload.** a) No attack for a due deadline above 256 weapon transactions in flight per
  channel (recommended: bounded, no queue, as WORLD-INTERACTION-0 R5); b) a queue. **Ruled a).**
- **R5. Requirements.** a) The manual: below the level the weapon cannot be used (recommended: an
  official source governs); b) Canary's half or zero damage. **Ruled a).**

- **R6. Hit chance.** a) One reading for every item (override or add); b) Canary's roles:
  ammunition and throwing `hit_chance` is an absolute override, the bow's is a signed relative
  modifier, non-anchor caps are flat (recommended: it matches the pinned source and its negative
  bow values). **Ruled b).** (Replaces the first draft's relative-points reading.)
- **R7. Post-commit accumulators.** a) A new durable key per swing for skill tries and wand mana;
  b) once-per-outcome live accumulation with a bounded loss on a crash and no duplication
  (recommended: no new durable surface; SKILLS-0 and A13 own those accumulators). **Ruled b)** (bounded loss recorded as the §11 owner notice).
- **R8. Transaction identity.** a) A TransactionId derived from a swing tuple; b) an allocated
  UUIDv7 frozen at PREPARE, reused by retries (recommended: DUR-03 §20 and the UUIDv7 audit check
  require an allocated identity; no durable counter is needed). **Ruled b).**

## 11. Owner questions

None blocks acceptance. Every choice is a Tibia-parity application or a bound under DUR-03 §28 and owner rule
5905825574. **Owner notice (R7):** acceptance accepts that a crash between a commit and its live accumulation loses at most
one swing's skill tries or wand `mana_spent`, never a duplicate; the owner may veto in the acceptance PR, in which case
SKILLS-0 or A13 must first supply a durable key.

## 12. Decision test

- **Must decide now:** YES. Without it paladins, sorcerers and druids cannot auto-attack.
- **Minimum sufficient:** no new wire; one burn cause with the accepted rune pattern; one container
  admission; the existing Ground `stack_ordinal`; the chase bounded by the owner's rows.
- **Superseding evidence:** an official formula, TibiaPal disagreeing with Canary, or a measured
  database cost of one transaction per shot above the DUR-03 §28 envelope.
- **Deliberately not decided:** `FOLLOW`, diagonal steps, Perfect Shot values, exercise weapons,
  creature ranged attacks (CREATURE-AI-0).

## 13. Before-freeze checklist

1. **Contract amendments**, specified in this decision and active on its acceptance (every
   pending-amendment note in DUR-03, CREATURE-AI-0, the composition decision, ITEM-MOVE-WIRE-1,
   MOVE-RL11 and ATTACK-0, and the STACK-0 dependency on the Ground order (`stack_ordinal`, not `ground_ordinal`), is edited by this PR to match it and stays pending): DUR-03 §15, §39.1 and §39.3 (§6.4 `WeaponUseAggregateV1`); the
   composition decision (server-originated swing variant, rule 1 scope, §6.2); ITEM-MOVE-WIRE-1 §4
   (quiver, Extra slot) and §5 and §6.2 (no schema change; Ground order is the existing
   `stack_ordinal`); BAGS-0 §3, §6 and §10 (quiver tree, reachable items); ATTACK-0 §4 (variants,
   RNG purposes); SPELL-PRESENT-0 §5 (weapon projectiles, after its stream is registered).
   ATTACK-0 §3 (chase has an effect for the player), CONDITIONS-0 §4.3 (actor-level pacing clock,
   client step wins), CREATURE-AI-0 §5 and `RL-09` (player chase searches, the `RANGED0-RL-03` row).
   **Withdrawn from the first draft:** CREATURE-AI-0 §7 and MOVE-RL-11 §4.3 (no ordinal change).
2. **Serialization:** one weapon-use slot per actor; the shot unit is reserved under DUR-03 §7.1, so a
   concurrent move of that entry is refused; rule 4's lock order; the Ground tile row lock and
   counter for drops; the merge receiver is reserved at PREPARE and rechecked at commit; a known abort
   releases, an ambiguous commit keeps the unit unspendable.
3. **Restart:** the TransactionId is allocated at PREPARE and receipts are unique on it; the old
   generation's ids are terminal by the fence and a replacement owner reads the provenance receipts
   before its first swing (§6.2); an uncommitted swing has no effect; chase, target and
   modes are runtime; skill tries and wand mana follow R7.
4. **Typed references:** weapons and ammunition are A12 keys; the shot entry is its ItemInstanceId
   with its parent quiver; provenance is fully typed (§6.2).
5. **Wire:** none new; the quiver is gated by `CONTAINER_TREE_V1`; chase adds no wire (the existing mode and target intents); missiles wait for
   the SPELL-PRESENT-0 stream registration and its u16 binding (§7). A session without
   `CONTAINER_TREE_V1` that logs in with a quiver equipped sees it as an item with no tree and its
   swings draw no ammunition (§3.1); the Extra slot's semantic key stays `EXTRA` and its domain name `EquipmentSlot::Ammo` (§3.3), so older clients
   render it as before. The comment at `apps/game-server/src/gameplay_transport/attack.rs:10-12` and the test comment at `apps/game-server/src/combat/attack/tests.rs:560` say chase is client-driven; CHASE-1 or #2113 rewrites both.
6. **Split work:** one swing, one transaction committed before the effect, one PRIMARY COMMIT.

## 14. Review amendments (2026-10-14)

| Lane | Blocker | Closed in |
|---|---|---|
| Movement | connection-local pacing clock, unordered buffered steps | §8 pacing rule (actor-level clock, client step wins) |
| Movement | server chase contradicts #1942 | §8, R3 a): owner decision 2026-10-14 |
| Movement | shared CREATURE-MOVE-1 §5.3 search not in code | §8 CHASE-1 depends-on |
| Protocol | chase conflict in wire comments | §8 (existing mode and target intents; no new wire) |
| Protocol | no client-visible server path | §8 what the client sees (ordinary movement, no path) |
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

**Round 2 (2026-10-14).**

| Lane | Blocker | Closed in |
|---|---|---|
| Combat | wand charm ruling unreachable; miss wiring | §4.2 Hooks (wiring rule; wand keeps `CharacterAttack`, per CHARM-0 answer 16) |
| Combat | R6 contradicts Canary roles; cap 90 and 100 rows; non-anchor caps | §2, §5, R6 |
| Persistence | DUR-03 pending text keyed by actor generation and one-unit split | DUR-03 §15, §39.1, §39.3 notes edited |
| Persistence | restart rule versus DUR-03 §20, §7.3 | §6.2 restart and reconciliation |
| Movement | CREATURE-AI-0 chase amendment `RANGED0-RL-03` | kept consistent with §8 in CREATURE-AI-0 |
| Movement | creature keep-distance steps | §4.2 Creatures (CREATURE-MOVE-1 gated) |
| Codex | wand charms; child-owned work in Depends-on; stale composition, ITEM-MOVE-WIRE-1, MOVE-RL11 and ATTACK-0 notes | §4.2 Hooks, brief, the five notes |
| Non-blocking | LOS query, 13.05 floor, `i25735`, NULLS FIRST, `DUR03-RL-07-WEAPON` numbers, merge scope, slot-9 key, RANGED-1 depends-on, `apps/game-server/src/gameplay_transport/attack.rs:10-12` | §4, §5, §6.1.2, §6.3, §3.3, brief, §13 |

Non-blocking items taken: PARITY_PENDING point-blank (§5), 13.05 level curve (§5), fight-mode
factor on distance (§5), wand damage through GAME-ABILITY-01 (§5), range plus line of sight in place
of `MELEE_RANGE` (§4), quiver vocation gate belongs to ITEM-SEM (§4), RETIRED row plus receipt on
quiver-entry deletion (§6.1.1), Rule 1 scope (§6.2), no cross-custody merge (§6.1), no-ammo
disposition (§4.2), non-cap-14 quiver rendering and the slot-9 compatibility note (§13.5).
