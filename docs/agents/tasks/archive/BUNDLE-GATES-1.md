# BUNDLE-GATES-1

```yaml
task_id: BUNDLE-GATES-1
title: "BUNDLE-GATES-1: audited, test-enforced inventory of the qualified_room gates on a bundle World"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bundle-gates-1-20261014
issue: 1622
governing_issue: 1622
pr: 2115
terminal_pr_policy: archive_pending
base_sha: 75ec7985
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-14
updated_at: 2026-10-14
depends_on: ["BUNDLE-COMBAT-ENABLE-1 (#2025)"]
owned_paths:
  - apps/game-server/src/gameplay_transport/bundle_gates_tests.rs
  - apps/game-server/src/gameplay_transport/mod.rs   # test module declaration only
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md   # BUNDLE-GATES-1 row only
  - docs/agents/tasks/active/BUNDLE-GATES-1.md
  - docs/agents/tasks/archive/BUNDLE-GATES-1.md
public_contracts: []
external_repositories: []
```

## Scope

Plan row BUNDLE-GATES-1 (residue of #2025): inventory every `qualified_room` gate of the gameplay seam on main, record which are open and by which merged PR, name the batch that opens each closed one, and add a guard test. No runtime, wire, persistence, migration or registry change.

## Result

Inventory of the gameplay seam's room reads, on main@75ec7985. A `combat_room` read is open on a bundle World; a `qualified_room` read is closed there (`serve.rs` withholds `qualified_room` from a bundle World).

| Gate | State | Opened by / owner |
|---|---|---|
| attack target (`mod.rs`), auto-attack drain (`attack.rs`), native combat cast and commit (`native_combat_cast.rs`), timer continuation (`spell_timer_callbacks.rs`) | open | BUNDLE-COMBAT-ENABLE-1, #2025 |
| monster AI (`monster_ai_cycle.rs`) | open | LOOP-MONSTER-1 child BUNDLE-MONSTER-AI-1, #2036 |
| periodic player damage (`spell_periodic.rs`) | open | MAGIC-COND-1 child COND-1c, #2086 |
| `locate_message` parameter cast (`parameter_cast.rs`) | open | MAGIC-CAST-1, #2112 (the bundle World locate gate) |
| `house_access`, `vertical_move` parameter casts | closed | MAGIC-SUMMON-1 (SPELL-RT-WORLD-1) |
| world item casts (`world_item_cast.rs`): `random_item_grant`; `conjure` | closed | MAGIC-SUMMON-1 (SPELL-RT-WORLD-1); MAGIC-ITEMS-1 (RUNE-CONJ-1) |
| field step ingress (`field_step_ingress.rs`) | closed | LOOP-WORLD-1 (WORLDINT-ADMIT-1) |
| source map initialization (`source_item_cycle.rs`) | closed | MAP-CUTOVER-1c (the bundle Ground persistence) |
| door overlay, step blocking, USE (`mod.rs`) | not a gate | a source-World branch; a bundle World is served by `bundle_map` |

`bundle_gates_tests.rs` enforces the table. It fails in each of these cases:
- a read is added, removed or switched;
- an open gate lacks its batch and merged PR;
- a closed gate has no owner, or an owner that is not a plan batch;
- the plan row stops naming an opener or owner;
- `serve.rs` stops withholding `qualified_room`;
- `parameter_room` opens another key.

The plan row BUNDLE-GATES-1 carries the same inventory and stays open until no gate is closed.

## Notes

- `conjure` goes to MAGIC-ITEMS-1, not MAGIC-CAST-1: that row owns "rune use with target and conjure".
- Source map initialization goes to MAP-CUTOVER-1c, because on a bundle World it is that row's Ground persistence.
- The active record `BUNDLE-MONSTER-AI-1.md` is stale: its PR #2036 merged. It is outside this task's paths.

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib: 3036 passed, 0 failed, 29 ignored
- negative check (world_item_cast.rs switched to combat_room): `every_seam_room_read_is_inventoried` fails as intended
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: OK
- review: Codex auto-review on the PR head
