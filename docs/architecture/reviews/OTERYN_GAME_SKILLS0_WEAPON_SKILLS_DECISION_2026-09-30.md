# SKILLS-0 Weapon skills, shielding and fishing

- Decision: `SKILLS0-WEAPON-SKILLS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review
  (persistence) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the architect programme plan (#162 5910870596, M1): no skill level is stored today,
  and ATTACK-0 needs skill values
- Amends: A13 (`OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md`) §4.1, §4.2, §4.4,
  §4.5, §4.6 and §5, by extending build state from magic level to all eight Tibia skills (in this PR, a
  pointer paragraph in A13)
- Builds on: A13 (D150, D151), DEATH-0 §3.1, DUR-02 rule 2, GAME-CHAR-01 Stage B (the eight skill
  categories), ATTACK-0, owner rule 5905825574
- Runtime, migration and production authority: NONE.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

- **CHAR-BUILD-1** (not yet allocated) takes the columns of §3.1 in the same migration, table and
  receipt kind as magic level. No second table and no second receipt kind.
- **W2b** (training) accumulates skill tries next to mana spent (§3.4), with the formula table of
  §3.5, and converts progress at the vocation choice (§3.3).
- **DEATH skill loss** joins the A13 "DEATH ML loss" child (§3.6).

## 1. Question

Where do the seven weapon and utility skills live, how do they advance, and what does a death
take?

## 2. Facts

**PROVEN**

- A13 makes vocation, magic level and `mana_spent` durable build state, with one build receipt kind
  (causes `training`, `vocation_choice`, `promotion`), checkpoints of at most 60 s, a flush before
  death and death-loss fields in the death receipt. CHAR-BUILD-1 is not allocated yet (#162
  5910691466 lists it after 0026 and 0027).
- No migration stores a skill level (`0009` has level and experience only).
- GAME-CHAR-01 Stage B fixes the eight categories: fist, club, sword, axe, distance, shielding,
  fishing and magic level.
- The Tibia manual (`characters.md` §5.1.5, §5.1.11): skills rise through use, not with level;
  a death takes progress, reduced by promotion and blessings.

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b51`)

- Tries needed to reach level L: `base x multiplier^(L - 11)`, so going from 10 to 11 costs
  `base`. Bases: 50 for fist, club, sword and axe, 30 distance, 100 shielding, 20 fishing
  (`vocation.cpp:236, 268-275`). Starting level 10.
- Multipliers (`data/XML/vocations.xml`), in the order fist, club, sword, axe, distance,
  shielding, fishing:
  - none: 1.5, 2.0, 2.0, 2.0, 2.0, 1.5, 1.1 (`:7-13`);
  - knight: 1.1 fist, 1.1 club, sword and axe, 1.4 distance (`:55-59`);
  - elite knight swaps fist and distance, 1.4 and 1.1 (`:112, 116`). This looks like a Canary
    data error; the row carries `PARITY_PENDING`.
  - The other vocations take their rows from the same file, cited in the content table.
- One try may advance several levels at once (`addSkillAdvance`, `player.cpp:1204-1208`).
- Training (`weapons.cpp`, `player.cpp`, `creature.cpp`):
  - melee: 1 try per attack on a target that is not immune, including an attack blocked by
    defence or armor (`weapons.cpp:591-596`). After 30 blocked-only attacks, no tries until one
    attack deals damage (`player.cpp:3857-3882`);
  - distance: 2 tries per unblocked hit, 1 per blocked hit (`weapons.cpp:942-963`); a miss
    repeats the previous block state (`:320-340`);
  - shielding: 1 try per block, only with a shield, only when the block used the defence budget
    of at most two per round, and capped at 30 in a row until the player deals damage again
    (`player.cpp:3843-3850`, `creature.cpp:959-991`);
  - fishing: 1 try per use (`fishing.lua:98`).
- Death, per skill (`player.cpp:4127-4143`): the cumulative tries are the tries of all levels
  from 11 up to the current level plus the current tries. The loss is
  `floor(cumulative x loss ratio)`, re-levelled from level 10 and 0 tries, never below that.
  Canary's ratio is the experience loss divided by the experience, but a flat 10% below level 24
  (`player.cpp:7370-7378`). Canary also loops over special skills; this decision covers the seven
  only.
- Vocation change (`dawnport.lua:22-86`): Canary sums the cumulative tries of every skill and the
  cumulative mana of magic level, then re-levels them under the new vocation's multipliers.
  Dawnport caps magic level only (`dawnport.lua:4-19`); weapon skills have no Dawnport cap.

## 3. Decision

### 3.1 Storage (amends A13 §4.1)

- `game_character_build_state` gains, per skill (fist, club, sword, axe, distance, shielding,
  fishing), a `level` and a `tries` column. No row still means vocation `none`, magic level 0,
  `mana_spent` 0 and every skill at level 10 with 0 tries.
- CHECKs on the row, the build receipt and the death receipt's build fields: every skill `level`
  is between 10 and 1,000; `tries` is between 0 and 2^63 - 1. Tries toward the next level are
  held as a count within the current level, as `mana_spent` is.

### 3.2 Receipt chain (amends A13 §4.2)

