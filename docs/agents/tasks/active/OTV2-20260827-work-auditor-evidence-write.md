# OTV2-20260827-work-auditor-evidence-write

```yaml
task_id: OTV2-20260827-work-auditor-evidence-write
title: Allow Work auditor bounded audit-evidence writes
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/work-auditor-evidence-write-20260827
pr: null
base_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: governance-author-session
created_at: 2026-08-27T22:29:19+02:00
updated_at: 2026-08-27T22:29:19+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/programs/OTERYN_V2_TERRA_SOL_EXECUTION_SCHEDULER.md
  - docs/agents/tasks/active/OTV2-20260827-work-auditor-evidence-write.md
public_contracts: []
depends_on:
  - issue: 222
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`Oteryn: work auditor` remains an independent non-implementation auditor, but may persist the result of a requested audit as bounded GitHub evidence on the exact audited PR/Issue/task target. The new write authority is comment/review evidence only and does not create implementation, control-plane, merge, production, tracked-file or cross-repository authority.

## Architecture and source of truth

- `PROVEN`: protected admission `main` is `6e6e37852b7a050a1c7117ab2a9f316907d09daf`.
- `PROVEN`: Issue #222 contains explicit owner authorization for bounded audit-evidence write authority.
- `PROVEN`: `docs/agents/AGENTS.md` classifies authority expansion as requiring genuinely independent exact-head review before merge.
- `DERIVED`: preserving tracked-repository mutation as forbidden while separately authorizing COMMENT/review/Issue-note evidence is the smallest authority expansion that satisfies the owner request without making the auditor an implementation writer.

## Acceptance criteria

- [ ] Any canonical Oteryn Game agent or the owner may request an audit of a uniquely identifiable PR/Issue/task/head.
- [ ] Auditor freezes exact target/head evidence and fails closed as `INSUFFICIENT_EVIDENCE` if the target cannot be uniquely resolved.
- [ ] A completed requested audit must persist one linked GitHub evidence note with target, exact SHA, disposition/verdict, P0/P1/P2/P3 counts/findings and exactly one `NEXT_ACTION`.
- [ ] Head movement makes prior evidence historical and requires a fresh audit for qualification.
- [ ] Auditor cannot edit tracked files, create branches/commits, implement fixes, merge/close/approve, dispatch workflows, mutate production/protected/live-data state or write cross-repository.
- [ ] Auditor evidence writes do not consume an implementation writer slot and do not make the auditor a control plane.
- [ ] README, lifecycle registry and Terra/Sol scheduler agree with the prompt.
- [ ] Governance validation and exact-head CI pass.
- [ ] Author whole-diff self-review is clean.
- [ ] Genuinely independent non-authoring exact-head review is clean before merge.

## Excluded scope

No runtime/product code, Cargo/workspace, protocol/schema/registry, production, protected environment, live data, external repository, Work/Terra control-plane selection or implementation-lane authority changes.

## Implementation / findings

Issue #222 records the owner-approved authority expansion. This task must remain bounded to audit evidence writes only; any broader repository mutation authority for the auditor is out of scope.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending

### Component/integration

- command/run: machine-readable lifecycle parse plus full changed-file semantic review
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — governance/evidence authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: PR
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: governance author session
- material findings: pending
- verdict: pending

## Independent review

- required: YES — explicit owner-approved authority expansion under `docs/agents/AGENTS.md`
- exact head: pending
- method/auditor: genuinely independent non-authoring exact-head reviewer
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: Issue #222
- protected auto-merge: not authorized
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner authorization captured in Issue #222 and bounded governance branch created from exact protected main
status: implementing
branch: docs/work-auditor-evidence-write-20260827
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
blocker: null
next_action: update the four canonical auditor prompt/registry/scheduler documents, then open the bounded governance PR
```
