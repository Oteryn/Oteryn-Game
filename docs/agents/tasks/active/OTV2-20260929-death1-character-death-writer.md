# OTV2-20260929-death1-character-death-writer

```yaml
task_id: OTV2-20260929-death1-character-death-writer
title: DEATH-1 - Character death writer, D58 calculator, pending-respawn consumption
mode: IMPLEMENT
status: implementing
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
blocks: [DEATH-1b, DEATH-2]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Split (packet split rule)

- **DEATH-1a (this PR):** integrity counts all receipt kinds, `commit_character_death`,
  `reconcile_character_death`, the D58 calculator.
- **DEATH-1b (next PR, successive head):** pending-respawn consumption at runtime respawn, admission
  and recovery. It touches `gameplay_transport/**`, which #1263 leases; it starts after #1263 merges.

## DEATH-1a design

- **Calculator** (`domain/progression.rs`, death decision §4.3.1): `ApplyDeathExperienceLoss` now
  takes `regular_blessings` and `promoted_with_current_premium` and computes
  `(L+50) × 50 × (L² − 5L + 8) × (100 − 8b − 30p) × num / (10000 × den)` exactly in `i128`, rounded
  once by the policy mode, then capped at the held experience (never below 0); the level follows.
  Reductions above 100% fail closed (`InvalidDeathReduction`). The old L→L+1 span is removed.
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
  policy `death_loss_numerator/denominator/rounding` stay as a declared scale and rounding (Reference:
  1/1, `Floor`); their policy-digest encoding is unchanged.
- No durable promotion or Premium state exists, so `promoted_with_current_premium` is always false
  in the writer and binds as 0 (D66 applies once Premium activation and promotion land).
- Every held blessing counts as regular and is consumed until DEATH-4 admits other kinds.
- Amulet, equipment/backpack snapshot and RNG stream bind as "none" (DEATH-3 delivery gap).

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
