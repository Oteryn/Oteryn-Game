# OTV2-20260825-work-delivery-independent-auditor

```yaml
task_id: OTV2-20260825-work-delivery-independent-auditor
title: Add independent high-effort Work delivery auditor prompt
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/work-delivery-independent-auditor-170
pr: 173
base_sha: a1a868dc3a7cbe5d3f6c2d3732038ae6cd5d4a3d
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT supervising architecture session
created_at: 2026-08-25T22:05:23Z
updated_at: 2026-08-25T22:10:30Z
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
- `PROVEN`: `docs/agents/prompts/OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md` already owns broad architecture/programme audit; the new prompt remains narrower and execution-forensic and explicitly does not supersede it.
- `PROVEN`: governance changes under `docs/agents/**` require governance validation and shared-index edits must remain narrow.
- `DERIVED`: a read-only audit prompt with zero mutation/merge authority is not an authority expansion and therefore does not by itself trigger the independent-review requirement reserved for safety reductions/authority expansions.

## Acceptance criteria

- [x] Add `docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md` with alias `Oteryn: work auditor`.
- [x] Auditor treats Work summaries as claims and reconstructs exact Issue/task/branch/PR/check/merge truth from live GitHub.
- [x] Auditor covers programme resolution, allocation timing, path/lease isolation, DAG correctness, architecture escalation, worker-result verification, exact-head CI/reviews, QA truthfulness, merge/closeout and retry-loop hygiene.
- [x] Auditor has no repository mutation, implementation, merge/close, production or cross-repository write authority.
- [x] README and lifecycle registry describe the prompt as reusable without superseding existing coordinator or broad audit prompts.
- [x] Prompt evaluates `PASS` against Authority, Resolution, Ownership, Architecture, Completeness, Evidence, Validation, Autonomy, Handover and Safety gates.
- [ ] Exact-head governance/repository policy and `game-gate` pass before merge.
- [ ] Post-merge readback, task archive, ownership release and branch cleanup are verified.

## Excluded scope

No runtime, Cargo/workspace, registry/stable-ID, workflow, architecture semantic decision, production, secret, Platform/Atlas/META/external-repository mutation. No changes to `OTV2_WORK_DELIVERY_COORDINATOR` authority. No owner-funded external AI invocation.

## Implementation / findings

Issue #170 and branch `docs/work-delivery-independent-auditor-170` were created from the exact admission main. PR #173 contains the dedicated read-only prompt plus narrow README/lifecycle registration and this lifecycle packet.

Prompt self-evaluation against `docs/agents/PROMPT_EVAL_STANDARD.md`:

- Authority: `PASS` — exact read-only authority and all mutation/production/cross-repository exclusions are explicit.
- Resolution: `PASS` — auditor resolves the current Work lifecycle from live GitHub and forbids hard-coded cached coordinator state.
- Ownership: `PASS` — auditor receives no write ownership and explicitly reconstructs/validates worker path and lease ownership.
- Architecture: `PASS` — accepted architecture is evidence; material decisions remain with the Supervising Architect.
- Completeness: `PASS` — startup, allocation, dispatch, concurrency, DAG, escalation, PR qualification, QA, merge and closeout are covered.
- Evidence: `PASS` — frozen SHAs, exact-head evidence and `PROVEN / DERIVED / UNKNOWN / CONFLICT` are mandatory.
- Validation: `PASS` — risk-proportional full-diff/check/review/E2E evidence is audited and skipped jobs require justification.
- Autonomy: `PASS` — auditor completes the bounded current programme audit without implementing fixes and returns an explicit coordinator disposition.
- Handover: `PASS` — frozen snapshot, exact lane/PR matrix, ordered required actions, confidence and missing-evidence fields make the result independently resumable/actionable.
- Safety: `PASS` — repository mutation, workflow retrigger, secrets, production and owner-funded AI are forbidden.

## Validation

### Focused

- command/run: repository governance/repository-policy validation through PR #173 exact-head workflows
- result: pending exact-head CI

### Component/integration

- command/run: prompt evaluation against `docs/agents/PROMPT_EVAL_STANDARD.md`
- result: `PASS` on all 10 gates as recorded above

### E2E

- scenario: `NOT_APPLICABLE` — reusable read-only prompt/governance metadata only; no runtime behavior changes
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending external exact-head evidence after this final candidate commit
- trigger source: pull_request #173
- workflow/run/job: pending
- runner assignment: pending
- classification: docs/agents governance-only
- result: pending

## Self-review

- exact head: pending external exact-head evidence after final candidate commit
- method/reviewer: implementing/coordinating agent, whole-diff comparison against admission main and Issue #170
- material findings: none in authored scope before exact-head CI; no runtime/authority widening detected
- verdict: `PASS_PENDING_EXACT_HEAD_CI`

## Independent review

- required: NO — read-only prompt adds no mutation/merge authority and does not reduce safety; repository governance/merge gates remain mandatory
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: four intended docs/agents paths only before final candidate commit
- unresolved review threads: pending
- related/superseded PRs: none identified; prompt explicitly does not supersede Work coordinator or broad audit prompt
- protected auto-merge: not used
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: PR #173 opened; prompt package and self-evaluation complete
status: validating
branch: docs/work-delivery-independent-auditor-170
head_sha: null
pr: 173
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pending
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
next_action: qualify PR #173 exact final head through repository-required CI and review gates
```
