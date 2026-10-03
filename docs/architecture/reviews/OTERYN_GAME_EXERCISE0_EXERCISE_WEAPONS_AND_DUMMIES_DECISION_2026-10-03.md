# EXERCISE-0 Exercise weapons and exercise dummies

- Decision: `EXERCISE0-EXERCISE-WEAPONS-AND-DUMMIES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence and
  determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - TIMED-ITEM-0 R4 ("exercise weapons: a separate EXERCISE-0");
  - OFFLINE-0 R3 (beds and exercise weapons get their own decisions);
  - SKILLS-0 §3.5 ("exercise weapons are later decisions");
  - owner answer 2a (#162 5929192803: offline training as in Tibia, with beds, training statues and
    exercise weapons);
  - control-plane allocation D291 (#1622).
- Builds on:
  - TIMED-ITEM-0 §4 (the timed-row table `game_item_timed_states`) and §5 (TIMED-ITEM-0B);
  - GAME-ITEM-01 §4.2 (the charge capability);
  - DUR-03 §11.1 (`STATE_MUTATION`) and the `TimedItemCause` reservation;
  - SKILLS-0 §3.4-§3.5 and A13 §4.1, §4.2 and §4.5 (skills, magic level and mana spent in build
    state; build checkpoints);
  - WORLD-INTERACTION-0 §3 (map-item `USE-WITH`, reach, PZ, exhaustion);
  - NPC-0 §5 (NPC BUY offers);
  - OFFLINE-0 §6 (training statues);
  - BED-0, HOUSE-RUNTIME-0 and HOUSE-CUSTODY-0 (house item state);
  - owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162/#1622
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| EXERCISE-CONTENT-1 | content lane | the exercise weapon definitions (§3.1) with their `charges`; the NPC offers (§3.5); the public Training School dummies as LocalObjects (§3.4); the per-charge value table (§3.3) | TIMED-CONTENT-1 (the `charges` fact shape); WO-2 |
| EXERCISE-1 | hard (persistence), persistence and determinism review | the `USE-WITH` start, the 2 s tick, the stop rules (§4); the training checkpoint (§5); the spent-weapon retire (§5.3) | TIMED-ITEM-0B; SKILLS-0's build-state skills; A13's magic-level training; WORLDINT-USE-1 |
| EXERCISE-PARITY-1 | impl | fixtures: tries and mana spent for one charge, one regular weapon and one lasting weapon, per type and vocation; stop on each listed act; crash between checkpoints | EXERCISE-1 |

Until EXERCISE-1, an exercise weapon is an ordinary item with charges that does nothing on use
(`NOTHING_TO_USE`). Later, each with its own decision:
- training weapons (50 charges, Daily Reward): DAILY-REWARD-0;
- house and expert dummies: §3.4, a house-lane decision;
- Store sale of exercise weapons and expert dummies: Platform;
- the client display of a running training session.

## 1. Question

How does a character train skills and magic level online by spending an exercise weapon's charges
on an exercise dummy, as in Tibia, on top of the timed-row table, the build writer and the
map-interaction model that already exist?

## 2. Facts

Sources:
- TibiaWiki (`TIBIAWIKI_STRUCTURED`, read 2026-10-03): Exercise Weapons rev 1126820, Exercise Dummy
  rev 1011205, Training rev 1126700, Training School rev 1101626.
- The Tibia manual (`CIPSOFT_OFFICIAL`, `docs/reference/tibia-manual/`).
- Canary at 04b83b51 (`OTS_HYPOTHESIS_ONLY`).

Facts:
- **Kinds.** There are exercise weapons for each trained skill:
  - sword, axe and club (melee skills), and wraps (fist fighting, Monk);
  - bow (distance fighting);
  - wand and rod (magic level);
  - shield (shielding).
- **Tiers.** Each kind has three tiers:

  | Tier | Charges | Lasts | NPC price (gold) | Store price |
  |---|---|---|---|---|
  | regular | 500 | 16 min 40 s | 347,222 | 25 TC |
  | durable | 1,800 | 1 h | 1,250,000 | 90 TC |
  | lasting | 14,400 | 8 h | 10,000,000 | 720 TC |

  Training weapons (Daily Reward) have 50 charges and last 1 min 40 s.
- **Rate.** One charge is spent every 2 s.
- **Value per charge.** One charge is worth:
  - melee: 7.2 hits;
  - bow: 2.16 hits that draw blood, or 4.32 misses;
  - wand or rod: 600 mana spent;
  - shield: 14.4 blocks.

  "Unlike regular training, when training on a Dummy all offensive skills have the same advance
  rate" (Training School).
- **Use.** The player uses the weapon on a dummy, and training then runs on its own until the
  charges run out.
  - The weapon disappears when its last charge is spent.
  - Training is allowed only inside a protection zone.
  - "If you do any action while attacking the Dummy, such as moving, casting spells or using items,
    the training will automatically stop."
  - A character can use a dummy once every 30 s.
  - While training, "the character will not be kicked from the game for being idle".
- **Dummies.**
  - Every Training School has one public exercise dummy (item ids 28558 and 28565), and many
    characters may train on it at once.
  - Expert dummies (Ferumbras, Demon, Monk) are Store items placed in houses and guildhalls. They
    give 10% more skill advance, and only one character can use one at a time.
- **Sellers.** Magic Shop NPCs sell wands and rods, and equipment NPCs sell the other kinds (one
  NPC per city in each list; the Exercise Weapons page lists them). Hirelings sell them too.
- **Canary (hypothesis).**
  - Canary trains through a recurring event that spends one charge per tick and adds skill tries
    or mana spent by a configured rate.
  - Its exact try amounts and its reach check are `OTS_HYPOTHESIS_ONLY`.
- **Repository.**
  - No exercise item is authored in `content/items`.
  - TIMED-ITEM-0 §4 already holds charges as a lazy row: an item without a row has the full charges
    of its definition.
  - TIMED-ITEM-0B owns charge checkpoints, expiry and the `TimedItemCause` variants.

## 3. Content (EXERCISE-CONTENT-1)

### 3.1 Weapons

- **Definitions.** There are 24 Item definitions: eight kinds (sword, axe, club, wraps, bow, wand,
  rod, shield) times three tiers.
  - Each has `charges {count: 500 | 1,800 | 14,400, show_count: true}` and the typed fact
    `exercise {trains: <skill family or magic_level>}`.
  - Each has no `temporal` fact, no attack value and no equip slot that grants anything.
  - Names, ids and weights come from TibiaWiki first and Canary `items.xml` second.
- **Not a weapon.** An exercise weapon is never a combat weapon. ATTACK-0, RANGED-0 and the wand
  attack never accept it.
- **Ordinary item.** It does not stack, and it moves and trades like
  any item, and has no binding. Character binding is not decided here.

### 3.2 Charges

- **Storage.** Charges live only in TIMED-ITEM-0 §4's `game_item_timed_states`, under its
  invariants:
  - lazy rows: a fresh item has full charges and no row;
  - absent row = revision 0;
  - `charges` never 0;
  - the revision only grows;
  - rows are never deleted while the item lives.

  An exercise definition counts as an admitted `timed` definition for that table.
- **No other writer.** This decision adds no column and no table. Only the training checkpoint
  (§5) and the spent-weapon retire (§5.3) write an exercise weapon's row.

### 3.3 Value table

- **One table.** The per-charge value is one versioned content table, keyed by kind:
  - tries for the skill families (melee kinds, wraps, bow, shield);
  - mana spent for wand and rod.
- **How a value is derived.** The value is computed from §2's per-charge equivalents. One charge
  gives the tries or mana that SKILLS-0 §3.5's and A13 §4.5's training rules give for those hits,
  blocks or mana, for the character's vocation.
- **Parity.** Each value is `PARITY_PENDING`, with Canary's rate as the starting hypothesis. An
  owner-trusted fan source or a CipSoft source governs once it is checked.
- **Bow.** The bow uses a fixed per-charge mix and does not draw a random hit or miss per charge.
  The mix is resolved by EXERCISE-PARITY-1 and is `PARITY_PENDING`. This keeps the tick
  deterministic (§4.2).
- **No multiplier.** No event or double-skill multiplier applies. A later decision may add one.

### 3.4 Dummies

- **Public dummies.** The public Training School dummy is a LocalObject with
  `exercise_target {multi_user: true, bonus_pct: 0}`. EXERCISE-CONTENT-1 places it from TibiaWiki
  and Canary map evidence: one in every Training School, on its PZ tiles.
- **House and expert dummies.** These are house items: placement, access, a single user and the
  +10% bonus. They need house item state, so they wait for a house-lane decision that reuses §4 and
  §5 unchanged. They are absent until then.

### 3.5 Sale

- **NPC offers.** NPC-0 §5 BUY offers at §2's prices are made by the sellers on the Exercise Weapons
  page. Each offer's `count` equals the definition's charges (TIMED-ITEM-0 §4's content rule).
- **Gold only.** The sale is an ordinary NPC BUY MINT with the NPC-0 gold debit. No new source is
  admitted.
- **Not here.** Hireling sale and the Store sale are not part of this decision.

## 4. Training (EXERCISE-1)

### 4.1 Start

- **The act.** Training starts when the actor uses `USE-WITH` with an exercise weapon on a dummy:
  the weapon in field 2 or 5, and the dummy in the `map_item` arm (WORLD-INTERACTION-0 §3.1,
  §10.1).
- **Admission checks, in order.** Each refusal changes nothing.
  1. The weapon is held by the actor, in a hand, the ammunition slot or a container in its
     inventory. Otherwise the act is refused as `NOT_POSSIBLE`.
  2. The weapon is an exercise definition with at least one charge, and the target is an
     `exercise_target`. Otherwise the act is refused as `NOTHING_TO_USE`.
  3. The actor stands in a protection zone. Otherwise the act is refused as `NOT_IN_PZ`.
  4. The dummy is on the actor's floor within `EXERCISE0-RL-01`. Otherwise the act is refused as
     `TOO_FAR`. `EXERCISE0-RL-01` is a Chebyshev distance taken from Canary at EXERCISE-CONTENT-1
     (`PARITY_PENDING`). It is at least WORLD-INTERACTION-0's `WORLDINT0-RL-01` and never beyond the
     client view.
  5. The actor's `exercise_dummy` cooldown (`EXERCISE0-RL-02`, 30 s, a typed key per
     WORLD-INTERACTION-0 §3.4) has elapsed. Otherwise the act is refused as `EXHAUSTED`. The key
     starts at an admitted start, never at a refused one.
  6. For a single-user dummy (house lane), no other character is training on it.
- **Admission.** An admitted start ends any running training session of the actor and opens a new
  one bound to the weapon's `item_instance_id` and the dummy placement.

### 4.2 Tick

- **The tick.** A session ticks every `EXERCISE0-RL-03` (2,000 ms) of simulation time on the
  channel owner's clock. The first tick fires 2 s after the start.
- **Each tick:**
  1. The weapon must still be held by the actor (§4.1 check 1), and the actor must still be in the
     PZ and in reach. If any of these fails, the session stops (§4.3) and nothing is spent.
  2. The live charge count drops by 1, and the live tries or mana spent grow by one row of §3.3, at
     the vocation the actor has at that tick. The bonus applies here when the house lane admits it.
- **Live only.** Neither change is written at the tick. Both are live values, made durable by §5.
  Advances follow SKILLS-0 §3.4 and A13 §4.5: a level advance commits a build receipt at once and
  uses the §5 shape.
- **Deterministic.** A tick depends only on the session, the live counts and §3.3. It draws no
  random number.

### 4.3 Stop

The session stops, with no refund of a spent charge, on the first of these:
- the last charge is spent (§5.3);
- any act of the actor that is not a chat or a look, including:
  - a step;
  - a turn;
  - a spell cast;
  - any `USE` or `USE-WITH`;
  - an item move;
  - an attack or follow;
  - a trade;
  - a logout request;
- the weapon leaving the actor's inventory, by any path;
- the actor leaving the PZ or the reach (checked at the tick);
- the dummy placement disappearing;
- a terminal session end, a handoff or a death.

A new start while a session runs is itself an act: it stops the old session and opens the new one
(§4.1).

### 4.4 Idle

While a session runs, the actor's idle-kick timer does not advance. It resumes from its stored
value at the stop. No other logout rule changes:
- a training actor can still be logged out by a terminal session end;
- the logout blocks of ATTACK-0 §4 and PARTY-PVP-0 §8.1 still apply.

## 5. Durability (EXERCISE-1)

### 5.1 The training checkpoint

- **One transaction.** The charges a session spent and the tries or mana they paid for become
  durable together, in one DUR-03 transaction under the actor's `character_root` lock. It holds:
  - the build receipt for the session's gained tries or mana (SKILLS-0 §3.2, A13 §4.2, cause
    `training`);
  - the weapon's timed-row `STATE_MUTATION` (DUR-03 §11.1) to the live charge count, at its expected
    revision (TIMED-ITEM-0 §4).
- **When it commits.**
  - at the A13 cadence of at most 60 s;
  - at every level advance;
  - at the stop (§4.3);
  - before a death;
  - at logout and handoff.

  A checkpoint with no spent charge is skipped.
- **Atomic, never split.** A checkpoint never commits the tries without the charges, or the charges
  without the tries. On a revision mismatch or a fence loss, the whole checkpoint writes nothing,
  and the session stops with its uncommitted live values discarded.
- **Crash bound.** A crash may lose at most one checkpoint of training: the charges and the tries
  of the same interval, together. It never loses a level, since level advances are durable.
- **Composition with TIMED-ITEM-0B.** This is the one composed writer that TIMED-ITEM-0B's
  checkpoint rule must admit (TIMED-ITEM-0 §5 items 1, 2 and 8):
  - per-item serialization;
  - no write per spend;
  - the §4 invariants.

  EXERCISE-1 does not start until TIMED-ITEM-0B is accepted. If 0B's checkpoint shape cannot carry
  a build receipt in the same transaction, this decision is reopened. Splitting the write is never
  allowed.

### 5.2 Value bound

- **Guard.** For one session, the durable gained tries or mana never exceed the durable charges
  spent times the actor's §3.3 row. This is checked per checkpoint by the receipt's
  `exercise {item_instance_id, charges_before, charges_after}` provenance fields.
- **Not a game value.** A spent charge is not a gameplay value, so there is no value line. The
  checkpoint's DUR-03 rows are the identity-preserving mutation and the build receipt.

### 5.3 Spent weapon

- **The last charge.** When the last charge is spent, the weapon is retired in the same checkpoint
  transaction as the last tries. It is one-item and atomic, and it never stores 0 charges.
- **The burn.** The BURN uses the expiry sink admitted by TIMED-ITEM-0B under `TimedItemCause`.
  This decision admits no cause of its own.
- **Row.** The row is left as TIMED-ITEM-0 §4 says for a retired item.

## 6. Rejected options

- **A durable write per charge.** That is one write every 2 s per trainer, for 8 h on a lasting
  weapon. TIMED-ITEM-0 §4 forbids writes per spend.
- **Separate checkpoints for charges and tries.** A crash between them either refunds a charge with
  the training kept, or burns a charge without its tries. Rejected for §5.1's atomic checkpoint.
- **Exercise weapons as combat weapons.** Rejected: an exercise weapon is not one in Tibia, and it
  would leak tries into combat.
- **Random bow hits per charge.** Rejected because it adds a random stream to a timed loop with no
  gameplay effect. A fixed mix gives the same expected value.
- **House and expert dummies now.** Rejected: there is no house item state yet (HOUSE-RUNTIME-0).
  §4 and §5 are written so the house lane only adds the dummy.

## 7. Architect rulings (owner rule 5905825574)

- **R1. Durability.**
  - a) One atomic training checkpoint for charges and tries, on TIMED-ITEM-0B (recommended);
  - b) charges first, then tries;
  - c) a write per charge.

  **Ruled a).**
- **R2. Dummies.**
  - a) Public Training School dummies now, house and expert dummies later (recommended);
  - b) all now.

  **Ruled a).**
- **R3. Training weapons.**
  - a) DAILY-REWARD-0, which reuses §4 and §5 with a 50-charge definition (recommended);
  - b) here.

  **Ruled a).**
- **R4. Bow.** a) A fixed per-charge mix (recommended); b) a random draw per charge. **Ruled a).**

## 8. Owner questions

None. Every choice above is a reversible architect ruling under owner rule 5905825574 and owner
answer 2a.

## 9. Decision test

- **Must decide now?** YES. Owner answer 2a puts exercise weapons in scope. TIMED-ITEM-0 R4 and
  SKILLS-0 §3.5 point here, and TIMED-ITEM-0B must know its one composed checkpoint writer before
  its shape is fixed.
- **Blocked:** EXERCISE-CONTENT-1 and EXERCISE-1. The TIMED-ITEM-0B checkpoint shape (§5.1).
  DAILY-REWARD-0's training weapons.
- **Harder later:** The atomic checkpoint couples the build receipt and the timed row in one
  transaction. Any later timed writer that pays for character progress uses the same shape.
- **Supersede if:**
  - Reference evidence shows a different tick, stop or reach rule;
  - TIMED-ITEM-0B cannot compose a build receipt (§5.1);
  - measured write load shows the 60 s cadence is too costly.
- **Deliberately not decided:**
  - house and expert dummies;
  - training weapons;
  - Store and hireling sale;
  - character binding;
  - event multipliers;
  - client display;
  - the exact §3.3 values and `EXERCISE0-RL-01` (`PARITY_PENDING`).

## 10. Before-freeze checklist

1. **Pointers:** TIMED-ITEM-0 R4, OFFLINE-0 line 15 and SKILLS-0 §3.5 already point to EXERCISE-0.
   No amendment is needed.
2. **Owned path only:** this file and its task record. No edit to DECISION_INDEX.
3. **Split work:** EXERCISE-CONTENT-1, EXERCISE-1, EXERCISE-PARITY-1, the house-lane dummies, and
   DAILY-REWARD-0's training weapons.
