# OTERYN-20261011-creature-move-1b

```yaml
task_id: OTERYN-20261011-creature-move-1b
title: "CREATURE-MOVE-1b: creature steps admitted on the accepted entry room without a tile document"
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/creature-move-1b-20261011
issue: 1622
pr: 1987
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session_01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/monster_ai_cycle.rs
  - docs/agents/tasks/active/OTERYN-20261011-creature-move-1b.md
  - docs/agents/tasks/archive/OTERYN-20261011-creature-move-1b.md
public_contracts: []
depends_on: [CREATURE-MOVE-1a]
blocks: []
```

## Outcome

Follow-up to CREATURE-MOVE-1a (#1977, merged as `ca6bec67e` while its Codex P1 4242014304 was still unrepaired). `native_entry_room.json`, the only room with a spawn source, has no `spell_tiles` document. As a result, `QualifiedSpellTiles` holds `tiles: None`, every lookup returns `Unknown`, and the creature step predicate refused every destination. No creature step could be committed there.

## Architecture and source of truth

- The Movement owner already admits a player step in this room from terrain and doors alone (`field_step_ingress`, accepted entry movement: not the source profile, and the lookup is `Unknown`).
- A creature step now follows the same rule. On the accepted entry movement (entry-room collision index, not the source profile), `Unknown` means "no tile document", so the cell has no protection-zone or floor-change tile.
- These cases still refuse, keeping fail-closed behaviour: an `Unknown` cell of a tile-qualified map, a cell absent from a tile document, and another scope (bundle `ScopeMismatch`).

## Acceptance criteria

- A step onto ordinary Walkable cells of the entry room is admitted.
- Blocked cells, door cells (open or closed) and cells outside the room are refused.
- A spawned rat with the room's own behaviour commits a chase step through `run_due_thinks`.
- Its neighbour's chase step onto the door cell is refused.

## Excluded scope

- Tile facts for `native_entry_room.json` (content).
- Combat targeting without protection-zone facts.
- Creatures passing doors (CREATURE-DOOR-1).
- The durable-item blocking overlay (CREATURE-MOVE-1c).

## Implementation / findings

- `creature_cell_admissible` and the new `accepted_entry_movement` helper are in `monster_ai_cycle.rs`.
- Tests:
  - `the_accepted_entry_room_admits_creature_steps_onto_its_ordinary_cells` uses the real room cells and the real door runtime. It also shows that opening the door leaves `collision_cells` unchanged.
  - `a_spawned_rat_commits_a_chase_step_in_the_accepted_entry_room`.
- LIMIT: in production the room still perceives no player. `melee_targets` drops a player whose tile lookup fails, and attacks refuse without protection-zone facts. The rat profile has no wander. Rats will not move live there until the room gets tile facts or a combat decision is made.

## Validation

On the owner PC, against the 1a head with this change:

- RED before the predicate change: `cargo test --locked -p oteryn-game-server --lib monster_ai_cycle` gave 6 passed, 2 failed (both new tests).
- GREEN after the change:
  - `monster_ai_cycle`: 8 passed, 0 failed.
  - `gameplay_transport`: 437 passed, 0 failed.
  - `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
  - `cargo fmt --all -- --check`: pass.
- Re-run on this branch (base `ca6bec67e`), all on the owner PC:
  - `cargo fmt --all -- --check`: pass.
  - `cargo test --locked -p oteryn-game-server --lib monster_ai_cycle`: 8 passed, 0 failed.
  - `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `python tools/agents/validate_governance.py`: pass (full checkout on the owner PC; the sparse cloud worktree lacks the handover evidence files)
- `python -m unittest discover -s tools/agents/tests`: OK (60 tests)

## Self-review

- exact head: the frozen head
- method/reviewer: task writer
- material findings: none open (Codex P1 4242296150 on #1987, task record not archived: #1987 merged from the merge queue before this move could be pushed, so the move lands in this docs-only follow-up)
- verdict: ready for independent review

## PR and closeout

- PR: https://github.com/Oteryn/Oteryn-Game/pull/1987
- changed-file review: owned paths only
- unresolved review threads: Codex 4242296150 answered; repaired by this archive follow-up
- merge commit/result: squash merge of #1987 (`21308b7f1`); record archived in a docs-only follow-up PR
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: #1987 merged; task record archived in a docs-only follow-up
status: completed
branch: claude/creature-move-1b-20261011
pr: 1987
blocker: null
next_action: none (control plane integrates the archive follow-up)
```
