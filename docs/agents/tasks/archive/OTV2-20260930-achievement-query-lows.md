# OTV2-20260930-achievement-query-lows

```yaml
task_id: OTV2-20260930-achievement-query-lows
title: Close the carried LOW findings of the ACCOUNT_ACHIEVEMENTS_QUERY review (#1378/#1379)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/eloquent-goldberg-ie1mqf
pr: null   # filled in the PR body; this record is archived in the PR's only authoring commit
base_sha: c70d899a
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "ACHIEVEMENT lane worker (Claude Code, session_01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs
  - docs/architecture/OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md
  - docs/agents/tasks/archive/OTV2-20260930-achievement-query-lows.md
public_contracts: []
depends_on:
  - "#1378 and #1379 (merged)"
  - "control-plane review 5918385373 on #1379 (LOW 1 and LOW 2)"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

Owner authorization in session, 2026-09-30 ("tak" to the follow-up batch). Closes the two actionable LOW
findings of the control-plane review of #1379 (comment 5918385373); LOW 3 (batch size) needs no change.

- LOW 1: `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1.md` §4 item 3 names the integrity-fault code
  `1050 ACCOUNT_DATA_INTEGRITY` (registered by #1379) and the error-level operator log line, matching the
  implementation and the owner decision of 2026-09-30.
- LOW 2: `serve_admitted` in `gameplay_transport/connection.rs` moves the encoded achievements page out of the
  dispatch (`std::mem::take`) instead of cloning up to 32 KiB. The later `match dispatch` ignores the
  achievements reply, so behaviour is unchanged.

## Excluded scope

The client panel, quest grants, the registry and every other path.

## Validation

- `cargo fmt --all --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`;
  `cargo test --locked -p oteryn-game-server --lib` (1199 passed, 2 ignored).
- Governance and repository-policy validators, `git diff --check`.
- The transport path is covered in CI by `account_achievements_query_reads_only_the_controller_account` and the
  PostgreSQL target `character_authority_postgres`.

## Independent review

The control plane decides whether the docs sentence and the one-line move need a review on the frozen head.

## PR and closeout

- PR: from `claude/eloquent-goldberg-ie1mqf`; merge commit/result: squash merge of the PR.
- Review state: pending at authoring.
- Ownership release: on merge.

## Context checkpoint

```yaml
last_progress: both LOWs authored and validated locally
status: completed
next_action: FREEZE_SHA on #162, then control-plane review and Merge Queue
```
