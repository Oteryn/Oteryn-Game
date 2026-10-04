# PREM-RELEASE-ABANDONED-1

```yaml
task_id: PREM-RELEASE-ABANDONED-1
title: Abandoned terminal release stops the account's Premium pulls (D476)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prem-release-abandoned-1
pr: 1722
base_sha: 53a60a6
owner: implementation worker for the CP (#1622), D476
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/agents/tasks/archive/PREM-RELEASE-ABANDONED-1.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

`release_terminal` now calls `premium.release` for every terminal release, abandoned or capability mismatch, but only on a proven durable `Released` or `Terminal` outcome (helper `ends_session`). An unended or unknown outcome keeps the pulls running. Origin: Codex review 5402996836 on #1534 (D438).

Codex P1 4176449967 (round 1): the refresher is keyed by account, so a late Terminal outcome of a retried release could cancel the pulls of a same-account successor session. Fix: `PremiumSessions` records which sessions hold each account's registration, and admission and release are decided under one lock. A release stops the pulls only when its session held a registration and was the account's last. Grace expiry, the abandoned release and the capability-mismatch release all go through `release_premium`, so all three are covered.

## Excluded scope

Grace expiry, the capability-mismatch settle after its retries run out, `PremiumRefresher`, and the areas #1534 edits are all unchanged.

## Validation

- `cargo fmt --check`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-game-server --lib gameplay_transport`: pass (113), including `only_a_proven_terminal_release_stops_premium` and `a_late_release_keeps_a_successor_session_premium`
- `cargo test -p oteryn-game-server --lib premium`: pass (25)
- Postgres premium fence cases: not run. `PremiumRefresher` is unchanged, and CI runs them.
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- Review: Codex round 1 P1 4176449967 fixed; round 2 pending
