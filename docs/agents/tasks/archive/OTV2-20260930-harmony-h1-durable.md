# OTV2-20260930-harmony-h1-durable

```yaml
task_id: OTV2-20260930-harmony-h1-durable
title: Durable monk Harmony and remaining forced Serene time (SPELL-D8 H-1)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1360
allocation_comment: "#162 5910691466 (point 2, H-1)"
base_branch: main
branch: claude/eager-pasteur-eobvo3
base_sha: 7b66294f
owner: "Oteryn: spell state" (Claude Code)
created_at: 2026-09-30T18:00:00Z
execution_policy: continuous_progress
risk: high (persistence, fenced Character writes)
owned_paths:
  - apps/game-server/migrations/0026_character_monk_harmony.sql
  - apps/game-server/src/durability/monk_state.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/character_death.rs
  - apps/game-server/src/durability/character_authority.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/agents/tasks/archive/OTV2-20260930-harmony-h1-durable.md
public_contracts: []
owning_contract: docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md §8.2 (SPELL-D8)
depends_on: []
blocks: [H-live (runtime actor wiring)]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

- **Migration 0026** (lease 0026; 0025 is HOUSE-CUSTODY-1's and is not created here). Two columns of the typed
  progression state row: `harmony` (0..5, default 0) and `serene_forced_remaining_micros` (the remaining forced
  Serene time, forced-until minus owner time floored at zero, 0..7,000,000 us, default 0). Existing rows read 0
  and 0. The Serene flag is not stored (D209).
- **Monk state receipts.** An actor-end save that changes either value advances the global CharacterRevision once
  with an immutable receipt (`game_character_monk_state_receipts`, experience and level unchanged, `before` and
  `after` of both values). An unchanged save writes nothing. The 0020 consistency guard is replaced with its body
  kept verbatim plus the monk arms: the sixth receipt kind in the chain, 0 and 0 at revision one, a change only
  through a monk receipt that matches the replaced and new row, and a death transition must empty both.
  `verify_character_integrity` counts the new kind.
- **Writer** `DurabilityRoot::commit_character_monk_state_save`: the shared XP gameplay fence (recovery fence,
  session generation, lease and scope generations, scope assignment, node incarnation, root at the expected
  revision). Exact replay returns the receipt; changed reuse conflicts; a stale fence writes nothing. While a
  death's respawn is pending, a save that would change the death's zeros is refused.
  `reconcile_character_monk_state_save` reads a retained receipt.
- **Load** `DurabilityRoot::read_character_monk_state` returns a typed `DurableMonkState` (0 and 0 without a
  progression row). A stored value outside the bounds fails closed with `Unavailable(InvalidStoredState)`.
  H-live passes it to `MonkState::load`.
- **Death.** `commit_character_death` sets both values to 0 in the DEATH-1 transaction (the §8.2 cross-owner item).

## Validation

- `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`,
  `cargo test --locked -p oteryn-game-server --lib --bins` (1145 passed) and
  `python tools/agents/validate_governance.py` pass. The other integration targets were compiled by clippy but not
  run locally, because the session disk filled; CI runs them.
- On PostgreSQL 17.6 (pinned digest), `character_authority_postgres` (793 passed) and `durability_postgres` (770
  passed) pass. `character_authority_postgres` has two new cases: the migration gives
  existing rows 0 and 0 and the CHECKs reject out-of-range values; and the actor-end save round trip, replay,
  conflict, reconcile, unchanged save, stale connection, lease, session and revision fences, a superseded session
  generation, the death reset, the pending-respawn refusal, the guard rejecting a Harmony change without a monk
  receipt, and the fail-closed load of planted out-of-range values.

## Closeout

Archived in the PR's final authoring commit. The PR number, freeze SHA and review state are in the PR and in #162.
