# OTV2-20260824-wave2-resource-limits

```yaml
task_id: OTV2-20260824-wave2-resource-limits
title: Prepare Wave-2 gameplay resource-limit decision packet
mode: CONTRACT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: docs/wave2-resource-limits-20260824
issue: 93
pr: null
base_sha: 22a3eb866dae19d048969edff1e1fa5012a429b6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260824-wave2-resource-limits
created_at: 2026-08-24T21:01:00+02:00
updated_at: 2026-08-24T21:18:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/reviews/OTERYN_GAME_WAVE2_RESOURCE_LIMITS_DECISION_PACKET_2026-08-24.md
public_contracts: []
depends_on:
  - Oteryn-Game#93
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/architecture/GAME-ABILITY-01_WHOLE_GATE_OWNER_ACCEPTANCE_BASELINE.md
  - docs/architecture/OTERYN_V2_REMAINING_FIRST_WAVE_OWNER_ACCEPTANCE_BASELINE_20260816.md
  - docs/architecture/OTERYN_V2_STAGE_C_VSL_OWNER_ACCEPTANCE_20260816.md
blocks:
  - OTV2-IMPL-ABILITY executable acceptance for exercised unregistered dimensions
  - OTV2-IMPL-INTERACTION executable acceptance for exercised unregistered dimensions
  - OTV2-IMPL-AI executable acceptance for exercised unregistered dimensions
  - OTV2-IMPL-MOVE allocation until Movement-exercised dimensions are closed
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Produce the bounded Issue #93 decision packet required by the next-wave master plan. Inventory every required Ability, Interaction, AI and Movement resource dimension, classify each using the Issue #93 vocabulary, record failure/allocation/client/boundary-test obligations, and identify evidence or explicit owner decisions still required. Do not choose numeric hard maxima without accepted evidence or owner disposition.

The worker owns only the decision-packet path above. `docs/contracts/RESOURCE_LIMITS_REGISTRY.json`, coordinator programme overlays, runtime code, Cargo/workspace files and workflows are not worker-owned.

Once this allocation PR is merged, this exact task record supersedes the older deferred `OTV2-WAVE2-RESOURCE-LIMITS: write_authority: none` wording in `OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md` **only for the single packet path listed in `owned_paths`**. The older no-write posture remains binding for the registry, runtime, shared/Cargo/workspace paths, workflows and every other unallocated surface.

## Architecture and source of truth

- `PROVEN`: Issue #93 requires this packet and forbids worker-invented numeric limits.
- `PROVEN`: `RESOURCE_LIMITS_REGISTRY.json` requires every externally controlled count/depth/length/byte size to have an absolute hard maximum before implementation acceptance; missing required values fail review rather than imply unlimited capacity.
- `PROVEN`: GAME-ABILITY-01, GAME-INTERACTION-01, GAME-AI-01 and VSL-MOVE-01 are owner-accepted for their bounded architecture scopes, while concrete numeric gameplay maxima remain deliberately unresolved.
- `PROVEN`: the next-wave master plan names the exact packet path owned by this task.
- `PROVEN`: the live allocation ledger's older `write_authority: none` remains authoritative outside this exact-path allocation and is not rewritten by this task.
- `DERIVED`: registry mutation is a later serialized coordinator action after accepted evidence/owner decision.
- `UNKNOWN`: exact numeric maxima for currently unregistered gameplay dimensions until accepted evidence or explicit owner disposition exists.
- `CONFLICT`: none after the exact-path authority precedence above is applied.

## Acceptance criteria

- [ ] Create `docs/architecture/reviews/OTERYN_GAME_WAVE2_RESOURCE_LIMITS_DECISION_PACKET_2026-08-24.md`.
- [ ] Cover the complete minimum inventories required by Issue #93 for Ability, Interaction, AI and Movement.
- [ ] Classify every listed dimension exactly once as `REGISTERED_EXACT`, `CONTRACT_EXACT_UNREGISTERED`, `EVIDENCE_CANDIDATE`, `OWNER_DECISION_REQUIRED` or `NOT_APPLICABLE_TO_FIRST_SLICE`.
- [ ] For every non-registered dimension record unit, owning contract, amplification/control source, failure behavior/category, allocation impact, client visibility, boundary-test obligations and evidence/owner-decision requirement.
- [ ] Preserve staged release: Ability/Interaction/AI may release independently only after each exercised dimension is registered or explicitly excluded fail-closed; Movement obligations remain live through pre-Movement closure.
- [ ] Do not edit `RESOURCE_LIMITS_REGISTRY.json` or select numeric maxima in this worker delivery.
- [ ] Complete whole-diff self-review, placeholder scan, exact-head repository checks and zero-thread verification for the worker PR.

## Excluded scope

No runtime/client/server/protocol/content implementation; no stable protocol/event/state ID allocation; no registry mutation; no production tuning; no generic FND ceiling reuse as a gameplay-semantic limit without explicit owning-contract equivalence; no Platform/external-repository/production mutation; no permanent Content-format or Durability-topology decision.

## Validation

### Focused
- command/run: document completeness/classification review against Issue #93 and accepted owning contracts
- result: pending

### Component/integration
- command/run: `NOT_APPLICABLE` — paper-only preparation
- result: `NOT_APPLICABLE`

### E2E
- scenario: `NOT_APPLICABLE` — no executable product mutation
- result: `NOT_APPLICABLE`

### Exact-head CI
- final head: pending
- trigger source: pull_request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review
- exact head: pending
- method/reviewer: worker whole-diff review against Issue #93/master plan
- material findings: pending
- verdict: pending

## Independent review
- required: NO — packet preparation accepts no numbers, reduces no safety gate and mutates no runtime/registry/protocol/persistence/value authority
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout
- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: allocation PR #114; Issue #93 remains governing lifecycle
- protected auto-merge: worker PR remains coordinator-owned for integration
- merge commit/result: pending
- ownership release: pending after worker delivery and closeout

## Context checkpoint

```yaml
last_progress: coordinator rebased allocation onto current main and made exact-path authority precedence explicit against the older deferred live-ledger wording
status: investigating
branch: docs/wave2-resource-limits-20260824
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
owner_action_required: exact numeric maxima require accepted evidence or explicit owner disposition before registry mutation
blocker: none for packet preparation
next_action: merge coordinator allocation, then produce the decision packet on the allocated worker branch
```
