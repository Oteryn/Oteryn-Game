# OFFLINE-0 Stamina and offline training

- Decision: `OFFLINE0-STAMINA-AND-OFFLINE-TRAINING-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  protocol and determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner answer 2a (#162 5929192803: offline training as in Tibia, beds, training statues and
  exercise weapons; no Rested for now) and the base-mechanics close-out plan (#162 5929069698). D118
  fixed stamina's values but no decision owns its storage, consumption or offline regeneration
  (PREY-0 §6.3: "PREY-EFFECT-1 owns it until stamina lands"); GAME-CHAR-01 Stage B decision 8
  accepted the offline-training counter model but no decision implements it. Both clocks measure the
  same thing, the time between a logout and the next admission, so one decision owns them.
- Scope: stamina, the offline-training pool and training statues. **Beds** (house sleeping, the
  sleeper on the bed, soul and vitals while asleep) are BED-0, a house-lane decision that reuses this
  activation; **exercise weapons** have charges and are TIMED-ITEM-0. Both stay in owner answer 2a's
  scope, sequenced after this one.
- Builds on: GAME-CHAR-01 Stage B decision 8, D118 (owner decision batch D118-D128 §2.4), A13 §4
  (build state, receipt chain, checkpoints), SKILLS-0 §3 (skills in build state), PREY-0 §6.3 (the
  hunting clock), CHAR-POSITION-0 §3.2 (the terminal release transaction), WORLD-INTERACTION-0 §3 and
  §10.1 (map-item USE), ATTACK-0 §4 and PARTY-PVP-0 §8.1 (logout blocks), PREMIUM-ACTIVATION (Premium
  evidence), D3 (database-clock anchoring), owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| OFFLINE-1 | hard (persistence), persistence review | the build-state columns and causes of §3; the logout write (§4.1); the admission settlement (§5) with stamina regeneration and offline training; the database-clock timestamps | SKILLS-0 merged (build-state skills); CHAR-POSITION-1 (terminal release) |
| STAMINA-1 | impl, combat review | stamina use on the hunting clock (§4.2), checkpointed with build checkpoints; D118's effects read the live value | OFFLINE-1; PREY-0's hunting clock owner moves here |
| STATUE-1 | impl, protocol review | the training statue USE (§6): Premium, logout-block and PZ-lock refusal, activation, graceful logout; the disposition `OFFLINE_TRAINING` | OFFLINE-1; WORLDINT-USE-1 |
| OFFLINE-CONTENT-1 | content lane | training statue LocalObjects with their skill, from Canary `skill_trainer.lua` and TibiaWiki | WO-2 |
| OFFLINE-PARITY-1 | impl | fixtures: stamina regeneration at the 39 h and 42 h edges; training tries per skill and vocation for 10 min, 1 h and 12 h | OFFLINE-1 |

Later, each with its own decision: BED-0 (house beds), exercise weapons (TIMED-ITEM-0), showing
stamina and the offline-training pool to the client (the character view wire decision), the
"During your absence you trained for ..." console line (client presentation), Rested (owner answer
2a: "for now", revisit later).

## 1. Question

How do stamina and offline training run across logouts, and how does a player start training at a
statue?

## 2. Facts

**PROVEN**

- GAME-CHAR-01 Stage B decision 8 (owner-accepted): activation selected; at least 10 minutes offline
  before training gain; at most 12 hours of effective continuous training; the pool falls 1 s per
  training second and is restored 1 s per second online or offline without training; consuming a
  refilled pool needs a new activation; the state is "character-specific durable progression state";
  coefficients stay parity gates.
- D118: stamina 42 hours; the top 3 hours give Premium characters +50% XP; at 14 hours or less XP is
  50% and the top-damage character gets no loot; at 0 no XP; regeneration starts after 10 minutes
  offline, 1 minute per 3 offline up to 39 hours, 1 per 6 from 39 to 42.
- A13 §4.1-§4.2 and SKILLS-0 §3: build state is one row per Character in
  `game_character_build_state`; every change is one CharacterRevision with one build receipt whose
  cause has a CHECK on its direction; commits happen at every advance, before a death, at logout and
  at a checkpoint of at most 60 s; a crash loses at most one checkpoint, never a level.
- PREY-0 §6.3: the hunting clock ticks once per 60 s of the session in which the character dealt
  damage to, or took damage from, a creature (not a player); "it is the stamina signal of the XP lane
  (D118)".
- CHAR-POSITION-0 §3.2: the final position write runs inside the terminal release transaction,
  before `session_state` becomes 3; a logout-blocked actor stays in the world until its deadline.
- D3: timestamps that bound gameplay are taken from the database clock (`clock_timestamp()`) at
  commit, never from a client or a runtime wall clock.
- The Tibia manual (`characters.md` §5.1.5): "logging out next to a training statue ... trains that
  skill passively while offline".

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b512114bfd888000d6e1433ed8ecaec7c5b`)

