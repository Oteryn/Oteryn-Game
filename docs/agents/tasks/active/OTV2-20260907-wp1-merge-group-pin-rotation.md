# OTV2-20260907-wp1-merge-group-pin-rotation

```yaml
task_id: OTV2-20260907-wp1-merge-group-pin-rotation
title: Preapprove exact WP1 Merge Queue credibility gate blob
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: governance/wp1-merge-group-pin-364
issue: 364
parent_coordinator_issue: 162
ci_programme_issue: 308
authority_comment: 5574626339
admission_main_sha: 1b41d485cc4bf126d2a9e5fe9717cc8530ece3d5
owner: Oteryn: work coordinator
created_at: 2026-09-07T18:48:00Z
owned_paths:
  - .github/workflows/merge-authority-audit.yml
  - docs/agents/tasks/active/OTV2-20260907-wp1-merge-group-pin-rotation.md
future_gate_blob: 539a726b7d39cabe785892f70ea30d1944189d91
current_gate_blob: e3291fe8fca8fc70166d5652b43d5a26fa0d762
external_repositories: []
blocks: []
```

## Purpose

Rotate protected merge-authority approval to exactly one precomputed future
`merge-group-gate.yml` blob produced by the WP1 RED→GREEN lane. This task does
not modify or activate the gate itself.

The future blob `539a726b7d39cabe785892f70ea30d1944189d91` differs only by the bounded
#308 comment 5574487313 credibility repair: direct lifecycle-discovery and queue
regression execution in the existing candidate/governance path, plus fail-closed
PowerShell native-command semantics before the existing four Windows commands.
All existing jobs, permissions and `game-gate` fan-in remain.

## Safety boundary

The audit remains protected-base `pull_request_target`, reads candidate bytes as
inert data, rejects candidate-controlled audit self-modification, requires
same-repository exact-head/main-base identity, exact one-blob equality, pinned
actions and forbids writes, `continue-on-error`, dispatch and status bypasses.
Literal future `${{ ... }}` expressions continue to be assembled inside Python
rather than evaluated by the audit workflow.

This pin candidate is expected to make the existing protected-base audit reject
its own self-modification. That is intentional safety evidence and must not be
renamed PASS. Integration requires the explicit coordinator authorization above,
one independent exact-head deep review, canonical required `game-gate`, normal
protected integration/readback and no unresolved material findings.

After protected readback, archive this task and release its audit-path ownership.
Only then may unchanged WP1 gate PR #396 proceed to final protected activation.
No ruleset, required-status, MQ settings, product/runtime/Cargo/registry,
production or external-repository authority is granted.
