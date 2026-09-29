# OTV2-20260929-proficiency0-decision

```yaml
task_id: OTV2-20260929-proficiency0-decision
title: "PROFICIENCY-0 Weapon Proficiency: content, persistence and wire"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/proficiency0-decision
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: d4cb72ee
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
  - docs/agents/tasks/archive/OTV2-20260929-proficiency0-decision.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records architect item PROFICIENCY-0 (#162 5899469936). It decides three things:

- **Content:** proficiency definitions are their own records keyed by the CipSoft
  `ProficiencyId`, and Items reference them by key. Unmapped perk codes stay staged.
- **Persistence:** one track per (Character, weapon Item key), and one receipt kind with lines
  per changed track. It is checkpointed like A13, with an indexed per-track chain.
- **Wire:** one `ACTOR_PROFICIENCY` state domain and one perk-selection command, behind a
  capability. This part is a contract candidate that needs owner acceptance.

Modification costs go to a later decision (PROFICIENCY-1). No code, migration or content change
is made.

## Architecture and source of truth

- `PROVEN`: #162 5899469936; #1283 staged sources; `docs/reference/tibia-manual/combat.md`
  §5.3.4; GAME-CHAR-01 Stage B decision 9; DUR-02 rule 2; `PROTOCOL_OTERYN_V1_REGISTRY.json`;
  A12 item keys; A13 (#1271).
- `UNKNOWN`: perk code meanings, the point table, shared progress between weapons, and
  modification costs.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is a docs-only decision. PROF-1 changes persistence and needs its own
persistence review.

## Acceptance criteria

- [x] The decision document is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Owner acceptance of the §4.4 wire candidate (through the control plane).
- [ ] Protected Merge Queue integration.

## Excluded scope

- Migrations, runtime code, content, registry rows and `.proto` files.

## Validation

- `python3 tools/agents/validate_governance.py`: PASS on the final authoring tree.
- `python3 tools/repository/validate_repository_policy.py`: PASS on the final authoring tree.
- `git diff --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- Self-review: `oteryn-hard-worker`, read-only, on the complete draft before freeze.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

## Context checkpoint

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/proficiency0-decision
owner_action_required: "§4.4 wire acceptance"
blocker: null
next_action: null
```
