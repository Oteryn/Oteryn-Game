# OTV2-20261003-prem-1c-harden

```yaml
task_id: OTV2-20261003-prem-1c-harden
title: "PREM-1c-HARDEN: the three deferred PREM-1b P2s"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: work/prem-1c-harden-20261003-b
issue: 162
pr: 1690
base_sha: 88d63a18
head_sha: "exact frozen head in the FREEZE_SHA entry"
final_head_sha: "exact frozen head in the FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code task worker (hard), control plane session_013KJX6mv8LQveCKKXYgAX94 (#1622)
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
lane_id: premium
leases: []
owned_paths:
  - apps/game-server/src/premium/
  - apps/game-server/tests/support/premium_fence_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20261003-prem-1c-harden.md
public_contracts: []
depends_on: ["PREM-1b (#1678, merged)"]
cross_repository_coordination_id: OTV2-PREMIUM-DELIVERY
external_repositories: []
```

## Outcome

- **4174554621.** A 429/503 `Retry-After` is honoured as delay-seconds or as an IMF-fixdate
  (time until it, zero once passed), still capped by the backoff cap (`client::parse_retry_after`).
- **4174554622.** `PremiumClientConfig::from_env` (via `from_vars`) is `Ok(None)` only with none
  of the three variables set; a partial set or an unreadable PEM file is `Err(Invalid(var))`.
- **4174554626.** The refresher's schedules are owned only by the refresher handles; dropping the
  last handle aborts every task, which holds only the consumer, root and client.

No migration taken.

## Validation

- `cargo fmt --all --check`, `cargo clippy --offline -p oteryn-game-server --all-targets -- -D
  warnings`: pass.
- `--lib premium` 24 pass; `--test premium_snapshot_client` 8 pass;
  `character_authority_postgres premium` 26 pass (local PostgreSQL 17.11, harness 17.6 gate
  relaxed locally only, not committed; the canonical 17.6 lane is CI).

## Closeout

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: work/prem-1c-harden-20261003-b
owner_action_required: null
blocker: null
next_action: "control plane: exact-head validation and review"
```
