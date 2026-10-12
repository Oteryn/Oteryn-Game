# OTV2-20261012-char-position-1

```yaml
task_id: OTV2-20261012-char-position-1
title: "CHAR-POSITION-1: durable logout position, written under the session fence and restored on admission"
mode: IMPLEMENT
status: validating
status_note: "position is delivered by this PR; HP/mana is blocked on an owner decision"
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-position-1-20261012
issue: none
pr: 2006
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-12
updated_at: 2026-10-12
migration_lease: "0080"   # apps/game-server/migrations/0080_character_last_position.sql; other packets take 0081 or later
decisions:
  - docs/architecture/reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_NPC0_NPC_RUNTIME_SERVICE_DECISION_2026-09-30.md   # §6.1 fallback only
resource_limits: [CHARPOS0-RL-01]
owned_paths:
  - apps/game-server/migrations/0080_character_last_position.sql
  - apps/game-server/src/durability/character_last_position.rs
  - apps/game-server/src/durability/character_progression.rs   # assert_session_fence extracted, behaviour unchanged
  - apps/game-server/src/durability/character_death.rs         # consume_pending_respawn_placed
  - apps/game-server/src/durability/fresh_admission.rs         # final write inside the terminal release
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/gameplay_transport/last_position.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/monk_save_postgres_tests.rs
  - apps/game-server/src/gameplay_transport/qualification.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/character_last_position_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20261012-char-position-1.md
public_contracts: []
depends_on: ["DEATH-2b (#1747)"]
external_repositories: []
blocked:
  - "HP/mana persistence: CP decision 1a, a separate contract and follow-up packet, not this PR"
next_action: "FREEZE at the round-3 head. Deferred to follow-up CHAR-POSITION-2 per D282, not this PR: Codex P1 4248578112 (quiesce gameplay before the shutdown flush) and P2 4248578103 (bound the shutdown flush's timeout)"
```

## Outcome

