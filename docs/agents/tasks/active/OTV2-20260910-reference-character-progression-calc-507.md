# OTV2-20260910 Reference Character progression calculator #507

```yaml
task_id: REFERENCE_CHARACTER_PROGRESSION_CALC_507
title: Implement the bounded Reference Character progression calculator
mode: IMPLEMENT
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/reference-character-progression-calc-507
issue: 507
pr: pending
base_sha: 1fdc37fdb8b8faf2aaab99f17a25c7a6e986f7c5
head_sha: pending
final_head_sha: pending
final_head_frozen_at: pending
owner: Codex
created_at: 2026-09-10T09:39:24Z
updated_at: 2026-09-10T10:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/progression.rs
  - apps/game-server/src/domain/mod.rs
  - apps/game-server/tests/reference_character_progression_calc.rs
  - docs/agents/tasks/active/OTV2-20260910-reference-character-progression-calc-507.md
public_contracts: []
depends_on:
  - "#162 activation comment 5616480651"
  - docs/agents/programs/OTV2_REFERENCE_CHARACTER_PROGRESSION_CALC_507_ALLOCATION_20260910.md
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Provide `REFERENCE_CHARACTER_PROGRESSION_CALC_V1`: a pure, checked, deterministic,
fixed-shape staged calculator for typed experience awards and Oteryn declared-difference
death experience loss. The component does not mutate a Character revision or any durable
state.

## Architecture and source of truth

- `PROVEN`: #162 comment `5616480651` activates this exact worker and path allocation.
- `PROVEN`: the protected allocation separates pure calculation from the future WP5
  Character/DUR-02 commit owner.
- `PROVEN`: checked arithmetic and explicit rounding reuse `simulation-determinism`.
- `UNKNOWN`: immutable 2026-07-28 target XP thresholds and reward/death arithmetic.
  Tests therefore use only an explicitly synthetic `NON_REFERENCE` finite table.
- `PROVEN`: the accepted Oteryn declared difference uses the full injected
  `LevelXPSpan(current_level)` and mutates neither skill nor magic progression.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: pure calculation only; no authority grant, production mutation, revision, fence, persistence, recovery, or commit boundary
```

## Acceptance criteria

- [x] Typed award and death operations produce a complete staged fixed-shape result.
- [x] Below-threshold, exact crossing, deterministic multi-threshold, and death-delevel
  behavior are covered with synthetic non-Reference data.
- [x] Death loss uses the complete injected level span and carries skill/magic state
  through unchanged by construction.
- [x] Context and operation revision mismatches, missing oracle data, unsupported
  operation families, invalid inputs, overflow, and underflow fail closed.
- [x] Repeated identical scalar inputs produce equal staged results.
- [x] No target parity, persistence, runtime, resource-registry, or production claim is made.

## Excluded scope

CharacterRevision mutation; SQL and DUR-02/DUR-03; GameSession and leases; audit and
outbox; Combat attribution/effects; item and loot mutation; Cargo/lock or SIM changes;
Content; gameplay transport; registries; workflows; Platform/external repositories;
production and live deployment; target-threshold parity.

## Implementation / findings

- The finite policy is a const-generic array plus an exclusive terminal bound, so an
  out-of-table result rejects instead of extrapolating.
- Experience arithmetic uses `ExactI64`; death policy scaling uses `FixedScale` and an
  injected `RoundingMode`.
- Projection validates the current snapshot against the same oracle before calculating.
- No arbitrary signed progression-delta API is exposed.

## Validation

### Focused

- command/run: `cargo test -p oteryn-game-server --test reference_character_progression_calc`
- result: PASS (9 tests)

### Component/integration

- command/run: `cargo test -p oteryn-game-server`
- result: PASS

### E2E

- scenario: `NOT_APPLICABLE` — persistence-neutral library component has no runtime path.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: draft pull request
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending until commit
- method/reviewer: implementing Codex agent, whole diff against protected base
- material findings: none after reviewing all four owned paths for authority leakage,
  unbounded collections, unchecked arithmetic, generic signed deltas, target-parity claims,
  and partial-result behavior
- verdict: PASS

## Independent review

- required: pending exact-head repository risk-policy evaluation
- exact head: pending
- method/auditor: repository-native review/check surface
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: PASS; exactly four allocated paths
- unresolved review threads: pending
- related/superseded PRs: none
- protected auto-merge: forbidden/not requested
- merge commit/result: not requested
- ownership release: pending coordinator disposition

## Context checkpoint

```yaml
last_progress: completed validation and whole-diff self-review of the bounded pure calculator
status: ready
branch: agent/reference-character-progression-calc-507
head_sha: pending
pr: pending
final_head_sha: pending
final_head_frozen_at: pending
ci_trigger_source: draft_pull_request
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
next_action: coordinator reviews the canonical exact-head draft PR and repository checks
```