- `skill_trainer.lua`: a statue per skill (sword, axe, club, distance, magic level, fist); use needs
  Premium and no PZ lock; it sets the training skill and logs the character out.
- `offline_training.lua`: offline time capped at 21 days; without a skill the whole offline time
  refills the pool; with a skill and less than 600 s offline, nothing; else training time =
  `min(offline, pool, 43,200 s)`, the remainder refills the pool; tries: melee and fist
  `(t / base attack speed s) / 2`, distance `/ 4`, magic level `t × mana gain amount / gain ticks s`
  as mana; shielding `t / 4` whenever the skill advanced; the skill is cleared.
- `regenerate_stamina.lua`: offline time capped at 21 days, minus 600 s; below 180 s nothing; then
  1 minute per 180 s up to 2,340 minutes (39 h), 1 per 360 s up to 2,520 (42 h).
- `events/scripts/player.lua` `useStamina`: 1 minute per minute of experience gain.

## 3. State (OFFLINE-1; amends A13 §4.1 and §4.2)

`game_character_build_state` gains:

| Column | Range | Absent-row value |
|---|---|---|
| `stamina_minutes` | 0..2,520 | 2,520 (a new character has full stamina, Tibia) |
| `offline_pool_s` | 0..43,200 | 43,200 |
| `offline_skill` | `none` or one of fist, club, sword, axe, distance, magic level | `none` |
| `last_logout_at` | database timestamp or NULL | NULL |
| `last_seen_at` | database timestamp or NULL | NULL |

Build receipts carry the before and after values of the three value columns. New cause directions
(A13 §4.2's CHECK, extended):

- `training` (existing) also admits `stamina_minutes` equal or lower and `offline_pool_s` equal or
  higher (online restore), every other new column equal.
- `logout`: one per terminal release; skills and magic level equal or higher (the final tries
  flush), stamina equal or lower, pool equal or higher, `offline_skill` `none` to a skill (a statue
  activation) or unchanged; `last_logout_at` set.
- `offline_settlement`: one per new lease (keyed by CharacterId and lease generation), at admission;
  skills and magic level equal or higher, stamina equal or higher, pool any value in range,
  `offline_skill` to `none`.

`last_seen_at` is written by every build receipt (the 60 s checkpoint) and the logout receipt, from
the database clock. Amended: A13 §4.1 and §4.2 (this PR).

## 4. Online (OFFLINE-1, STAMINA-1)

### 4.1 Logout

The terminal release transaction (CHAR-POSITION-0 §3.2) commits one `logout` receipt: the final
tries and stamina flush, the online pool restore (`pool + (logout − admission)` seconds, capped at
43,200), the statue activation if one is pending (§6), and `last_logout_at` from the database clock.
A logout-blocked actor (ATTACK-0 §4, PARTY-PVP-0 §8.1) commits it when its deadline ends it.

### 4.2 Stamina use

- Stamina falls one minute per tick of PREY-0's hunting clock (one per 60 s of the session in which
  the character dealt damage to, or took damage from, a creature). Ownership of the clock moves from
  PREY-EFFECT-1 to STAMINA-1; PREY-0 reads the same tick. Amended: PREY-0 §6.3 (this PR).
- The runtime keeps the live value; it commits with the A13 build checkpoints (at most 60 s), before
  a death and at logout. A crash returns at most one checkpoint of stamina (bounded, in the player's
  favour, as A13 and PREY-0 R3).
- D118's effects (the Premium bonus hours, the 14-hour penalty, 0 means no XP) read the live value;
  the XP binding freezes the grade (PREY-0).
- `PARITY_PENDING`: Canary consumes per minute of experience gain; the hunting clock is PREY-0's
  accepted signal.

## 5. Admission settlement (OFFLINE-1)

