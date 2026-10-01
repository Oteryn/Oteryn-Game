# BED-0 House beds

- Decision: `BED0-HOUSE-BEDS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner answer 2a (#162 5929192803: offline training as in Tibia, beds, training statues and
  exercise weapons; no Rested for now); OFFLINE-0's scope ("Beds ... are BED-0, a house-lane decision
  that reuses this activation") and its ruling R3; HOUSE-RUNTIME-0 §6.2 ("Beds are inert until the
  beds decision").
- Builds on: OFFLINE-0 §3-§6 (markers, settlement, statue activation); HOUSE-RUNTIME-0 §3, §5, §6
  and §7 (house scope, roles, map items, logout and login, disposition quiesce); HOUSE-OWN-0
  (property row, ownership transitions, disposition fence); EXP-HOUSES-01 §16.4 and §19;
  CONDITIONS-0 §6.3 (durable food time); the vitals rule of the spell cast and vitals contract
  (HP, mana and soul are runtime-actor-local until DUR-02 durable vitals); WORLD-INTERACTION-0 §3 and
  §10.1; PREMIUM-ACTIVATION; D3 (database clock); the Tibia manual `houses.md` §5.7.1 and
  `characters.md` §5.1.5; owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| BED-CONTENT-1 | content lane | the bed facts on Item definitions (§3): the partner direction, the free and occupied item types per sex, from Canary `items.xml` (`partnerdirection`, `transformonuse`, `transformto`) with TibiaWiki for names | ITEM-SEM-2b |
| BED-1 | hard (persistence), persistence and protocol review | OFFLINE-0's markers in a house scope (§5.1); the sleeper table and its writers (§5.2), the bed USE and sleep command (§4), owner wake, the frees on ownership change and disposition (§6), the settlement step (§7.1), the view of an occupied bed (§8) | OFFLINE-1; STATUE-1 (the shared activation path); HOUSE-RUNTIME-1; HOUSE-VIEW-1; WORLDINT-WIRE-1 |
| BED-REGEN-1 | impl, persistence review | the vitals and food part of the settlement (§7.2) | BED-1; DUR-02 durable vitals |

Later, each with its own decision: bed modification kits (a transform of both bed parts by the
owner), Rested (owner answer 2a, "for now"), the Residence bed requirement (EXP-HOUSES-01 §19).

## 1. Question

What does a bed in a house do, who may sleep in it, and how does sleeping join OFFLINE-0's offline
training?

## 2. Facts

**PROVEN**

- Tibia manual `houses.md` §5.7.1: beds are usable only by Premium characters, while offline;
  asleep, a character regains 1 soul point per 15 minutes, and hit points and mana while its food
  lasts, slower than online; skills train while sleeping.
- Tibia manual `characters.md` §5.1.5: selecting a skill to train while sleeping in a bed trains
  that skill passively while offline.
- OFFLINE-0 §4.1, §5 and §6: an activation (`offline_skill`) commits only with the `logout` marker;
  the `offline_settlement` marker at the next admission trains and clears it; offline time `d` is
  positive only when the latest build receipt is the previous lease's `logout` marker; a crash
  trains nothing (R2).
- HOUSE-RUNTIME-0: each active house is one scope with one writer per `HouseId` (§3); the interior is
  a protection zone (§3); owner, subowner and guest may use objects (§5.2); beds are map-authored,
  never pickupable, and inert until this decision (§6.2); a login into a house revalidates access and
  the tile (§6.3); a disposition moves everyone out and closes entry (§7).
- EXP-HOUSES-01 §16.4: the ACL **may** distinguish bed use; §19 leaves bed counts and the bed ACL
  tunable.
- CONDITIONS-0 §6.3: the remaining `FOOD_REGENERATION` time is durable, capped at 1,200 s.
- Spell cast and vitals contract (SPELL-D2): HP, mana and soul are runtime-actor-local; a fresh
  admission restarts them at the maximum until DUR-02 durable vitals land.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`, `beditem.cpp`)

- A bed is two items joined by a partner direction; sleeping transforms both to the occupied item
  type for the sleeper's sex, and waking transforms them back.
