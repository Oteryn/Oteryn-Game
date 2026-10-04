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
interval. So ATTACK-PARITY-1 can check only the player attack range, in the calculator's implied
fight mode.

These values keep `PARITY_PENDING` and are listed as unchecked in the record:
- the fight-mode factors;
- defence, armor and block;
- the creature melee formula;
- the attack interval and the in-fight deadline.

An official CipSoft value (the manual) still governs any of them where it exists.

### 1.2 The fixture grid

The fixtures cover ATTACK-0 §6:
- every vocation (knight, paladin, sorcerer, druid, monk);
- levels 8, 50, 100, 300, 600 and 1000;
- skills 10, 50, 100 and 120;
- fists, plus one weapon for each melee class (axe, club and sword).

The weapons are named by their TibiaTools id and name, and mapped to Oteryn item keys in the
fixture.

Each fixture row records the request, the `Auto-attack` `raw` triple, the capture time and the
API description string. Stances, perks, charms, imbuements and the wheel are left out of every
request; bonus and crit are 0.

The fixtures are captured once by a tool and checked in. CI never calls the network.

### 1.3 Divergence known now

On 2026-10-04, five probe requests with fists (knight, sorcerer):

| Vocation, level, skill | TibiaPal min / avg / max | Oteryn ATTACK-1a `[min, max]` |
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

The distribution between min and max is out of scope: the engine draws uniformly. The `avg`
column is recorded but not asserted.

### 1.4 Passing and failing

A row passes when Oteryn's min and max each equal TibiaPal's, within the calculator's rounding
(±1).

- **Every row passes:** 1b sets `parity` to `MATCHED_TIBIAPAL` for the attack values only. The
  other values stay `PARITY_PENDING` (§1.1).
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
- Every row has its request, the `raw` triple and the capture time.
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
- `parity_tests.rs` loads the fixture and asserts min and max for every row, within ±1.
- The constants file states its parity per value: `MATCHED_TIBIAPAL` for the attack values, and
  `PARITY_PENDING` for the rest.
- The existing ATTACK-1a and 1b tests still pass. Any changed expected value is listed in the
  record with its fixture row.
- If the rows do not all pass, follow §1.4.
- Not in scope:
  - fight-mode factors, defence, armor, block and creature melee (§1.1);
  - distance weapons (RANGED-PARITY-1);
  - critical hits and charms.

## 3. Rejected options

- **Scraping the calculator page.** The page calls a JSON API, and the API is the stable surface.
- **Live API calls in CI.** These are non-deterministic and depend on the network. The fixtures are
  captured once.
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
