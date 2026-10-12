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
  - "HP/mana persistence: no accepted contract decides it (see Open questions)"
```

## Outcome

Position only, per CHAR-POSITION-0 (still CANDIDATE at this base):

- §3.1 storage, migration `0080`: `game_character_last_positions`, one row per Character, never
  deleted. Guards: the row's World is the Character root's World; an UPDATE must keep Character
  and World and strictly raise (`character_lease_generation`, `write_sequence`); no TRUNCATE.
  Runtime role SELECT/INSERT/UPDATE, control role SELECT.
- §3.2 writes, under the session fence only (no root lock), applied only when the key is greater:
  - final write inside the terminal fresh-admission release, before `session_state = 3`, for
    logout, grace expiry, abandonment and capability mismatch;
  - periodic write of a changed cell, at most every 300 s (`CHARPOS0-RL-01`), from the gameplay
    owner cadence; an unchanged cell writes nothing;
  - node shutdown flushes every tracked session before readiness is withdrawn (3 s of the 10 s
    shutdown budget);
  - nothing is written while a pending respawn exists (`PendingObligation`) or in an instance
    scope.
- §3.3 admission: a pending respawn is consumed in one transaction that upserts the placed cell
  under the new lease key. Otherwise the stored cell is restored if it is of the same World and
  coordinate frame, Walkable, and not a house tile (fail closed when house metadata is unknown),
  through the NPC-0 §6.1 radius-3 fallback, else the entry start.
- The write sequence is process-wide, strictly increasing and at least the Unix time in
  microseconds, so a restarted node orders after its predecessor under the same lease.

## Validation

Candidate: this branch on `origin/main` (e128fd6), PostgreSQL 17.6.

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet --lib`: pass (2803 passed, 0 failed, 28 ignored)
- `--test character_authority_postgres last_position`: pass (6)
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
  - unit tests in `gameplay_transport/last_position.rs` for the interval bound, sequence order,
    frame match and tracking.

## Open questions

1. HP/mana persistence has no accepted contract. CHAR-POSITION-0 covers position only. Not
   implemented here.
2. The fixture-room world has no house metadata, so a stored cell there fails closed: no restore,
   entry start. Bundle worlds use the compiled house-tile facts.
3. The `CHARPOS0-RL-01` boundary "a configured value above 300 s is rejected" does not apply: the
   interval is a constant, not configuration.
