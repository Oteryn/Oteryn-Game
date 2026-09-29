# OTV2-20260925-serve-withdrawal-budget

```yaml
task_id: OTV2-20260925-serve-withdrawal-budget
title: Clamp readiness-withdrawal retry sleeps to the shutdown budget
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zen-dirac-mxruho
issue: 885
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude
created_at: 2026-09-25T00:00:00Z
updated_at: 2026-09-25T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/node/serve.rs
  - docs/agents/tasks/active/OTV2-20260925-serve-withdrawal-budget.md
public_contracts:
  - OPS-NODE-BOOT-01
depends_on:
  - pr:878
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

- **Governing decision:** `OPS-NODE-BOOT-01` (#830), D3 step 10. This is a follow-up from the #878 review (#885).
- **Scope:** the budgeted readiness-withdrawal retries (the guard-chain read and the publication replay) never sleep past the shutdown deadline, so `ready=false` publication returns by the deadline even while the durability holder stays unavailable. Unbudgeted boot retries are unchanged.
- **Validation:** a unit test of the clamped delay, and a real-clock test that a 5 s backoff returns by a 300 ms deadline. Both fail without the clamp.
- **Excluded:** every other shutdown or boot behavior.
