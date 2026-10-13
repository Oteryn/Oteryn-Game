# PVP-RT-1

```yaml
task_id: PVP-RT-1
title: "PVP-RT-1: PvP legality, combat lock and PVP-1 state tables"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/pvp-rt-1-20261013
issue: 1622
governing_issue: 1622
pr: 2032
migration_lease: "0081"   # apps/game-server/migrations/0081_pvp_state.sql; other packets take 0082 or later
depends_on:
  - "PVP-1 (tables): taken by this task as its second PR, CP D(2026-10-13); the first PR #2029 merged"
  - "PR 2018 (landing/teleport): the landing hook is CombatLock::landing_allowed, not yet called"
  - "BUNDLE-COMBAT-ENABLE-1 (another worker): live wiring waits for its qualified_room gate"
owned_paths:
  - apps/game-server/src/combat/pvp/
  - apps/game-server/src/lib.rs
  - apps/game-server/src/gameplay_transport/field_step_ingress.rs
  - apps/game-server/migrations/0081_pvp_state.sql
  - apps/game-server/src/durability/pvp_state.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/support/pvp_state_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - docs/agents/tasks/archive/PVP-RT-1.md
```

## Outcome

Governing contracts: PARTY-PVP-0 §7-§8 (accepted); PVP-0 candidate §6.3, §6.4, §6.6 (D971 answers 1a 2a 3a 4a).

- `combat/pvp/legality.rs`: rules 1-7 in order with 3b (no-PvP tile, fail closed) and 6b (expert mode); damage factor 50% / 100% on black skull.
- `combat/pvp/lock.rs`: logout, PZ and kill blocks that only rise; `restore`; `landing_allowed`; aggression relations (cap 64, first / retaliation).
- PVP-1 (PR #2032): migration 0081 with `game_character_pvp_state`, `game_character_pvp_ledger`, `game_character_unjustified_points` and `game_character_revenge_marks` (World and cap guards, deadlines only rise, no CharacterRevision advance); `durability/pvp_state.rs` with the session-fenced deadline write-ahead (database clock + 70 s, at most two targets in CharacterId order), the admission read as remaining time, and the bounded World cleanup (not yet called by a node job).
- Not wired: PvP stays off until the PVP-0 §8 go-live gate; `attackable_kind` stays creature-only.

## Remaining scope

Closed with this record by CP D(2026-10-13): PVP-RT-1 ends with PR #2029 (pure half) and PR #2032 (PVP-1 tables, write-ahead, admission read). The runtime wiring continues as PVP-RT-1b under its own record: ledger writer and snapshot restore, kill classification, Join Aggression, Adventurer forfeiture, runtime write-ahead wiring and the World job caller, the `resolve_landing`/`bundle_step` hook (after #2018) and the live legality stage (after BUNDLE-COMBAT-ENABLE-1). The go-live gate must not treat PvP as landed on these PRs alone.

## Not in this slice

Death-transaction skull, points and marks (PVP-DEATH-1), ledger writer, kill classification, skull rendering, Join Aggression, Adventurer forfeiture, wire. The `NoPvp` `pvp_zone` field exception is removed (PVP-0 §6.4): `field_step_ingress.rs` now skips a NoPvP-World field hit on a player regardless of `pvp_zone`.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server --lib` (pvp and field_step filters): pass.
- `cargo test --locked -p oteryn-game-server --test character_authority_postgres pvp_state_postgres_cases`: pass (PostgreSQL 17.11 locally, canonical 17.6 in CI).
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.

## Review

Codex rounds on #2032: the World cleanup takes the scope authority and checks it against its transaction; points and marks are written by the death transaction of their victim (receipt created in the same transaction, recipient in the death World, anchors stamped by the database, points immutable, marks are insert-only here); a revenge mark expires exactly seven days after creation. Each round fixed in one commit with Postgres cases.

Round 6: rows are protected by DELETE guards (points >45 days, marks past expires_at, ledger rows older than 5 minutes or removed in the victim's death transaction); a point/mark recipient must appear in the victim's ledger within the 5-minute window. Revenge-mark CLOSURE (avenge) is cut from PVP-1 and belongs to PVP-DEATH-1.
