# OTV2-20261003-timed-rt-1a

```yaml
task_id: OTV2-20261003-timed-rt-1a
title: "TIMED-RT-1a: timed-item rows, write records, checkpoint and the per-item lane"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/timed-rt-1a
pr: 1681
base_sha: 98a2f95c
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-015sWxwCsfiHgQNMeQvbYbUc (hard worker)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
migration_lease: "0054 (D353)"
owned_paths:
  - apps/game-server/migrations/0054_item_timed_states.sql
  - apps/game-server/src/durability/item_timed_state.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/domain/timed_item.rs
  - apps/game-server/src/domain/mod.rs
  - apps/game-server/tests/item_timed_state_postgres.rs
  - apps/game-server/tests/support/item_timed_state_postgres_cases.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md
  - docs/architecture/reviews/OTERYN_GAME_TIMED_ITEM0_CHARGES_DURATION_AND_REPAIR_DECISION_2026-10-01.md
  - docs/agents/tasks/archive/OTV2-20261003-timed-rt-1a.md
public_contracts: []
depends_on: ["#1662 merged", "#1667 (binding text)"]
blocks: [TIMED-RT-1b, TIMED-REPAIR-1, EXERCISE-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Allocation: #1622 CP answer a (split). TIMED-RT-1's dependencies (TIMED-CONTENT-1, ITEM-MOVE-2a,
ITEM-MOVE-2b, EQUIP-RT-1) are not on `main`, so TIMED-RT-1a builds the parts that need none of
them (TIMED-ITEM-0B §4, §5.2-§5.3, §6, §12, §14):

- Migration 0054: `game_item_timed_states` with `deadline_at` and its partial index; the immutable
  `game_item_timed_state_writes` keyed by (item, expected revision); `game_item_timed_definitions`
  pinning a definition revision's full values at the first write; the guard (revision 1 then +1,
  never deleted or truncated, row and record proven by each other in the same physical
  transaction, values within the pinned definition, a deadline only on a lit item on a tile, a lit
  item in no container); grants as §4.
- `durability::item_timed_state`: the closed `TimedItemCause`; `commit_timed_checkpoint`, the plain
  checkpoint under the holder's current gameplay fence (replay returns the record, a key committed
  by another write writes nothing, a stale fence writes nothing); the row read and the
  ambiguous-write record lookup.
- `domain::timed_item`: live values, the lane (one write in flight, expiry keyed to the last
  committed revision, retry, ambiguous lookup and hold, unexpected revision), stop, and the actor's
  live-item bound with the "all lanes empty" check for logout, handoff and death (a stopping lane
  or a pending expiry is never empty).
- Codex round 1 (#1681): P1 4174557660 (fence evidence at the checkpoint boundary) and P1
  4174557664 (a stopping lane counted as empty) fixed.
- Registry rows `TIMEDITEM0B-RL-01`, `-02`, `-04`, `-05` with max and max+1 tests.

Left to TIMED-RT-1b (after its dependencies): the Expire, SetDeadline, ClearDeadline and PutOut
records with their TRANSFORM, BURN, move and audit lines (0054's guard refuses those causes
today), the hosting runtime's fenced transaction and lane wiring, the Ground deadline scheduler and
`TIMEDITEM0B-RL-06`, the DUR-03 shape ceilings, and the ITEM-MOVE-WIRE-1, ITEM-USE-0, MARKET-0 and
WORLD-INTERACTION-0 amendments. `TIMEDITEM0B-RL-03` is TIMED-WIRE-1's.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet`: pass
- `cargo test --locked -p oteryn-game-server --test item_timed_state_postgres` and
  `--test check_function_privileges_postgres` against a local PostgreSQL 17.11 (the 17.6 version
  assertion relaxed locally only; CI runs 17.6): pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass
