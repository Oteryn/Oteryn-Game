# OTV2-20260827-work-auditor-evidence-write

```yaml
task_id: OTV2-20260827-work-auditor-evidence-write
title: Allow Work auditor bounded audit-evidence writes
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/work-auditor-evidence-write-20260827
pr: 223
base_sha: 6e6e37852b7a050a1c7117ab2a9f316907d09daf
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: governance-author-session
created_at: 2026-08-27T22:29:19+02:00
updated_at: 2026-08-27T22:38:00+02:00
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

- [x] Any canonical Oteryn Game agent or the owner may request an audit of a uniquely identifiable PR/Issue/task/head.
- [x] Auditor freezes exact target/head evidence and fails closed as `INSUFFICIENT_EVIDENCE` if the target cannot be uniquely resolved.
- [x] A completed requested audit must persist one linked GitHub evidence note with target, exact SHA, disposition/verdict, P0/P1/P2/P3 counts/findings and exactly one `NEXT_ACTION`.
- [x] Head movement makes prior evidence historical and requires a fresh audit for qualification.
- [x] Auditor cannot edit tracked files, create branches/commits, implement fixes, merge/close/approve, dispatch workflows, mutate production/protected/live-data state or write cross-repository.
- [x] Auditor evidence writes do not consume an implementation writer slot and do not make the auditor a control plane.
- [x] README, lifecycle registry and Terra/Sol scheduler agree with the prompt.
- [ ] Governance validation and exact-head CI pass.
- [ ] Author whole-diff self-review is clean.
- [ ] Genuinely independent non-authoring exact-head review is clean before merge.

## Excluded scope

No runtime/product code, Cargo/workspace, protocol/schema/registry, production, protected environment, live data, external repository, Work/Terra control-plane selection or implementation-lane authority changes.

## Implementation / findings

Issue #222 records the owner-approved authority expansion. The canonical prompt, README, lifecycle registry and Terra/Sol scheduler now consistently implement bounded audit evidence writes only; any broader repository mutation authority for the auditor remains out of scope.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending exact-head PR CI / governance workflow evidence

### Component/integration

- command/run: machine-readable lifecycle parse plus full changed-file semantic review
- result: pending exact-head PR CI / author review

### E2E

- scenario: `NOT_APPLICABLE` — governance/evidence authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: external GitHub evidence after final content commit
- trigger source: PR #223
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending external GitHub review evidence
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

- changed-file review: exact scope is 5 governance paths
- unresolved review threads: pending
- related/superseded PRs: Issue #222 / PR #223
- protected auto-merge: not authorized
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #223 opened with the four canonical auditor changes plus this bounded task record; content is frozen for exact-head qualification
status: validating
branch: docs/work-auditor-evidence-write-20260827
head_sha: external GitHub branch/PR evidence
pr: 223
final_head_sha: external GitHub evidence after final content commit
final_head_frozen_at: 2026-08-27T22:38:00+02:00
ci_trigger_source: pull_request
ci_check_generation: current-final-head
ci_checks_for_current_head: pending
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: genuinely independent non-authoring exact-head review remains mandatory before merge
next_action: qualify the unchanged PR #223 head with exact-head CI, author whole-diff self-review and genuinely independent non-authoring review
```
