# RUNE-USE-0 Using runes

- Decision: `RUNE-USE0-USING-RUNES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (protocol,
  persistence and combat) and protected integration. It extends ITEM-USE-0, whose item target,
  hotkey form and `ItemUseCause` it reuses, and integrates after it.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner direction of 2026-09-30 (build runes now, full Tibia Global parity) and the
  rune deferral of ITEM-USE-0 (a rune burn before PRIMARY COMMIT would charge for a cast that
  fails, so runes need their own reservation design)
- Builds on: ITEM-USE-0 (capability `ITEM_USE_V1`, fields 2, 4 and 5, §4 and §5), ITEM-MOVE-WIRE-0
  and ITEM-MOVE-WIRE-1 (handles, main backpack entries, equipment), USE-WIRE-V1, the spell cast
  contract (SPELL-D1 to SPELL-D3, SPELL-D7), the spell authoring schema (S2, S9, S16, S20, S21,
  S27), the native behaviours candidate (B.1 1e, D.3, D.6.1), ATTACK-0, CONDITIONS-0, MOVE-RL-11
  (D84, D87 and D222), the world-object owners proposal (D38, the value boundary), DUR-03 §7,
  §11.1, §11.3, §11.5, §14, §15, §17, §23 and §39, the composition decision rule 1, A13
  (vocation and magic level), GAME-ABILITY-01, owner rule 5905825574 (Global parity)
- Amends, each pending on acceptance of RUNE-USE-0 (#162 5912405163), in this PR: DUR-03 §15,
  §39.1 and §39.3 (§5); ITEM-USE-0 §4.2 (the `Rune` variant, §5.1); the spell cast contract §5
  and §6 (§6); the spell authoring schema §6 (`rune.charges`, §3)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| RUNE-WIRE-1 | impl, protocol review | capability `RUNE_USE_V1`, the position arm of field 4, two dispositions (§4) | ITEM-USE-WIRE-1 |
| RUNE-CONTENT-1 | content lane | rune items as stacks of 100; the rune checks the #162 spell audit left open; lowering of the ready rune bundles (§3, §9) | ITEM-SEM-USE |
| RUNE-1 | hard, persistence review | the `ItemUseCause::Rune` burn and its audit, the rune slot, holds and ambiguity (§5, §7) | ITEM-USE-1; RUNE-WIRE-1 |
| RUNE-CAST-1 | spell lane, combat review | rune resolution, the frozen cast and its application through the ability pipeline; the 18 direct runes of §9 (§6, §8) | RUNE-1; SPELL-TARGET-1 (ATTACK-0); VIS-2; CHAR-BUILD-1 |
| RUNE-CONJ-1 | hard, persistence, combat and protocol review | the `ConjureCause` BURN and MINT with the receipt-recorded mana and soul debit, the holds, two cast dispositions under `RUNE_USE_V1` (§10) | RUNE-1; the instant cast composition (SPELL-D4) |
| FIELD-1 | hard (combat), combat review | the runtime field overlay, `create_item`, decay, step-in conditions on creatures; no field that affects or blocks a player (§11, `RUNEUSE0-C7`) | COND-1; RUNE-CAST-1 |
| FIELD-WIRE-1 | impl, protocol review | capability `WORLD_SPATIAL_FIELDS`, the `FIELD` kind in a new `world_spatial_v1` schema revision, the runtime field identity and its delta rules (§11.4) | VIS-2; FIELD-1 |

Later, each with its own decision: runes against players (PARTY-PVP-0, which also owns the 10 s
login rule and the PZ block), the six native-behaviour runes of §9, conjuring ammunition and food
(a `ConjureCause` MINT without a reagent), nested bags for hotkeys, the Tibiadrome, and fields
that affect or block players (Magic Wall, Wild Growth, creature-made fields; PvP fields with
PARTY-PVP-0), which first need an FND-04 gameplay-admission rule (`RUNEUSE0-C7`, R5).

## 1. Question

How does a player conjure a rune and use it on a creature, a tile or themself, with the rune spent
exactly once and only for a cast that happens?

## 2. Facts

**PROVEN**

- The spell authoring schema: S2 (owner) gives two carriers, `instant` and `rune`; a conjuring
  spell is an instant spell with a `conjure` execution (reagent, result, count). The rune block is
  `rune {item, charges, magic_level, allow_far_use, blocking {solid, creature}}`
  (`tools/content-schema/spell-authoring/spell.schema.json`). S9 declares cooldown groups as a
  closed catalogue. S20 forbids `needs_target` with `cast_at_position`. S27 accepted the native
  behaviour keys; each stays rejected until its runtime lands.
- The readiness report (`samples/spell-readiness-p2.json`, `spell-p2-r14`): 252 records, 172
  ready. Of them 36 are rune carriers: 28 ready, 8 blocked (§9). The #162 spell audit adds: 5 rune
  aliases were skipped by the name joins, and rune item id, rune vocations, group and targeting
  were not checked.
- Starter bundles: the Sudden Death rune carrier has `charges` 3, magic level 15, level 45, all
  ten vocations, cooldown 2 s, group `attack` 2 s, `needs_target`, `block_walls`,
  `allow_far_use`; its conjuring spell has reagent item 3147, result 3155, count 3, mana 985 and
  soul 5. The rune item `oteryn:item.tibia.i3155` is `StackCapable` and carries a legacy
  `charges` count 3.
- The spell core (`apps/game-server/src/spell/`): `SpellBook::rune(item)` finds a rune spell by its
  item; `resolve_cast` checks magic level only for a rune; `Carrier::Rune` keeps `charges`.
- The spell cast contract: SPELL-D3 (owner-accepted): costs and cooldowns at PRIMARY COMMIT with
  the Effect Plan, nothing paid on failure, and a retry never charges twice. §6 lists rune use and
  conjure as not castable in V1 (use-with command, DUR-03 decrement, MINT). SPELL-D7 checks a
  position against range, line of sight, floor and protection zone. `SPELL-RL-02` is 2 effects.
- ITEM-USE-0 (merged): the used item by handle (field 2) or definition index (field 5, the first
  unreserved main backpack stack in B3 order); field 4 `use_with` names a creature or, absent,
  the user; one-unit BURN under `ItemUseCause`, committed before the effect; one use in flight
  per actor; ambiguity bound `ITEMUSE0-RL-04` 2,000 ms.
- DUR-03: §11.1 and §11.5 (a stack keeps its identity, or retires at zero); §14 (a mint into an
  existing stack needs source and lineage); §17 classes; `DECAY_RETIRE` exists only for the named,
  non-caller causes `CorpseDecay` and `WorldReset` (§39.1).
- The composition decision rule 1: an item-only transaction does not advance `CharacterRevision`.
- A13 and CHAR-BUILD-1a (#1393): vocation and magic level are durable Character build state.
- ATTACK-0: first slice creatures only; an attacker or target in a protection zone is refused;
  re-entry protection refuses attacks; offensive actions refresh the 60 s in-fight deadline.
- CONDITIONS-0: a field's damage-over-time application always replaces; standing on a field of
  the same element does not use ticks up; ticks carry frozen provenance.
- The owners proposal (D38): world-object state is scope-ephemeral; anything a player can pick up
  is an item. MOVE-RL-11 D84: the 15 × 11 view reaches 7 tiles on x and 5 on y.
- USE-WIRE-V1 (`world_object_v1.proto`): the result already has `TOO_FAR`.

**CIPSOFT_OFFICIAL** (the Tibia manual)

- Runes are single use and stack up to 100 (`magic.md` §5.4.5). A conjure charges one blank rune
  and yields 2, 3 or up to 10 runes. Using one needs a level and a magic level.
- "Use with": crosshair on a creature or tile, or a battle list target (`controls.md`,
  `magic.md` §5.4.6). Hotkeys aim at self, the current target, the cursor or a crosshair
  (`interface.md`).
- Soul points are spent on rune spells (`characters.md`, `interface.md`).
- Offensive runes cause a logout block; field runes cause a PZ block without a hit (`combat.md`
  §5.3.12.c). Field damage to the caster depends on the world type.

**OTS_HYPOTHESIS_ONLY** (as recorded in the repository)

- Canary and Crystal remove mana, soul and a rune unit only after the rune spell succeeds
  (`RuneSpell::executeUse`, spell cast contract §5; native behaviours B.1 1e: "A charge is used
  only when the script succeeds").
- Rune checks: same floor, range and line of sight when the rune has one, then blocking, then the
  script (native behaviours B.1 1e). Magic Wall and Wild Growth refuse a floor-change tile or a
  creature on the tile, and last 16-24 s and 30-60 s (D.6.1).

## 3. The rune item

- A rune is a stack of up to 100 units. **One use burns one unit.** This is current Global
  (manual §5.4.5).
- `rune.charges` in the spell schema and `charges` on the rune item are legacy values. They equal
  the conjure count, and nothing at runtime reads them for use. RUNE-CONTENT-1 sets the rune
  items' stack facts (stackable, 100) and checks that `rune.charges` equals the conjuring spell's
  `count` where both exist. The spell authoring schema §6 records this reading.
- A rune spell is found by the used item's definition (`SpellBook::rune`). An item with no
  admitted rune spell is `NOTHING_TO_USE`.

## 4. Wire (RUNE-WIRE-1, amends ITEM-USE-0 §3)

- **Capability `RUNE_USE_V1`**, which requires `ITEM_USE_V1`; its number is reserved on #162 at
  allocation. Without it a rune is `NOTHING_TO_USE`, and the new arm and dispositions are not
  sent. It also gates conjuring and the two conjure dispositions (§10). A field rune also needs
  `WORLD_SPATIAL_FIELDS` (§11.4, `RUNEUSE0-C7`).
- **The used rune:** field 2 (a handle to a main backpack direct entry, and to an equipped item
  after ITEM-MOVE-2a) or field 5 (the hotkey form, resolved as ITEM-USE-0 §3). The client never
  opens a container to use a rune.
- **Field 4 `use_with`** gains a `position {x, y, floor}` arm, in the SPELL-D7 frame (int16
  floor), valid only for a rune. Its field number is assigned by RUNE-WIRE-1. `creature` names a
  D85 identity visible to the session. Absent means the user.
- **Target mapping:**
  - a `needs_target` rune with a position takes the top visible creature of that tile;
  - a position rune with a creature takes that creature's current tile;
  - absent: the caster for a creature rune, the caster's tile for a position rune.
- **New dispositions** (only under `RUNE_USE_V1`): `TARGET_ILLEGAL` (line of sight, floor,
  blocked tile, illegal target, re-entry protection) and `PROTECTION_ZONE`. Out of reach is the
  existing `TOO_FAR`. The ITEM-USE-0 dispositions map the rest: `REQUIREMENT_NOT_MET` (level,
  magic level, vocation), `EXHAUSTED` (a cooldown or the slot), `NO_TARGET`. The result stays at
  most 4 bytes, and the payload within 529 bytes.
- The cast result travels in the USE result. No spell index is sent for a rune use.

## 5. Consumption (RUNE-1)

### 5.1 Class and cause

- Rune use is a **BURN** (§17) of exactly one unit of the used stack S. S keeps its identity
  (§11.1) or retires at zero (§11.5). One item touched. No split, no temporary item.
- It is **not** `DECAY_RETIRE`. That class retires a whole instance to no location, only under
  named non-caller causes. A rune use is a caller-chosen quantity decrement: the ITEM-USE-0 burn
  shape exactly.
- **Cause:** a third variant `Rune` of the closed `ItemUseCause`, keyed by the using command's
  CommandRef, with the same `OneItemTransactionV1` audit operation (ITEM-USE-0 §4.2 said later
  variants amend its list). No new sink.
- **Amendment of DUR-03:** §15 adds the variant; §39.1's parenthesis and a §39.3 paragraph admit
  the rune burn and the conjure shape (§10), for these shapes only. Every other §39 obligation is
  unchanged.

### 5.2 Why the burn commits first

The durable burn commits before the effect, as for potions. The other order lets a crash, a fence
loss or a database fault keep the rune after a committed cast, and a kill made by that cast could
already have committed its loot and experience. The rune unit is value; the effect is runtime
state. DUR-03 §7 puts the value first.

## 6. The cast (RUNE-CAST-1)

SPELL-D3 is kept by moving every refusal before the burn, and by treating a cast whose burn has
committed as cast. The rune cast has three steps.

### 6.1 PREPARE (owner lane)

1. Resolve the rune (handle or hotkey) and its spell.
2. Run the `resolve_cast` order: spell and group cooldowns (and the pending ones of §7), level,
   magic level (A13), vocation (the rune's users), premium, mana and soul (§6.4).
3. Resolve and check the target (§8).
4. **Freeze the cast:** the target identity or position, the direction, the area, and the drawn
   magnitudes (the RNG draw bound to the occurrence, derived from the CommandRef).
5. Place the holds of §7 and reserve S under DUR-03 §7.1. Send the burn.

A refusal in steps 1-3 is answered at once. Nothing is held or burnt.

### 6.2 Commit

The DUR-03 burn commits (§5). A known abort releases every hold: nothing was paid.

### 6.3 PRIMARY COMMIT (owner lane, on the known commit)

- The frozen Effect Plan runs through the GAME-ABILITY-01 pipeline with the invocation origin
  `RuneUse`, keyed by the CommandRef. The holds settle in the same owner mutation (SPELL-D3).
  Resistances, the mana shield and the charm hooks apply here, as for any damage.
- The caster must still be the same runtime actor in the channel. If it has died or left, nothing
  applies (as after a crash).
- **A creature target** is hit if it is alive and on the same floor. Range and line of sight are
  not checked again: the rune left at PREPARE. A dead or vanished target is not hit.
- **A position** takes the area at the frozen position. Creatures are those on its tiles now.
  Object creation re-checks each tile (§11).
- **Protection-zone filter at application (`RUNEUSE0-C3`).** For an aggressive rune, every tile
  of the frozen area that is in a protection zone is dropped before its creatures are taken, and a
  creature target that now stands on a protection-zone tile is not hit. ATTACK-0 (a target in a
  protection zone is refused) thus holds for every victim, not only for the target or centre tile
  checked at PREPARE (§8 step 5). If no eligible creature or tile remains, the cast is still cast
  (below). Non-aggressive runes are not filtered.
- **Area split (`RUNEUSE0-C5`).** `SPELL-RL-02` (2 effects) bounds each plan, not the victim
  count, so an area cast is not one plan with one effect per victim. At application the owner
  lane derives one single-target sub-plan per eligible creature (at most 2 effects each), in a
  fixed order: the tile order of the authored area, then the creature id. Sub-plan `k` uses the
  drawn magnitude frozen for draw `k` of the occurrence stream at PREPARE.
- **Sub-plan identity (`RUNEUSE0-C6`).** Each sub-plan is its own root occurrence for the existing
  writer, with no batch API. Its `AbilityOccurrenceId` is derived deterministically from the cast
  and `k` (the cast's CommandRef-derived occurrence id, then `/victim/`, then `k` in decimal, a
  valid identifier atom), so `AbilityEngine::commit` keys sub-plan `k` apart from every other
  sub-plan of the cast, and its effect ordinals stay 0-1 (`EffectPlan::sub_occurrence`). In the
  channel owner the damage receipt lives in the victim's own slot under the existing attributed
  key (character, session, sequence, sub-ordinal); one creature is in at most one sub-plan of a
  cast, so the keys never meet. This is the GAME-ABILITY-01 rule for repeated application: each
  one is "an explicit bounded deterministic occurrence or sub-occurrence under the same
  GAME-ABILITY pipeline" (cast, channel and commit baseline, repeated occurrences). The owner lane
  commits sub-plans 0 to n-1 in order and then settles the holds, all in one owner-lane turn, so
  no other command sees a half-applied cast. A replay of the CommandRef derives the same
  identities and plans: the writer returns `applied: false` for a sub-plan already committed and
  applies one not yet committed, so every victim is hit exactly once; a different plan under a
  derived identity is `OccurrencePlanConflict` and applies nothing (fail closed). A sub-plan the
  writer refuses leaves that victim unhit and does not stop the others; a committed sub-plan is
  never undone (GAME-ABILITY-01: no silent undo of committed consequences).
- **Victim cap (`RUNEUSE0-RL-08`, owner-decided).** Each rune's cap is 2 × the number of tiles in
  its authored area (a single-target rune: 1 tile, cap 2). The absolute ceiling is **74**: 2 × 37,
  the largest authored rune area in the reference data (the radius-3 circle of Great Fireball,
  Avalanche, Stone Shower and Thunderstorm, 37 tiles in both the Canary `99902524` and the Crystal
  `ff7ede5` census). RUNE-CONTENT-1 content validation rejects a rune whose cap would exceed 74
  (an authored area above 37 tiles); the rune stays blocked. Over its cap the first N creatures
  in the fixed order are hit and the rest are not, fail-closed and never a plan rejection. Alarm
  at 80% of the rune's cap. Owner decision R4 (#162, 2026-09-30, §15).
- **A cast that lands on nobody is still cast.** The unit is spent and the cooldowns stand. This
  is the one declared difference from Global. Tibia checks and applies at one instant; here the
  application follows by the commit time (`RUNEUSE0-RL-03`). Architect ruling R1 (§15).

### 6.4 Costs

- Most runes cost no mana and no soul at use. A rune whose use costs mana or soul (Convince
  Creature) fails closed until its native behaviour child; it is blocked already.
- The rune spell's mana-spent progress follows the same later GAME-CHAR training path as other
  spells.
- An aggressive rune refreshes the ATTACK-0 in-fight deadline at PRIMARY COMMIT. `pz_locks_caster`
  and the PZ block belong to PARTY-PVP-0.

## 7. Holds, slot, retry (RUNE-1)

- **One rune slot.** An actor has at most one rune use or conjure between PREPARE and its
  outcome. Another meanwhile is `EXHAUSTED`. The slot is separate from ITEM-USE-0's item slot,
  so a potion and a rune can be used together, as in Tibia. Their reservations are disjoint rows.
- **Pending cooldowns.** At PREPARE the rune spell and its groups become pending cooldowns from
  the PREPARE time. Any cast of the same spell or group meanwhile is `EXHAUSTED` (runes and
  instant spells share groups, S9). A known commit makes them real; a known abort removes them.
- **Mana and soul holds** (conjure, §10): taken from the vitals at PREPARE, shown as spent,
  settled at PRIMARY COMMIT from the committed `ConjureCause` receipt, returned (capped at the
  maximum) on a known abort. A mana shield hit uses only unheld mana.
- **Retry.** A replay of the same CommandId returns the original result while FND-02 retains it,
  then `COMMAND_OUTCOME_EXPIRED` with reconciliation (SPELL-D3). The burn's TransactionId and the
  occurrence come from the CommandRef, so a replay never burns, draws or applies twice.
- **Ambiguous commit.** After `ITEMUSE0-RL-04` (2,000 ms) the slot is freed. The pending
  cooldowns and the mana and soul holds are not finalized and not released: they stay held
  (shown as spent, unusable) until reconciliation reads the durable receipt. A committed receipt
  settles them as a debit; no committed receipt (an abort) releases them and removes the pending
  cooldowns. S and the conjure reagent stay unspendable until reconciliation (hotkeys skip
  them). A commit known only after reconciliation applies no late effect: the unit is spent, as
  after a crash, and a conjure's minted runes are published with the settlement.
- **Channel transfer and logout** wait for the slot's outcome, within the same bound.
- Each use is one item-only transaction under the composition rule 2 fence, with no
  `CharacterRevision` advance (rule 1, main backpack entries).

## 8. Target legality (RUNE-CAST-1)

Checked at PREPARE, in this order, from the rune's content:

1. **Floor.** The target is on the caster's floor (`check_floor`); else `TARGET_ILLEGAL`.
2. **Reach.** `allow_far_use`: within the 15 × 11 view, 7 tiles on x and 5 on y (D84); a
   `range_tiles` value narrows it. Without far use: Chebyshev distance 1. Else `TOO_FAR`.
3. **Line of sight** when `block_walls`; else `TARGET_ILLEGAL`.
4. **Blocking** (`rune.blocking`): a solid item or a creature on the target tile where refused;
   else `TARGET_ILLEGAL`.
5. **Protection zone.** An aggressive rune is `PROTECTION_ZONE` when the caster stands in one, and
   `TARGET_ILLEGAL` when the target or target tile is in one. Non-aggressive runes (healing) work
   in a protection zone.
6. **Target kind.**
   - Aggressive runes: creatures only in the first slice. A player target, and the caster, are
     `TARGET_ILLEGAL` until PARTY-PVP-0. Re-entry protection refuses them (ATTACK-0).
   - Healing runes: the caster or its own summoned or convinced creatures (`allowed_targets`,
     N8833 as in native behaviours D.3).
   - A `needs_target` rune with no creature is `NO_TARGET`.

The attack target (the hotkey "current target") is sent by the client as a creature; the server
checks it like any other.

## 9. Rune carriers in the spell catalogue (RUNE-CONTENT-1)

| Group | Runes | Needs |
|---|---|---|
| Direct damage and healing (18, ready) | Avalanche, Explosion, Fireball, Great Fireball, Heavy Magic Missile, Holy Missile, Icicle, Light Magic Missile, Lightest Magic Missile, Lightest Missile, Light Stone Shower, Stalagmite, Stone Shower, Sudden Death, Thunderstorm, Intense Healing, Ultimate Healing, Antidote | RUNE-CAST-1 |
| Condition (1, ready) | Soulfire | RUNE-CAST-1; COND-1 |
| Fields (9, ready) | Fire, Energy and Poison Field; Fire, Energy and Poison Wall; Firebomb, Energybomb, Poison Bomb | FIELD-1; FIELD-WIRE-1 (the caster's `WORLD_SPATIAL_FIELDS`, `RUNEUSE0-C7`) |
| Blocking objects (2, blocked) | Magic Wall, Wild Growth | a later decision with an FND-04 gameplay-admission rule (`RUNEUSE0-C7`, R5); then FIELD-1 and the D.6.1 `create_item` extension |
| Native behaviours (6, blocked) | Animate Dead, Chameleon, Convince Creature, Desintegrate, Destroy Field, Paralyze | each key's own child (S27) |

- Lightest Missile and Light Stone Shower lost their conjuring spells (S25); their rune items stay
  usable. Paralyze also needs SPEED-1.
- RUNE-CONTENT-1 closes the audit gap before any rune is admitted: for each of the 36 carriers it
  checks the rune item id, the rune users (vocations), the cooldown group and the targeting
  against the S15 and S3 sources, and resolves the 5 aliases the name joins skipped. A rune that
  fails stays blocked.

## 10. Conjuring (RUNE-CONJ-1)

- A conjure is an instant spell (S2). It checks as any cast, then needs one blank rune (the
  reagent) in the main backpack direct entries or, after ITEM-MOVE-2a, the hands, found in B3
  order. None: `REAGENT_MISSING`, nothing held.
- **Shape:** one transaction under a new closed cause `ConjureCause`, keyed by the cast's
  CommandRef: one BURN line of one reagent unit (§11.1 or §11.5), and one MINT line (§14) of
  `count` result units, into a compatible result stack with room for all of them, or into a fresh
  entry planned in the reservation (§11.3). No room: `NO_ROOM`, nothing held
  (`PARITY_PENDING`). At most 2 items touched.
- **Wire.** The conjure is sent as a spell cast (SPELL-D1). Two dispositions are added to
  `SpellCastDisposition`, `REAGENT_MISSING` and `NO_ROOM`, numbered by RUNE-CONJ-1 with the
  protocol owner; the cast result is sent at the outcome, after the commit.
- **Capability gate (`RUNEUSE0-C1`).** Conjuring and the two dispositions exist only for a
  session that negotiated `RUNE_USE_V1` (§4). Without it a conjure spell is refused as the
  existing `NOT_AVAILABLE` (SPELL-D1), checked first, before the reagent search, any hold, any
  reservation and any cost; `REAGENT_MISSING` and `NO_ROOM` are never sent to that session.
- **Why MINT.** Conjured runes are new value made from mana. A MINT line names that source for
  conservation and economy checks; a transform would hide it.
- **Costs:** mana, soul and cooldowns are held at PREPARE and settle at PRIMARY COMMIT (§7). The
  effect is the conjure presentation (`magic_red`, S18); the runes appear through the item
  delta after the commit. A known abort returns the holds.
- **Payment is failure-atomic with the mint (`RUNEUSE0-C2`).** Mana and soul are
  runtime-actor-local in V1 (SPELL-D2), so there is no durable vitals row to write. The
  `ConjureCause` transaction therefore records the debit durably: its receipt and audit event
  carry the held `mana` and `soul` amounts as the MINT's source, in the same transaction as the
  BURN and MINT lines. The durable outcome alone decides the payment: a committed receipt means
  the debit is owed and the holds settle; no committed receipt means nothing was minted and the
  holds are released. Neither the result stack's delta nor the cast result is published before
  the holds settle, in the same owner mutation as the PRIMARY COMMIT. A conjure never settles or
  releases a hold on a timeout (§7). If the runtime actor ends before settlement, its vitals end
  with it and a new actor starts at the maximum (SPELL-D2); that is the accepted V1 vitals
  limitation, and it restores no more than a settled conjure followed by the same actor end.
  When durable vitals land (DUR-02), the mana and soul debit becomes a line of the same
  `ConjureCause` transaction. Architect ruling R2 (§15).
- Blank runes come from NPC trade (NPC-0). Conjuring ammunition and food uses the same cause
  without a reagent, in a later amendment.

## 11. Fields and walls (FIELD-1)

### 11.1 What a field is

- A field (fire, energy, poison; a wall or a bomb of them; Magic Wall; Wild Growth) is a
  **runtime world object** in a per-channel field overlay. It is not an ItemInstance and has no
  DUR-03 value: it cannot be picked up, moved or traded (the D38 value boundary).
- Scope-ephemeral (D38): a channel restart clears it. Global also loses fields at server save.
- Each field records its definition (the `create_item` key), position, provenance (caster actor
  id and generation, CharacterId, spell key and revision), creation time and expiry.

### 11.2 Creation and decay

- Created at PRIMARY COMMIT by the `create_item` effect on each tile of the frozen area. A tile
  that is missing, floor-changing, in a protection zone, or refused by the definition
  (`refuse_on`, D.6.1) gets no field (`PARITY_PENDING` for the PZ tile). Other tiles still get
  theirs.
- At most one field per tile; a new one replaces the old (`PARITY_PENDING`).
- Expiry comes from content (the Item temporal facts; a range is drawn under the RNG purpose
  `field_duration`). A decay chain (a fire field shrinking) follows content. Expiry is a timer
  occurrence with the `DEADLINE_STATE` catch-up.

### 11.3 Effect on creatures

- Stepping on or standing on a damage field applies its condition through CONDITIONS-0 §3.1 (a
  field always replaces; same-element ticks are not used up), with the field's provenance, so
  kill credit follows the caster.
- A player-made field affects creatures only. Its effect on players, including the caster,
  follows the world type in a later decision (PARTY-PVP-0). This decision creates no
  creature-made field (`RUNEUSE0-C7`).
- Magic Wall (blocks movement, projectiles and sight) and Wild Growth (blocks movement only,
  D.6.1) are not in this decision (`RUNEUSE0-C7`). Their later decision makes movement, pathing
  and the line-of-sight query read the overlay; monsters that destroy walls use SW-2
  `remove_items`.

### 11.4 Visibility (FIELD-WIRE-1)

Fields travel as VIS-2 entities in domain 1, after actors in the D222 order, with a definition
index and a position. Creation and expiry emit deltas. FIELD-WIRE-1 amends the MOVE-RL-11 decision
at allocation.

- **Capability gate (`RUNEUSE0-C4`).** `world_spatial_v1.proto` closes `EntityKind` at `PLAYER`,
  `CREATURE`, `CORPSE` and `GROUND_ITEM`, and a VIS-2 client fails closed on an unknown enum value
  (domain 1, snapshot and delta type 2, capability 6 `WORLD_SPATIAL_ENTITIES`). Fields therefore
  are sent only to a session that negotiated a new optional capability `WORLD_SPATIAL_FIELDS`,
  which requires capability 6; its number is reserved on #162 at allocation. FIELD-WIRE-1 adds
  `ENTITY_KIND_FIELD` in a new schema revision of domain 1 (snapshot and delta type 3, the type 2
  messages with the field kind, its field rules and the entity cap unchanged), and a session with
  the capability receives type 3 in place of type 2.
- **Without the capability** fields are omitted: no field entity, no type 3 payload, and the type
  2 stream is exactly as today. The field still acts in the simulation (§11.3); its effect on a
  creature is presented only through the existing condition and damage presentation. Architect
  ruling R3 (§15). This is safe only because of `RUNEUSE0-C7`.
- **No invisible field hazard (`RUNEUSE0-C7`).** FND-02 §9 admits an optional capability only
  where an older peer can safely continue without it, so no field may touch a player who cannot
  see it:
  - A field rune (the Fields group of §9, and Magic Wall and Wild Growth when they land) is used
    only by a session that negotiated `WORLD_SPATIAL_FIELDS`, so a caster always sees its fields.
    Without it the rune is `NOTHING_TO_USE` at PREPARE step 1, before any hold, reservation or
    burn, as a rune without `RUNE_USE_V1` (§4). `RUNE_USE_V1` itself does not require the field
    capability: RUNE-WIRE-1 would then wait for FIELD-WIRE-1, which waits for FIELD-1,
    RUNE-CAST-1 and RUNE-1 (a cycle), and the other runes do not need it.
  - No field affects or blocks a player character in this decision. A player-made field affects
    creatures only (§11.3), no creature-made field is created, and Magic Wall and Wild Growth,
    the only blocking objects, stay blocked (§9). No player takes damage from an unseen tile or is
    blocked by an unseen wall, and player movement, pathing and line of sight do not read the
    overlay.
  - Fields that affect or block players wait for a later decision (with PARTY-PVP-0 for PvP
    fields). That decision first needs an accepted gameplay-admission rule, owned by FND-04, that
    refuses a session without `WORLD_SPATIAL_FIELDS` on any channel where such a field can exist.
    This decision does not invent that admission refusal. Architect ruling R5 (§15).
- **Field identity and deltas.** Each field has a runtime field identity: 16 bytes, non-nil,
  derived deterministically (name-based) from the creating cast's occurrence id and the tile's
  index in the authored area, and never reused in the channel's runtime life. A replay derives the
  same identity, and the overlay creates no field whose identity already exists. The identity
  fills the entry's `identity`; `generation` is 0, as for corpses and ground items. A channel
  restart clears the overlay (§11.1), and no field identity survives it. FIELD-WIRE-1 fixes the
  derivation function and the field kind's entry rules (field 8 the definition, no field 9) in
  the type 3 schema. Deltas:
  - creation is an `enter`;
  - a decay step to the next definition of its chain, on the same tile, keeps the identity and is
    an `update` with the new definition;
  - expiry or removal is a `leave`;
  - a replacement on the same tile (`RUNEUSE0-RL-05`) is a `leave` of the old identity and an
    `enter` of the new one in the same delta. The identities differ, so an identity still appears
    at most once per delta.

## 12. Rows (registered by each child before implementation)

| Row | Value |
|---|---|
| `DUR03-RL-01-RUNE` touched items | 1 |
| `DUR03-RL-01-CONJURE` touched items | 2 |
| `DUR03-RL-02-RUNE`, `-CONJURE` location/custody lines | rune 1, conjure 2 (the touched items); a cap above the generic row sends the shape back |
| `DUR03-RL-06-RUNE-PARTICIPANTS`, `-CONJURE-PARTICIPANTS` | rune 1, conjure 2 (the default maximum is 1, so the conjure shape is suffixed) |
| `DUR03-RL-06-RUNE-EFFECT-WORK-UNITS`, `-CONJURE-EFFECT-WORK-UNITS` | counted as the registry does (participant plus its effects; the receipt is not a work unit): rune 3 (a partial burn is 2, participant + quantity; a terminal burn is 3, participant + removal + retirement); conjure 5 (terminal reagent burn 3, plus the output 2: participant + placement into a fresh entry, or participant + quantity into a compatible stack). Boundary tests max and max + 1 (rune 3 and 4, conjure 5 and 6). The rune transaction burns one unit and carries no victim, so the 74-victim ceiling (`RUNEUSE0-RL-08`) adds no DUR-03 work: 3 is the bound at the ceiling. A value above this sends the shape back |
| `RUNEUSE0-RL-01` rune uses or conjures in flight per actor | 1 (the rune slot, beside ITEM-USE-0's) |
| `RUNEUSE0-RL-02` rune and conjure commits per channel per second, database p99 | measured before RUNE-1 ships |
| `RUNEUSE0-RL-03` PREPARE to PRIMARY COMMIT, p99 | measured by RUNE-1; the ambiguity bound stays `ITEMUSE0-RL-04` (2,000 ms) |
| `RUNEUSE0-RL-04` `USE_INTENT` payload with the position arm | at most 529 bytes, measured by RUNE-WIRE-1 |
| `RUNEUSE0-RL-05` fields per tile | 1 |
| `RUNEUSE0-RL-06` fields per channel | 20,000, alarm at 80%; a creation over it makes no field |
| `RUNEUSE0-RL-07` fields created by one cast | per rune, the tiles of its authored field area; absolute ceiling **12**, the Energy Wall diagonal area, the largest field-rune area in the Canary `47dfd51f` and Crystal `ff7ede5` census (`spell-census-canary-47dfd51f-crystal-ff7ede5.json`: bombs 9, Fire and Poison Wall 9 diagonal and 5 straight, Energy Wall 12 and 7), checked by RUNE-CONTENT-1 validation; a field rune above it stays blocked. At most 12 field identities per cast |
| `RUNEUSE0-RL-08` victims hit by one area cast | per rune 2 × its authored area in tiles; absolute ceiling 74 (2 × 37), checked by RUNE-CONTENT-1 validation; alarm at 80% of the rune's cap; over it the first N in the fixed order are hit and the rest are not. Runtime work at the ceiling: at most 74 sub-plans and 148 effects (2 each) per cast, in one owner-lane turn. Owner-decided (R4) |

## 13. Rejected options

- **Effect first, burn after.** A crash or database fault keeps the rune after a committed cast
  and a committed kill.
- **Burn, then refund a failed cast.** A refund is a new value source with its own audit, and the
  frozen cast has no failure after the burn to refund.
- **Checking the target again after the burn.** Every late refusal would charge for a cast that
  did not happen, which SPELL-D3 forbids.
- **Reading `rune.charges` as uses per item.** Current Global runes are single use.
- **`DECAY_RETIRE` for rune use.** It is a non-caller, whole-instance retirement.
- **A transform for conjuring.** It hides a value source; a MINT names it.
- **Fields as durable items.** They cannot be picked up and live seconds; DUR-03 rows and
  audits would carry no value.
- **Sharing ITEM-USE-0's slot.** A potion and a rune could not be used in one moment.
- **A spell index for runes.** The item is the carrier (S2); the rune's spell follows from it.

## 14. Owner-rule applications

**Global parity kept:** single-use runes in stacks of 100; conjuring one blank rune into several;
level and magic level gates; use-with on a creature, a tile or self, from a hotkey without opening
a container; shared cooldown groups with instant spells; offensive runes refused in a protection
zone, and harming no creature standing in one (`RUNEUSE0-C3`); fields that burn, poison, shock
and decay on creatures; healing runes on self and own summons.

**Declared differences:**
- A rune whose target dies or leaves within the commit time is spent (§6.3, R1).
- Aggressive runes and fields against players wait for PARTY-PVP-0.
- Hotkeys search only main backpack direct entries (`PARITY_PENDING`, as ITEM-USE-0).
- Conjuring with no room, field on a PZ tile, and one field per tile are `PARITY_PENDING`.
- Fields vanish at a channel restart, not only at server save.
- A session without `WORLD_SPATIAL_FIELDS` does not see fields and cannot use a field rune
  (§11.4, R3, R5).
- Fields that affect or block players (Magic Wall, Wild Growth, creature-made fields) wait for a
  later decision with an FND-04 gameplay-admission rule (`RUNEUSE0-C7`, R5).

## 15. Architect ruling (owner rule 5905825574)

**R1. A rune whose target dies in the commit time.** Tibia checks and applies a rune in one
instant. Here the rune unit commits to the database first, and the frozen cast applies about one
commit later (`RUNEUSE0-RL-03`, expected tens of ms). A creature target that dies or vanishes in
that time is not hit, and the rune is spent. a) Accept: the rune is spent, as when a Great
Fireball hits an empty area (recommended: no free runes after a crash, and SPELL-D3 is kept);
b) apply the effect first and burn after: exact timing, but a crash or database fault can give
free runes and free kills; c) refund the rune when nothing was hit: a new value source and audit
path for a case of tens of milliseconds. **Ruled a)**: it is a Global-parity application, which
owner rule 5905825574 gives to the architect, and it keeps DUR-03 §7 and SPELL-D3 exactly as
ITEM-USE-0 does for potions. No owner question is open.

**R2. Conjure payment without durable vitals.** Mana and soul are runtime-actor-local (SPELL-D2)
and cannot share the MINT's database transaction. The conservative, fail-closed choice is
`RUNEUSE0-C2` (§10): the `ConjureCause` receipt records the mana and soul debit atomically with
the MINT, and the holds settle or release only from that durable outcome, never from a timeout.
Rejected: finalizing holds on the ambiguity bound (charges for an aborted mint) and releasing
them on the bound (a free mint). DUR-02 durable vitals move the debit into the same transaction.

**R4. Area runes against the two-effect plan cap.** `SPELL-RL-02` bounds one plan. a) Split into
ordered single-target sub-plans, each its own derived root occurrence (CommandRef-derived id and
`k`) committed through the existing writer in one owner-lane turn (recommended: no change to the
accepted spell contract or the writer, replay-safe, fail-closed); b) raise the plan cap: a change
to an accepted limit for one carrier; c) the same sub-plans committed over separate owner turns:
other commands would see a half-applied cast; d) a new atomic batch commit API: new pipeline
machinery for one carrier. **Ruled a)** (`RUNEUSE0-C5`, `RUNEUSE0-C6`, §6.3).
**Victim cap: owner-decided** (#162, 2026-09-30). Each rune's cap is 2 × the tiles of its
authored area, with an absolute ceiling of 74 (2 × 37, the largest authored rune area in the
reference data) enforced by content validation; over the cap the first N in the fixed order are
hit; alarm at 80% (`RUNEUSE0-RL-08`). It replaces the architect's flat 50 per cast.

**R3. Fields for a client without the field schema.** The current VIS-2 schema has no field kind,
and a client rejects an unknown kind. a) Omit fields for that session (recommended: the accepted
type 2 stream is unchanged and nothing undecodable is sent); b) present them through a magic
effect: no such presentation contract exists yet, so it would be a new wire surface in any case;
c) send them as `GROUND_ITEM`: misrepresents a non-item as value (D38). **Ruled a)**, gated by
`RUNEUSE0-C4` (§11.4), and safe only with R5.

**R5. Fields a session cannot see.** Omitting fields (R3) would let a session take damage from an
unseen field or be blocked by an unseen wall. a) Require `WORLD_SPATIAL_FIELDS` to use a field
rune, and keep every field that affects or blocks a player out of this decision until an FND-04
gameplay-admission rule refuses a session without the capability on a channel where such a field
can exist (recommended: fail-closed, no new authority invented here; the nine creature-only field
runes still ship); b) refuse admission here: FND-04 owns admission and has no such refusal code,
so this decision would invent one; c) make `RUNE_USE_V1` require `WORLD_SPATIAL_FIELDS`: a
dependency cycle between RUNE-WIRE-1 and FIELD-WIRE-1, and it does not protect sessions without
runes; d) let fields ignore a player without the capability: a downgraded client would walk
through walls. **Ruled a)** (`RUNEUSE0-C7`, §11.4). Magic Wall and Wild Growth were already
blocked; they now also wait for that rule.

## 16. Decision test

- **Must decide now:** YES. The owner asked for runes now; the spell cast contract lists rune use
  and conjure as not castable, and ITEM-USE-0 deferred them here.
- **Minimum sufficient:** two capabilities (`RUNE_USE_V1`, `WORLD_SPATIAL_FIELDS`), one field-4 arm, four dispositions, one `ItemUseCause`
  variant, one conjure cause, one rune slot, a runtime field overlay; the ability pipeline, the
  spell core and ITEM-USE-0's handles and hotkey form are reused.
- **Superseding evidence:** official Global values for reach, fields per tile and
  conjuring with no room.
- **Deliberately not decided:** PvP runes, native-behaviour runes, conjuring ammunition and food,
  nested bags, the Tibiadrome, magic-level training from mana spent, fields that affect or block
  players (`RUNEUSE0-C7`).

## 17. Before-freeze checklist

1. **Contract amendments:** DUR-03 §15, §39.1 and §39.3; ITEM-USE-0 §4.2; the spell cast
   contract §5 and §6; the spell authoring schema §6. Each is written "pending on acceptance of
   RUNE-USE-0". The capability numbers are reserved at allocation.
2. **Serialization:** one rune slot per actor; pending cooldowns and holds; reserved S; one
   item-only DUR-03 transaction before the effect; rule 2 fence; replay by CommandRef; conjure
   holds settle or release only from the durable receipt (`RUNEUSE0-C2`).
3. **Restart:** items are durable; the frozen cast, holds, cooldowns and fields are runtime state.
4. **Typed references:** handles, definition indexes, D85 identities, SPELL-D7 positions,
   `ProductionKey` in audit.
5. **Wire:** §4, capability `RUNE_USE_V1`; the two conjure dispositions under it, else
   `NOT_AVAILABLE` (§10, `RUNEUSE0-C1`); §11.4 at FIELD-WIRE-1, field entities only under
   `WORLD_SPATIAL_FIELDS`, else omitted (`RUNEUSE0-C4`); a field rune needs that capability and
   no field affects or blocks a player (`RUNEUSE0-C7`); a stable field identity with enter,
   update and leave rules. Both capability numbers are reserved at allocation.
6. **Split work:** one unit per use; one reagent unit and one result stack per conjure; at most
   the rune's field area (ceiling 12) fields per cast; per area cast at most the rune's victim
   cap (ceiling 74) sub-plans, each its own derived root occurrence (`RUNEUSE0-C6`).
