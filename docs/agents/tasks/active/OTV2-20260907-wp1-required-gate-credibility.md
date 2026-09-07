# OTV2-20260907-wp1-required-gate-credibility

```yaml
task_id: OTV2-20260907-wp1-required-gate-credibility
title: Repair required PR/MQ gate credibility for F01/F02
mode: REPAIR
status: TDD_RED_PENDING
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/wp1-required-gate-credibility-364
issue: 364
parent_coordinator_issue: 162
ci_programme_issue: 308
authority_comment: 5574487313
admission_main_sha: 1b41d485cc4bf126d2a9e5fe9717cc8530ece3d5
owner: Oteryn: work coordinator
created_at: 2026-09-07T18:42:00Z
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-group-gate.yml
  - tools/repository/test_validate_merge_group_pg_sim.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20260907-wp1-required-gate-credibility.md
  - docs/superpowers/plans/2026-09-07-wp1-required-gate-credibility.md
conditional_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/test_validate_pr_gate_pg_sim.py
protected_pin_path: .github/workflows/merge-authority-audit.yml
protected_pin_custody: separate_coordinator_rotation_only_not_owned_by_this_candidate
public_contracts: []
external_repositories: []
blocks: []
```

## Authority and exact purpose

This is the single serialized WP1/F01+F02 lane granted by #308 comment
`5574487313` after fresh reconciliation of the old #262/#150 proposals. It is
not another CI optimization programme and does not cancel repository protection.

The repair may make only the existing required PR/MQ evidence truthful:

1. every native command in the affected Merge Queue Windows multi-command
   PowerShell block must fail the job immediately on a nonzero exit;
2. the real governance lifecycle regression suite must execute inside the
   ordinary PR and merge-group paths feeding the required `game-gate`.

Current inspection shows the ordinary PR Windows commands are already separate
steps and `merge-gate.yml` already executes
`test_validate_pr_gate_pg_sim.py`, which imports the queue regression. Therefore
`merge-gate.yml` and `test_validate_pr_gate_pg_sim.py` are conditional only and
must remain unchanged unless later evidence demonstrates an actual need.

## TDD sequence

### RED

Before any workflow change, strengthen only the existing queue regression so
current protected `merge-group-gate.yml` fails because it lacks:
- explicit fail-closed PowerShell native-command policy; and
- execution of the real lifecycle regression command from the required MQ
  candidate/governance path.

Publish that test-only RED and preserve the exact hosted failure. A metadata,
syntax or unrelated policy failure is not accepted as RED.

### GREEN

After RED is durable:
- add the lifecycle regression command to the existing MQ candidate/governance
  layer without changing its job identity, permissions or fan-in;
- add fail-closed PowerShell semantics before the four existing native Windows
  commands without deleting, replacing or weakening any command;
- exercise positive controls, an injected failure at every native command
  position, and the actual lifecycle positive/injected-negative suite;
- compute and bind the exact future Git blob of `merge-group-gate.yml` in the
  repository-policy and queue regression.

The gate activation candidate is not allowed to approve its own new protected
blob. `merge-authority-audit.yml` remains outside this candidate. The future gate
blob must first receive a separate protected-base pin-rotation allocation,
independent exact-head review, normal protected integration and readback.

## Invariants and exclusions

Preserve ruleset 20991995, sole required `game-gate`, FULL native Merge Queue,
all existing jobs/tests, exact merge-group identity, dependency review, CodeQL,
Linux, PostgreSQL 17.6, Windows, simulation and supply-chain qualification.
No `continue-on-error`, paths-only bypass, workflow/status renaming, permission
expansion, direct merge, candidate self-approval, ruleset change or queue semantic
change.

No runtime/product/Cargo/registry/Foundation/Durability/Server Seam/production,
credential or external-repository authority. WP2/#361, WP3/#356, WP4/#335 and
#247 remain unreleased.

## Acceptance

- visible test-only RED on the exact pre-workflow head;
- GREEN tests prove all four native command positions fail closed and valid
  commands still pass;
- real lifecycle discovery/injected-failure tests execute in both required PR
  and MQ aggregate paths;
- exact future gate blob receives separate protected pin authorization before
  gate activation;
- one independent exact-head deep review on each material protected control
  candidate;
- canonical PR checks, FULL Merge Queue, protected readback;
- terminal archive releases this special write authority.

Runtime E2E is not a substitute for these control-plane negative controls. WP1
closes only when the required aggregate itself cannot accept the specified false
greens.
