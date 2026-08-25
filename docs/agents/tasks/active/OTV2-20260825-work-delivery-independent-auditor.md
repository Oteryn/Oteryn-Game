# OTV2-20260825-work-delivery-independent-auditor

```yaml
task_id: OTV2-20260825-work-delivery-independent-auditor
title: Add independent high-effort Work delivery auditor prompt
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/work-delivery-independent-auditor-170
pr: null
base_sha: a1a868dc3a7cbe5d3f6c2d3732038ae6cd5d4a3d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT supervising architecture session
created_at: 2026-08-25T22:05:23Z
updated_at: 2026-08-25T22:05:23Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/prompts/README.md
  - docs/agents/PROMPT_LIFECYCLE.json
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-independent-auditor.md
public_contracts: []
depends_on:
  - Issue #170 owner-approved scope
  - OTV2_WORK_DELIVERY_COORDINATOR canonical prompt
  - PROMPT_EVAL_STANDARD.md
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Publish a reusable read-only `Oteryn: work auditor` prompt that independently audits Work coordinator execution from live GitHub evidence with higher reasoning depth, without gaining implementation, merge, architecture or production authority.

## Architecture and source of truth

- `PROVEN`: protected admission `main` is `a1a868dc3a7cbe5d3f6c2d3732038ae6cd5d4a3d`.
- `PROVEN`: Issue #170 records the owner-approved alias, read-only authority and acceptance criteria.
- `PROVEN`: `docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md` is the audited coordinator contract.
- `PROVEN`: `docs/agents/prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md` already owns broad architecture/programme audit; the new prompt must remain narrower and execution-forensic rather than superseding it.
- `PROVEN`: governance changes under `docs/agents/**` require governance validation and shared-index edits must remain narrow.
- `DERIVED`: a read-only audit prompt with zero mutation/merge authority is not an authority expansion and therefore does not by itself trigger the independent-review requirement reserved for safety reductions/authority expansions.

## Acceptance criteria

- [ ] Add `docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md` with alias `Oteryn: work auditor`.
- [ ] Auditor treats Work summaries as claims and reconstructs exact Issue/task/branch/PR/check/merge truth from live GitHub.
- [ ] Auditor covers programme resolution, allocation timing, path/lease isolation, DAG correctness, architecture escalation, worker-result verification, exact-head CI/reviews, QA truthfulness, merge/closeout and retry-loop hygiene.
- [ ] Auditor has no repository mutation, implementation, merge/close, production or cross-repository write authority.
- [ ] README and lifecycle registry describe the prompt as reusable without superseding existing coordinator or broad audit prompts.
- [ ] Prompt evaluates `PASS` against Authority, Resolution, Ownership, Architecture, Completeness, Evidence, Validation, Autonomy, Handover and Safety gates.
- [ ] Exact-head governance/repository policy and `game-gate` pass before merge.
- [ ] Post-merge readback, task archive, ownership release and branch cleanup are verified.

## Excluded scope

No runtime, Cargo/workspace, registry/stable-ID, workflow, architecture semantic decision, production, secret, Platform/Atlas/META/external-repository mutation. No changes to `OTV2_WORK_DELIVERY_COORDINATOR` authority. No owner-funded external AI invocation.

## Implementation / findings

Issue #170 and branch `docs/work-delivery-independent-auditor-170` were created from the exact admission main. New prompt is being packaged as a read-only specialized audit profile.

## Validation

### Focused

- command/run: pending governance validation
- result: pending

### Component/integration

- command/run: prompt evaluation against `docs/agents/PROMPT_EVAL_STANDARD.md`
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — reusable read-only prompt/governance metadata only; no runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: NO — read-only prompt adds no mutation/merge authority and does not reduce safety; repository governance/merge gates remain mandatory
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none identified
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: issue and branch created; dedicated read-only Work auditor prompt added
status: implementing
branch: docs/work-delivery-independent-auditor-170
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
next_action: register prompt in README and lifecycle registry
```
