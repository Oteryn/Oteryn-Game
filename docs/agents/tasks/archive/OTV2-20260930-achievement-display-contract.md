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
  - docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md   # pending-amendment pointer only
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
- Rows are exactly the account's earned facts (owner answer 2b, 2026-09-30); no unearned record is sent. Paging
  confirmed by the owner (answer 1a).
- Order: grade ascending, then name, then key.

## Validation (local)

- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: see the worker report.

## Closeout

- Review: independent exact-head review routed by the lead on the frozen head. The worker triggered no
  owner-funded review.
- Merge commit/result: squash merge of the PR (resolve with `git log --grep`).

## Review round 1 (FIX on f497ecb1, repeated on 8fdb464f)

- HIGH: the owner contract §2.3 and §4 kept their accepted text; the display-contract change is a pending
  amendment that applies on its acceptance. The header says "Pending amendment" instead of "Amended by".
- MEDIUM: the owner's words are recorded verbatim with their questions on #162 (comment 5911933242); the display
  contract cites that record, and the control plane assigns the D-numbers. The D49 source
  (account-progress decision §4.4) gains a pending-amendment back-pointer.
- LOW: pages carry `fact_count` (append-only facts, so every grant raises it, including a zero-point one); the
  client refetches on a `fact_count` mismatch instead of `total_points`.
- LOW: command type 10 reserved on #162 in the same record and cited.
- Nits: lines over 120 characters in the display contract reflowed. The stale owner-assumption paragraph on
  paging is replaced by the owner's answer (record display-6).
- Owned paths add `docs/architecture/reviews/OTERYN_GAME_ACCOUNT_PROGRESS_AND_QUEST_707_DISPOSITION_DECISION_2026-09-28.md`
  (pending-amendment pointer only).
