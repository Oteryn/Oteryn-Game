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
| OFFLINE-1 | hard (persistence), persistence review | the build-state columns, causes and marker keys of §3; the logout marker (§4.1); the admission settlement (§5); the death-receipt fields; `verify_character_integrity` and the binding version | CHAR-BUILD-1 (`0030`, merged); SKILLS-0's build-state skills; CHAR-POSITION-1; a slot in the shared guard-rewrite order |
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
  as mana (gain amount of the current vocation, ticks of the top vocation); shielding `t / 4` only
  when the trained skill's level or percent changed; tries are truncated to integers; with `t < 60`
  the pool is consumed but no tries are granted; the skill is cleared.
- `regenerate_stamina.lua`: offline time capped at 21 days, minus 600 s; below 180 s nothing; then
  1 minute per 180 s up to 2,340 minutes (39 h), 1 per 360 s up to 2,520 (42 h).
- `events/scripts/player.lua` `useStamina`: 1 minute per minute of experience gain.

## 3. State (OFFLINE-1; amends A13 §4.1, §4.2, §4.6 and SKILLS-0 §3.2)

`game_character_build_state` gains three value columns:

| Column | Range | Absent-row value |
|---|---|---|
| `stamina_minutes` | 0..2,520 | 2,520 (a new character has full stamina, Tibia) |
| `offline_pool_s` | 0..43,200 | 43,200 |
| `offline_skill` | `none` or one of fist, club, sword, axe, distance, magic level | `none` |

- **No timestamp columns.** Every time this decision needs is a build receipt's `committed_at`
  (the database clock, D3), which the `0009` guard already binds: the latest `logout` receipt gives
  the logout time, the latest `offline_settlement` receipt the session start. Nothing a row-only
  write could fake.
- **Receipts** carry the before and after values of the three columns. A13's row guard and chain
  rule (before equals the previous after) cover them; death receipts carry them equal before and
  after (A13 §4.6, DEATH-0 §3.1), and the death flush commits pending stamina with pending tries.
- **Causes** (A13 §4.2's CHECK, extended; vocation equal in all three):
  - `training` (existing): skills and magic level equal or higher, stamina equal or lower, pool and
    `offline_skill` equal; at least one of the eight families or stamina changes (the strict-OR and
    the no-op rule extended to stamina).
  - `logout`: a **marker**, committed once per lease even when nothing else changes (exempt from the
    no-op rule): skills, magic level and stamina as `training` (the final flush), pool equal or higher
    (the online restore, §4.1), `offline_skill` `none` to a skill (a statue activation) or unchanged.
  - `offline_settlement`: a **marker**, committed once per lease at admission even when nothing else
    changes: stamina equal or higher; pool any value in range; `offline_skill` to `none`; every skill
    family equal except the trained family and shielding, which may rise.
- **Keys.** Both markers carry the lease generation; `UNIQUE (character_id, lease_generation)` per
  marker cause. A same-key retry returns the first receipt.
- **Time binding.** A marker reads the database time once under the `character_root` lock, binds it
  as an `as_of` input of the binding, and its after values are a pure function of `as_of` and the
  stored row; the receipt's `committed_at` is that same transaction's time.
- Amended: A13 §4.1, §4.2 and §4.6; SKILLS-0 §3.2 (this PR). OFFLINE-1 extends
  `verify_character_integrity` and bumps the build binding version, in the shared guard-rewrite order.

## 4. Online (OFFLINE-1, STAMINA-1)

### 4.1 Logout

- The `logout` marker is the last queued build commit of the session, committed in its own
  transaction before the terminal release (the H-1 actor-end pattern), not inside it. Amended:
  CHAR-POSITION-0 §3.2 (this PR).
- It carries the final tries and stamina flush, the statue activation if one is pending (§6), and
  the online pool restore: `pool + (as_of − session start)` seconds, capped at 43,200, where the
  session start is this lease's `offline_settlement` `committed_at`. Checkpoints never restore the
  pool.
- It is skipped (no marker, no activation) when a respawn is pending, when the character has no
  progression row yet, or outside a Channel scope (the build writer serves Channel scopes only); the
  next admission then settles as after a crash (§5).
- A logout-blocked actor (ATTACK-0 §4, PARTY-PVP-0 §8.1) commits it when its deadline ends it.

### 4.2 Stamina use

- Stamina falls one minute per tick of PREY-0's hunting clock (one per 60 s of the session in which
  the character dealt damage to, or took damage from, a creature). Ownership of the clock moves from
  PREY-EFFECT-1 to STAMINA-1; PREY-0 reads the same tick. Amended: PREY-0 §6.3 (this PR).
