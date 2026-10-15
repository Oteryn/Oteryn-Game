# MAP-VIEWPORT-OPT-1 map-view budget under option B

Task `OTV2-20261015-map-viewport-opt-1`, decision `ARCH-MAP-VIEWPORT-BUDGET-V1` §1.5 (owner decision
**option B**, 2026-10-15). No code optimisation, no wire, registry, runtime or limit change.

## Status: provisional

The walking pace and the moving share below are **assumptions**, not measurements of players. Follow-up
**MAP-VIEWPORT-BUDGET-REVAL** re-validates them with bots that simulate players (load-sim) on a composed
Channel: the share of players walking, their step rate and the CPU of the map views. Until it does, this budget
holds only under these assumptions.

## Model

- Budget (§1.2, D128): the map views of a Channel take at most 20% of one core at 500 players.
- Option A (#1873): every player steps every 200 ms. It does not fit: 0.41-0.44 core.
- **Option B (§1.5):** only a player who stepped onto another tile or was moved in a tick rebuilds a view. A
  player who stands rebuilds nothing. Cores = 500 x moving share x steps per second x CPU per update.
- **Walking pace:** the SPEED-1 step duration (`apps/game-server/src/movement/speed.rs`,
  `content/movement/step_speed_v1.json`) of a level-100 player without haste or equipment speed (effective speed
  209) on a ground with no ground speed of its own (150): 350 ms, **2.86 steps per second**.

  | effective speed (level) | ground 100 | ground 150 |
  |---|---|---|
  | 110 (1) | 2.50 | 1.82 |
  | 160 (51) | 3.33 | 2.50 |
  | 209 (100) | 4.00 | **2.86** |
  | 310 (201) | 5.00 | 4.00 |
  | 410 (301) | 6.67 | 5.00 |

  Steps per second; step duration `floor(1000 x ground speed / step speed)` ms rounded up to 50 ms.
- **Moving share:** 50% of the players of a Channel walk at any moment.

## Result

On the MAP-VIEWPORT-MEASURE-1 evidence (#1873; 162.5-174.0 us CPU per update):

| CPU per update | option A (5 steps/s, all) | option B (50% at 2.86 steps/s) | break-even, all players walking | break-even share at 2.86 steps/s |
|---|---|---|---|---|
| 162.5 us (1 thread) | 0.406 core | 0.116 core | 2.46 steps/s | 86% |
| 174.0 us (4 threads) | 0.435 core | **0.124 core** | 2.30 steps/s | 80% |

**Under option B the map views fit 20% of one core** with 0.076 core to spare on the largest measured cost. The
§1.2 optimisation slice is not needed. A Channel breaks the budget only if more than 80% of its players walk at
the level-100 pace at once, or all of them hold 2.30 steps per second.

`ARCH-MAP-VIEWPORT-BUDGET-V1` §1.5 uses the #1873 numbers because they are larger (the conservative case).

## Re-run on this head

Command: `cargo test --release --locked -p oteryn-game-server --lib map_viewport_real -- --ignored --nocapture`
(`apps/game-server/src/gameplay_transport/world_map_real_tests.rs`, now with the option B line).

- Machine: Intel Xeon Processor, 4 vCPU (the reference node class), release profile, one run.
- Bundle: 26,078,850 bytes, digest `bb67df67d1605c42f9fbddcf18ab70abd86ee7087c17da9233d4b2c8c36bad06` (the content
  changed since #1873), 19,373,519 tiles, 5,818,415 enterable. Seeds and method as #1873; the staged update sent
  the same bytes as `SessionMapView::update` on 400 steps.

| threads | snapshot p50 / p99 us | delta p50 / p99 us | CPU per update | option A | option B |
|---|---|---|---|---|---|
| 1 | 173.8 / 359.7 | 106.3 / 215.5 | 108.1 us | 0.270 core | 0.077 core |
| 4 | 173.2 / 382.8 | 107.9 / 228.0 | 111.4 us | 0.278 core | 0.080 core |
| 8 | 175.0 / 4264.2 | 107.1 / 4168.0 | 111.6 us | 0.279 core | 0.080 core |

The update is about a third cheaper than in #1873. No code in this task changed it: it comes from changes merged to
`main` since then. Even so, option A still does not fit (0.27-0.28 core). The 8-thread p99 is
oversubscription (8 threads on 4 vCPU), as in #1873. Delta stage split at 4 threads (p50 us): plan 56.7, tiles
23.1, encode 16.2.

## Bench and test

- The measurement prints an option B line per concurrency level: the cores at 50% walking and 2.86 steps/s, and
  the break-even step rate with every player walking.
- `option_b_budget_fits_the_measured_cost` (a unit test that is not ignored, in the same module): the SPEED-1 walking
  pace is a 350 ms step, option A at 174.0 us exceeds 0.20 core, and option B at 174.0 us is 0.124 core.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib option_b_budget`: pass
- `cargo test --release --locked -p oteryn-game-server --lib map_viewport_real -- --ignored --nocapture`: ok (37 s)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