- The build receipt carries the before and after values of all seven skills as well.
- "At least one changes" covers all eight families: a receipt that changes nothing is not a
  receipt.
- Cause directions:
  - `training`: vocation equal; for each family, (`level`, `tries`) is equal or strictly larger,
    compared in that order; at least one family strictly larger. The row CHECK is each family
    `>=` and an OR of `>`.
  - `vocation_choice`: see §3.3. The SQL CHECK only requires `none` to a vocation key; the writer
    checks the conversion.
  - `promotion`: a vocation key to another key; all eight families equal.
- The chain seed for the first build-carrying receipt becomes (`none`, 0, 0, and seven times
  (10, 0)).
- The binding digest covers every before and after value, the skills included.

### 3.3 Vocation choice (amends A13 §4.2 and §4.4)

- Architect decision, applying owner rule 5905825574 (Global parity) to the Canary evidence: the
  vocation choice keeps each family's cumulative progress and re-levels it under the new
  vocation's multipliers. That covers the seven skills and magic level (cumulative mana).
  Keeping raw values equal would leave tries above the new requirement.
- The build writer computes the conversion from the content formula table and binds the table
  revision in the binding. SQL does not check it: SQL checks the cause direction only.
- This replaces A13's "`magic_level` and `mana_spent` equal" for `vocation_choice`.
- `PARITY_PENDING` until checked against an owner-trusted or official source.

### 3.4 Training (amends A13 §4.5)

- The live session accumulates tries by the training rules of §2, fed by ATTACK-0 (melee,
  distance, shielding) and ITEM-USE-0 (fishing). Both are pending decisions; their references
  resolve when they merge.
- The commits are the A13 ones, for all families together: every level advance (always durable;
  the live level changes after its receipt commits), before a death, at logout, and at a
  checkpoint of at most 60 s. A checkpoint with no change is skipped.
- A receipt may carry an advance of several levels, in one family or several.
- A crash may lose at most one checkpoint of tries, never a level.
- Skills go through the build writer, not through progression. So the XP calculator's
  `UnsupportedSkillOrProficiencyMutation` marker (`progression.rs`) stays valid.

### 3.5 Formula

- The Canary formula, multipliers and training rules of §2 are the implementation, as one
  versioned table in content. Each value carries `PARITY_PENDING` until checked against an
  owner-trusted fan source or an official CipSoft source; an official source governs.
- Offline training (beds, statues) and exercise weapons are later decisions.

### 3.6 Death (amends A13 §4.6)

- The flush before death includes pending tries.
- The death receipt's nullable build fields gain the before and after values of the seven skills.
  All build fields stay all NULL or all non-NULL.
- When non-NULL: for every family, (`level`, `tries`) after is equal or lower than before, and at
  least one family is strictly lower.
- The skill seed is (10, 0), and no family can go below it. So a death is never the first
  build-carrying receipt, as in A13.
- **Loss ratio.** Architect decision:
  - The numerator is the experience the death actually takes by DEATH-0 (D58 with its blessing
    and promotion reductions, capped at the current experience).
  - The denominator is `experience_before`.
  - With `experience_before = 0`, no skill progress is lost.
  - This follows D58, which uses one formula from level 1, rather than Canary's flat 10% below
    level 24. It carries `PARITY_PENDING`.
- **Loss per skill.** The cumulative tries (§2) times the ratio, computed in exact integer
  arithmetic and floored. The remaining tries are re-levelled from (10, 0), so a level can drop.
- Until the DEATH skill-loss child merges, a death writes NULL build fields and takes no skill
  progress. That is not Global parity, so skill training is not enabled in production before it
  merges, as A13 §4.6 says for magic level.

## 4. Rejected options

- **A separate skills table and receipt kind.** It would add a second training cause chain for the
  same checkpoint cadence. One build receipt covers every trained value.
- **One receipt per hit.** DUR-02 rule 2 would advance `CharacterRevision` on every swing.
- **Skill loss as a flat level decrement.** Tibia takes progress in proportion, as Canary does.
- **Equal skills at the vocation choice.** The tries would sit above the new vocation's
  requirement, which is neither Canary nor Global.

## 5. Decision test

- **Must decide now:** YES. ATTACK-0 needs skill values, and CHAR-BUILD-1 is about to be
  allocated, so adding the columns now avoids a second migration.
- **Minimum sufficient:** seven column pairs on an existing table and receipt kind.
- **Superseding evidence:** an official skill formula, or an owner-trusted source that disagrees.
- **Deliberately not decided:**
  - offline training, exercise weapons, skill boosts from equipment and imbuements;
  - client visibility. `ActorVitalsV1` has no skill or magic-level fields, so players do not see
    their skills until a later wire decision adds them.

## 6. Before-freeze checklist

1. **Contract amendments:** A13 §4.1, §4.2, §4.4, §4.5, §4.6 and §5 (pointer in A13).
2. **Serialization:** the A13 build writer and its queue.
3. **Restart:** durable levels; at most one checkpoint of tries lost.
4. **Typed references:** the skill families are a closed set.
5. **Wire:** none. A later wire decision is needed before skills are visible.
6. **Split work:** one build receipt per checkpoint or advance.