- The runtime keeps the live value; it commits with the A13 build checkpoints (at most 60 s), in the
  death flush and in the `logout` marker. A crash returns at most one checkpoint of stamina
  (bounded, in the player's favour, as A13 and PREY-0 R3).
- D118's effects (the Premium bonus hours, the 14-hour penalty, 0 means no XP) read the live value;
  the XP binding freezes the grade (PREY-0).
- `PARITY_PENDING`: Canary consumes per minute of experience gain; the hunting clock is PREY-0's
  accepted signal.

## 5. Admission settlement (OFFLINE-1)

- After admission commits and before the actor becomes playable, the build writer commits the
  `offline_settlement` marker for the new lease, on a Channel scope (on another scope it waits until
  the character is on one). A same-GameSession reconnect keeps its lease and settles nothing.
- **Offline time.** If the latest `logout` marker is newer than the latest `offline_settlement`,
  `d = max(0, as_of − logout committed_at)`, capped at 21 days. Otherwise the last session ended
  without a logout marker (a crash, or a skipped marker): `d = 0`, so nothing regenerates and no
  training happens (R2). With no earlier marker at all (a first login), `d = 0`.
- **Stamina:** with `o = d − 600 s`, nothing below 180 s; else `floor(o / 180)` minutes up to 2,340,
  then `floor(rest / 360)` up to 2,520 (D118, Canary).
- **Offline training**, when `offline_skill` is set (it was set by a logout marker):
  - `d < 600 s`: nothing trains and the pool is unchanged (Canary); the skill is cleared.
  - else `t = min(d, offline_pool_s, 43,200)`; the pool becomes `offline_pool_s − t + (d − t)`, capped
    at 43,200; if `t < 60` no tries are granted (Canary); else, in integers:
    - melee and fist: `floor(t × 1,000 / (base attack speed ms × 2))` tries; distance `× 4` in the
      divisor;
    - magic level: `floor(t × mana gain amount × 1,000 / max(gain ticks ms, 1,000))` as mana spent,
      with the current vocation's gain amount and the top vocation's ticks (Canary);
    - shielding `floor(t / 4)` tries, only if the trained family's (level, tries) rose;
    - applied through SKILLS-0 §3's arithmetic, advances included.
  - The skill is cleared.
- Without a skill: the pool becomes `offline_pool_s + d`, capped at 43,200.
- Every coefficient is `PARITY_PENDING` (decision 8, point 3).

## 6. Training statues (STATUE-1)

- A training statue is a LocalObject with a skill (OFFLINE-CONTENT-1). A USE on it (WORLD-INTERACTION-0
  §3: reach, protection zone, `world_action` cooldown) by a Premium character (PREMIUM-ACTIVATION
  evidence, fail closed; refused until PREM-3 delivers Premium) that is neither logout-blocked nor
  PZ-locked marks the pending activation and starts a graceful logout; the client gets the
  disposition `OFFLINE_TRAINING` under `WORLD_INTERACTION_V1`. A blocked character gets
  `PZ_BLOCKED`; a non-Premium one `SEALED` with the Premium message id. Amended: WORLD-INTERACTION-0
  §10.1 (this PR).
- The activation commits only with the `logout` marker; a session that ends without one trains
  nothing.

## 7. Rejected options

- **Separate stamina and offline rows outside the build state.** They are character progression
  values; the build writer already has a revision, receipts and checkpoints.
- **Timestamp columns on the row.** A row-only write could fake offline time; receipt
  `committed_at` is bound by the guard.
- **The logout marker inside the terminal release.** The release takes no `character_root` lock or
  revision fence, and a build refusal would abort it.
- **Computing offline time from the client or the runtime clock.** D3 binds gameplay time to the
  database clock.
- **Training after a crash.** The activation is part of the graceful logout (Canary removes the
  character after setting the skill).
- **A write per stamina minute.** A13 checkpoints bound both the write rate and the crash loss.

## 8. Architect rulings (owner rule 5905825574)

- **R1. Stamina signal.** a) PREY-0's hunting clock (recommended: one accepted signal for prey time
  and stamina); b) Canary's experience-gain minute. **Ruled a)**, `PARITY_PENDING`.
- **R2. Crash offline time.** a) A last-seen time (needs a heartbeat write per online character); b)
  no regeneration and no training after a session that ended without a logout marker (recommended:
  no new write path, fail closed, and Tibia's crash rollback also loses time). **Ruled b)**, a
  declared difference (`PARITY_PENDING`).
- **R3. Beds and exercise weapons.** a) Their own decisions after this one (recommended: beds need
  house item state, exercise weapons charges); b) all here. **Ruled a).**

## 9. Owner questions

None. Owner answer 2a and decision 8 fix the scope and the model; D118 fixes the stamina values.

## 10. Decision test

- **Must decide now:** YES. Without it stamina never falls or regenerates, D118's XP rules have no
  input, and statues do nothing.
- **Minimum sufficient:** three columns and two marker causes on the existing build writer, one
  settlement per lease, one statue USE.
- **Superseding evidence:** an official source on stamina use or offline-training coefficients.
- **Deliberately not decided:** beds, exercise weapons, client display of stamina and the pool,
  Rested.

## 11. Before-freeze checklist

1. **Contract amendments:** A13 §4.1, §4.2, §4.6 (columns, causes, death); SKILLS-0 §3.2 (no-op and
   strict-OR rules); CHAR-POSITION-0 §3.2 (the logout marker before the release); PREY-0 §6.3 (clock
   ownership); WORLD-INTERACTION-0 §10.1 (disposition). Applied in this PR.
2. **Serialization:** every write is a build receipt on A13's ordered build writer with its expected
   revision and the `character_root` lock; markers are keyed per lease with a UNIQUE constraint and
   replay.
3. **Restart:** all state is durable; a crash loses at most one checkpoint of stamina and tries, and
   the next login regenerates nothing and trains nothing (R2).
4. **Typed references:** marker keys are (CharacterId, lease generation); times are receipt
   `committed_at` values from the database clock.
5. **Wire:** one new disposition under `WORLD_INTERACTION_V1`; no new command.
6. **Split work:** logout and settlement are one marker receipt each, in their own transactions; the
   terminal release follows the logout marker.
