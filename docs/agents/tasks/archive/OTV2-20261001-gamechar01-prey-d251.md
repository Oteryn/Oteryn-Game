# OTV2-20261001-gamechar01-prey-d251

```yaml
task_id: OTV2-20261001-gamechar01-prey-d251
title: "GAME-CHAR-01 baseline amendment: Prey scope Account-wide (D251)"
mode: CONTRACT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/arch-gamechar01-prey-d251
pr: "1426"
base_sha: 2e8d34dc
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session-01KbqAgmFfAYDSKHKkFmWKWW (Sol Supervising Architect)
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md
  - docs/architecture/GAMEPLAY_AND_PRODUCT_ARCHITECTURE_HORIZON.md
  - docs/agents/tasks/archive/OTV2-20261001-gamechar01-prey-d251.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D251 (2026-10-01, #162, answering Q14 / PREY-0 §15 Y2) makes Prey Wildcards, the
permanent Prey slot and the Weekly Task Expansion an Account-wide balance shared by all the
account's characters (a declared difference from Tibia). The accepted GAME-CHAR-01 Stage B owner
baseline listed permanent Prey slots as character-specific.

- Baseline §12: the list entry points to the amendment, and an "Amendment (D251, 2026-10-01)" note
  states the Account-wide scope and cites PREY-0 (PR #1413).
- Horizon: a one-line pointer on the same list entry.
- Hunting Task Points and permanent Hunting Task slots stay character-specific.

No code, migration or content change is made.

## Architecture and source of truth

- `PROVEN`: owner decision D251 (#162); PREY-0 decision
  `docs/architecture/reviews/OTERYN_GAME_PREY0_PREY_AND_HUNTING_TASKS_DECISION_2026-09-30.md` (PR #1413).

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: docs only.

## Acceptance criteria

- [ ] Amendment on an exact frozen head with passing validators.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, migrations and content; the Store delivery contract; other accepted docs that state the
  per-character rule (the Stage B minimum-closure packet, reference evidence delta 02 and DUR-02
  schema packet are evidence or packets and are left as written; the amendment governs).

## Validation

- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 tools/repository/validate_repository_policy.py`: PASS.
- `git diff --cached --check`: clean.

## Closeout

- PR: the one named in the #162 FREEZE_SHA entry. Merge commit/result: its squash merge.
- This record was archived in the PR's final authoring commit (`docs/agents/tasks/archive/README.md`).

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/arch-gamechar01-prey-d251
owner_action_required: null
blocker: null
next_action: null
```
