# Oteryn spell chain behaviour (candidate v1)

- Date: 2026-09-28
- Status: CANDIDATE / behaviour specification for review; no schema, converter, runtime or `content/` change.
  The authoring shape in §5 extends a shared contract (monster D12 `Ability.chain`) and needs the owner's
  acceptance and independent review before any implementation.
- Request: owner request of 2026-09-28 (specify the native behaviours that block spells, starting with the chain);
  programme story KAN-16; coordination #162.
- Parent: `OTERYN_SPELL_AUTHORING_SCHEMA_V1.md` (S3, S7, S11, S21; plan phase P5).
- Related: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` D12 (`Ability.chain`) and the players-only chain picker.
- Sources:
  - `opentibiabr/canary@99902524e052f37574194466c2949c576e4ab269`:
    - `src/creatures/combat/combat.cpp`: `doCombatChain`, `pickChainTargets`, `pickForkTargets`,
      `isValidChainTarget`, `doChainEffect`;
    - `src/creatures/combat/spells.cpp`: `InstantSpell::playerCastInstant`;
    - `src/lua/functions/creatures/combat/combat_functions.cpp`: `luaCombatExecute`;
    - `src/io/io_wheel.cpp`;
    - the eight spell scripts named in §2;
    - the history of the monk scripts, commits `49c6068`, `b428b4d`, `7f89f91` and `3f75ece` of 2026-07-13.
  - `zimbadev/crystalserver@ff7ede5`: the same eight scripts.
  - TibiaWiki (Fandom): the spell pages and `Updates/*` pages as of 2026-09-27, in the pinned capture.
  - TibiaWiki BR: the 2026-09-27 capture (run 36355786702).
  - Official patch notes as quoted on the Fandom `Updates/*` pages: 13.10.12892, 15.01.4b0877, 15.01.84f39b, 15.10.77ba00, 15.11.dd3523,
    15.25.3a4a52 and 15.25.bd5a04.

## 1. Outcome

Eight player spells hit a chain of creatures. Today all eight are blocked, either as scripts that do not evaluate
to plain combats or as custom scripts.

For five of them the chain is the only missing piece: Chained Penance, Forked Thorns, Forked Glacier, Lightning and
Executioner's Throw. Each of these is a plain damage combat delivered along a chain. Executioner's Throw also stays
behind `wheel_unlock`.

The other three need a chain plus a behaviour of their own:
- Spiritual Outburst: the Harmony recast;
- Divine Dazzle and Chivalrous Challenge: the melee-forcing and challenge effect on each creature hit.

This document fixes three things:
1. the behaviour one shared chain implementation must have (§3);
2. the value of each parameter for each spell, with its source (§4);
3. a proposed authoring shape that reuses the monster `Ability.chain` (§5).

It also lists the engine tests (§6) and the open questions (§7).

## 2. The chain spells

| Spell | Vocation | Shape | Start | Today's blocker |
|---|---|---|---|---|
| Chained Penance | monk | sequential | no target needed | Canary chain callback; P4 |
| Spiritual Outburst | monk (Wheel) | sequential | no target needed | P4; Harmony recast (`monk_harmony_virtue`) |
| Forked Thorns | druid | fork | no target needed | P4 |
| Forked Glacier | druid | fork | no target needed | P4 |
| Lightning | sorcerer | sequential | target or direction | the script branches on the variant (`other`) |
| Executioner's Throw | knight (Wheel) | sequential | target needed | P4; `wheel_unlock` |
| Divine Dazzle | paladin | sequential | no target, support | custom: boss check and per-creature effect |
| Chivalrous Challenge | knight | sequential | no target, support | custom: boss check and per-creature effect |

## 3. Shared behaviour

The engine walks a chain from a first creature and applies the spell's per-creature effect to each creature it
reaches. The steps below describe what Canary 15.30 does, amended where an official patch note says otherwise
(S11).

1. **First creature.** For a cast with a target (Lightning, Executioner's Throw), the first creature is that
   target.
   - For a cast without one, the first creature is the attacked creature, when it is valid (step 4) and within
     the initial range (step 3).
   - Otherwise, the first creature is the nearest valid creature within the initial range.
   - Source: the official patch 15.01.4b0877, which says: "if your current target is out of range, these spells
     will now try to start their chain on a valid target within range".
   - Canary always starts from the nearest creature and ignores the attacked creature. This is a Canary defect
     (S11 over S21).
2. **No first creature.** If there is no first creature, the cast fails, and no mana, Harmony or cooldown is spent.
   - Source: the same patch: "if there is no valid target in range, the spell will not be cast (this affects all
     player chain spells)". Patches 15.01.84f39b (Chained Penance mana) and 15.11.dd3523 (Spiritual Outburst Harmony)
     fixed casts that spent them anyway.
3. **Ranges.** The search area is a square of the given radius around the searching creature, on its floor only.
   Canary uses `Spectators().find` with equal x and y ranges and no multi-floor search.
   - The first creature is searched from the caster within `initial_range_tiles`.
   - Each further creature is searched from the creature before it (sequential shape) or from the first creature
     (fork shape) within `range_tiles`.
4. **Valid creature.** A creature is valid when all of these hold:
   - the caster may hit it (Canary `canDoCombat` for the spell's aggressive flag);
   - the spell's `target_filter` admits it;
   - there is a clear line of sight from the searching creature to it (`isSightClear`, same floor);
   - it is not the caster;
   - it has not been hit by this cast yet.

   Canary's own pickers for the damage chains (not an NPC, not the caster, not in a protection zone) add nothing
   to `canDoCombat` for an aggressive player cast, so they need no filter.
5. **Next creature, sequential shape.** Among the valid creatures in range of the last creature hit, pick the one
   at the smallest Euclidean distance.
   - Official 15.25.3a4a52 says Chained Penance and Spiritual Outburst "now chain to the closest target instead
     of the target with the highest health percentage".
   - Ties: see §7 Q1.
   - When no valid creature is in range, the chain stops. With `backtracking`, the engine instead retries from
     the creature before, at most 10 times (Canary); no player spell uses it.
6. **Fork shape.** All further creatures are chosen at once from within `range_tiles` of the first creature,
   nearest first, with ties going to the lowest creature id (Canary `pickForkTargets`).
7. **Count.** One cast hits at most `1 + max_targets` creatures: the first creature and up to `max_targets`
   further ones. Wheel augments can add to `max_targets` (§4.3).
8. **Damage per jump.** A creature reached at step `i` gets `(100 + i × damage_step_percent)%` of the rolled
   value, clamped at 0. The first creature is step 0; in the fork shape every further creature is step 1.
   - Canary: `1 - damageReduction/100 × i`, linear rather than compounded.
   - Only Chained Penance has a step, of −5.
9. **Timing.** Step `i` resolves `i × 50 ms` after the cast (Canary `combatChainDelay`, minimum 50 ms).
   - The owner's recording of Chained Penance on Tibia Global shows all hits in about the same frame, which
     agrees with this.
10. **Presentation.** For each creature hit, the chain effect (`chain_asset_binding`) is shown on every tile of
    the walking path from the previous position to the creature, and on the creature's tile. Canary searches a
    path of at most 9 steps; with no path, it shows only the creature's tile.
    - The owner's recording shows the effect on the path tiles.
11. **Per-creature effect.** Each creature hit gets the spell's normal effects (damage, conditions) with the
    step's multiplier.
    - The `affected` count for damage bonuses that depend on the number of targets is the total number of
      creatures hit by the cast.

## 4. Per-spell values

The source rules are those of the parent document:
- S3: the wiki decides;
- S11: an official change supersedes the wiki text it changes;
- S21: when neither a wiki nor tibia.com states the value, the Canary 15.30 branch decides.

### 4.1 Damage chains

| Spell | `max_targets` (further) | `range_tiles` | `initial_range_tiles` | Shape | Step % | Sources and superseded values |
|---|---:|---:|---:|---|---:|---|
| Chained Penance | 4 | 4 | 4 | sequential | −5 | (1) |
| Spiritual Outburst | 7 | 4 | 4 | sequential | 0 | (2) |
| Forked Thorns | 5 | 4 | 7 | fork | 0 | (3) |
| Forked Glacier | 6 | 4 | 7 | fork | 0 | (3) |
| Lightning | 2 | 4 | — (needs the target) | sequential | 0 | (4) |
| Executioner's Throw | 2 / 3 / 4 by Wheel stage | 3 | — (needs the target) | sequential | 0 | (5) |

1. **Chained Penance.**
   - Five creatures in all. Sources:
     - Fandom: "a target … and chains to up to 4 additional targets";
     - official 15.10.77ba00: "Number of targets from 4 to 5";
     - Canary returns 4 jumps.
     - BR says "até 6 criaturas (sem os perks da Wheel)"; this is superseded under S11, see §7 Q2.
   - Jump range 4. Sources:
     - official 15.25.3a4a52 raised it by 1;
     - Canary `b428b4d` changed 3 to 4;
     - BR says 4.
     - Fandom's "chain jump distance is 2" and "prioritize the highest percentage of HP" predate that patch and
       are superseded (S11).
   - Initial range 4: official 15.25.bd5a04, "Initial range increased to radius 4"; Canary sets 4. Fandom's
     "within range 3" is superseded.
   - Step −5%: official 15.10, "Jump bonus from -9% to -5%"; Fandom states it too; Canary uses 5.
   - Crystal keeps the older values. It is superseded (S21).
2. **Spiritual Outburst.**
   - Eight creatures in all. Sources:
     - BR: "até 8 inimigos";
     - Fandom: "chaining to 7 additional enemies";
     - Canary returns 7.
   - Jump range 4: no wiki states it, so Canary decides (S21), after its patch changes `49c6068` and `3f75ece`.
   - Initial range: Canary sets none, so the first creature is searched within the jump range, 4.
   - The Harmony recast is not part of the chain. It is a second cast of the same chain 1 s later at
     37.5 / 50 / 62.5% damage by Wheel stage, only at full Harmony, and it belongs to the `monk_harmony_virtue`
     behaviour.
3. **Forked Thorns and Forked Glacier.**
   - Up to 5 and 6 further creatures. Sources:
     - Fandom and BR: "chaining your target and up to 5 / 6 nearby enemies";
     - Canary returns 6 and 7 in all, fork shape.
   - Range 4 "calculated from the initial target": Fandom; Canary sets 4.
   - Initial range 7: Canary, and the spell range 7.
   - Crystal uses a sequential chain with range 5, superseded (S3/S21).
4. **Lightning.**
   - Two further creatures: official 15.25.3a4a52, "now chains to 2 additional targets"; Fandom; Canary.
   - Jump range 4: only Canary states it (S21).
   - A cast by direction, with no target, hits the tile in front and does not chain (Canary `lightning.lua`
     branches on the variant).
5. **Executioner's Throw.**
   - 2 / 3 / 4 further creatures by Wheel stage 1 / 2 / 3. Sources:
     - Fandom stage table;
     - the official 13.10.12892 patch: "Additional targets raised from 1/2/3 to 2/3/4 per stage".
   - Canary returns `bounces + 1`, which makes 3 / 4 / 5. This is superseded (S3).
   - Jump range 3: only Canary states it (S21).
   - The stage count is Wheel state; until a Wheel owner exists the spell stays fail-closed through
     `wheel_unlock`.

### 4.2 Support chains

These also need a per-creature behaviour, so they stay blocked after the chain exists.

| Spell | Further creatures | `range_tiles` | Filter | Per-creature effect | Sources and superseded values |
|---|---:|---:|---|---|---|
| Divine Dazzle | 2 | 7 | ranged monster | fights in melee for 8 s | (6) |
| Chivalrous Challenge | 3 | 7 | ranged monster | fights in melee for 12 s and targets the caster for 6 s | (7) |

- The filter admits a monster that is not a summon, not a reward boss and whose preferred target distance is
  greater than 1 (both Canary pickers). Fandom adds: "Summoned creatures are unaffected", "Players are not
  affected".
- The cast fails when a reward boss is among the spectators (Canary). Fandom: "cannot be used in rooms of Lever
  Bosses".

6. **Divine Dazzle.**
   - Three creatures in all: Fandom and BR, "up to 3 creatures"; Canary returns 2.
   - Crystal returns 3, which makes four creatures, and its duration is 12 s instead of 8 s. Both are superseded
     (S3).
   - Jump range 7: Fandom history, official 15.25.3a4a52.
7. **Chivalrous Challenge.**
   - Four creatures in all. Sources:
     - Fandom and BR: "up to 4 creatures";
     - official 15.25.3a4a52 added one target to the earlier 3.
   - Canary and Crystal both return 6, which makes 7 creatures. This is superseded (S3).
   - Jump range 7: official 15.25.3a4a52; Canary.

### 4.3 Wheel augments (Canary `io_wheel.cpp`)

These belong to the Wheel system and are listed here so its owner does not need to search:

| Spell | Augment I | Augment II |
|---|---|---|
| Chained Penance | +1 further creature | +18% damage (official 15.10) |
| Forked Thorns, Forked Glacier ("Forked Spells") | −2 s cooldown | +1 further creature (official 15.25) |
| Divine Dazzle | +2 further creatures | +4 s duration, −8 s cooldown |
| Lightning ("Special Spells") | shared with Strong Energy/Flame Strike | |

Chivalrous Challenge has no augment since 15.25 (Shield Slam replaced it).

## 5. Proposed authoring shape

This reuses the monster `Ability.chain` (D12) instead of adding a spell-only native behaviour. A chain damage spell
then becomes a plain `Ability` with a `chain` block. No `native_behavior` key is needed, which is consistent with
the D13 rule of one shared behaviour per pattern.

| Field | Today (D12) | Proposed |
|---|---|---|
| `max_targets` | required, ≥ 1 | Unchanged value. **Clarify** the meaning as "further creatures after the first" (Canary's returned value: a Canary chain hits the returned value plus one creature, from a target or from the caster). The D12 description "up to max_targets creatures" is one short. The monster conversion already stores the Canary value, so no monster data changes. |
| `range_tiles` | required | Unchanged: the jump range. |
| `backtracking` | required | Unchanged. |
| `chain_asset_binding` | optional | Unchanged: the path and tile effect of step 10. |
| `target_filter` | `players` | Add `ranged_monsters` (§4.2). |
| `shape` | — | New, optional: `sequential` (default) or `fork`. |
| `initial_range_tiles` | — | New, optional: the search radius for the first creature of a cast without a target. Absent means `range_tiles`. |
| `damage_step_percent` | — | New, optional, integer: the per-step change of §3 step 8. Absent means 0. |

Until the chain runtime exists, the game core must reject a spell whose ability carries `chain`. The core already
rejects `cast_at_position` and `wheel_unlock` the same way. Today `ability_effects` ignores the block and would
cast single-target, which is a silent wrong result.

## 6. Engine tests

Each of these is a deterministic world with fixed creature ids.

1. A cast without a target starts on the attacked creature when it is valid and within `initial_range_tiles`.
   Otherwise it starts on the nearest valid creature.
2. With no valid creature in range, the cast fails; mana, Harmony and cooldown are unchanged.
3. A sequential chain always picks the closest valid creature to the last one hit. It skips a creature behind a
   wall (line of sight), on another floor, in a protection zone, or already hit.
4. The count is capped at `1 + max_targets`, and a Wheel `+1` raises it by one.
5. Chained Penance with 5 creatures in a line 1 tile apart: the damage multipliers are 100, 95, 90, 85, 80%, and
   the hit delays are 0, 50, 100, 150, 200 ms.
6. Fork: every further creature is chosen within `range_tiles` of the first creature, not of the previous one. A
   creature 5 tiles from the first creature and 1 tile from another hit creature is not picked.
7. Lightning by direction hits the tile in front and does not chain; Lightning at a target chains to 2 more.
8. Divine Dazzle and Chivalrous Challenge skip melee monsters, summons, players and reward bosses. They fail
   with a reward boss in view.
9. The chain effect appears on each path tile between hits.

## 7. Open questions

- **Q1. Tie between equally close creatures.** Canary takes the first in spectator order, which is not
  deterministic across implementations. Proposal: the lowest creature id, as Canary's fork shape does.
  Confirming this needs an in-game test with two creatures at the same distance.
- **Q2. Chained Penance, 5 or 6 creatures.** BR says 6 without Wheel perks. The official 15.10 note, Fandom and
  Canary give 5. The owner can check it in game without a Wheel augment on 6 or more creatures in range.
- **Q3. Chained Penance jump range 4.** BR states 4 and it matches Canary. The owner can confirm it with a
  creature 4 tiles from the first.
- **Q4. The `max_targets` wording in D12.** Clarifying the wording (§5) is a monster contract text change. It
  needs the monster schema owner's acceptance together with this shape.
