> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Phase 1 PR #748 protected-integrated as `86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1`. Phase 2 PR #759 protected-integrated through governed Merge Queue as `19c15ebf8377608db5b72cf7082dc856c4fd76be`. Real merge-group run `35833113476` finished with aggregate `game-gate=SUCCESS`; protected-main Agent Governance, CodeQL and Rust push runs also finished SUCCESS. Any earlier nonterminal checkpoint below is historical provenance only.

# OTV2-20260922-control-plane-simplification-745

```yaml
task_id: OTV2-20260922-control-plane-simplification-745
title: Simplify candidate qualification and coordinator authority
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/control-plane-simplification-745-phase2
issue: 745
pr: 759
base_sha: fcf4be6384e6d590094060ea73078cbbdfe70e5c
head_sha: 7710316fc4f66f04c63272bc6197d3c72b2f8ed8
final_head_sha: 7710316fc4f66f04c63272bc6197d3c72b2f8ed8
final_head_frozen_at: null
owner: Oteryn: work coordinator
created_at: 2026-09-22T09:43:00+02:00
updated_at: 2026-09-23T10:09:00+02:00
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_pr_routing_contract.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20260922-control-plane-simplification-745.md
public_contracts:
  - Game candidate qualification and coordinator lifecycle
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The control-plane simplification is protected-integrated without weakening exact-head checks, protected-main health, required `game-gate`, Merge Queue or real `merge_group` qualification.

## Completed safety changes

- PR live-task validation is candidate-scoped while protected-main/non-PR health still validates the complete active set.
- `OTV2_IMPLEMENTATION_COORDINATOR` is retired; `Oteryn: work coordinator` remains the sole reusable mutating Game control plane.
- inherited routing snapshot drift is advisory only for a candidate that changed no audited routing inputs;
- candidate-caused routing drift remains conservative/FULL;
- trusted protected-base routing snapshot drift remains fail-closed;
- protected-main routing snapshot drift remains fail-closed;
- exact-head PR qualification runs `git diff --check` before Merge Queue;
- repository-policy pins and mutation regressions protect the new trusted-base and whitespace checks.

## Qualification and integration evidence

- Phase 1: PR #748 -> protected `86e25ab6c830159d9cb32aee1c3c8ff7726cdcd1`.
- Phase 2 final candidate: `7710316fc4f66f04c63272bc6197d3c72b2f8ed8`.
- Exact-head Agent Governance: SUCCESS.
- Exact-head Architecture Semantic Audit: SUCCESS.
- Exact-head Merge Gate / `game-gate`: SUCCESS.
- Independent Codex re-review on `7710316fc4`: no remaining major issue.
- Governed Merge Queue accepted the exact head.
- Real merge-group run `35833113476`: all lanes SUCCESS including PostgreSQL, Linux, Windows, supply-chain and aggregate `game-gate`.
- Protected integration: `main@19c15ebf8377608db5b72cf7082dc856c4fd76be`.
- Post-merge protected-main runs: Agent Governance `35833838674` SUCCESS, CodeQL `35833838770` SUCCESS, Rust workspace `35833838864` SUCCESS.

## Closeout

No ruleset weakening, direct merge, generic auto-merge, force, reset, rebase, product/runtime/protocol/persistence mutation or production mutation was used. Writer custody is released by moving this packet out of `tasks/active`.

```yaml
last_progress: Phase 2 protected-integrated through real Merge Queue and protected-main readback is fully green
status: completed
branch: ci/control-plane-simplification-745-phase2
head_sha: 7710316fc4f66f04c63272bc6197d3c72b2f8ed8
pr: 759
final_head_sha: 7710316fc4f66f04c63272bc6197d3c72b2f8ed8
final_head_frozen_at: null
ci_trigger_source: merge_group
ci_check_generation: final
ci_checks_for_current_head: 3
ci_run_ids:
  - 35832201901
  - 35832201772
  - 35832201630
  - 35833113476
runner_assignment_state: terminal
terminal_ci_checks_for_current_generation: 4
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: none
```
