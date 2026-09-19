# OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment

```yaml
task_id: OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment
title: WP3-v2 P1-A maintenance-task layout architecture amendment
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/wp3-v2-tokio-maintenance-task-layout-amendment-351
pr: 681
base_sha: 03a821edd828e24ccff6e2cb7fc819a776cbd238
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: astra wp3-v2 architecture lead"
created_at: 2026-09-19
updated_at: 2026-09-19
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_V2_SUPERSEDING_ARCHITECTURE_DECISION_2026-09-12.md
  - docs/agents/programs/OTV2_WP3_A_UPSTREAM_FIRST_ACCEPTANCE_ALLOCATION_20260916.md
  - docs/agents/tasks/active/OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment.md
public_contracts: []
depends_on:
  - "#162 comment 5745227522"
blocks:
  - "PR #673 P1-A successor implementation lease"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Encode the superseding minimum-sufficient P1-A owner decision so PR #673 can later
receive one bounded implementation lease without inventing a task-size constant,
second budget or generic Tokio resource-accounting subsystem.

This task is documentation-only. It grants no source/vendor/Cargo/runtime mutation
authority and does not resume PR #673 by itself.

## Architecture and source of truth

- **PROVEN** — #162 comment `5745227522` supersedes layout-only release
  `5744804452` and closes the two source-proof gaps reported by the architecture
  worker in `5744812736`.
- **PROVEN** — protected base at release:
  `main@03a821edd828e24ccff6e2cb7fc819a776cbd238`.
- **PROVEN** — predecessor architecture blob:
  `2004dd41537e1c11e23090e4d029b74de065b376`.
- **PROVEN** — predecessor upstream-first allocation blob:
  `02fa3c9c06d10e136719368280aef5b221a9dd6d`.
- **PROVEN** — the accepted root budget remains
  `DFR-TOTAL-RESIDENT-BYTES = 12,582,912` with
  `I + max(R,T) + Q + A <= 12 MiB`; no registry change is authorized.
- **PROVEN** — PR #673 remains paused until this amendment is protected-integrated,
  read back and followed by a fresh coordinator lease.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: "Documentation-only architecture amendment; no production mutation, PREPARE/COMMIT authority, controller installation, recovery write or live-state mutation is performed."
```

## Acceptance criteria

- [x] Production WP3 durability root is fixed to Tokio
      `RuntimeFlavor::MultiThread`; wrong/unavailable flavor fails closed before
      root/pool acceptance.
- [x] Current-thread runtime is explicitly test-only/non-production and cannot
      qualify the production root.
- [x] Exactly one pinned Tokio 1.53.1 read-only representation query is admitted
      for the SQLx root-maintenance future's optional boxed-future and exact
      MultiThread task-cell allocation requests.
- [x] The query grants no allowance, allocator interception, owner propagation,
      generic scheduler/task accounting or lifetime policy.
- [x] One root-specific SQLx 0.9.0 maintenance construction is admitted to remove
      the heap-allocating `CloseEvent/EventListener` dependency while preserving
      the silent 10m/30m reaper and max1/min0 semantics.
- [x] All admitted source-derived maintenance backing remains charged to the same
      root `I` ledger from pre-allocation reservation through complete
      task/shared-tail finality.
- [x] No new numeric limit, second budget or resource-registry change is introduced.
- [ ] Exact branch readback contains only the three allocated paths.
- [ ] Agent Governance exact-head SUCCESS.
- [ ] Architecture Semantic Audit exact-head SUCCESS.
- [ ] FULL Merge Gate / aggregate `game-gate` exact-head SUCCESS.
- [ ] Required independent HIGH whole-diff architecture/resource/security review
      completes on the stable exact head with no material blocker.

## Excluded scope

No implementation code, Cargo/lockfile, dependency source/vendor mutation, resource
registry, workflows/rulesets/protection, production/live data, rustls widening,
WP4/WP5/Server-Seam, `apps/game-server/src/durability/fresh_admission.rs`, direct
merge, enqueue or PR #673 implementation mutation.

The successor implementation surface remains prospective and coordinator-only after
protected amendment integration/readback.

## Implementation / findings

The earlier layout-only direction was insufficient for two independent reasons:

1. SQLx `spawn_maintenance_tasks` creates `CloseEvent/EventListener` heap backing
   before Tokio `Handle::spawn`.
2. Tokio 1.53.1 current-thread scheduling may allocate queue backing after the
   initial task allocation.

The superseding decision closes both without a generic scheduler fork by fixing the
production root to MultiThread and removing the root-maintenance
`CloseEvent/EventListener` dependency while retaining only the exact read-only
task-allocation representation query.

## Validation

### Focused

- command/run: repository-native exact branch/blob/path readback
- result: pending final exact head

### Component/integration

- command/run: repository exact-head Agent Governance + Architecture Semantic Audit
- result: pending

### E2E

- scenario: FULL repository Merge Gate / aggregate `game-gate`
- result: pending

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- runner assignment: repository-native CI
- classification: required
- result: pending

## Self-review

- exact head: pending
- method/reviewer: `Oteryn: astra wp3-v2 architecture lead`
- material findings: pending
- verdict: pending

## Independent review

- required: YES — #162 comment `5745227522`
- exact head: pending
- method/auditor: independent HIGH whole-diff architecture/resource/security review
- trigger owner: active control plane under standing required-review authorization
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: PR #673 remains parked; this amendment supersedes only the insufficient layout-only P1-A formulation
- protected auto-merge: forbidden to this worker
- merge commit/result: coordinator-only
- ownership release: after protected integration/readback or explicit coordinator handoff

## Context checkpoint

```yaml
last_progress: "PR #681 opened; exact three-path pre-freeze readback clean; exact-head qualification in progress"
status: validating
branch: agent/wp3-v2-tokio-maintenance-task-layout-amendment-351
head_sha: null
pr: 681
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: repository_native
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: "qualify final head with exact three-path readback and repository CI; then hand the independent-review packet to the active control plane"
```
