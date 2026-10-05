# PREM-E2E-1

```yaml
task_id: PREM-E2E-1
title: "PREM-E2E-1 Premium end to end: Game client matches PREM-P, cross-repository fixture test, candidate activation record"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prem-e2e-1
pr: #1774
base_sha: 51a040a3
owner: hard implementation worker for the CP (#1622), owner D540
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
lane_id: premium
owned_paths:
  - apps/game-server/src/premium/
  - apps/game-server/tests/premium_platform_fixtures.rs
  - apps/game-server/tests/premium_snapshot_client.rs
  - apps/game-server/tests/support/premium_fence_postgres_cases.rs   # test-data literals only; granted by CP D544 (option (a))
  - tests/fixtures/premium-snapshot-v1/
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_DELIVERY0_PREMIUM_EVIDENCE_TRANSPORT_DECISION_2026-09-30.md
  - docs/architecture/reviews/OTERYN_GAME_PREMIUM_ACTIVATION_RECORD_CANDIDATE_2026-10-04.md
  - docs/agents/tasks/archive/PREM-E2E-1.md
public_contracts: []
cross_repository_coordination_id: OTV2-PREMIUM-DELIVERY
external_repositories: ["Oteryn/Oteryn-Platform (read-only, commit 71bbe6c5cffc29d430195fa286b3c6941af4abb8)"]
depends_on: ["PREM-1b (#1678)", "Platform PREM-P (#1432, #1433)"]
blocks: ["Premium production activation (owner authority)"]
```

## Outcome

- The client calls PREM-P's path `/internal/v1/products-entitlements/premium-snapshots/read` and
  requires TLS 1.3.
- `snapshot::validate` enforces Platform's v1 rules after the compatibility comparison:
  identifier forms, revision bounds, whole-second timestamps, and the cutoff, refresh and state
  arithmetic. A body that breaks one is `Malformed`.
- Kept evidence is `StaleWithinBound` (denied) from `refresh_after`. The refresher pulls 5
  minutes before it, never sooner than 60 seconds after the last success.
- The test producer and the test bodies use contract-conformant identifiers.
- Platform's fixtures are vendored with pinned hashes. `premium_platform_fixtures.rs` and
  `platform_fixtures_classify_as_the_contract_requires` are the Game half of the
  cross-repository end-to-end test.
- PREMIUM-DELIVERY-0 §12 records every difference from PREM-P. The candidate activation record
  carries the §10.3 and §6.6 fields.

## Excluded scope

- No Platform write.
- No activation: no `PremiumActivation`, no switch-over date, no production configuration,
  certificates or secrets.
- No migration, and no change to the durable fence.
- The real-endpoint run, PREM-P going live and Platform #1431 closeout stay open (activation
  record §4).

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib premium`: pass (29)
- `cargo test --locked -p oteryn-game-server --test premium_platform_fixtures`: pass (6)
- `cargo test --locked -p oteryn-game-server --test premium_snapshot_client`: pass (8)
- `cargo test --locked -p oteryn-game-server --test character_authority_postgres premium_` on PostgreSQL 17.6: pass (11)
- `cargo test --locked -p oteryn-game-server --quiet` on PostgreSQL 17.6: exit 0
- `git diff --check`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- Review: pending (the CP requests it)
