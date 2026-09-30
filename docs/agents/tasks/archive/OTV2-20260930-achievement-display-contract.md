# OTV2-20260930-achievement-display-contract

```yaml
task_id: OTV2-20260930-achievement-display-contract
title: ACHIEVEMENT display - contract for the account achievements panel (one command pair, D49 display amendment)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/achievement-display-contract
issue: 162
lane_id: ACHIEVEMENT
pr: null   # opened by the lead; recorded in the FREEZE_SHA packet on #162
base_sha: 1614042d
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "achievement display contract worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md
  - docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md
  - docs/agents/tasks/archive/OTV2-20260930-achievement-display-contract.md
public_contracts:
  - docs/architecture/OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md
depends_on:
  - "docs/architecture/OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1.md"
  - "docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md §4, §7-§9, §12-§14, §18, §19, §22"
  - "apps/game-server/migrations/0021_account_achievements.sql"
blocks: ["ACHIEVEMENT: display implementation PR (proto, registry, server handler, tests)", "ACHIEVEMENT: client panel"]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Docs and contract only: no code, no `.proto`, no registry change.

- New contract candidate `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md`: one `ClientCommand`/`CommandResult` command
  type `ACCOUNT_ACHIEVEMENTS_QUERY` (POST_ADMISSION, `COMMAND_ID` / `SERVER_SEQUENCED`), a read-only server path
  over `game_account_achievements` joined with the catalogue (migration 0021 already grants `SELECT` to
  `oteryn_game_runtime`), the client list and delivery order.
- Owner decisions of 2026-09-30 recorded as owner direction: secret unearned achievements not shown; one total
  points number; fetch on request; all earned facts shown and counted (decision 4).
- Decision 4 is a D49 display amendment and an owner-acknowledged supersession of one D49 sentence (display of a
  fact whose key a world's catalogue lacks, and the world-catalogue source of points). The fact, its write-once
  rule and the grant path are unchanged. The owner contract §2.3, §4 and §6 are edited minimally to point to it.

## Assumptions and owner questions

- The query carries one `page` field, not an empty payload: 571 records with descriptions exceed the 64 KiB
  command-result bound of FND-02 §19, so the reply is paged at 64 rows. Alternative: drop `description` from the
  reply. Recommendation: keep paging.
- Rows are the earned facts plus non-secret unearned records (shown as not earned); this follows decision 1.
- Order: grade ascending, then name, then key.

## Validation (local)

- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: see the worker report.

## Closeout

- Review: independent exact-head review routed by the lead on the frozen head. The worker triggered no
  owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).
