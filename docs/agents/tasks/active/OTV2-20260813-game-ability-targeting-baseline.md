# OTV2-20260813-game-ability-targeting-baseline

```yaml
task_id: OTV2-20260813-game-ability-targeting-baseline
title: Record GAME-ABILITY-01 deterministic targeting and legality boundary
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/game-ability-targeting-baseline
pr: 228
base_sha: be80a3c6a8a5d3fd71c5a23786d3e34c7572aef3
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: ChatGPT architecture coordinator
created_at: 2026-08-13T17:58:00+02:00
updated_at: 2026-08-13T18:03:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-game-ability-targeting-baseline.md
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
public_contracts:
  - docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md
depends_on:
  - docs/architecture/GAME-ABILITY-01_TYPED_EFFECT_PIPELINE_OWNER_BASELINE.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
  - docs/architecture/ANL-01_GAME_EVENT_AND_AUDIT_FOUNDATION_CONTRACT.md
blocks:
  - safe continuation of GAME-ABILITY-01 cast timing, costs, cooldowns, conditions and effect composition decisions
cross_repository_coordination_id: OTV2-GLOBAL-ARCHITECTURE
external_repositories: []
```

## Outcome

Record the owner-accepted second bounded `GAME-ABILITY-01` subdecision: target discovery is a separate deterministic authoritative stage. Ability content describes target policy; it does not directly enumerate or mutate authoritative world objects. Every player, AI, NPC and system origin uses the same bounded Target Resolver before legality evaluation and Effect Plan construction.

Overall `GAME-ABILITY-01` remains open / `REQUIRED_FOR_ALPHA`.

## Architecture and source of truth

- **PROVEN:** PR #226 merged the accepted typed-effect pipeline baseline as `be80a3c6a8a5d3fd71c5a23786d3e34c7572aef3`.
- **PROVEN:** the owner explicitly accepted the targeting recommendation in the current architecture session on 2026-08-13.
- **PROVEN:** FND-03 requires one logical authoritative mutation owner and deterministic owner-local ordering; cross-channel state cannot become an implicit target search surface.
- **PROVEN:** SIM-DETERMINISM requires stable normalized inputs, explicit tie-breaking/order and bounded reproducible behavior.
- **PROVEN:** DUR-04 scripts are capability-bounded and proposal-only; targeting extensions cannot become a second direct world-query/mutation authority.
- **UNKNOWN:** exact target grammar, shape catalogue, range metric, LoS algorithm, floor semantics, PvP/PZ/immunity rules and precedence, dynamic retarget snapshot timing and protocol/client representation remain undecided.

## Acceptance criteria

- [x] Add one canonical owner baseline recording only the accepted targeting/legality boundary.
- [x] Preserve one ability pipeline for player, AI, NPC and system origins.
- [x] Separate Target Intent, bounded typed Target Query, authoritative Target Resolution, legality evaluation and Effect Plan generation.
- [x] Make resolver output deterministic, ordered and bounded; never depend on hash/container iteration order.
- [x] Prevent client/content/Wasm from supplying an authoritative final target set.
- [x] Require chain/jump/dynamic retargeting to be explicit bounded deterministic resolution steps rather than hidden world re-queries during effect generation.
- [x] Preserve FND-03 scope/ownership, DUR-04 capability limits, SIM determinism and ANL observability boundaries.
- [x] Keep exact gameplay values/algorithms and all runtime/protocol/DDL/production implementation unauthorized.
- [ ] Complete exact-head full-diff self-review and required documentation/governance CI before merge.

## Excluded scope

No Rust runtime, target-query implementation, combat formula, exact range/LoS/floor/PvP/PZ/immunity rule, target priority/error precedence, protocol schema, client UI, physical serializer, database schema/migration, Platform write, production behavior or external-repository mutation.

## Implementation / findings

PR #228 contains exactly this task record and `docs/architecture/GAME-ABILITY-01_TARGETING_AND_LEGALITY_BOUNDARY_OWNER_BASELINE.md`. The baseline is semantic only and preserves explicit extension points for areas, beams, cones, chains, jumps, nearest-N and staged retargeting.

Issue creation was unavailable through the connector for this bounded task; the task record and PR are the durable execution sources.

## Validation

### Focused

- command/run: inspect complete PR #228 diff against typed-effect, FND-03, DUR-04, SIM-DETERMINISM and ANL-01 boundaries
- result: pending final-head self-review

### Component/integration

- command/run: `NOT_APPLICABLE` — architecture-only documentation
- result: `NOT_APPLICABLE`

### E2E

- scenario: `NOT_APPLICABLE` — no executable/player-visible behavior
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- runner assignment: pending
- classification: documentation/governance
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: `NO` unless self-review discovers material uncertainty; bounded paper-only partial baseline with no security/protocol/durable-schema/production authority change
- exact head: `NOT_APPLICABLE`
- method/auditor: `NOT_APPLICABLE`
- material findings: `NOT_APPLICABLE`
- verdict: `NOT_APPLICABLE`

## PR and closeout

- changed-file review: pending final head
- unresolved review threads: pending
- related/superseded PRs: #227 is non-semantic closeout of the preceding typed-effects task; #162 and #191 are non-overlapping
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Owner baseline added and draft PR #228 opened; task metadata reconciled before final-head freeze.
status: validating
branch: docs/game-ability-targeting-baseline
head_sha: null
pr: 228
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
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
owner_action_required: false
blocker: null
next_action: Inspect the exact full diff, perform self-review, resolve findings and require exact-head documentation/governance CI.
```
