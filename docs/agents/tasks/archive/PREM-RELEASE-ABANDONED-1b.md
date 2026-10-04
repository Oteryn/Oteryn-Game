# PREM-RELEASE-ABANDONED-1b

```yaml
task_id: PREM-RELEASE-ABANDONED-1b
title: Reconciled TERMINAL settle stops the session's Premium pulls (#1722 Codex P2 4176492358)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prem-release-abandoned-1b
pr: 1729
base_sha: 6f77383
owner: implementation worker for the CP (#1622), D476
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/gameplay_transport/mod.rs
  - docs/agents/tasks/archive/PREM-RELEASE-ABANDONED-1b.md
public_contracts: []
depends_on: [PREM-RELEASE-ABANDONED-1]
blocks: []
```

## Outcome

Every reconciled `UnendedSettle::Terminal` settle now goes through `retire_reconciled`, which releases the session-scoped Premium registration and then retires the session. That covers grace expiry (NotApplicable and NotExpired), the abandoned and mismatch releases inside the retry loop, and the mismatch settle after the retries run out.

## Excluded scope

Proven `Released`/`Terminal` outcomes, `PremiumSessions` and `PremiumRefresher` are unchanged.

## Validation

- `cargo fmt --check`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-game-server --lib gameplay_transport`: pass (119), including `a_reconciled_terminal_release_stops_premium_once`
- No transport-level harness runs without Postgres; the retire paths are covered through `PremiumSessions` and are checked by CI.
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- Review: pending (CP requests it)
