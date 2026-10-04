# ATTACK-PARITY-1 packet: auto-attack parity scope

- Packet: `ATTACK-PARITY-1-PACKET-1`
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings and the two packets below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane order of 2026-10-04, item 3 (a short scope packet for ATTACK-PARITY-1).
- Builds on:
  - ATTACK-0 §5 and §6 (`OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md`);
  - ARCH-BATCH-CORE-LOOP-PACKETS §1.1 (ATTACK-1a and 1b; ATTACK-1b owns `combat/attack/**`);
  - ATTACK-1a (#1737): `content/combat/attack_constants_v1.json` and `combat/attack/{constants,formulas}.rs`;
  - owner rule 5905825574: an official source governs, and owner-trusted fan sources are allowed.
    TibiaPal is owner-trusted (#162 5905825574, 5905851791).
- Amends: nothing.
- Runtime, migration, protocol and production authority: NONE. Each packet needs its own #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Order and shared files

- **ATTACK-PARITY-1a** captures the fixtures. It owns only new paths, so it can start now, in
  parallel with ATTACK-1b.
- **ATTACK-PARITY-1b** compares the fixtures with the formulas and corrects them. It writes
  `combat/attack/**`, which ATTACK-1b owns, so it starts from `main` after ATTACK-1b merges.

Neither needs a wire change, a capability or a migration.

## 1. Rulings

### 1.1 What the calculator can check

The TibiaPal damage calculator (`tibiapal.com/damage-calculator`) is a page over the TibiaTools
API: `POST https://tibiatools.io/api/v1/damage`, with the catalogues under `/api/v1/meta/*`. The
request admits:
- vocation, level and skill;
- a weapon id (id 1 is fists), a shield id and an ammunition id;
- stances, perks and charms.

The `Auto-attack` row of the response gives `raw.min`, `raw.avg` and `raw.max`.

The API has no fight-mode field (it rejects `fightMode`). It reports no defence, armor, block or
interval. So ATTACK-PARITY-1 can check only the player attack range.

**Fight mode of every assertion (#1768 P1 4177982266).** The probes of §1.3 match Oteryn's
formula with `attack_factor` 1.0, which is `fight_modes.offensive.attack_factor` in
`content/combat/attack_constants_v1.json`. Oteryn's default mode is Balanced (0.75). Every parity
assertion is therefore bound explicitly:
- each fixture row records `oteryn_fight_mode: OFFENSIVE` and `oteryn_attack_factor: 1.0`;
- 1b evaluates every row with `FightMode::Offensive` and first asserts that the constants table
  still gives it `attack_factor` 1.0. A different value fails the test and does not get refitted;
- 1b never changes the base formula to fit a row under another mode. The Balanced and Defensive
  factors stay `PARITY_PENDING`.

**Monk is excluded (#1768 P1 4177982263).** The TibiaTools calculator guide
(`tibiatools.io/tools/calculator/guide`) says that the monk is not properly implemented and that
it is hardcoded to assume VoH. Monk rows are not captured and not asserted. Monk melee stays
`PARITY_PENDING` until the calculator supports it or another reliable source exists.

These values keep `PARITY_PENDING` and are listed as unchecked in the record:
- the Balanced and Defensive attack factors;
- monk melee;
- defence, armor and block;
- the creature melee formula;
- the attack interval and the in-fight deadline.

An official CipSoft value (the manual) still governs any of them where it exists.

### 1.2 The fixture grid

The fixtures cover ATTACK-0 §6:
- the vocations knight, paladin, sorcerer and druid (no monk, §1.1);
- levels 8, 50, 100, 300, 600 and 1000;
- skills 10, 50, 100 and 120;
- fists, plus one weapon for each melee class (axe, club and sword).

The weapons are named by their TibiaTools id and name, and mapped to Oteryn item keys in the
fixture.

Each fixture row records the request, the `Auto-attack` `raw` triple, the capture time, the API
description string and the Oteryn mode binding (`OFFENSIVE`, 1.0, §1.1). Stances, perks, charms, imbuements and the wheel are left out of every
request; bonus and crit are 0.

The fixtures are captured once by a tool and checked in. CI never calls the network.

### 1.3 Divergence known now

On 2026-10-04, five probe requests with fists (knight, sorcerer):

| Vocation, level, skill | TibiaPal min / avg / max | Oteryn ATTACK-1a `[min, max]`, Offensive (1.0) |
|---|---|---|
| knight 8, 10 | 3 / 5 / 9 | [0, about 7] |
| knight 100, 100 | 34 / 49 / 79 | [0, 79] |
| knight 300, 100 | 74 / 89 / 119 | [0, 119] |
| knight 1000, 120 | 200 / 218 / 253 | [0, 253] |
| sorcerer 100, 10 | 22 / 24 / 28 | [0, 26] |

The maximum matches at higher skills, including the official level curve at level 1000. It
differs at skill 10. The minimum is never 0 in TibiaPal.

ATTACK-0 §5 resolves divergences toward TibiaPal. So ATTACK-PARITY-1b:
- derives a minimum term;
- derives the low-skill maximum correction from the grid;
- expresses both as `player_expression` trees on the existing formula engine.

No new engine is built.

**The average is part of the gate (#1768 P1 4178014589).** The engine draws uniformly between
min and max, so its mean is `(min + max) / 2`. TibiaPal does not. For knight 100, 100 the uniform
mean is 56.5, while TibiaPal reports 49. So a row that matches min and max can still differ in
average damage, and matching min and max alone is not parity.

1b computes the engine's exact mean for every row and asserts it against `raw.avg`. It does not
change the draw distribution. The distribution is a separate decision, ATTACK-DIST-0: what
TibiaPal's distribution is, and whether the formula engine needs a non-uniform draw. The control
plane allocates it from 1b's residual report (§1.4).

### 1.4 Passing and failing

A row passes when Oteryn's min, max and mean each equal TibiaPal's `raw.min`, `raw.max` and
`raw.avg`, within the calculator's rounding (±1).

- **Every row passes:** 1b sets `parity` to `MATCHED_TIBIAPAL` for the attack values only. The
  other values stay `PARITY_PENDING` (§1.1).
- **Min and max pass, the mean does not:** this is the expected case under a uniform draw. The
  attack values stay `PARITY_PENDING`, and no row or value is labelled `MATCHED_TIBIAPAL`. 1b
  keeps its min and max corrections and asserts them. The test records each row's mean residual
  as an expected failure list in the fixture comparison, not as a pass. 1b reports the residuals
  on #162, and the control plane allocates ATTACK-DIST-0 (§1.3).
- **No closed form fits every row:** 1b does not fit a lookup table. It matches what it can and
  reports the failing rows on #162. It sends the control plane a QUESTION with:
  - (a) keep the closest closed form and record the residual;
  - (b) a level-band table;
  - (c) keep Canary.

## 2. Packets

### 2.1 ATTACK-PARITY-1a

```yaml
task_id: OTV2-20261004-attack-parity-1a-fixtures
decision: ATTACK-0 §6; this packet §1.1, §1.2
worker: oteryn-impl-worker
review: combat review (Codex, final frozen head)
branch: claude/attack-parity-1a-20261004
base: main
migration_lease: none
depends_on: [ATTACK-1a]
owned_paths:
  - tools/combat-parity/capture_tibiapal_auto_attack.py       # new
  - tools/combat-parity/test_capture_tibiapal_auto_attack.py  # new, offline
  - tools/combat-parity/README.md                              # new
  - content/combat/parity/tibiapal_auto_attack_v1.json         # new: the fixture grid
  - docs/agents/evidence/OTV2-20261004-attack-parity-1a-capture.md  # new
  - docs/agents/tasks/archive/OTV2-20261004-attack-parity-1a-fixtures.md
validation:
  - python3 -m unittest tools/combat-parity/test_capture_tibiapal_auto_attack.py
```

Acceptance:
- The fixture holds the full §1.2 grid. The record states the row count.
- Every row has its request, the `raw` triple, the capture time and the mode binding. No row has
  vocation monk (test).
- The tool reproduces the request bodies offline from the grid definition (test). A live rerun is
  documented, not run in CI.
- The weapon mapping names one Oteryn item key per TibiaTools weapon.
- Not in scope: any Rust change or comparison.

### 2.2 ATTACK-PARITY-1b

```yaml
task_id: OTV2-20261004-attack-parity-1b-formulas
decision: ATTACK-0 §5, §6; this packet §1.3, §1.4
worker: oteryn-impl-worker
review: combat review (Codex, final frozen head)
branch: claude/attack-parity-1b-20261004
base: main after ATTACK-1b and ATTACK-PARITY-1a merge
migration_lease: none
depends_on: [ATTACK-1b, ATTACK-PARITY-1a]
owned_paths:
  - apps/game-server/src/combat/attack/formulas.rs
  - apps/game-server/src/combat/attack/constants.rs
  - apps/game-server/src/combat/attack/parity_tests.rs   # new
  - apps/game-server/src/combat/attack/mod.rs            # the test module line only
  - content/combat/attack_constants_v1.json
  - docs/agents/tasks/archive/OTV2-20261004-attack-parity-1b-formulas.md
validation:
  - cargo test --locked -p oteryn-game-server --quiet combat::attack
  - cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings
```

Acceptance:
- `parity_tests.rs` loads the fixture and asserts min and max for every row, within ±1, under
  `FightMode::Offensive`. It first asserts that the Offensive `attack_factor` is 1.0 and that no
  row is monk (§1.1).
- It also computes the engine's exact mean for every row and compares it with `raw.avg` (±1).
  Rows whose mean differs are listed by key in a checked-in residual list. The test fails if a
  row passes but is still listed, or fails but is not listed, so the list cannot go stale.
- The constants file states its parity per value. It says `MATCHED_TIBIAPAL` for the Offensive
  attack values of the four vocations only when every row passes min, max and mean (§1.4). Every
  other value is `PARITY_PENDING`, including the Balanced and Defensive factors and monk melee.
- The existing ATTACK-1a and 1b tests still pass. Any changed expected value is listed in the
  record with its fixture row.
- If the rows do not all pass, follow §1.4.
- Not in scope:
  - the Balanced and Defensive factors, monk melee, defence, armor, block and creature melee
    (§1.1);
  - distance weapons (RANGED-PARITY-1);
  - changing the draw distribution (ATTACK-DIST-0, §1.3);
  - critical hits and charms.

## 3. Rejected options

- **Scraping the calculator page.** The page calls a JSON API, and the API is the stable surface.
- **Live API calls in CI.** These are non-deterministic and depend on the network. The fixtures are
  captured once.
- **Asserting min and max only.** A uniform 34-79 draw averages 56.5 against TibiaPal's 49. That
  is a 15 % higher damage output under a `MATCHED_TIBIAPAL` label (#1768 P1 4178014589).
- **Changing the distribution in 1b.** It may need a new draw shape in the engine. That needs its
  own decision (ATTACK-DIST-0).
- **A lookup table instead of formulas.** It hides the formula and grows with every level band. It
  is only option (b) of §1.4.
- **Waiting for ATTACK-1b to capture.** The capture owns no shared path.

## 4. Decision test

- **Must decide now:** YES. Every attack value is `PARITY_PENDING`, and the probes in §1.3 show
  that the minimum damage is wrong today.
- **Smallest sufficient:** one capture tool, one fixture file, one test module, and formula
  corrections only where the fixtures disagree.
- **Superseding evidence:** an official formula, or a TibiaTools revision. The latter means a
  recapture by rerunning the tool.