- After admission commits and before the actor becomes playable, the build writer commits one
  `offline_settlement` receipt for the new lease, keyed by (CharacterId, lease generation); a replay
  returns the first outcome. A same-GameSession reconnect or a channel transfer is not a new lease
  and settles nothing.
- **Offline time** `d` = settlement database time − `last_logout_at`, or − `last_seen_at` when the
  last session ended without a terminal release (a crash), capped at 21 days; 0 when both are NULL.
- **Stamina:** with `o = d − 600 s`, nothing below 180 s; else 1 minute per 180 s of `o` up to 2,340
  minutes, then 1 per 360 s up to 2,520 (D118).
- **Offline training**, only when `offline_skill` is set and the last session ended with a terminal
  release:
  - `d < 600 s`: no training; the skill is cleared.
  - else `t = min(d, offline_pool_s, 43,200)`; tries by the Canary formulas of §2 for the skill, the
    character's top vocation's base attack speed or mana gain, and shielding `t / 4`, applied
    through SKILLS-0 §3's arithmetic (advances included); `offline_pool_s` becomes
    `offline_pool_s − t + (d − t)`, capped at 43,200; the skill is cleared.
- Without a skill: `offline_pool_s` becomes `offline_pool_s + d`, capped at 43,200.
- Every coefficient is `PARITY_PENDING` (decision 8, point 3).

## 6. Training statues (STATUE-1)

- A training statue is a LocalObject with a skill (OFFLINE-CONTENT-1). A USE on it (WORLD-INTERACTION-0
  §3: reach, protection zone, `world_action` cooldown) by a Premium character (PREMIUM-ACTIVATION
  evidence, fail closed) that is neither logout-blocked nor PZ-locked marks the pending activation and
  starts a graceful logout; the client gets the disposition `OFFLINE_TRAINING` under
  `WORLD_INTERACTION_V1`. A blocked character gets `PZ_BLOCKED`; a non-Premium one `NOT_POSSIBLE` with
  the Premium message. Amended: WORLD-INTERACTION-0 §10.1 (this PR).
- The activation commits only with the `logout` receipt; a session that ends without a terminal
  release trains nothing.

## 7. Rejected options

- **Separate stamina and offline rows outside the build state.** They are character progression
  values; the build writer already has a revision, receipts and checkpoints.
- **Computing offline time from the client or the runtime clock.** D3 binds gameplay time to the
  database clock.
- **Training after a crash.** The activation is part of the graceful logout (Canary removes the
  character after setting the skill).
- **A write per stamina minute.** A13 checkpoints bound both the write rate and the crash loss.

## 8. Architect rulings (owner rule 5905825574)

- **R1. Stamina signal.** a) PREY-0's hunting clock (recommended: one accepted signal for prey time
  and stamina); b) Canary's experience-gain minute. **Ruled a)**, `PARITY_PENDING`.
- **R2. Crash offline time.** a) From `last_seen_at` (recommended: at most 60 s off, no training); b)
  no regeneration after a crash. **Ruled a).**
- **R3. Beds and exercise weapons.** a) Their own decisions after this one (recommended: beds need
  house item state, exercise weapons charges); b) all here. **Ruled a).**

## 9. Owner questions

None. Owner answer 2a and decision 8 fix the scope and the model; D118 fixes the stamina values.

## 10. Decision test

- **Must decide now:** YES. Without it stamina never falls or regenerates, D118's XP rules have no
  input, and statues do nothing.
- **Minimum sufficient:** five columns and three causes on the existing build writer, one settlement
  per lease, one statue USE.
- **Superseding evidence:** an official source on stamina use or offline-training coefficients.
- **Deliberately not decided:** beds, exercise weapons, client display of stamina and the pool,
  Rested.

## 11. Before-freeze checklist

1. **Contract amendments:** A13 §4.1 and §4.2 (columns, causes); PREY-0 §6.3 (clock ownership);
   WORLD-INTERACTION-0 §10.1 (disposition). Applied in this PR.
2. **Serialization:** every write is a build receipt on A13's ordered build writer with its expected
   revision; the settlement is keyed per lease and replays.
3. **Restart:** all state is durable; a crash loses at most one checkpoint of stamina and tries, and
   no training starts after a crash.
4. **Typed references:** the settlement key is (CharacterId, lease generation); timestamps are
   database times.
5. **Wire:** one new disposition under `WORLD_INTERACTION_V1`; no new command.
6. **Split work:** logout and settlement are one receipt each, in their own transactions.
