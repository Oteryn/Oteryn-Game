# OTV2-20260930-achievement-docs-nits

```yaml
task_id: OTV2-20260930-achievement-docs-nits
title: Achievement contract docs nits from #1343 and #1348
mode: DOCS
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/achievement-docs-nits
issue: 162
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-authorized Claude Code session, 2026-09-30
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md
  - docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md
  - docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md
  - docs/agents/tasks/archive/OTV2-20260930-achievement-docs-nits.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

- DONE: owner contract §6 said protocol and client display "are in" the display contract; that contract is still a
  candidate, so §6 now says it is covered by the candidate and applies on its acceptance (as §2.3 and §4 do).
- DONE: the display contract cites D223-D228 for display-1 to display-6 (control plane record, #162 comment
  5911933242); the 707 disposition §4.4 pending-amendment bullet cites D226 (world scope, display-4).
- CHECKED: no prose line over 120 characters remains in the two contracts; the only longer line is an indented
  table row, left as is.

## Validation (local)

- `tools/agents/validate_governance.py`, `tools/repository/validate_repository_policy.py`, `git diff --check`.
