# OTV2-20260915-closure-convergence-protocol-624

```yaml
task_id: OTV2-20260915-closure-convergence-protocol-624
title: Add reusable closure convergence protocol
mode: GOVERNANCE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/closure-convergence-protocol
pr: null
base_sha: ebf581da4c824ca7ad68faf4b52cd3cb237a3ab1
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT
created_at: 2026-09-15T10:49:00Z
updated_at: 2026-09-15T10:49:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/CLOSURE_CONVERGENCE_PROTOCOL.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md
  - docs/agents/prompts/OTV2_IMPL_DURABILITY.md
  - docs/agents/prompts/OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md
  - docs/agents/tasks/active/OTV2-20260915-closure-convergence-protocol-624.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Introduce a reusable late-stage convergence protocol so delivery can perform one full read-only defect sweep, freeze root causes, repair compatible material blockers as one coherent generation, qualify once, and complete one final whole-diff review without weakening current authority or protected integration rules.

## Architecture and source of truth

- `PROVEN`: `OTERYN_ORGANIZATION_AGENT_POLICY@3.1.0` is bound by `docs/agents/META_AGENT_POLICY_BINDING.json` at META commit `23b21e9b1b2d4b6c3a5cac3d4c7a18747804c090`.
- `PROVEN`: the prompting standard requires task-specific deltas rather than duplicating the full agent operating system.
- `PROVEN`: current Work coordinator already owns evidence caching and anti-loop retry control, but its ordinary `one bounded task per worker` model has no explicit late-stage coherent-repair-generation override.
- `PROVEN`: current Durability prompt requires exactly one next handoff action but has no convergence-mode batch semantics or explicit low-level Git-object publication fallback prohibition.
- `PROVEN`: current independent auditor supports bounded requested audits and can consume a stricter convergence-mode dispatch without gaining implementation or control-plane authority.

## High-risk authority/recovery qualification

```yaml
applicable: NOT_APPLICABLE
reason: Documentation/prompt governance only; no production mutation, authority-bearing session replacement, PREPARE/COMMIT, persisted recovery interpretation, runtime/database mutation, or protected integration is performed by this task.
```

## Acceptance criteria

- [ ] Add one routed closure convergence protocol with explicit activation, one-shot sweep, finding classification, root-cause collapse, frozen inventory, coherent repair generation, final qualification/review and anti-drip novelty triggers.
- [ ] Add publication-safety fail-closed behavior for unavailable normal Git publication; no direct low-level Git-object fallback on canonical material branches.
- [ ] Work coordinator routes convergence-mode workers and auditors through the protocol and treats a bounded task as a bounded coherent repair generation during closure.
- [ ] Durability implementer consumes frozen root-cause batches and does not stop after the first compatible blocker.
- [ ] Independent auditor supports explicit `DISCOVERY_SWEEP` and `FINAL_CANDIDATE_REVIEW` convergence modes without gaining implementation authority.
- [ ] No change to #356 material source, runtime/product behavior, workflows, rulesets, Merge Queue semantics, Platform/Atlas/META or production state.

## Excluded scope

No WP3 product repair, no #356 mutation, no Cargo/runtime source, no workflow/ruleset/protection change, no Merge Queue submission, no external repository write.

## Implementation / findings

Use a compact shared protocol and prompt-specific deltas. Do not copy the full convergence procedure into every prompt.

## Validation

### Focused

- changed-file review: pending
- prompt semantic self-review: pending

### Component/integration

- repository prompt/governance checks: pending

### E2E

- scenario: NOT_APPLICABLE — prompt/governance documentation only
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: YES — reusable control-plane/audit instruction change
- exact head: pending
- method/auditor: Oteryn: work auditor or current policy-selected equivalent
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: NOT_AUTHORIZED_BY_TASK
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Added closure convergence protocol on dedicated path-disjoint branch.
status: implementing
branch: agent/closure-convergence-protocol
head_sha: aa17c49c598728b699d2ce089cbb450f58ea4321
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
next_action: Wire the protocol into the coordinator, Durability implementer and independent auditor prompts.
```