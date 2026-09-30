# OTV2-20260930-prof-2-progression-ruleset

```yaml
task_id: OTV2-20260930-prof-2-progression-ruleset
title: PROF-2 ruleset slice - Weapon Proficiency thresholds, Mastery and kill points
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/inspiring-lamport-nc623x
issue: 162
pr: null
base_sha: de1a622
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "owner-launched Claude Code session (session_012FuykcnY5errmsP3T1E4Nm)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - rulesets/progression/weapon-proficiency/**
  - apps/game-server/src/domain/weapon_proficiency.rs
  - apps/game-server/src/domain/mod.rs
  - docs/agents/tasks/active/OTV2-20260930-prof-2-progression-ruleset.md
  - docs/agents/tasks/archive/OTV2-20260930-prof-2-progression-ruleset.md
public_contracts: []
depends_on:
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
  - "owner decision #162 5905899852 (thresholds) and 2c 2026-09-30 (points, TibiaWiki revid 1192598)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Scope

The rules-data slice of PROF-2 that does not need PROF-1 (Character state). Owner answer `1c`
(2026-09-30): this session does this slice; PROF-1, the PROF-2 accrual/selection/effects,
PROF-WIRE-1 and runtime loading of `content/proficiencies/` go to the control plane on #162.

- `rulesets/progression/weapon-proficiency/progression.json`
  (`OTERYN_GAME_WEAPON_PROFICIENCY_PROGRESSION/v1`): the three progress threshold tables, perk levels
  and Mastery (`n + 2`), the kill-credit rule, and the published point table (6 difficulties x
  stacks 0-5 + fiendish, bosses bane/archfoe/nemesis), copied from TibiaWiki revid 1192598 as
  published, not computed.
- `apps/game-server/src/domain/weapon_proficiency.rs`: pure, persistence-neutral rules (level and
  unlocked perk levels from progress, Mastery, saturation, creature and boss points), with a test that
  the ruleset file matches the implemented constants.

## Excluded

Accrual at kill credit, perk selection and clearing, perk effects, checkpoints, persistence, wire,
runtime content loading. The catalogue copy of the threshold tables in
`tools/content-schema/proficiency-authoring/` stays until PROF-CONTENT-1 (#1341/#1342) has merged.

## Validation

- `cargo test -p oteryn-game-server --lib domain::weapon_proficiency` (6 tests)
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`, `cargo fmt --check`
- `tools/content-schema/validate_materialized_game_tree.py` (97/97)
- `tools/agents/validate_governance.py`
