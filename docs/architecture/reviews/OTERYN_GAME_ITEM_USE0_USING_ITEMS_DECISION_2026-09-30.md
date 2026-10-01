# ITEM-USE-0 Using items: food and potions

- Decision: `ITEM-USE0-FOOD-AND-POTIONS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence and combat) and protected integration. It integrates after ITEM-MOVE-WIRE-0
  (PR #1344), whose item target it extends.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the architect programme plan (#162 5910870596, M1): a player cannot eat or drink a
  potion
- Builds on: USE-WIRE-V1 (command type 2), ITEM-MOVE-WIRE-0 (capability 4, the item target in
  field 2, handles), DUR-03 §7, §11.1, §11.3, §11.5, §13, §15, §17, §28, §39.1 and §39.3, the
  composition decision §3, GAME-ABILITY-01 (one pipeline; cooldown, condition and reservation
  baselines), the spell cast contract (SPELL-D1 content indexes, SPELL-D2, SPELL-D5), D76, D133/D134,
  owner rule 5905825574 (Global parity)
- Amends: USE-WIRE-V1 (a new capability, fields 4 and 5, §3); DUR-03 §15, §39.1 and §39.3 (the
  `ItemUseCause` shapes, §4)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| ITEM-USE-WIRE-1 | impl, protocol review | capability `ITEM_USE_V1`, fields 4 and 5, dispositions (§3) | ITEM-VIEW-1 |
| ITEM-SEM-USE | content lane | food time, potion effect, requirements and empty-flask definition, from TibiaWiki first and Canary as fallback | ITEM-SEM-2b |
| ITEM-USE-1 | hard, persistence review | the `ItemUseCause` shapes and their audit operation (§4), reservations and cooldowns (§5), the potion ability (§6.2), the resource rows (§4.4) | ITEM-USE-WIRE-1; ITEM-SEM-USE |
| FOOD-REGEN-1 | spell lane (condition runtime), combat review | the `FoodRegeneration` condition definition and its runtime (§6.1) | the GAME-ABILITY-01 condition runtime; ITEM-USE-1 |

Using items from equipment slots follows ITEM-MOVE-2a. Potions on other players follow VIS-2. A
flask on the ground follows ITEM-MOVE-2b.

Later, each with its own decision: runes (RUNE-USE-0: a rune burn before PRIMARY COMMIT would
charge for a cast that fails, which SPELL-D3 forbids, so runes need their own reservation design
with the spell lane), tools on the world (a `USE_WITH` interaction edge), fishing (a new MINT
source), ammunition use, keys, beds, writing.

## 1. Question

How does a player eat and drink a potion?

## 2. Facts

**PROVEN**

- USE-WIRE-V1: command type 2 `USE_INTENT` has one registered target, a world object. Fields 2, 3
  and 4 are reserved; unknown fields and enum values fail closed (`world_object_v1.proto`). The
  registry's `max_payload_bytes` for it is 529 and the result is at most 4 bytes.
- ITEM-MOVE-WIRE-0 (candidate) makes field 2 `item = ItemTargetV1 {handle}` under capability 4,
  to open a corpse; handles also name entries of an open corpse.
- DUR-03: §11.1 keeps an item's identity on a quantity adjustment; §11.5 retires a stack at zero;
  §11.3 plans fresh identities in the reservation; §7.1 reserves sources and destinations; §15
  admits `DECAY_RETIRE`, `FeeBurnCause` and, with NPC-0 (merged), `NpcTradeCause`; §17
  classes BURN and TRANSFORM; §39.1 excludes burn, transform, mint into an existing stack and
  multiple touched items outside named shapes. The fee burn has its own suffixed rows
  (`DUR03-RL-01-FEE-BURN` and others).
- The composition decision rule 1: an item-only transaction does not advance
  `CharacterRevision`; §3.1 covers the main backpack entries.
- The spell cast contract: vitals are runtime-actor-local (SPELL-D2); SPELL-D1 sends content
  references as canonical uint32 indexes per content generation; SPELL-D5 orders the sources of
  per-vocation vitals data; regeneration is left to conditions (§10), and the authoring schema's
  S17 `regeneration` has fixed gains and intervals only.
- GAME-ABILITY-01: a condition needs a ConditionDefinition (duration, stacking, conflict key); work
  that leaves the owner lane needs an explicit bounded reservation.

**CIPSOFT_OFFICIAL** (the Tibia manual)

- Health and mana do not regenerate in a protection zone, but food still runs down (`combat.md`).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Food adds `food value x 12` seconds of regeneration and is refused when the remaining time plus
  the new time would reach 1,200 s (`foods.lua:140-158`). Regeneration rates come from the
  vocation and are fixed when the condition starts (`player.lua:4-21`). Food uses the general
  action gate of 200 ms.
- Potions share `nextPotionAction`, 1,000 ms, set after a successful use (`actions.cpp:474-478`);
  a use during it is queued (`game.cpp:4686-4704`). A potion reaches one tile. It leaves an empty
  flask, dropped on the map when there is no room (`potions.lua:108-114`); attribute potions leave
  none.

## 3. Wire (ITEM-USE-WIRE-1, amends USE-WIRE-V1)

- **Capability `ITEM_USE_V1`**, which requires capability 4; its number is reserved on #162 at
  allocation. Without it the server keeps ITEM-MOVE-WIRE-0's meaning (a non-corpse item is
  `NOTHING_TO_USE`), sends no new disposition, and a command with field 4 or 5 is `REJECTED`.
- **The used item** (the `target` oneof):
  - field 2, `ItemTargetV1 {handle}`, for a main backpack direct entry (and, after ITEM-MOVE-2a,
    an equipped item). A corpse handle still opens the corpse (ITEM-MOVE-WIRE-0). Any other corpse
    entry or ground item is `NOTHING_TO_USE`, so D133 and D134 cannot be bypassed;
  - new field 5, `ItemByDefinitionV1 {definition_index}`: the hotkey form, a canonical uint32 index
    of the item definition in the active content generation (as SPELL-D1). At PREPARE the server
    resolves it to the first matching, unreserved stack of the main backpack in B3 display order;
    equipment slots join after ITEM-MOVE-2a. Nested bags are out of scope (`PARITY_PENDING`:
    Canary searches them).
- **Field 4, `use_with`**, outside the oneof, valid only with field 2 or 5 for a potion:
  `creature {actor_id, generation}`, a D85 identity visible to the session. Absent means the user.
  Field 4 with food, with a corpse, or with field 1 fails closed. Field 3 stays reserved.
- **Reach:** food and potions on the user; a potion on a creature at Chebyshev distance 1, same
  floor.
- **New dispositions** (only under `ITEM_USE_V1`): `REQUIREMENT_NOT_MET`, `EXHAUSTED`, `FULL`,
  `NO_TARGET`. The result stays at most 4 bytes, and the payload stays within 529 bytes.

**Amendment (pending on acceptance of BAGS-0;
`reviews/OTERYN_GAME_BAGS0_CONTAINERS_WITH_CONTENTS_DECISION_2026-09-30.md` §8).**
Field 2 may name any entry of an open container view in the character's own trees. Field 5 searches
breadth-first: the equipment slots, the main backpack's direct entries in display order, then each
container level by level, closed bags included. The runtime search reads at most 509 items.

## 4. DUR-03 (ITEM-USE-1)

### 4.1 Shapes

- **Burn** (food, and a potion without a flask): one BURN line (§17) of exactly one unit from the
  used stack S. S keeps its identity (§11.1) or retires at zero (§11.5). No split and no temporary
  item. One item touched.
- **Flask** (a potion whose content names an empty flask): one TRANSFORM line (§17) at unit level.
  The input is one unit of S (§11.1 or §11.5). The output is one unit of the flask definition:
  either a quantity adjustment of a compatible flask stack E in the main backpack (§11.1), or a
  fresh flask item F in a new entry, planned in the reservation (§11.3). §16.1's instance policy
  does not apply, because no whole instance changes type. At most two items touched. Nothing is
  minted.
- **No room for the flask** (no compatible stack with room and no free entry): until ITEM-MOVE-2b
  ships, the potion takes the burn shape and the audit records the missing flask
  (`PARITY_PENDING`); refusing the drink could kill the character. After 2b the flask goes to the
  character's tile as a Ground item under §32 and `ITEMMOVE1-RL-01`; a full tile falls back to the
  burn shape.

### 4.2 Cause and audit

A closed `ItemUseCause` with variants `Food` and `Potion`. Its identity is the using command's
CommandRef. One audit event per use, as a new `OneItemTransactionV1` operation whose tag and ANL
registry entry ITEM-USE-1 assigns. Later variants (rune, ammunition, bait) need an amendment of
this list.

**Amendment (pending on acceptance of RUNE-USE-0;
`OTERYN_GAME_ITEM_USE0_USING_ITEMS_DECISION_2026-09-30.md` §4.2).** RUNE-USE-0 §5 adds a third
variant, `Rune`: a one-unit burn of the used rune stack with the same audit operation, committed
before the rune's frozen cast applies. Runes use their own in-flight slot, beside this decision's
(RUNE-USE-0 §7).

**Amendment (pending on acceptance of EQUIP-0; `reviews/OTERYN_GAME_EQUIP0_EQUIPMENT_EFFECTS_TIMED_AND_CHARGED_ITEMS_DECISION_2026-10-01.md` §5.3).** A fourth variant, `Transform`: using a torch
turns the whole non-stackable item into its lit or unlit definition, one `TRANSFORM` line
(`PRESERVE_INSTANCE`), in this decision's item slot with its reservation and cooldown rules.

### 4.3 Supersession

For these shapes only, this decision supersedes the §39.1 exclusions of burn, transform, mint
into an existing stack (the flask unit into E) and multiple touched items, and adds
`ItemUseCause` to the sinks of §15. The DUR-03 text is amended in §15, in the §39.1 list of
admitted shapes and by a paragraph at the end of §39.3. Every other §39 obligation is unchanged.

### 4.4 Rows (registered by ITEM-USE-1 before implementation, DUR-03 §28)

| Row | Value |
|---|---|
| `DUR03-RL-01-ITEM-USE` touched items | 2 (1 for the burn shape) |
| `DUR03-RL-06-ITEM-USE` participants | 2 |
| `DUR03-RL-02-ITEM-USE`, effect work units, envelope and payload | measured on the flask shape; a cap that would exceed the generic rows sends the shape back |
| `ITEMUSE0-RL-01` item uses per actor per second | 5 (the 200 ms food gate; potions 1 per second) |
| `ITEMUSE0-RL-02` item-use commits per channel per second, and their database p99 latency | measured before ITEM-USE-1 ships |
| `ITEMUSE0-RL-03` `USE_INTENT` payload with fields 4 and 5 | at most 529 bytes (unchanged `max_payload_bytes`), measured by ITEM-USE-WIRE-1 |
| `ITEMUSE0-RL-04` ambiguous-commit bound before the in-flight slot is freed | 2,000 ms |

## 5. Reservations and cooldowns (ITEM-USE-1)

- **One in flight.** An actor has at most one item use between PREPARE and its outcome. Another
  use meanwhile is `EXHAUSTED`; a replay of the same CommandId returns its original result.
- **PREPARE,** in the owner lane: resolve the item, check requirements, reach and cooldown, start
  the cooldown, and reserve under DUR-03 §7.1 the source stack and the destination (the flask
  stack E or a free entry), with the planned identities (§11.3). The database checks those same
  rows and answers `STALE_STATE` if they changed; it never searches again.
- **Commit, then effect.** The DUR-03 transaction commits first. On a known commit the runtime runs
  PRIMARY COMMIT of the effect in the same owner lane. If the commit fails, there is no effect.
- **Why potions and food differ from runes (SPELL-D3).** SPELL-D3 governs a spell's own costs
  (mana, soul, cooldowns), which commit with its effect; it says nothing about an item cost. For
  food and potions every check that can fail (requirements, reach, cooldown, the target's
  visibility) runs at PREPARE, before the burn. After a known commit the effect is a heal, a mana
  restore or a regeneration condition that cannot be refused. The one remaining case is a potion
  target that leaves reach or dies between PREPARE and PRIMARY COMMIT: then the effect is not
  applied and the unit stays spent, as in Tibia, where a used potion is gone even if its target
  moved. This is the defined semantics for a failed effect. Runes have spell target validation at
  PRIMARY COMMIT that can fail for many reasons, which is why they wait for RUNE-USE-0.
- **Ambiguous commit.** After `ITEMUSE0-RL-04` (2,000 ms) without an outcome, the actor's
  in-flight slot is freed, but the reserved stack stays unspendable until reconciliation (hotkeys
  skip it). A commit known only after reconciliation applies no late effect; the unit is spent, as
  after any crash.
- **Cooldowns.** Typed cooldown keys owned by the ChannelRuntime (GAME-ABILITY-01): `potion`,
  1,000 ms, shared by every potion; `food`, 200 ms. Both `PARITY_PENDING`. They start at PREPARE and
  stay after a failed commit, a deliberate deviation from Canary (which sets them after success) to
  bound database load. A use during a cooldown is `EXHAUSTED`; Canary queues it, and a client may
  repeat the hotkey.
- Each use is one item-only transaction under the composition rule 2 fence, with no
  `CharacterRevision` advance.

## 6. Uses

### 6.1 Food (FOOD-REGEN-1)

- A new ConditionDefinition, `FoodRegeneration`, with its own conflict key:
  - eating adds the food's time (content) to the remaining duration; when remaining plus added
    would reach 1,200 s the use is `FULL` and nothing is burned (`PARITY_PENDING`);
  - each tick regenerates health and mana at the vocation's rates, from the SPELL-D5 vocation data,
    fixed when the condition starts; the Premium promotion benefit is re-checked at each tick as
    D76 says;
  - in a protection zone the ticks regenerate nothing, but the duration still runs down;
  - it coexists with other regeneration conditions (a Recovery buff) by its own key.
- Like vitals, it is runtime-actor-local in V1 and must become durable before production
  (SPELL-D2).

### 6.2 Potions

- The heal or mana restore runs through the GAME-ABILITY-01 pipeline as an ability with the
  invocation origin `ItemUse`, keyed by the CommandRef; the draw is bound to it. Requirements
  (level, vocation) come from content; a failed requirement is `REQUIREMENT_NOT_MET` before any
  reservation.
- A potion on another player needs VIS-2; until then field 4 is `NO_TARGET`.

## 7. Rejected options

- **Split a unit, then burn it.** DUR-03 §13 needs no temporary item for fungible units.
- **Minting the empty flask.** A transform conserves value; a mint would add a value source.
- **Applying the effect before the burn commits.** A crash could heal for free and keep the potion.
- **A food timer outside GAME-ABILITY.** It would be a second healing path.
- **Reusing capability 4.** Its clients read the new dispositions as unknown values.
- **Using items from corpses or the ground.** It would bypass D133 and D134.
- **Runes in this decision.** A burn before PRIMARY COMMIT charges for a cast that can still fail,
  which the owner-accepted SPELL-D3 forbids; RUNE-USE-0 designs that reservation with the spell
  lane.

## 8. Decision test

- **Must decide now:** YES. Without potions and food, a character cannot recover in a fight or
  regenerate.
- **Minimum sufficient:** two USE fields, one capability, one burn cause, one unit transform, two
  cooldown keys, one condition; the ability pipeline is reused.
- **Superseding evidence:** official food, potion or exhaust values.
- **Deliberately not decided:** runes, tools, fishing, ammunition, conjuring, keys, beds,
  writing, nested bags for hotkeys.

## 9. Before-freeze checklist

1. **Contract amendments:** USE-WIRE-V1 (§3, edited by ITEM-USE-WIRE-1 after #1344); DUR-03 §15,
   §39.1 and §39.3 (§4.3), written pending on acceptance of ITEM-USE-0 (#162 5912405163). The
   capability number is reserved at allocation.
2. **Serialization:** one in-flight use per actor; one item-only DUR-03 transaction before the
   effect; reserved source and destination; rule 2 fence; replay by CommandRef.
3. **Restart:** items are durable; the condition and cooldowns are runtime state.
4. **Typed references:** handles, definition indexes, D85 identities.
5. **Wire:** §3, capability `ITEM_USE_V1`.
6. **Split work:** one unit per use, at most two items touched.
