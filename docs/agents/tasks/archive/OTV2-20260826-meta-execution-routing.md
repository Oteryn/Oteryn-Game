# OTV2-20260826-meta-execution-routing

```yaml
task_id: OTV2-20260826-meta-execution-routing
title: Adopt merged META execution-routing policy
mode: GOVERNANCE
status: archived
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/meta-execution-routing-201
issue: 201
pr: null
base_sha: f31453f65477ae9966d724d67bdd2c1857318be1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: oteryn-governance-controller
created_at: 2026-08-26T16:55:00Z
updated_at: 2026-08-26T16:55:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - AGENTS.md
  - docs/agents/tasks/active/OTV2-20260826-meta-execution-routing.md
public_contracts:
  - Oteryn META execution-routing policy at 8fac1d55805fc3372351ea0a55ad7728b3570ebc
depends_on:
  - Oteryn/Oteryn PR #90 merged
blocks: []
cross_repository_coordination_id: Oteryn/Oteryn#90
external_repositories:
  - Oteryn/Oteryn
```

## Outcome

Game adopts the merged META execution-routing policy by reference without runtime, runner-host or deployment changes.

## Acceptance criteria

- [ ] Root instructions require CI/isolated-workspace first, RDC default-deny and fresh GitHub resume state.
- [ ] Substantial tasks require parallel-first lanes with exclusive worktree ownership.
- [ ] Governance validation and exact-head CI pass.

## Excluded scope

No Game runtime, Cargo, workflows, runner configuration, Desktop Commander session or production system change.

## Supersession

Issue #201 closed after delivery in `c72ef27` (#202). Issue #367 supersedes its local organization-policy copy with the central META v3 binding. This packet is historical and non-dispatchable.
