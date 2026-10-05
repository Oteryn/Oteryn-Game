# OTV2-20261005-spells-import-carry-1

```yaml
task_id: OTV2-20261005-spells-import-carry-1
title: Carry PR #1817 (Canary/Crystal spell import and runtime mechanics) to merge
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/spells-runtime-main-refresh-20261004
issue: 1622
pr: 1817
decision: D736 (2a)
owned_paths:
  - the existing changed paths of PR #1817
  - files required by the origin/main merge resolution
  - docs/agents/tasks/archive/OTV2-20261005-spells-import-carry-1.md
```

Merge `origin/main` into the PR head (merge commit), keep main's merged behaviour and the PR's intent, regenerate generated content with `tools/merge-driver/regen.sh`.

## Merge resolution

- `apps/game-server/src/gameplay_transport/monster_ai_cycle.rs`: kept main's `QualifiedMelee`/`qualified_melee` and the PR's `current_player_target` (dead players do not occupy the nearest-target slot).

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-game-server --lib`: 2221 passed
- `cargo test -p oteryn-game-server --test current_spell_sources`: pass
- `cargo test -p oteryn-game-server --test content_world_project_repository`: pass
- spell-authoring and spell-import unit tests (5 files, 100 tests): pass
- `tools/merge-driver/regen.sh` content checks: pass, except the two main-side `DANGLING_KEY` errors (`i65536`, `i70000`) fixed by #1821
- `python3 tools/agents/validate_governance.py`: pass
