# OTV2-20260929-death1-character-death-writer

```yaml
task_id: OTV2-20260929-death1-character-death-writer
title: DEATH-1 - Character death writer, D58 calculator, pending-respawn consumption
mode: IMPLEMENT
status: blocked   # LANE_BLOCKED: 0016 runtime EXECUTE grant (see Blocker)
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/death1-character-death-writer
issue: 162
lane_id: GAME-CHAR durability / death
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: cf025b7
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
owner: "DEATH-1 hard worker (claude-code-session-01Y1aRVstBEF8u4Sq4MGNhSd)"
control_plane: session_01MnSvpbKjAZEdzEaFrwiu7D
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/domain/progression.rs
  - apps/game-server/tests/reference_character_progression_calc.rs
  - apps/game-server/tests/support/character_progression_postgres_cases.rs   # one #[path] child include
  - apps/game-server/tests/support/character_death_writer_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260929-death1-character-death-writer.md
public_contracts: []
depends_on: [DEATH0-CHARACTER-DEATH-RECEIPT-V1, REFERENCE-FIRST-PLAYER-DEATH-V1, "0016 (#1264)", "0017 (#1270)"]
blocks: [DEATH-1 respawn consumption, DEATH-2]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Split (packet split rule)

The labels `DEATH-1a` (#1154, pure outcome calculator) and `DEATH-1c` (fresh bag) are already
used on `main`, so this task's two PRs carry no letter:

- **DEATH-1 writer (PR #1278):** integrity counts all receipt kinds, `commit_character_death`,
  `reconcile_character_death`, the `ProgressionOperation` wiring of #1154's D58 calculator.
- **DEATH-1 respawn consumption (next PR, successive head):** pending-respawn consumption at runtime
  respawn, admission and recovery. It touches `gameplay_transport/**`, which #1263 leases; it starts
  after #1263 merges.

## DEATH-1 writer design

- **Calculator** (`domain/progression.rs`, death decision §4.3.1): `ApplyDeathExperienceLoss` takes
  `regular_blessings: u8` and `promoted_with_current_premium` and delegates to #1154's
  `domain::death::death_experience_loss` (one source of truth, its bounds: at most
  `MAX_REGULAR_BLESSINGS` = 7, else `TooManyBlessings`), then caps the loss at the held experience
  (never below 0); the level follows. The policy ratio and rounding are pinned for a death to
  D58/D68's 1/1 and `Floor`; any other ratio or rounding is rejected (`InvalidDeathPolicy`)
  instead of scaling the loss. Awards under the same policy are unaffected. The old L→L+1 span and
  the duplicate calculator are removed.
- **Writer** (`durability/character_death.rs`, DEATH-0 §3.5): mirrors `commit_character_experience`
  (recovery fence, admission relation locks, occurrence advisory lock, FND-04 session/lease/scope,
  scope assignment, node incarnation, admission guards, `character_root` row lock). Exact replay
  returns the first receipt before any authority check; a changed binding conflicts first. A new
  death additionally requires the death cell in the fence's Channel, no pending respawn and the
  intent's held blessings equal to the durable set; it advances the root and state, inserts the
  receipt, then deletes the blessings, then inserts the pending respawn.
- **Integrity** (`verify_character_integrity`): one `chain` over XP, death and stance receipts:
  count and distinct revisions = revision − 1, predecessor continuity across kinds, current receipt
  = state, no receipt ahead of the state or with another context.

## Stated assumptions (reversible)

- The D58 closed form is used for the L−1→L span, so level 1 (no L−1 table row) follows D59. The
  policy `death_loss_numerator/denominator/rounding` keep their policy-digest encoding; a death
  requires them to be 1/1 and `Floor`.
- **Blessings:** no non-regular blessing kind exists yet, so every held blessing is counted as a
  regular blessing for D58 and every held blessing is deleted by the death (until DEATH-4 admits
  other kinds). More than seven held blessings fail the death closed (`TooManyBlessings`).
- **Promotion:** no durable promotion or Premium state exists, so `promoted_with_current_premium`
  is always `false` in the writer and binds as 0 (D66 applies once Premium activation and
  promotion land).
- Amulet, equipment/backpack snapshot and RNG stream bind as "none" (DEATH-3 delivery gap).

## Runtime role privileges

The writer runs as `oteryn_game_runtime`. Row locks (`FOR UPDATE`/`FOR SHARE`) need UPDATE, which
0016 does not grant on `game_character_blessings` (SELECT, DELETE) or
`game_character_pending_respawns` (SELECT, INSERT, DELETE); the death reads of both take no row
lock (the fence's `character_root` row lock and the admission relation locks already serialize
every Character writer). The shared gameplay fence read the current interpretation `FOR SHARE`,
which the runtime (SELECT only, 0006) cannot do either, so no fenced XP award or death could
commit as the runtime role. That pre-existing XP-writer gap is fixed here because the death commit
shares the fence: the lock is dropped, which changes nothing else (interpretation rows are
immutable, 0005 trigger, and a new one is only inserted, so the row lock serialized nothing).
Follow-up: no PostgreSQL case runs the XP writer (`commit_character_experience`) as the runtime
role; add one with the next XP-writer change.

## Validation (local, isolated workspace)

- `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo test -p oteryn-game-server` with PostgreSQL configured: 48 targets, all pass.
- PostgreSQL 17.6 (`postgres:17.6-bookworm@sha256:f3bd19c6…`): all `*_postgres` targets, including 4
  new cases run through `character_progression_postgres` and `character_authority_postgres`:
  commit/replay/conflict/reconcile/restart integrity; stale connection, lease, scope and revision,
  wrong Channel, wrong blessings and context write nothing; mixed chain XP → death → stance → death;
  a concurrent XP award and death on two roots serialize (one commits, the other gets
  `CharacterRevisionMismatch`, no deadlock).
- RED: with the old XP-only integrity check the restart and mixed-chain cases fail; a chain gap
  (stance receipt removed) fails integrity.

### Repair of 34a3b190 (independent review, disposition FIX)

1. Duplicate D58 calculator: `ApplyDeathExperienceLoss` delegates to
   `domain::death::death_experience_loss` (#1154 bounds, > 7 regular blessings rejected); PR and
   task label corrected (no `DEATH-1a`).
2. Death policy pinned to ratio 1/1 and `Floor`; other ratios/roundings rejected. The XP writer's
   unit fixture uses 1/1 too.
3. `FOR UPDATE` removed from the pending-respawn and blessing reads, `FOR SHARE` from the shared
   fence's interpretation read; a PostgreSQL case commits, replays and rejects a second death as a
   login in `oteryn_game_runtime`.
4. A PostgreSQL case commits the same death occurrence concurrently on two roots: exactly one
   receipt; the other attempt replays it (`AlreadyCommitted`, equal receipt).
5. Blessing and promotion assumptions recorded above.

RED: the runtime-role case fails with the blessings `FOR UPDATE` (`permission denied for table
game_character_blessings`) and with the interpretation `FOR SHARE` (`permission denied for table
game_character_interpretations`); the concurrent same-occurrence case fails when the receipt replay
is disabled (`CharacterRevisionMismatch`); the ratio/rounding pin case fails when the pin is
disabled.

## Blocker (LANE_BLOCKED)

The runtime-role case fails on this head: `permission denied for function
game_character_is_blessing_set` on the death receipt INSERT. 0016 revokes EXECUTE on that function
(used by the `blessings_before`/`blessings_after` CHECKs) from PUBLIC and grants it to no role, and
a CHECK function runs with the inserting role's privileges, so no `oteryn_game_runtime` login can
commit a death (the owner-run cases never saw it). Fix, outside this packet's scope (migrations
excluded): an additive migration
`GRANT EXECUTE ON FUNCTION game_character_is_blessing_set(text[]) TO oteryn_game_runtime;`
(0016 cannot change after merge). With that grant applied in the test setup, the runtime-role case
passes and `cargo test -p oteryn-game-server` against PostgreSQL 17.6 passes (48 targets, 9484
tests, 0 failed); without it exactly that case fails. The case is not weakened to pass.
