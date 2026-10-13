# MAGIC-CAST-1

```yaml
task_id: MAGIC-CAST-1
title: "MAGIC-CAST-1: widen the castable starter book with free self-heals"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 2048
base_branch: main
branch: claude/magic-cast-1-20261013
pr: 2072
base_sha: 4ac13390
owner: claude-code-session-01QigX6fgNLhCNtPzkTKXaHD
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/spell/cast_tests.rs
  - docs/agents/tasks/archive/MAGIC-CAST-1.md
public_contracts: []
depends_on: []
```

## Outcome

`V1_BUNDLES` gains Magic Patch `exura infir` (SPELL-STARTER-1 bundle, unchanged), a plain self-target
instant on the `healing` cooldown group, so the existing heal path casts it. It takes index 4;
indices 1-3 are unchanged. No adapter, protocol, registry, migration or client change.

## Follow-ups (separate PRs after merge, per control plane)

- Ultimate Healing `exura vita` (index 5) and Wound Cleansing `exura ico` (index 6): the stock client
  bar has `SPELL_SLOTS = 4` (`apps/client/src/spell.rs`, client-owned #1942), so they are unreachable
  until the bar grows. Codex P1 on 7b562348.
- Bruise Bane `exura infir ico`: sorts first and moves indices 1-3; add with the index-pinned tests.
- Parametric, support and summon spells and the `executable_catalog` adapters (rest of rows 49-54).
- Validation filters go after `--`: `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- spell:: gameplay_transport::parameter_cast gameplay_transport::actor_spell`.

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- lib filters above: 441 passed; `--test ability_engine`: 74 passed
- New test: `magic_patch_casts_and_shares_the_healing_group`.
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 60 tests, OK
