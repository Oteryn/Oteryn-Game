# OTV2-20261001-creature-ai0

```yaml
task_id: OTV2-20261001-creature-ai0
title: "CREATURE-AI-0 creature AI, spawns and summons decision (owner 4a, 5a, 6a)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-creature-ai-0
pr: "named in the #162 FREEZE_SHA entry"
base_sha: b73fe23c
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_018aTt5eRKVUGPJJYwwoMcqw (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_CREATURE_AI0_CREATURE_AI_SPAWNS_AND_SUMMONS_DECISION_2026-10-01.md
  - docs/architecture/reviews/OTERYN_GAME_AI_ACTION_INTEGRATION_FIRST_CREATURE_SLICE_DECISION_2026-09-28.md
  - docs/architecture/reviews/OTERYN_GAME_CONDITIONS0_ACTOR_CONDITIONS_DECISION_2026-09-30.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/architecture/ARCHITECTURE_ANALYSIS_GAP_REGISTER.md
  - docs/agents/tasks/archive/OTV2-20261001-creature-ai0.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decisions 4a (full Tibia creature behaviour from creature data), 5a (player summons) and 6a
(world object state per channel, reset at the server save), 2026-10-01 on #162, are turned into
the CREATURE-AI-0 decision: one channel owner per creature; perception by the MOVE-RL-11 relation;
Canary-order targeting, target change, flee, dance and keep-distance; a creature step timer (the
CONDITIONS-0 §4.3 amendment); paths on the writer inside a deterministic 50 ms window budget; no
floor change by walking; walk back and leash; the reference map's spawns per channel with player
blocking, the spawn warning and the accepted occupancy chain; monster and player summons with cap,
following, removal and attribution; everything runtime-only; rows `CREATUREAI0-RL-01` to `-19`.

Pointer amendments, each pending on acceptance: the first creature slice (after §4.9),
CONDITIONS-0 §4.3, the horizon `GAME-AI-01` entry and gap register §11. No code, registry,
protocol or content change is made.

## Architecture and source of truth

- `PROVEN`: owner decisions 4a, 5a, 6a (#162, 2026-10-01); GAME-AI-01 and its first creature slice;
  MOVE-RL-11; ATTACK-0; CONDITIONS-0; PARTY-PVP-0; D3; DUR-03 A4; ADR-0021; the content behaviour
  and creature families.
- `CIPSOFT_OFFICIAL`: the Tibia manual notes in `docs/reference/tibia-manual/`.
- `OTS_HYPOTHESIS_ONLY`: Canary `04b83b51`, read-only, cited by path and line.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only.

## Acceptance criteria

- [ ] Decision and amendments on an exact frozen head with passing validators.
- [ ] Independent review (AI determinism and performance, movement, combat; protocol for §8.6).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, registry rows, protocol numbers and content; bosses, raids and encounters (BOSS-RAID-0);
  pushing, voices, spawn periods, respawn modifiers, invisibility, familiars, NPCs, the player
  chase mode and diagonal steps.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 tools/repository/validate_repository_policy.py`: PASS.
- `git diff --cached --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Reported to the control plane: architect rulings R2 (no cross-floor chase, against the literal
  wording of 4a) and R4 (D57's new owner decision read as 4a with D188 and 6a).
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-creature-ai-0
owner_action_required: null
blocker: null
next_action: null
```