- Use: Premium is required; a free bed can be used; an occupied bed refuses everyone except the
  house owner, whose use wakes the sleeper and frees the bed (and does not put the owner to sleep).
- Sleeping sends the offline training dialog; then the character is logged out.
- Waking: `slept = now − sleep start`; if a food regeneration condition exists, HP and mana each
  rise by `min(food remaining s, slept) / 30` and the food time falls by 30 s per point; soul rises
  by `slept / 900`.
- A change of house owner wakes every sleeper of the house.

## 3. Bed content (BED-CONTENT-1)

An Item definition that is a bed part carries `bed {partner_direction, free_type,
occupied_type_male, occupied_type_female}`. A validator checks that both parts of every placed bed
in the active bundle name each other. A bed is identified by its **head part's tile** in the house
(`BedKey = (HouseId, x, y, z)`). Bed parts stay map items: never pickupable, never moved
(HOUSE-RUNTIME-0 §6.2).

## 4. Going to sleep (BED-1)

- **USE on a bed part** (WORLD-INTERACTION-0 §3: reach, `world_action` cooldown) inside the house
  scope, by any character the house admits: owner, subowner or guest (HOUSE-RUNTIME-0 §5.2, Tibia;
  no separate bed capability, R1). Refusals, in this order:
  - not Premium: `SEALED` with the Premium message id (PREMIUM-ACTIVATION evidence, fail closed;
    refused until PREM-3 delivers Premium);
  - logout-blocked or PZ-locked: `PZ_BLOCKED`;
  - the bed is occupied and the user is not the house owner: `BED_OCCUPIED`;
  - the bed is occupied and the user is the house owner: the bed is freed (§6), the answer is
    `BED_WOKEN`, and the owner does not fall asleep.
- **Free bed:** the house runtime reserves the bed for the user for `BED0-RL-01` (60 s) and answers
  `BED_CHOOSE_SKILL`. The client shows the offline training dialog.
- **Choice:** the new command `BED_SLEEP_INTENT {bed map_item handle, skill}`, with `skill` one of
  fist, club, sword, axe, distance or magic level. The house runtime checks the reservation, the
  refusals above and reach again. On success it marks the pending activation `{skill, BedKey}` and
  starts the graceful logout, exactly as a training statue does (OFFLINE-0 §6); the answer is
  `OFFLINE_TRAINING`. A cancel, a timeout, a move out of reach or a session end releases the
  reservation and nothing is written.
- The client's dialog choices are the six trainable skills; whether Tibia also allows sleeping
  without a skill is `PARITY_PENDING`, and v1 has no skill-less sleep (R2).

## 5. The sleeper record (BED-1)

### 5.1 Markers in a house scope

A sleeper logs out from a house scope, and usually logs back in there (HOUSE-RUNTIME-0 §6.3).
OFFLINE-0 §4.1 skipped the markers outside a Channel scope, which would also lose every house
logout's stamina regeneration. HOUSE-RUNTIME-1 therefore runs the build writer's marker path for
the characters in its scope: the `logout` marker at a graceful logout and the `offline_settlement`
marker at an admission into the house, with OFFLINE-0's keys, receipts, `character_root` lock and
session fence (the fences HOUSE-RUNTIME-0 §6.1 already takes). Amended: OFFLINE-0 §4.1 and §5.

### 5.2 Table

Table `game_house_bed_sleepers`:

| Column | Meaning |
|---|---|
| `house_id`, `bed_x`, `bed_y`, `bed_z` | `BedKey` |
| `character_id` | the sleeper; `UNIQUE` (at most one row per character, open or freed) |
| `lease_generation` | the lease whose `logout` marker created the row |
| `occupied_variant` | `male` or `female`, read from the character at insert |
| `slept_at` | the `logout` marker's `committed_at` (database clock, D3) |
| `freed_at`, `freed_cause` | NULL while the sleeper occupies the bed; else the database time and one of `owner_wake`, `owner_change`, `disposition`, `bed_removed` |

