# OTV2-20261012-char-position-1

```yaml
task_id: OTV2-20261012-char-position-1
title: "CHAR-POSITION-1: durable logout position, written under the session fence and restored on admission"
mode: IMPLEMENT
status: completed
status_note: "position is delivered by this PR; HP/mana is blocked on an owner decision"
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-position-1-20261012
issue: none
pr: 2006
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-12
updated_at: 2026-10-17
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
next_action: "FREEZE at the round-4 head (Codex P1 4249667405 fixed). Deferred to follow-up CHAR-POSITION-2 per D282, not this PR: Codex P1 4248578112 (quiesce gameplay before the shutdown flush) and P2 4248578103 (bound the shutdown flush's timeout)"
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
- §3.3 admission: the durable position (a pending respawn, else the last position) is read
  under the session fence before the first-entry Channel-owner step and replaces `entry_start`
  as its input: the actor is initialized and placed in that one owner step, so party presence
  and the familiar's login placement, which run after it, read the placed cell. A pending
  respawn is consumed in one transaction that upserts the placed cell
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

Archive candidate (#2163, after the merge of #2006):

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK

## Review round 1 (Codex, head 559bc229)

- P1 4245761767 (admission respawn, cell not captured): accepted and fixed; no consumption.
- P1 4245761770 (300 s bound under cadence load): accepted and fixed; own deadline loop.
- P1 4246368083 (live death not durable): accepted and fixed; atomic respawn upsert. The sweep
  found the same family at release: the final write captured a dead actor's death cell. Fixed:
  dead actors are never captured.
- P2 4245761776 (shutdown flush after failed writes): fixed.
- P2 4246368094 (saved house tile used the radius fallback): fixed.

## Review round 2 (Codex, head eef64dc5)

- P1 4248014751 (admission playable after a failed last-position read): fixed. A failed read
  or placement makes `consume_admitted_respawn` return `false`: the first entry is
  `RefusedByChannel`, the session stays untracked and nothing is written.
- P1 4248014759 (sequential write pass can exceed 300 s under load): fixed. Each batch is
  captured, its fences read in one root pass and written in one transaction; the bound
  (`MAX_WRITE_PASS`, `MAX_WRITE_LAG`) is asserted at compile time and by a unit test. A storage
  error fails the whole batch, which stays due.

## Review round 3 (Codex, head e9fce464)

- P1 4248578094 (writer bound by connections, not tracked sessions): fixed. The bound is
  `MAX_TRACKED_SESSIONS` (the 500 player slots, D128); batch 64, so a full pass is 8 batches
  (32 s) and `MAX_WRITE_LAG` 37 s, within the 60 s slack.
- P1 4248578112 (quiesce gameplay before the shutdown flush) and P2 4248578103 (shutdown flush
  timeout bound): deferred to follow-up CHAR-POSITION-2 per D282.

## Merge with main after #2004 (DEATH-RESTART-SAFETY-1, 7cced55e7)

- `character_death.rs` conflict: main's side taken; `commit_death` calls `settle_player_death`.
- §3.3 step 1 moves to main's path: `place_and_consume_pending_respawn_placed` upserts the
  placed cell in the fenced transaction that places the actor and deletes the respawn row (only
  when both happened), with the checks of `consume_pending_respawn_placed` (`validate_placed`).
  `place_and_consume_settled_respawn` refuses a cell other than the respawn cell's record.

## Review round 4 (Codex, head ecebaf1b)

- P1 4249667405 (saved cell restored only after the first-entry owners ran): fixed.
  `initialize_first_entry` reads the durable position (`read_admission_position`: a pending
  respawn, else the last position) under the session fence before the Channel-owner lock; the
  owner step initializes the actor and places it at that cell in one step
  (`initialize_admitted_position`, `initialize_first_entry_at`), before party presence and the
  familiar's login placement run. House tile, World and frame checks still go to `entry_start`;
  a blocked or occupied cell uses the NPC-0 §6.1 fallback. A failed read leaves the actor
  unpositioned. `consume_admitted_respawn` then consumes the respawn already placed (placing
  only one the first entry did not see) or tracks the stored cell (`track_restored`, replacing
  `restore_last_position`). New unit tests in `gameplay_transport/last_position.rs`:
  `the_saved_cell_replaces_the_entry_start_in_the_first_entry_step`,
  `a_blocked_saved_cell_uses_the_radius_fallback_then_the_entry_start`,
  `a_house_tile_another_frame_or_no_row_enters_at_the_entry_start`,
  `a_pending_respawn_is_the_first_entry_input`.

Rounds 2 to 4 and the merge: `cargo` was not run locally (D1011); `rustfmt --check` on the
changed Rust files, `validate_governance.py`, the `tools/agents/tests` unit tests and `git diff
--check` ran locally; compile, clippy and the tests are left to CI.

## Open questions

1. HP/mana persistence: decided 1a (separate contract and follow-up packet).
2. The fixture-room world has no house metadata, so a stored cell there fails closed: no restore,
   entry start. Bundle worlds use the compiled house-tile facts.
3. The `CHARPOS0-RL-01` boundary "a configured value above 300 s is rejected" does not apply: the
   interval is a constant, not configuration.

## Archive closeout

- canonical PR #2006 merged on 2026-10-12 (`git log --grep "(#2006)"`); the record stayed in
  `tasks/active/` and failed the protected-main live-state check, so #2163 archives it
  as a ported `main` governance fix
- merge commit/result: squash merge of #2006
- ownership release: released with the merge of #2006
