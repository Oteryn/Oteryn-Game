# OTV2-20260824-durability-topology-prep-allocation

```yaml
task_id: OTV2-20260824-durability-topology-prep-allocation
title: Allocate Issue #94 Durability topology preparation
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/durability-topology-prep-allocation-20260824
issue: 94
pr: null
base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260824-durability-topology-coordinator
created_at: 2026-08-24T21:00:00+02:00
updated_at: 2026-08-24T21:00:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260824-durability-topology-prep-allocation.md
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260824-prep-durability-topology.md
public_contracts:
  - docs/agents/prompts/OTV2_PREP_DURABILITY_TOPOLOGY.md
  - docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md
depends_on:
  - Oteryn-Game#94
blocks:
  - OTV2-20260824-prep-durability-topology
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Merge one docs-only coordinator allocation that grants the Issue #94 preparation worker authority to write only its decision packet and task record. It does not grant Durability implementation, SQL/DDL, migration, Cargo/workspace, registry, workflow, production database or external-repository authority.

## Architecture and source of truth

- `PROVEN`: exact allocation base is `main@22a3eb866dae19d048969edff1e1fa5012a429b6`.
- `PROVEN`: Issue #94 is open and explicitly requires a merged coordinator allocation before its preparation output may be written.
- `PROVEN`: current `docs/agents/tasks/active/` contains no active implementation coordinator task; the prior coordinator snapshot is archived.
- `PROVEN`: no GitHub branch matching Durability preparation existed at preflight.
- `PROVEN`: `main@22a3eb866dae19d048969edff1e1fa5012a429b6` contains the Content Format Spike allocation on disjoint paths; this Durability preparation allocation does not claim those paths or any shared lease.
- `PROVEN`: current implementation live allocation leaves `OTV2-IMPL-DURABILITY.write_authority: none`.
- `PROVEN`: `RESOURCE_LIMITS_REGISTRY.json` contains no DUR-03-owned entries; this allocation does not select or register numeric values.
- `DERIVED`: the safe next action is a path-bounded preparation allocation only. Any later implementation allocation remains separately blocked on the accepted Issue #94 packet and applicable DUR-03 hard-max closure.

## Acceptance criteria

- [ ] Allocation grants only `docs/architecture/reviews/OTERYN_GAME_DURABILITY_TOPOLOGY_DECISION_PACKET_2026-08-24.md` and `docs/agents/tasks/active/OTV2-20260824-prep-durability-topology.md` after merge.
- [ ] Worker branch is fixed as `docs/otv2-prep-durability-topology-94` from the exact allocation merge base.
- [ ] `OTV2-IMPL-DURABILITY` remains without runtime/DDL/Cargo/registry write authority.
- [ ] No shared serialized workspace lease is assigned by this preparation allocation.
- [x] `python tools/agents/validate_governance.py` passes.
- [x] `git diff --check` passes.
- [x] Mandatory whole-diff coordinator self-review passes; exact final head will be bound in immutable PR review evidence after PR metadata is recorded.
- [ ] Exact-head repository required checks including `game-gate` pass before squash merge.

## Excluded scope

No runtime code, SQL/DDL, migration artifacts, database provisioning, dependencies, Cargo/workspace files, resource registry, workflows, production configuration, secrets, Platform writes or external-repository writes. This task does not accept the topology packet and does not create `OTV2-IMPL-DURABILITY` implementation authority.

## Implementation / findings

The allocation deliberately separates preparation from implementation. The preparation worker may evaluate technologies, paths and hard-max blockers and persist the result, but it cannot mutate any proposed implementation path.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: `PASS` ? Governance validation passed for 25 required policy documents and 9 project lanes.

### Component/integration

- command/run: `git diff --check`
- result: `PASS` ? no whitespace errors.

### E2E

- scenario: `NOT_APPLICABLE` — coordinator allocation documentation only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: to be bound in immutable PR review evidence after the final metadata commit
- method/reviewer: coordinating agent whole-diff review against Issue #94, root/docs agent governance and concurrent Content Format Spike allocation
- material findings: `P0=0 / P1=0 / P2=0`; allocation is path-bounded and preserves all implementation/resource-limit gates
- verdict: `PASS`

## Independent review

- required: `NO` — docs-only path allocation narrows authority and changes no architecture/runtime/security/persistence implementation semantics
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: Issue #94; archived prior implementation coordinator snapshot
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after allocation merge, this coordinator allocation owns no worker implementation paths; later lifecycle archive is coordinator-owned

## Context checkpoint

```yaml
last_progress: exact `main@22a3eb866dae19d048969edff1e1fa5012a429b6`, Issue #94 and concurrent Content Format Spike allocation verified; three-file allocation diff passes governance and diff-check with zero material self-review findings
status: validating
branch: coord/durability-topology-prep-allocation-20260824
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: allocation must merge before the preparation worker writes its packet
next_action: commit and open the bounded allocation PR, bind its number, then require exact-head game-gate before squash merge
```
