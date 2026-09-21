> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #681 merged as `b13548d1f18507ec9d99f09fb0b6383f4618c9cc`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment

```yaml
task_id: OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment
title: WP3-v2 P1-A finite dedicated-runtime architecture amendment
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
updated_at: 2026-09-20
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WP3_V2_SUPERSEDING_ARCHITECTURE_DECISION_2026-09-12.md
  - docs/agents/programs/OTV2_WP3_A_UPSTREAM_FIRST_ACCEPTANCE_ALLOCATION_20260916.md
  - docs/agents/tasks/active/OTV2-20260919-wp3-v2-tokio-maintenance-task-layout-amendment.md
public_contracts: []
depends_on:
  - "#162 comment 5745227522"
  - "#162 comment 5748461271"
  - "#162 comment 5748332099"
blocks:
  - "PR #673 P1-A successor implementation lease"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Encode the final minimum-sufficient P1-A owner decision so PR #673 can later
receive one bounded implementation lease with an exact finite WP3-owned Tokio
runtime topology and complete same-root runtime/maintenance accounting, without
inventing byte constants, a second budget or generic Tokio resource-accounting
subsystem.

This task is documentation-only. It grants no source/vendor/Cargo/runtime mutation
authority and does not resume PR #673 by itself.

## Architecture and source of truth

- **PROVEN** — #162 comment `5745227522` supersedes layout-only release
  `5744804452` and closes the two source-proof gaps reported by the architecture
  worker in `5744812736`.
- **PROVEN** — independent HIGH review P1 `4054801878` proved that MultiThread
  flavor alone does not bound worker topology/backing.
- **PROVEN** — architecture qualification `5748151621` found no protected finite
  server topology and returned `OWNER_DECISION_REQUIRED`.
- **PROVEN** — owner-authorized host evidence `5748332099` classified the explicit
  bounded builder as `BOUNDED_TOPOLOGY_EMPIRICALLY_STABLE` while preserving the
  architecture decision as open.
- **PROVEN** — owner decision `5748461271` accepts
  `WP3_DEDICATED_BOUNDED_TOKIO_RUNTIME/v1`: Tokio 1.53.1 MultiThread,
  `worker_threads=1`, `max_blocking_threads=1`, `thread_stack_size=2 MiB`,
  dedicated WP3 ownership, no second budget and no DFR maximum increase.
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

- [x] Production WP3 durability root owns one dedicated Tokio `1.53.1`
      `RuntimeFlavor::MultiThread` runtime.
- [x] Exact finite topology is frozen to `worker_threads=1`,
      `max_blocking_threads=1`, `thread_stack_size=2 MiB`.
- [x] Arbitrary ambient Tokio `Handle` qualification is forbidden for the
      production root; current-thread remains test-only/non-production.
- [x] The narrow pinned-Tokio read-only representation seam is extended only far
      enough to prove source-derived backing for the exact accepted dedicated
      runtime plus the exact SQLx root-maintenance future/task.
- [x] The seam grants no allowance, allocator interception, generic
      scheduler/task accounting, owner propagation or lifetime policy.
- [x] One root-specific SQLx 0.9.0 maintenance construction remains admitted to
      remove the heap-allocating `CloseEvent/EventListener` dependency while
      preserving the silent 10m/30m reaper and max1/min0 semantics.
- [x] Every attributable source-derived dedicated-runtime and maintenance charge
      remains in the same root `I` ledger from pre-allocation reservation through
      complete runtime/task/shared-tail finality.
- [x] `max_blocking_threads=1` is a finite cap only; implementation must prove
      blocking-worker reachability rather than assume presence or absence.
- [x] Virtual stack reservation and resident/committed DFR accounting remain
      distinct; host RSS evidence is not promoted into a resource maximum.
- [x] No new numeric limit, second budget or resource-registry increase is
      introduced.
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

The amendment now closes three successive P1-A gaps without widening to a generic
Tokio ownership subsystem:

1. SQLx `spawn_maintenance_tasks` created `CloseEvent/EventListener` heap backing
   before Tokio task allocation.
2. Tokio current-thread scheduling could grow queue backing after initial spawn.
3. MultiThread flavor alone still left worker count, thread stacks and associated
   scheduler/runtime backing host/config dependent.

Revision 5 closes the third gap through the explicit owner decision
`WP3_DEDICATED_BOUNDED_TOKIO_RUNTIME/v1`: one WP3-owned Tokio 1.53.1 MultiThread
runtime with one worker, one blocking-thread cap and an explicit 2 MiB thread stack.
The dedicated runtime replaces arbitrary ambient scheduler ownership for production
WP3 root work.

The host probe in `5748332099` is retained only as diagnostic support: it observed
16 ambient workers on the tested 16-logical-CPU host and one stable worker for the
explicit bounded builder. It does not supply final DFR byte accounting.

Final implementation still must source-derive the candidate's complete attributable
runtime/maintenance backing, prove blocking-worker reachability, reserve the charge
against the same root `I` ledger before allocation/root acceptance, and fail closed
if the frozen 12 MiB equation cannot be satisfied.

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
last_progress: "Owner accepted finite dedicated runtime topology; Revision-5 three-path amendment authored on canonical PR #681 branch; final exact-head qualification pending"
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
next_action: "freeze the final stable head, verify exact three-path diff/readback and hosted governance/architecture/Merge Gate; then hand one fresh independent-HIGH review packet to the active control plane"
```
