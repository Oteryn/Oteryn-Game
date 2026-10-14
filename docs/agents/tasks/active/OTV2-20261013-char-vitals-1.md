# OTV2-20261013-char-vitals-1

```yaml
task_id: OTV2-20261013-char-vitals-1
title: "CHAR-VITALS-1: durable health and mana in the last-position projection, restored on admission"
mode: IMPLEMENT
status: validating
status_note: "the CHAR-POSITION-1 HP/mana part (owner decision 2026-10-13); CHAR-VITALS-0 ACCEPTED by the owner (ACCEPT-CHAR-VITALS-0, #2087), status flipped and pin amended here"
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/char-vitals-1-20261013
issue: none
pr: 2045
head_sha: "the PR head; the control plane records the frozen SHA"
created_at: 2026-10-13
updated_at: 2026-10-14
migration_lease: "0084"   # apps/game-server/migrations/0084_character_last_vitals.sql; 0081/0082/0083 are leased by #2032/#2040/#2039
decisions:
  - docs/architecture/reviews/OTERYN_GAME_CHAR_VITALS0_DURABLE_VITALS_DECISION_2026-10-13.md
  - docs/architecture/reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md
resource_limits: [CHARPOS0-RL-01]
owned_paths:
  - apps/game-server/migrations/0084_character_last_vitals.sql
  - apps/game-server/src/durability/character_last_position.rs
  - apps/game-server/src/spell/cast.rs
  - apps/game-server/src/gameplay_transport/actor_spell.rs
  - apps/game-server/src/gameplay_transport/last_position.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/character_last_position_postgres_cases.rs
  - docs/architecture/reviews/OTERYN_GAME_CHAR_VITALS0_DURABLE_VITALS_DECISION_2026-10-13.md
  - docs/architecture/reviews/OTERYN_GAME_ACCEPT_CHARVITALS0_ACCEPTANCE_DECISION_2026-10-13.md   # pin amendment only (§2 of that packet)
  - docs/agents/tasks/active/OTV2-20261013-char-vitals-1.md
public_contracts: []
depends_on: ["CHAR-POSITION-1 (#2006)"]
external_repositories: []
blocked: []
next_action: "CP: trigger the Codex re-review on the new head (CHAR-VITALS-0 now ACCEPTED, Codex P1 answered)"
```

## Outcome

Health and mana are part of the CHAR-POSITION-0 runtime-state projection (CHAR-VITALS-0):

- Migration `0084`: nullable `health` and `mana` on `game_character_last_positions`; both NULL or
  both set, health 1..`MAX_VITAL_POOL` and mana 0..`MAX_VITAL_POOL`. NULL means "start at the
  maxima". The 0080 order, World and no-TRUNCATE guards and grants cover the columns unchanged.
- `LastPositionRecord.vitals: Option<LastVitals>`, written by the same fenced upsert under the
  same (`character_lease_generation`, `write_sequence`) key; a stored row that is half set or out
  of range is `InvalidStoredState`.
- Capture: in the owner step that captures the cell, from the session's spell state
  (`PlayerSpellState::saved_vitals`: `None` at both maxima). Dead actors are never captured. A
  vitals-only change makes the periodic capture changed. A respawn placement writes `None`.
- Admission: `initialize_first_entry` passes the stored vitals only for a Positioned admission from
  a stored row of the same World (`last_position::admitted_vitals`); `initialize_admitted`
  restores them only when it creates the spell state, each capped at the current maximum, never
  to health 0, without a revision bump. A reconnect inside grace keeps its live actor.
- No wire or protocol change; no new resource limit (`CHARPOS0-RL-01` bounds the crash loss).

## Validation

Candidate: this branch merged with `origin/main` (af85035f5).

- `rustfmt --edition 2024 --check` on the changed Rust files: pass
- `git diff --check`: pass
- `cargo` did not complete locally (memory and disk, as in CHAR-POSITION-1 rounds 2 to 4, D1011):
  on the merge with `origin/main` a `cargo clippy -p oteryn-game-server --all-targets -j 1` run was
  OOM-killed in the session memory cgroup while compiling the crate; compile, clippy, the unit
  tests and the Postgres targets are left to CI.
- Merge with `origin/main` (af85035f5): conflicts in `spell/cast.rs` and
  `gameplay_transport/actor_spell.rs` were both additive (CHAR-VITALS-1 functions next to
  FOOD-REGEN-1 `after_food` and `has_training_to_flush`); both sides kept. No generated files.
- `python tools/agents/validate_governance.py`: pass; `python -m unittest discover -s
  tools/agents/tests`: 60 OK.
- New or extended tests:
  - unit: `vitals_are_alive_and_within_the_wire_bound`,
    `stored_vitals_are_both_or_neither_and_valid` (`durability/character_last_position.rs`);
    `an_admitted_actor_restores_its_saved_vitals_only_when_created` (`actor_spell.rs`);
    `a_vitals_change_alone_makes_the_capture_changed`, `only_a_stored_rows_vitals_are_restored`
    and the respawn case (`last_position.rs`);
  - Postgres: `saved_vitals_are_ordered_and_fenced_with_the_cell_and_survive_a_restart` (lower
    and equal keys superseded; stale connection, lease and scope rejected; foreign session
    refused; vitals-only change and clear; no root revision; restart read); the live-death case
    (pre-death vitals, NULL after the respawn placement, stale capture superseded); the table
    guards (old row NULL, seven invalid pairs refused, vitals change without a key raise refused,
    bound values accepted);
  - `durability_postgres` scenario 6: the terminal release writes the final cell with its
    vitals; a release that fails after the write leaves no row and the session reconnectable
    (atomicity); a replay after the terminal state changes nothing.

## Open questions

1. Resolved: CHAR-VITALS-0 accepted by the owner (`ACCEPT-CHAR-VITALS-0`, #2087); this PR flips its
   status line to ACCEPTED and amends the packet's blob pin to the landed file (status line only).
2. Instance actors are not saved (no instance-scope write, CHAR-POSITION-0); their vitals restore
   from the last row written outside the instance.
3. Crash loss is at most 300 s of vitals changes, the same bound as the cell.
