# OTV2-20260930-lcfa-1b-carried

```yaml
task_id: OTV2-20260930-lcfa-1b-carried
title: LCFA-1b - close the five carried LCFA-1 review findings (migration 0028)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/lcfa-1b-carried
issue: 162
lane_id: durability
pr: 1389
base_sha: 07138ee5
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet on #162
final_head_sha: null
final_head_frozen_at: null
owner: "LCFA-1b hard worker (Oteryn work coordinator, #162)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0028_account_characters_projection_retry.sql (migration lease 0028)
  - apps/game-server/src/durability/account_characters_projection.rs
  - apps/game-server/src/native_admission_source/account_characters.rs (+ _tests.rs)
  - apps/game-server/tests/character_authority_postgres.rs, tests/support/account_characters_projection_postgres_cases.rs
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md (one sentence)
  - docs/agents/tasks/archive/OTV2-20260930-lcfa-1b-carried.md
public_contracts:
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
depends_on: [PR 1330 (LCFA-1, merged)]
blocks: [LCFA publisher enablement]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Closes the five carried findings of the LCFA-1 review (#1330, comment 5910198689) before the
publisher is enabled on any host. The publisher stays off (A1); no wire change; 0024 is unchanged.

## Excluded scope

Publisher enablement, the Platform repository, wire changes, edits to migration 0024.

## Closeout

- Finding 1 (MEDIUM): `revised_at` (0028) holds each revision's assignment time; `source_observed_at` =
  `revised_at / 1000`, so a retry of the same pair is byte-identical across restarts. Contract: one sentence.
- Finding 2 (LOW): every Character is listed; lifecycle != 1 is `UNAVAILABLE`; the 65-row bound counts sent entries.
- Finding 3 (LOW): the resync touches every account (new revision), so a pre-resync acknowledgement cannot clear it.
- Finding 4 (LOW): epoch raise = max(current + 1, now in Unix ms); residual lead documented in 0028.
- Finding 5 (LOW): watermark attempt and snapshot phase end within `MAX_WATERMARK_GAP` (10 s) of the attempt start.
- Validation (local, PostgreSQL 17.6): fmt, clippy -D warnings; `--lib` 1164, `character_authority_postgres` 884,
  `check_function_privileges_postgres` 1, `native_admission_source_postgres` 699, `native_admission_source_transport`
  30, all pass; governance and repository policy validators and `git diff --check` pass.
- Review: independent review pending on the frozen head (review packet to the control plane).
- Merge commit/result: squash merge of #1389.
- Stated assumption: `source_observed_at` is the revision's assignment time (never later than the read), which
  makes it durable without a runtime write privilege.
