# OTV2-20260909-graphics-engine-bakeoff

```yaml
task_id: OTV2-20260909-graphics-engine-bakeoff
title: Qualify Oteryn graphics engine backend
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/graphics-engine-bakeoff-465
pr: null
base_sha: 466d16abc0d10a018b2c4b5019bf0e0bcb59da46
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-gpt5.6-sol
created_at: 2026-09-09T10:02:53Z
updated_at: 2026-09-09T10:02:53Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260909-graphics-engine-bakeoff.md
  - experiments/graphics-engine-bakeoff/**
  - .github/workflows/graphics-engine-bakeoff.yml
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Produce reproducible evidence for Issue #465 comparing a custom Rust/wgpu presentation backend with a Bevy challenger using the same benchmark-only Oteryn render workload. The task does not select or activate a production engine by itself.

## Architecture and source of truth

- `PROVEN`: protected `main` at allocation was `466d16abc0d10a018b2c4b5019bf0e0bcb59da46`.
- `PROVEN`: production workspace remains Rust 1.94.0 with direct `wgpu = 30.0.0`; the existing renderer is a bounded surface/device spike rather than a production world renderer.
- `PROVEN`: `GRAPHICS_APPEARANCE_ANIMATION_AND_SEASONALITY_HORIZON_NOTE.md` explicitly defers permanent renderer texture/container choices until representative evidence exists.
- `DERIVED`: a benchmark-only nested workspace can compare the candidates without changing the production Cargo graph or accepted architecture.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task performs no production mutation, authority-bearing session operation, persisted recovery interpretation, PREPARE/COMMIT, controller installation or live deployment.

## Acceptance criteria

- [ ] Both backends consume the same benchmark-only `RenderSnapshot` and deterministic scenario generator.
- [ ] BASIC, NORMAL and STRESS scenarios are available at 32, 64 and 128 pixel sprite densities.
- [ ] Synthetic art is generated in memory; no proprietary Tibia art is committed.
- [ ] Production root Rust 1.94/Cargo dependency graph, `apps/client` and `crates/renderer` remain unchanged.
- [ ] Both candidates compile in an isolated Rust 1.95 nested workspace.
- [ ] Output records comparable frame-time percentiles, throughput and backend-owned batching/submission evidence without fabricating unsupported counters.
- [ ] Final performance evidence names physical hardware, OS, adapter and run parameters; CI compile evidence is not represented as GPU-performance evidence.
- [ ] Decision is exactly `ADOPT_CUSTOM_WGPU`, `ADOPT_BEVY` or `INSUFFICIENT_EVIDENCE`, with caveats.

## Excluded scope

No production engine migration, Bevy gameplay-ECS adoption, server/protocol/content authority change, permanent asset format, KTX2/DDS choice, atlas-vs-array freeze, live deployment, proprietary asset publication or performance claim from hosted CI.

## Implementation / findings

The experiment is isolated under `experiments/graphics-engine-bakeoff/`. A temporary branch-scoped compile workflow may qualify source compatibility but must not alter protected merge-gate semantics. Physical GPU measurements belong to named hardware evidence.

## Validation

### Focused

- command/run: pending
- result: pending

### Component/integration

- command/run: pending
- result: pending

### E2E

- scenario: physical renderer bake-off on named Windows hardware
- result: pending

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

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: none found at allocation
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: issue #465 and dedicated branch allocated
status: implementing
branch: agent/graphics-engine-bakeoff-465
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
next_action: implement isolated shared workload and both renderer candidates
```
