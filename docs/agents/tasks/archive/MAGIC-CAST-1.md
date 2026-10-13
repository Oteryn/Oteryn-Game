# MAGIC-CAST-1

```yaml
task_id: MAGIC-CAST-1
title: "MAGIC-CAST-1: widen the castable starter book with free self-heals"
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 2048
base_branch: main
branch: claude/magic-cast-1-20261013
pr: null
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

`V1_BUNDLES` gains Magic Patch `exura infir`, Ultimate Healing `exura vita` and Wound Cleansing
`exura ico` (bundles from SPELL-STARTER-1, unchanged). All three are plain self-target instants on the
`healing` cooldown group, so the existing heal path casts them; no adapter, protocol, registry or
migration change. Indices 1-3 are unchanged; the new spells take 4-6 (canonical key order).

## Decisions and open points

- Bruise Bane `exura infir ico` is NOT added: it sorts first and would move indices 1-3 (SPELL-STARTER-1
  note). Add it with the index-pinned tests or with SPELL-BOOK-ACTIVATE-1. Knights have no level-1 heal until then.
- Only the self-heal slice of rows 49-54 is delivered. Parametric, support and summon spells and the
  `executable_catalog` adapters are not touched; their runtime gaps remain.
- The control-plane validation command lists several filters before `--`; cargo accepts one. Used
  `cargo +1.95.0 test --locked -p oteryn-game-server --lib -- spell:: gameplay_transport::parameter_cast gameplay_transport::actor_spell`.

## Validation

- `cargo +1.95.0 fmt --all --check`: pass
- lib filters above: 441 passed; `--test ability_engine`: 74 passed
- New test: `the_added_self_heals_cast_for_their_vocation_and_share_the_healing_group`.
