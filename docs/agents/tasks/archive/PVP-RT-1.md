# PVP-RT-1

```yaml
task_id: PVP-RT-1
title: "PVP-RT-1: PvP legality and combat lock (pure half)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/pvp-rt-1-20261013
issue: 1622
governing_issue: 1622
pr: 2029
depends_on:
  - "PVP-1 (tables; not landed): durable write-ahead and admission restore wait for it"
  - "PR 2018 (landing/teleport): the landing hook is CombatLock::landing_allowed, not yet called"
  - "BUNDLE-COMBAT-ENABLE-1 (another worker): live wiring waits for its qualified_room gate"
owned_paths:
  - apps/game-server/src/combat/pvp/
  - apps/game-server/src/lib.rs
  - apps/game-server/src/gameplay_transport/field_step_ingress.rs
  - docs/agents/tasks/active/PVP-RT-1.md
  - docs/agents/tasks/archive/PVP-RT-1.md
```

## Outcome

Governing contracts: PARTY-PVP-0 §7-§8 (accepted); PVP-0 candidate §6.3, §6.4, §6.6 (D971 answers 1a 2a 3a 4a).

- `combat/pvp/legality.rs`: rules 1-7 in order with 3b (no-PvP tile, fail closed) and 6b (expert mode); damage factor 50% / 100% on black skull.
- `combat/pvp/lock.rs`: logout, PZ and kill blocks that only rise; `restore`; `landing_allowed`; aggression relations (cap 64, first / retaliation).
- Not wired: PvP stays off until the PVP-0 §8 go-live gate; `attackable_kind` stays creature-only.

## Not in this slice

Durable write-ahead and restore (PVP-1 tables), damage ledger, kill classification, skull rendering, Join Aggression, Adventurer forfeiture, wire. The `NoPvp` `pvp_zone` field exception is removed (PVP-0 §6.4): `field_step_ingress.rs` now skips a NoPvP-World field hit on a player regardless of `pvp_zone`.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server --lib` (pvp and field_step filters): pass.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.

## Review

Codex P1 findings on bea2534 (exception removal, archive move) fixed in one commit.
