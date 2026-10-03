# OTV2-20261001-prof-effect0

```yaml
task_id: OTV2-20261001-prof-effect0
title: "PROF-EFFECT-0 Weapon Proficiency kill credit, points and perk effects"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-prof-effect-0
pr: "the PR opened from this branch"
base_sha: aad17f99
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_016c5MQoe5CoMk9fxmuuMcFJ (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PROF_EFFECT0_KILL_CREDIT_AND_PERK_EFFECTS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
  - docs/agents/tasks/archive/OTV2-20261001-prof-effect0.md
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- Kill credit: every character in the creature's damage-contributor record with damage within 5
  minutes before the death earns full points for the weapon equipped at the death (manual, TibiaWiki;
  Canary's last-hitter rule rejected). No durable write per kill; replay bound `PROFEFF0-RL-01`.
- Points: TibiaWiki's difficulty and influence table and Bosstiary values as content rows.
- Perk effects: each of the 33 kinds placed at an existing GAME-ABILITY-01 §10/§11 stage, derived
  read or RANGED-0 input; one stacking rule (sum, clamp, apply once, truncate once); no new stage.
- Homing missile inactive until evidenced; other unconfirmed readings are `PARITY_PENDING` release
  gates.
- No owner question.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the authoring tree.

## Closeout

- PR: the PR opened from this branch; frozen heads in the #162 FREEZE_SHA entries. Merge commit/result: its squash merge.
- Amendments, pending on acceptance: PROFICIENCY-0 §4.3 and §4.5; IMBUE-FORGE-0 §5 (one critical roll).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
```
