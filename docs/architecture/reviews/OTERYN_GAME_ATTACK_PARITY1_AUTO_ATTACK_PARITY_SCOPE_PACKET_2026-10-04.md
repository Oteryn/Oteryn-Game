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
- fists, plus weapons of each melee class (axe, club and sword) whose attack values cover every
  residue of the attack value mod 5, one weapon per residue and class where the TibiaTools
  catalogue has one. This exposes the `floor(6 * attack / 5)` step of §1.3 (#1768 P1 4178079948).
  Missing residues are listed in the record.

The weapons are named by their TibiaTools id and name, and mapped to Oteryn item keys in the
fixture.

**Weapon attack values are checked before any fitting (#1768 P1 4178032396).** The calculator
uses the weapon's attack stat, and `player_melee_formula` reads Oteryn's `attack_value`. A name
mapping alone does not prove that both are the same number, and a mismatch would be absorbed into
the shared melee formula. So:
- each weapon row records `tibiatools_attack`, the attack stat that the `/api/v1/meta/*` weapon
  catalogue returns at capture;
- 1b resolves the mapped item's `definition.semantics.weapon.value.attack` in
  `content/items/definitions/`. It must be `state: KNOWN`;
- 1b asserts that the two are equal for every weapon row before it evaluates any formula on that
  row, and before any parity label.

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

ATTACK-0 §5 resolves divergences toward TibiaPal.

**The upstream expression is pinned, not fitted (#1768 P1 4178079948).** TibiaTools publishes
its calculator source. At commit `a1d368906caa8ae98bcb7123f2431733d318d710` of
`github.com/kik-tibia/tibiatools`, the auto-attack branch of `computeRaw`
(`src/lib/damage-calc/damage.ts` lines 13-25) and the level term
(`src/lib/damage-calc/character-state.ts` lines 73-75) are:

```text
step   = floor((sqrt(2 * level + 2025) + 5) / 10)
flat   = step * 100 - 450 + floor((level + 1000) / step - 50 * step)
attack = weapon attack + ammunition attack            (fists: 7)
v      = floor(6 * attack / 5) * (skill + 4) / 28     (no monk factor, §1.1)
min    = floor(flat + v / 2)
avg    = floor(flat + v)
max    = floor(flat + 2 * v)
```

This reproduces all five probes above exactly. So ATTACK-PARITY-1b:
- implements this expression as `player_expression` trees on the existing formula engine,
  including the `floor(6 * attack / 5)` step before the skill scaling. It does not fit a new
  expression to the grid;
- records the commit, file and lines in the constants file and the record;
- checks it offline, without the network, against a test oracle that transcribes the same
  lines. The check covers every attack value from 0 to the highest melee attack in
  `content/items/definitions/`, skills 10 to 130 and the §1.2 levels, so weapons that were not
  sampled are covered too;
- checks it against the captured fixtures, which are the evidence that the pinned source is what
  the API serves.

If the fixtures disagree with the pinned source, 1b changes no formula. It reports the rows on
#162 and asks the control plane for a recapture or a newer pinned commit. No new engine is built.

**The average is part of the gate (#1768 P1 4178014589).** The engine draws uniformly between
min and max, so its mean is `(min + max) / 2`, that is `flat + 1.25 v`. TibiaPal's `avg` is
`flat + v`. For knight 100, 100 the uniform
mean is 56.5, while TibiaPal reports 49. So a row that matches min and max can still differ in
average damage, and matching min and max alone is not parity.

1b computes the engine's exact mean for every row and asserts it against `raw.avg`. It does not
change the draw distribution. The distribution is a separate decision, ATTACK-DIST-0: what
TibiaPal's distribution is, and whether the formula engine needs a non-uniform draw. The control
plane allocates it from 1b's residual report (§1.4).

### 1.4 Passing and failing

A row passes when Oteryn's min, max and mean each equal TibiaPal's `raw.min`, `raw.max` and
`raw.avg`. Min and max must be exactly equal, since both sides use the same pinned integer
expression. The mean is compared after the same `floor`.

- **Every row passes:** 1b sets `parity` to `MATCHED_TIBIAPAL` for the attack values only. The
  other values stay `PARITY_PENDING` (§1.1).
- **Min and max pass, the mean does not:** this is the expected case under a uniform draw. The
  attack values stay `PARITY_PENDING`, and no row or value is labelled `MATCHED_TIBIAPAL`. 1b
  keeps its min and max corrections and asserts them. The test records each row's mean residual
  as an expected failure list in the fixture comparison, not as a pass. 1b reports the residuals
  on #162, and the control plane allocates ATTACK-DIST-0 (§1.3).
- **A weapon row whose attack value is unknown or different:** the row is excluded from fitting
  and from every pass count. It is listed by key with both values in a checked-in attack
  mismatch list, which the test keeps exact in both directions. The melee values of that weapon
  class stay `PARITY_PENDING`, and the row is never used to change a formula. 1b reports the list
  on #162, and the content lane corrects the item data through the control plane.
- **The fixtures disagree with the pinned source:** 1b does not fit a formula or a lookup table.
  It reports the failing rows on #162. It sends the control plane a QUESTION with:
  - (a) recapture the fixtures;
  - (b) pin a newer TibiaTools commit;
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
- For each melee class, the fixture covers every attack value residue mod 5 that the catalogue
  has (§1.2). The record lists any missing residues.
- Every weapon row records `tibiatools_attack` from the capture-time weapon catalogue. A weapon
  row without it fails the test. Fist rows have none.
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
- Before any formula check, `parity_tests.rs` resolves each weapon row's Oteryn item and asserts
  that its `weapon.attack` is `KNOWN` and equal to `tibiatools_attack`. Rows that fail go to the
  attack mismatch list (§1.4). The test fails if a listed row now matches or an unlisted row
  differs. No listed row takes part in a fit, a pass count or a parity label.
- The formulas implement the pinned expression of §1.3, `floor(6 * attack / 5)` step included.
  `parity_tests.rs` compares the engine with the transcribed oracle over the whole offline range
  of §1.3: every attack value, skills 10 to 130 and the §1.2 levels. Min and max must be exactly
  equal.
- `parity_tests.rs` loads the fixture and asserts that min and max are exactly equal for every
  row under `FightMode::Offensive`. It first asserts that the Offensive `attack_factor` is 1.0 and that no
  row is monk (§1.1).
- It also computes the engine's exact mean for every row and compares it with `raw.avg` after the
  same `floor`.
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
- **Trusting the name mapping for weapon attack.** A different or unknown Oteryn attack value
  would be fitted into the shared melee formula and corrupt every weapon (#1768 P1 4178032396).
- **Changing the distribution in 1b.** It may need a new draw shape in the engine. That needs its
  own decision (ATTACK-DIST-0).
- **Fitting a formula to the sampled weapons.** Three weapons cannot expose the
  `floor(6 * attack / 5)` step, so a fitted formula could pass the grid and still be wrong for
  weapons that were not sampled. The upstream expression is pinned and checked over every attack
  value instead (#1768 P1 4178079948).
- **A lookup table instead of formulas.** It hides the formula and grows with every level band. It
  is only option (b) of §1.4.
- **Waiting for ATTACK-1b to capture.** The capture owns no shared path.

## 4. Decision test

- **Must decide now:** YES. Every attack value is `PARITY_PENDING`, and the probes in §1.3 show
  that the minimum damage is wrong today.
- **Smallest sufficient:** one capture tool, one fixture file, one test module, and the pinned
  upstream expression in place of the current formulas.
- **Superseding evidence:** an official formula, or a TibiaTools revision. A revision means a new
  pinned commit and a recapture by rerunning the tool.
