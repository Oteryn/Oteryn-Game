# OTV2-20260825-next-wave-limit-decisions

```yaml
task_id: OTV2-20260825-next-wave-limit-decisions
title: Accept evidence-backed first-slice next-wave resource limits
mode: CONTRACT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
branch: arch/next-wave-limit-decisions-133
issue: 133
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT decision/evidence worker for Issue #133
created_at: 2026-08-25T01:06:00+02:00
updated_at: 2026-08-25T01:06:00+02:00
execution_budget_minutes: 120
large_budget_reason: One reproducible evidence harness must classify and justify exact first-slice limits across three blocker issues without conflating semantic domains.
owned_paths:
  - docs/agents/tasks/active/OTV2-20260825-next-wave-limit-decisions.md
  - tools/next-wave-limit-evidence/**
  - docs/agents/evidence/OTV2-20260825-next-wave-limit-evidence.json
  - docs/architecture/reviews/OTERYN_GAME_NEXT_WAVE_FIRST_SLICE_LIMITS_DECISION_2026-08-25.md
public_contracts:
  - docs/architecture/reviews/OTERYN_GAME_NEXT_WAVE_FIRST_SLICE_LIMITS_DECISION_2026-08-25.md
depends_on:
  - issue:93
  - issue:116
  - issue:123
  - issue:131
blocks:
  - issue:93
  - issue:116
  - issue:123
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A reproducible checked-accounting harness and reviewed decision baseline identify exact conservative first-slice maxima for Ability, Interaction, AI, the Foundation authority-journal Durability substrate and TCP/TLS listener resource handling. Every omitted inventory row is explicitly excluded fail-closed. Canonical registration remains a separately serialized task.

## Architecture and source of truth

- `PROVEN`: Issue #133 is the child task authorized by Issue #131/#128.
- `PROVEN`: #93, #116 and #123 own the unresolved resource decisions and prohibit silent reuse of generic Foundation envelopes.
- `PROVEN`: this task owns no canonical registry, Cargo/workspace or product-code path.
- `DERIVED`: Movement-only semantic limits require a dedicated non-current successor because no exact Movement executable slice is allocated.
- `UNKNOWN`: exact worker base SHA until this allocation PR merges.

## Acceptance criteria

- [ ] A Rust/std-only harness observes RED before the checked model exists, then GREEN at every exact maximum/max+1 boundary.
- [ ] Evidence JSON records units, checked equations, representative retained-state/work costs and deterministic outcomes.
- [ ] Every exercised #93/#116/#123 inventory row has one exact candidate; every omitted row is explicitly fail-closed `NOT_APPLICABLE_TO_FIRST_SLICE`.
- [ ] Movement unresolved rows move to a dedicated successor issue without releasing Movement implementation.
- [ ] No canonical registry, product code, production tuning/default, port/certificate/key/deployment or gameplay balance value changes.
- [ ] Whole-diff review, governance, semantic audits, exact-head `game-gate` and merge pass.

## Excluded scope

`docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, Cargo/workspace/lockfile, Foundation/product code, listener bind/runtime, Durability/gameplay/client implementation, production sizing/defaults, Reference parity, secrets, deployment and external-repository write.

## Implementation / findings

Waiting for the separately merged child allocation and exact-base branch creation. Candidate values are not authoritative until the harness, decision baseline, review and merge complete.

## Validation

### Focused

- command/run: `rustc --edition 2024 --test tools/next-wave-limit-evidence/main.rs -o <temp-test> && <temp-test>`
- result: pending RED then GREEN evidence

### Component/integration

- command/run: optimized harness emission + JSON invariant validation + governance + architecture checks
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — evidence/decision task creates no runtime path
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: contract/evidence only
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing decision/evidence worker, complete changed-file diff
- material findings: pending
- verdict: pending

## Independent review

- required: NO — this task selects bounded safety ceilings but changes no executable security/durable/runtime authority; protected semantic audits remain mandatory
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: after decision/evidence merge and handoff to serialized registry task

## Context checkpoint

```yaml
last_progress: Child task record prepared; no decision/evidence write authority exists until allocation merge.
status: waiting
branch: arch/next-wave-limit-decisions-133
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
blocker: allocation_not_merged
next_action: After allocation merge, create `arch/next-wave-limit-decisions-133` at the exact merge SHA and begin the harness RED test.
```