- A partial `UNIQUE (house_id, bed_x, bed_y, bed_z) WHERE freed_at IS NULL`: one sleeper per bed.
- **Insert:** in the `logout` marker's transaction (OFFLINE-0 §4.1), together with the activation.
  The same transaction first deletes any older row of the character (one a skipped settlement left),
  so the `character_id` constraint never blocks a new sleep.
  It takes `FOR SHARE` on the house property row and inserts only when the property is `OWNED` or
  `MOVE_OUT_PENDING` (or the guildhall equivalent) with no content fence set. The fence transaction
  takes the property row `FOR UPDATE`, so it either commits first (the marker's `FOR SHARE` read then
  sees the fence and inserts nothing) or waits for the marker and then frees the new row with the
  others (§6). On a unique conflict (the bed was taken or the house fenced in between), the marker commits without
  the row **and without the activation**: no bed, no training.
- **Free:** an `UPDATE` setting `freed_at` and `freed_cause` (§6). The row stays until the
  settlement consumes it, so the sleep time still counts up to `freed_at`.
- **Delete:** only by the settlement (§7) of the sleeper's next admission, which deletes the
  character's row whatever its state, or by the next `logout` marker's insert (above).
- **Replay:** the row is part of the `logout` marker. A marker retry is answered from its first
  receipt by key (OFFLINE-0 §3) before any write, so the insert never runs twice; a settlement retry
  likewise never deletes twice.
- The house runtime reads the open rows of its house at activation and on every change it makes;
  it is the only writer of `owner_wake` and `bed_removed`.

## 6. Waking without a login (BED-1)

- **Owner wake:** the owner's USE on an occupied bed (§4).
- **Owner change and disposition:** the HOUSE-OWN-0 §7 fence transaction (move-out, eviction,
  catalogue retirement) frees every open row of the house (`disposition`), and so does any later
  transition that changes the owner without a disposition, such as a player-to-player transfer
  (`owner_change`), in its own transaction. This matches Canary, which wakes every sleeper when the
  owner changes. Amended: HOUSE-OWN-0 §7.
- **Bed removed:** when the house runtime activates and an open row's `BedKey` is no longer a bed
  head in the active bundle, it frees the row (`bed_removed`).
- An access revocation does not wake a sleeper. At the sleeper's next login HOUSE-RUNTIME-0 §6.3
  revalidates access and places it at the entrance if it lost access, and the settlement then
  consumes its row.

## 7. Waking at login (OFFLINE-0's settlement)

### 7.1 Training and the bed (BED-1)

The `offline_settlement` marker (OFFLINE-0 §5) also reads the character's sleeper row, in the same
transaction:

