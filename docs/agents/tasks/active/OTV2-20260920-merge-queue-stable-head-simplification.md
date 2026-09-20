# OTV2-20260920-merge-queue-stable-head-simplification

```yaml
task_id: OTV2-20260920-merge-queue-stable-head-simplification
title: Simplify stable-head delivery under Merge Queue
mode: GOVERNANCE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/merge-queue-stable-head-simplification-20260920
pr: null
base_sha: e765a314ceb2b81e4260dd1dee1f1e547f6ce920
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-authorized ChatGPT session
created_at: 2026-09-20T23:46:00+02:00
updated_at: 2026-09-20T23:46:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
  - docs/agents/evidence/OTV2-20260920-merge-queue-stable-head-simplification-evaluation.md
  - docs/agents/tasks/active/OTV2-20260920-merge-queue-stable-head-simplification.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Remove refresh-only branch mutations from the normal Game delivery path while preserving source reconciliation when it is genuinely required. A stable published PR head should survive unrelated protected-`main` movement and use canonical Merge Queue `merge_group` qualification instead of being merged up merely to inherit a newer base or workflow generation.

## Architecture and source of truth

- **PROVEN** — protected admission base is `main@e765a314ceb2b81e4260dd1dee1f1e547f6ce920`.
- **PROVEN** — bound META 3.1 execution policy already states that protected-`main` movement alone does not require merge-up where canonical Merge Queue owns freshness.
- **PROVEN** — Game `CONTRIBUTING.md` already tells contributors to keep the accepted PR head unchanged under Merge Queue instead of merging `main` merely to refresh it.
- **PROVEN** — current Work coordinator preflight correctly retains isolated Git/non-force publication requirements for real source mutation.
- **DERIVED** — the recurring failure mode is provider/control-plane over-application of source-reconciliation machinery to candidates that need only read-only upstream reconciliation and Merge Queue qualification.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: No production mutation, persisted recovery authority, PREPARE/COMMIT authority, controller install/restore, session replacement or durable authority interpretation is introduced.
```

## Acceptance criteria

- [x] Work coordinator explicitly treats protected-`main` movement as read-only reconciliation first.
- [x] Missing local Git is not a blocker when an already-published candidate requires no source mutation.
- [x] Normal merge-up remains required for a proven semantic/contract/source conflict or an explicit non-Merge-Queue strict-base requirement.
- [x] Convergence mode does not mutate a frozen candidate merely to track a newer protected base or workflow generation.
- [x] Current `merge_group` remains the authority for composition against protected `main`.
- [x] Force push, rebase/reset, direct merge, bypass, weakened checks and low-level candidate reconstruction remain forbidden.
- [ ] Exact-head repository CI passes.
- [ ] One independent deep review completes on the stable material control-plane candidate.

## Excluded scope

No runtime/source/Cargo/schema/content mutation. No change to Merge Queue primitive, branch protection, rulesets, required checks, credentials, Remote Desktop policy, API publication safety or protected integration authority. No attempt to create a general shell executor.

## Implementation / findings

The change is deliberately narrow: it removes unnecessary mutation requirements rather than weakening mutation safety. It does not create a new execution route. If source reconciliation is actually necessary, the existing isolated-workspace, custody, validation and normal non-force publication requirements still apply.

`PROMPT_LIFECYCLE.json` remains unchanged because the Work coordinator prompt identity, alias, owner, lifecycle status and supersession relation are unchanged.

## Validation

### Focused

- command/run: repository exact-head governance/prompt validation through canonical CI
- result: pending

### Component/integration

- command/run: Merge gate / Agent Governance / Architecture Semantic Audit as selected by repository routing
- result: pending

### E2E

- scenario: NOT_APPLICABLE — governance/prompt-only change
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: GitHub-hosted
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: none known before CI
- verdict: pending

## Independent review

- required: YES — material control-plane governance can alter autonomous worker dispatch/integration behavior
- exact head: pending
- method/auditor: one Codex deep review on stable material candidate
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: forbidden; canonical Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: provider stable-head rule implemented on dedicated branch
status: validating
branch: docs/merge-queue-stable-head-simplification-20260920
head_sha: pending
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: github_hosted
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: publish evaluation evidence, open Draft PR, run exact-head CI and independent review
```
