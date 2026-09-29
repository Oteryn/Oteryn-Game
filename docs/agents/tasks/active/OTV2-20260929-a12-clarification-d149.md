# OTV2-20260929-a12-clarification-d149

```yaml
task_id: OTV2-20260929-a12-clarification-d149
title: "A12 clarifications: D149 and binding evidence"
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/gifted-rubin-a0axzx
issue: 162
pr: 1245
base_sha: cf7777d7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01XdHJyZNPJMcmMnmSDgwQvZ (Sol Supervising Architect)
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md
  - docs/agents/tasks/active/OTV2-20260929-a12-clarification-d149.md
  - docs/agents/tasks/active/OTV2-20260929-a12-item-identity-tibia-id.md   # archive move after #1237
  - docs/agents/tasks/archive/OTV2-20260929-a12-item-identity-tibia-id.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

This task records two follow-ups to A12 (#1237, `8af7f88e`), both answering the coordinator's
request on #162 (5893830130). The ruling is 5894110248.

- **Owner decision D149:** the 4,590 OT records with no CipSoft appearance are removed from
  authored content. Their old keys get `retired_without_successor`.
- **Architect clarifications:**
  - §4.2 defines `EXACT` evidence: the appearance reference plus CipSoft record continuity. A
    differing OT name alone does not block `EXACT`.
  - A repurposed id becomes `CONFLICT`.
  - §5 "no binding lost" means every binding row survives with a disposition.

The task also archives the #1237 task record. No code, content or key change is made.

## Architecture and source of truth

- `PROVEN`: A12 on `main@cf7777d7`; the coordinator request 5893830130; the G4 decision.
- `UNKNOWN`: none added.

## High-risk authority/recovery qualification

Not applicable to this docs-only task. ITEM-ID-1 carries the identity and migration review.

## Acceptance criteria

- [ ] The amendment is on an exact frozen head with passing validators.
- [ ] Independent exact-head review.
- [ ] Protected Merge Queue integration.

## Excluded scope

- Code, content, keys, bindings and migrations.

## Validation

- `python3 tools/agents/validate_governance.py`
- `python3 tools/repository/validate_repository_policy.py`
- `git diff --check`

## Context checkpoint

```yaml
last_progress: amendment authored
status: validating
branch: claude/gifted-rubin-a0axzx
head_sha: null
pr: 1245
owner_action_required: null
blocker: null
next_action: "open the PR, bind it, freeze, request review"
```
