# OTV2-20261008-npc-place-1a

```yaml
task_id: OTV2-20261008-npc-place-1a
title: "NPC-PLACE-1a: Npc.Placement World Project family, first starter-area set"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-place-1a-20261008
issue: 1622
pr: 1940
head_sha: "exact frozen head in the PR FREEZE comment"
final_head_sha: "exact frozen head in the PR FREEZE comment"
owner: single writer session (control plane session_01CwP6d84eCPvpgoEuyci8Tx)
created_at: 2026-10-08
updated_at: 2026-10-08
execution_policy: continuous_progress
owned_paths:
  - content/world/npc-placements/
  - content/world/pins/oteryn.json
  - tools/content-schema/npc-authoring/convert_placements.py
  - tools/content-schema/npc-authoring/npc_placement_scope.json
  - tools/content-schema/npc-authoring/test_convert_placements.py
  - docs/agents/tasks/archive/OTV2-20261008-npc-place-1a.md
public_contracts: []
depends_on: [NPC-PLACE-1]
blocks: [NPC-PLACE-1b]
```

## Outcome

- Implements NPC-PLACE-1 §4 (decision `OTERYN_GAME_NPC_PLACE1_NPC_PLACEMENTS_DECISION_2026-10-06.md`):
  family `Npc.Placement` in `content/world/npc-placements/` (index, `held.json`, one shard), frame
  `global-target-2026-09-27`, written by `convert_placements.py` from the committed promotion
  candidates for the 25 Rookgaard NPCs of `npc_placement_scope.json`.
- 25 records, 0 held (no scoped cell is shared; `position_conflicts` is empty). Zirella is the
  wiki-origin placement, written north with the note in its provenance.
- Pin: only `inputs_digest` moved; the bundle digest is unchanged (the compiler does not read the
  family before 1b). `pin-check` passes.
- Known gap: promotion candidate placements carry no spawn-file path or blob, so provenance rows
  hold origin, repository, revision, source key, definition sha256 and the arbitration rows, not
  path and blob (§4.1). Closing it needs the Canary and Crystal checkouts; later packets that widen
  the scope should supply them.
- Not touched: crates/world-bundle, COIN-PROFILE-1, SPELL-AVAIL-1. `regenerate_content.py` rewrites
  unrelated item and world-project files on this checkout; those changes were not taken.

## Validation

- `python3 tools/content-schema/npc-authoring/convert_placements.py --check`: pass
- `python3 -m unittest tools/content-schema/npc-authoring/test_convert_placements.py`: pass (6)
- `python3 tools/content-schema/validate_materialized_game_tree.py`: pass
- `cargo run --locked -p oteryn-world-bundle-compiler -- pin-check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