- The row counts only when its `lease_generation` is the previous lease's and that lease's `logout`
  marker is the latest build receipt (the same test as OFFLINE-0's `d > 0`). Otherwise it is deleted
  and nothing else happens.
- When it counts: `sleep_s = min(d, (freed_at − slept_at) if freed, else d)`, capped at 21 days.
  Offline training runs as OFFLINE-0 §5 says, over `d`. A bed freed by the owner or an owner change
  does not cut training short (Canary keeps the training skill set when the owner wakes a sleeper);
  only the vitals of §7.2 use `sleep_s`.
- The settlement deletes the row and the bed shows as free from then on.

### 7.2 Vitals and food (BED-REGEN-1, waits for DUR-02 durable vitals)

When the character has durable vitals:

- with `f` the durable food time (CONDITIONS-0 §6.3): `p = floor(min(f, sleep_s) / 30)`; HP and
  mana each rise by `p` up to their maximum; `f` falls by `30 × p`;
- soul rises by `floor(sleep_s / 900)` up to its maximum.

Until DUR-02 lands, vitals restart at their maximum at every fresh admission, so BED-1 applies
neither part and **does not consume food time** (a declared, temporary difference). All three
coefficients are `PARITY_PENDING`; the manual confirms the soul rate.

## 8. What others see (BED-1, HOUSE-VIEW-1)

- An occupied bed shows both parts as its `occupied_type_<variant>`; a free bed shows `free_type`.
  HOUSE-VIEW-1 renders the parts from the open rows in house scope snapshots and deltas, and in the
  Channel window projection, as a map item presentation change (no new map message).
- Look on an occupied bed adds "<name> is sleeping there." from the row's character.

## 9. Wire (WORLDINT-WIRE-1, under `WORLD_INTERACTION_V1`)

- New command `BED_SLEEP_INTENT {target map_item handle, skill u8}`, at most `BED0-RL-02` (24 bytes),
  type number reserved on #162 at allocation, non-durable, replay-safe by the reservation.
- New dispositions: `BED_CHOOSE_SKILL`, `BED_OCCUPIED`, `BED_WOKEN`; reused: `OFFLINE_TRAINING`,
  `SEALED`, `PZ_BLOCKED`, `TOO_FAR`, `EXHAUSTED`, `STALE`. Amended: WORLD-INTERACTION-0 §10.1.

## 10. Rows

| Row | Value |
|---|---|
| `BED0-RL-01` bed reservation awaiting the skill choice | 60 s |
| `BED0-RL-02` `BED_SLEEP_INTENT` size | 24 bytes |
| `BED0-RL-03` sleep time counted | 21 days (OFFLINE-0's offline cap) |

Each with max and max+1 tests.

## 11. Rejected options

- **A bed capability in the ACL now.** Tibia lets every invited character sleep; EXP-HOUSES-01
  §16.4 allows but does not require one. A later revision may add it.
- **Waking the sleeper by writing its character state.** The sleeper is offline; another actor never
  writes its revision. The row records the free time and the sleeper's own settlement uses it.
- **Bed state in the house item custody.** Beds are map items, not `HouseInterior` items; occupancy
  is a separate row with its own constraint.
- **Consuming food now.** Without durable vitals the food would buy nothing.

## 12. Architect rulings (owner rule 5905825574)

- **R1. Who may sleep.** a) Everyone the house admits (recommended: Tibia, and HOUSE-RUNTIME-0 §5.2
  "use objects"); b) owner and subowners only. **Ruled a).**
- **R2. Sleeping without a skill.** a) No, the dialog has the six skills and cancel aborts
  (recommended: matches Canary's dialog; the pool still refills from any offline time without a
  skill); b) a "no training" choice. **Ruled a)**, `PARITY_PENDING`.
- **R3. Owner wake and training.** a) The sleeper keeps its training up to the next login
  (recommended: Canary keeps the skill; training is OFFLINE-0's, not the bed's); b) training stops
  at the wake. **Ruled a).**

## 13. Owner questions

None. Owner answer 2a sets beds as in Tibia; the manual and Canary give the rules.

## 14. Decision test

- **Must decide now:** YES. Owner answer 2a puts beds in the base; HOUSE-RUNTIME-0 left them inert.
- **Minimum sufficient:** one table, one command, three dispositions, one settlement step, and
  OFFLINE-0's existing markers run in the house scope too, reusing its activation and the house
  runtime.
- **Superseding evidence:** an official source on skill-less sleep or the regeneration rates.
- **Deliberately not decided:** bed modification kits, Rested, the Residence bed requirement, a bed
  ACL capability.

## 15. Before-freeze checklist

1. **Contract amendments:** HOUSE-RUNTIME-0 §6.2 (beds); HOUSE-OWN-0 §7 (the fence frees the beds);
   OFFLINE-0 §4.1 and §5 (both markers also in a house scope), §5 and §6 (the settlement reads the
   sleeper row; a bed is a second activation source);
   WORLD-INTERACTION-0 §10.1 (command and dispositions). Applied in this PR.
2. **Serialization:** the house runtime is the single writer per `HouseId` for reservations, owner
   wake and bed removal; the insert runs in the `logout` marker under the property row lock; owner
   changes free rows in their own transition transaction; the partial unique index keeps one
   sleeper per bed.
3. **Restart:** the row is durable; a reservation is runtime-only and lost at a restart; a crash
   before the `logout` marker writes no row and trains nothing (OFFLINE-0 R2).
4. **Typed references:** `BedKey` from the active bundle, `CharacterId`, lease generation, database
   times.
5. **Wire:** one command and three dispositions under `WORLD_INTERACTION_V1`.
6. **Split work:** none.
