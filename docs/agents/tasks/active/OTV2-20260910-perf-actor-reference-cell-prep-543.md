# OTV2-20260910-perf-actor-reference-cell-prep-543

```yaml
task_id: OTV2-20260910-perf-actor-reference-cell-prep-543
title: Prepare fail-closed PERF-01 actor reference-cell evidence processor
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
issue: 543
base_branch: main
branch: agent/perf-actor-reference-cell-prep-543
pr: null
base_sha: d2367d7088727d3c1d61ab77ea2df7fc01ed1050
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex
created_at: 2026-09-10T00:00:00Z
updated_at: 2026-09-10T12:00:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/perf-actor-reference-cell/**
  - docs/agents/evidence/OTV2-20260910-perf-actor-reference-cell-prep.schema.json
  - docs/agents/evidence/OTV2-20260910-perf-actor-reference-cell-placeholder.json
  - docs/agents/evidence/OTV2-20260910-perf-actor-reference-cell-prep.md
  - docs/agents/tasks/active/OTV2-20260910-perf-actor-reference-cell-prep-543.md
public_contracts: []
depends_on: [PERF-01-ACTOR-CARRIER-CELL-V1]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Prepare a deterministic standalone evidence processor that cannot turn missing,
synthetic, drifted, inconclusive or partially enforced measurements into a
capacity claim.

## Architecture and source of truth

- **PROVEN:** protected `main@d2367d7088727d3c1d61ab77ea2df7fc01ed1050`
  contains the allocation and `PERF-01-ACTOR-CARRIER-CELL-V1` method contract.
- **PROVEN:** the allocation permits only the paths listed above and grants no
  runtime, workflow, registry, production or architecture authority.
- **UNKNOWN:** physical reference-cell fingerprint, `S`, `P`, and provisional
  `M`; this task does not measure or select them.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this package processes non-production evidence and performs no
authority-bearing mutation, persistence recovery, or PREPARE/COMMIT operation.

## Acceptance criteria

- [x] Placeholder/unbound input never yields capacity.
- [x] Contract inputs, fingerprint, enforcement and identities fail closed.
- [x] Retained samples deterministically recompute nearest-rank percentiles.
- [x] Progressive repetitions, reproducible saturation and soaked `P` are required.
- [x] Checked candidate arithmetic applies saturation, qualified-pass and all lower caps.
- [x] Atomic exact-`M`/`M+1` evidence is mandatory.
- [x] Synthetic/correctness/AI01 sources are rejected and broader PERF-01 remains open.
- [x] Canonical JSON/check mode and focused tests cover ordering, drift, overflow and boundaries.

## Excluded scope

No benchmark execution, Synology access, workflow or runner mutation, runtime,
apps, crates, Cargo, registry, architecture, Movement, Ability, AI, Combat,
production, external repository, capacity acceptance, or RL-01 closeout.

## Implementation / findings

Implemented a stdlib Python processor, unit suite, strict evidence schema,
unbound placeholder and preparation evidence record. No numeric physical result
is included.

## Validation

### Focused

- command/run: `python3 -m unittest discover -s tools/perf-actor-reference-cell -p 'test_*.py'`
- result: PASS (15 tests)

### Component/integration

- command/run: JSON parse and Python compile checks
- result: PASS

### E2E

- scenario: placeholder write then byte-stable `--check`; complete synthetic unit fixture remains test-only
- result: PASS; no physical benchmark was invoked

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, whole effective diff
- material findings: strengthened explicit workload/enforcement inputs and scalar observation validation during review; no unresolved material finding
- verdict: PREP_HARNESS_READY_FOR_REVIEW

## Independent review

- required: YES; evidence logic implements a protected performance method
- exact head: pending
- method/auditor: repository PR review
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: PASS; exactly seven files under allocated paths
- unresolved review threads: pending
- related/superseded PRs: original preparation request only
- protected auto-merge: forbidden for this worker
- merge commit/result: NOT_APPLICABLE
- ownership release: after PR handoff

## Context checkpoint

```yaml
last_progress: focused checks and whole-diff self-review passed
status: ready
branch: agent/perf-actor-reference-cell-prep-543
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: not_requested
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish the dedicated branch and open one review PR
```
