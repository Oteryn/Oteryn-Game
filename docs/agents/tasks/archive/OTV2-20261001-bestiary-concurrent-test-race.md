---
task_id: OTV2-20261001-bestiary-concurrent-test-race
title: Accept every valid serialization in the concurrent Bestiary kill test
mode: REPAIR
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: cbf6380
branch: claude/quirky-gates-6bzn81
owned_paths:
  - apps/game-server/tests/support/bestiary_progress_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261001-bestiary-concurrent-test-race.md
---

# Accept every valid serialization in the concurrent Bestiary kill test

## Defect

`concurrent_kills_serialize_on_the_character_revision` races two `commit_bestiary_kill` calls with `join_two`. It accepted only `(Committed, Unavailable(RootUnavailable))` in either order. The durability root has one pooled connection, and each call takes it with a non-blocking `try_acquire` on its first poll. Usually the first call holds it, so the second call is refused as `RootUnavailable`. But if every query of the first call completes without returning `Pending`, the first call commits within its first poll and gives the connection back. The second call then runs after the commit, and its result is still correct:

- For the same occurrence (62), the second call finds the receipt and returns `AlreadyCommitted`. This was observed on #1430, CI run 36834346293: `(Ok(Committed(..)), Ok(AlreadyCommitted(..)))`.
- For distinct occurrences (60/61) on one predecessor revision, the second call finds no receipt. The gameplay fence then sees the advanced revision and returns `CharacterRevisionMismatch`.

## Outcome

Test only. Both matches now also accept the late-starter outcome. The assertions that follow are unchanged: the loser's retry gets `CharacterRevisionMismatch`, the replay gets `AlreadyCommitted`, the root revision is `3`, and `kill_count` is `2`. Production code is not changed.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: no production mutation, fence or authority path changes. The test still requires exactly one commit per race and the same final state.

## Validation

- `cargo test -p oteryn-game-server --test character_authority_postgres bestiary_progress` against local PostgreSQL 17: PASS.
- `cargo fmt --check`, `cargo clippy` for the test target, and the governance and repository policy validators: PASS.

## Follow-up

`character_progression_postgres_cases.rs` (the duplicate XP commit race, about line 807) uses the same `join_two` pattern and accepts only `(Committed, RootUnavailable)`, so it can fail the same way. It is outside this task's owned paths.