Position only, per CHAR-POSITION-0 (still CANDIDATE at this base; CP decision 2a: the control
plane accepts it in a separate docs PR, and this PR's code does not wait for it):

- §3.1 storage, migration `0080`: `game_character_last_positions`, one row per Character, never
  deleted. Guards: the row's World is the Character root's World; an UPDATE must keep Character
  and World and strictly raise (`character_lease_generation`, `write_sequence`); no TRUNCATE.
  Runtime role SELECT/INSERT/UPDATE, control role SELECT.
- §3.2 writes, under the session fence only (no root lock), applied only when the key is greater:
  - final write inside the terminal fresh-admission release, before `session_state = 3`, for
    logout, grace expiry, abandonment and capability mismatch;
  - periodic write of a changed cell, at most every 300 s (`CHARPOS0-RL-01`), from its own writer
    loop: it wakes at the earliest absolute deadline, measured from the capture the row holds, with
    60 s left for the pass, most overdue first; a later capture is reported; an unchanged cell
    writes nothing. A pass works in batches of 64 sessions, each one root pass reading the
    sessions' fences (`current_sessions_at`) and one transaction writing them, each write under
    its own session fence (`write_character_last_positions`); 256 sessions due together take at
    most 8 batches of two 2 s passes, and a due session completes at most 37 s after its
    deadline, within the 60 s slack (checked at compile time);
  - a live death leaves its respawn pending (DEATH-RESTART-SAFETY-1, #2004); the in-session
    placement upserts the respawn cell in the fenced transaction that places the actor and
    consumes the respawn (`place_and_consume_pending_respawn_placed`), only when it deletes the
    row; a respawn kept for the next admission is written by that admission's consumption; a
    dead actor is never captured;
  - node shutdown flushes every tracked session before readiness is withdrawn (3 s of the 10 s
    shutdown budget); failed writes are retried and the flush is done only when none failed;
  - nothing is written while a pending respawn exists (`PendingObligation`) or in an instance
    scope.
- §3.3 admission: a pending respawn is consumed in one transaction that upserts the placed cell
  under the new lease key; a placed actor whose cell cannot be read back consumes nothing and the
  admission is not playable. Otherwise the stored cell is restored if it is of the same World and
  coordinate frame and not a house tile (a house tile goes straight to the entry start; fail
  closed when house metadata is unknown), at the cell or through the NPC-0 §6.1 radius-3
  fallback, else the entry start. A last-position read that fails (storage or fence error) or a
  placement that cannot complete leaves the admission not playable, as an unplaced respawn does.
- The write sequence is process-wide, strictly increasing and at least the Unix time in
  microseconds, so a restarted node orders after its predecessor under the same lease.

## Validation

Candidate: this branch on `origin/main` (e128fd6), PostgreSQL 17.6.

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet --lib`: pass (2804 passed, 0 failed, 28 ignored)
- `--test character_authority_postgres last_position`: pass (7)
- `--test character_authority_postgres -- death respawn`: pass (104)
- `--test durability_postgres owning_fresh_loss_is_atomic`: pass (1)
- the remaining `*_postgres` targets are left to CI
- New tests:
  - `last_position_orders_by_key_and_survives_a_node_restart`;
  - `stale_fences_never_overwrite_the_last_position` (stale connection, lease, scope, session,
    World, foreign record, later lease, terminal session);
  - `a_pending_respawn_decides_and_its_consumption_writes_the_placed_cell`;
  - `the_table_refuses_foreign_worlds_lower_keys_and_deletes`;
  - scenario 6 of `owning_fresh_loss_is_atomic_and_raw_prepare_does_not_supply_authority`: the
    terminal release writes the final cell, and a replay after the terminal state changes nothing;
  - `an_in_session_respawn_writes_the_placed_cell_with_its_consumption` (review round 1,
    reworked for the #2004 merge);
  - `an_in_session_respawn_writes_only_the_cell_it_places` (unit, #2004 merge);
  - unit tests in `gameplay_transport/last_position.rs` for the interval bound, sequence order,
    frame match, capture-based deadlines and tracking.

## Review round 1 (Codex, head 559bc229)

- P1 4245761767 (admission respawn, cell not captured): accepted and fixed; no consumption.
- P1 4245761770 (300 s bound under cadence load): accepted and fixed; own deadline loop.
- P1 4246368083 (live death not durable): accepted and fixed; atomic respawn upsert. The sweep
  found the same family at release: the final write captured a dead actor's death cell. Fixed:
  dead actors are never captured.
- P2 4245761776 (shutdown flush after failed writes): fixed.
- P2 4246368094 (saved house tile used the radius fallback): fixed.

## Review round 2 (Codex, head eef64dc5)

- P1 4248014751 (admission playable after a failed last-position read): accepted and fixed.
  `restore_last_position` returns whether the placement was decided from a successful read;
  a failed read or a failed placement makes `consume_admitted_respawn` return `false`, so the
  first entry is `RefusedByChannel` (not input-eligible), the session stays untracked and
  nothing is written.
- P1 4248014759 (sequential write pass can exceed 300 s under load): accepted and fixed. The
  pass captures each batch first, then reads the batch's fences in one root pass and writes the
  batch in one transaction, so its durability work no longer grows with one pass per session.
  The bound (`MAX_WRITE_PASS`, `MAX_WRITE_LAG`) is asserted at compile time and by the unit test
  `a_pass_of_every_admitted_session_fits_in_the_slack`. A storage error fails the whole batch,
  which stays due.
- Validation of this round: `cargo` was not run locally (lead directive D1011); `rustfmt
  --check` on the changed Rust files, `python tools/agents/validate_governance.py`, `python -m
  unittest discover -s tools/agents/tests` and `git diff --check` ran locally; fmt, clippy and the
  tests are left to CI.

## Review round 3 (Codex, head e9fce464)

- P1 4248578094 (writer bound by connections, not tracked sessions): accepted and fixed. A track
  outlives its socket, so the bound is now `MAX_TRACKED_SESSIONS` (the 500 player slots the
  carrier reserves, D128), not the 256-connection limit. The batch grows to 64, so a pass of every
  tracked session is still 8 batches (32 s) and `MAX_WRITE_LAG` 37 s, within the 60 s slack; the
  compile-time asserts and `a_pass_of_every_tracked_session_fits_in_the_slack` hold.
- P1 4248578112 (quiesce gameplay before the shutdown flush) and P2 4248578103 (shutdown flush
  timeout bound): deferred to follow-up CHAR-POSITION-2 per D282.

## Merge with main after #2004 (DEATH-RESTART-SAFETY-1, 7cced55e7)

- Conflict in `character_death.rs`: main's side taken. The death commit no longer consumes its
  respawn, so `settle_player_death_respawned` is removed and `commit_death` calls
  `settle_player_death` again.
- The CHAR-POSITION-0 §3.3 step 1 guarantee moves to main's new path:
  `place_and_consume_pending_respawn_placed` shares the body of
  `place_and_consume_pending_respawn` and, under the same fence and in the same transaction,
  upserts the placed cell after the row of the occurrence is deleted (only when the placement
  returned `true` and the delete removed the row), with the World and nonzero-sequence checks of
  `consume_pending_respawn_placed` (shared `validate_placed`, refused before `place` runs).
  `place_and_consume_settled_respawn` passes the respawn cell's record, taken after the death,
  and the placement refuses a cell other than that record. The admission path
  (`consume_pending_respawn_placed`) is unchanged.
- Validation of this merge: `cargo` was not run locally (D1011); `rustfmt --check` on the changed
  Rust files, `python3 tools/agents/validate_governance.py`, the `tools/agents/tests` unit tests
  and `git diff --check` ran locally; compile, clippy and the tests are left to the CP and CI.

## Open questions

1. HP/mana persistence: decided 1a (separate contract and follow-up packet).
2. The fixture-room world has no house metadata, so a stored cell there fails closed: no restore,
   entry start. Bundle worlds use the compiled house-tile facts.
3. The `CHARPOS0-RL-01` boundary "a configured value above 300 s is rejected" does not apply: the
   interval is a constant, not configuration.
