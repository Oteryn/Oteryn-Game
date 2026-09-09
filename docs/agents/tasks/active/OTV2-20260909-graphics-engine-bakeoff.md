# OTV2-20260909-graphics-engine-bakeoff

```yaml
task_id: OTV2-20260909-graphics-engine-bakeoff
title: Qualify Oteryn graphics engine backend
mode: IMPLEMENT
status: ready_for_review
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/graphics-engine-bakeoff-465
pr: 468
base_sha: 0ca0f6d257d6eb982c4ff9d05bc9a84e44ba48da
head_sha: beaa169095ad3f1ed781c4a60354ef4f7b152b14
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-gpt5.6-sol
created_at: 2026-09-09T10:02:53Z
updated_at: 2026-09-09T11:03:00Z
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

**Measured bounded verdict: `ADOPT_CUSTOM_WGPU`.** Use the project-owned Rust/`wgpu` renderer as the foundation for the next native-client rendering slices. Do not adopt Bevy as the primary client engine/rendering ownership layer on the measured evidence. This verdict does not freeze asset-container, atlas/array, streaming, animation-schema, final UI or World Bundle decisions and grants no live deployment authority.

## Architecture and source of truth

- `PROVEN`: branch was reconciled by ordinary merge with protected `main@0ca0f6d257d6eb982c4ff9d05bc9a84e44ba48da`; after reconciliation it was zero commits behind that protected base.
- `PROVEN`: production workspace remains Rust 1.94.0 with direct `wgpu = 30.0.0`; the production `apps/client`, root Cargo dependency graph and `crates/renderer` were not mutated by the experiment.
- `PROVEN`: `GRAPHICS_APPEARANCE_ANIMATION_AND_SEASONALITY_HORIZON_NOTE.md` explicitly defers permanent renderer texture/container choices until representative evidence exists.
- `PROVEN`: the isolated workspace pins Rust 1.95.0 only for the Bevy comparison and resolves custom `wgpu 30.0.0` plus Bevy 0.19.1's separate `wgpu 29.0.4` graph.
- `DERIVED`: the physical results are sufficient to choose the renderer foundation while preserving later presentation/asset technology gates.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this task performs no production mutation, authority-bearing session operation, persisted recovery interpretation, PREPARE/COMMIT, controller installation or live deployment.

## Acceptance criteria

- [x] Both backends consume the same benchmark-only `RenderSnapshot` and deterministic scenario generator.
- [x] BASIC, NORMAL and STRESS scenarios are available at 32, 64 and 128 pixel sprite densities.
- [x] Synthetic art is generated in memory; no proprietary Tibia art is committed.
- [x] Production root Rust 1.94/Cargo dependency graph, `apps/client` and `crates/renderer` remain unchanged.
- [x] Both candidates compile in an isolated Rust 1.95 nested workspace.
- [x] Output records comparable frame-time percentiles, throughput and backend-owned batching/submission evidence without fabricating unsupported counters.
- [x] Final performance evidence names physical hardware, OS, adapter and run parameters; CI compile evidence is not represented as GPU-performance evidence.
- [x] Decision is exactly `ADOPT_CUSTOM_WGPU`, `ADOPT_BEVY` or `INSUFFICIENT_EVIDENCE`, with caveats.

## Excluded scope

No production engine migration, Bevy gameplay-ECS adoption, server/protocol/content authority change, permanent asset format, KTX2/DDS choice, atlas-vs-array freeze, live deployment, proprietary asset publication or performance claim from hosted CI.

## Implementation / findings

The experiment is isolated under `experiments/graphics-engine-bakeoff/` and uses one backend-neutral deterministic workload. The custom candidate owns a shared atlas, storage instance buffer and one draw submission; the Bevy challenger uses minimal 2D features and presentation-only Sprite entities. Baseline `z` sorting was neutralized so stack-ordering cost is not paid by only one candidate.

Physical matrix on Molehill-PC:

- Windows 11 Pro `10.0.26200` build `26200`;
- AMD Ryzen 7 9800X3D;
- AMD Radeon RX 9070 XT;
- driver `32.0.31035.1003`;
- DX12 / high-performance adapter / Ultimate Performance;
- 1920x1080;
- 180 warm-up + 1,200 measured frames;
- 5 repetitions for each of 18 backend/scenario/density combinations;
- **90/90 successful final measurements; zero retries**.

Measured ranges across all nine comparable cells:

- Bevy/custom p95 frame-time ratio: **3.45x–9.20x**;
- Bevy/custom p99 frame-time ratio: **3.02x–9.39x**;
- custom/Bevy mean-throughput ratio: **4.80x–11.64x**;
- Bevy/custom peak working-set ratio: roughly **1.60x BASIC**, **1.77–1.79x NORMAL**, **2.06–2.08x STRESS**;
- Bevy/custom startup: roughly **1.30–1.38x**;
- clean release build: custom `23.436 s`, Bevy `140.405 s`;
- executable: custom `5,400,064 B`, Bevy `55,859,712 B`.

Representative STRESS medians:

- 32 px p95: custom `0.708 ms`, Bevy `6.510 ms`; RAM `166.7 MiB` vs `344.1 MiB`;
- 128 px p95: custom `0.939 ms`, Bevy `6.498 ms`; RAM `166.7 MiB` vs `347.1 MiB`.

An earlier no-delay harness saw one Bevy DX12 `ResizeBuffers`/invalid-surface failure after aggressive process teardown/restart. It is retained as a bounded reliability observation only. The hardened final matrix with a 1-second inter-process delay completed all measurements on attempt 1.

Full results and caveats: `experiments/graphics-engine-bakeoff/RESULTS.md`.

## Validation

### Focused

- command/run: `cargo +1.95.0 test -p oteryn-graphics-bakeoff-shared`
- result: **PASS, 3/3** on Molehill-PC.

### Component/integration

- command/run: release build of both isolated renderer candidates plus `cargo +1.95.0 clippy --workspace --all-targets -- -D warnings`
- result: **PASS** on Molehill-PC.

### E2E

- scenario: physical renderer bake-off on Molehill-PC, 2 backends × 3 scenarios × 3 densities × 5 repetitions
- result: **PASS, 90/90 successful measurements, zero retries** in final hardened run.
- raw evidence: `experiments/graphics-engine-bakeoff/evidence/molehill-20260909/raw.jsonl`
- raw SHA-256: `1a115de0f3b4629a7181d2922a12b1d3725bf0f948f4b4f0bd0d071d5fe72388`
- summary SHA-256: `b10ff724dfac5a659c3a1ef3ce33483a5f2eefcc79eb397823868cd773c1b16b`

### Exact-head CI

- final head: pending after this task checkpoint commit
- trigger source: branch push; workflow explicitly covers experiment, workflow and this task-record path
- workflow: `Graphics engine bake-off compile`
- classification: compile/format/shared-tests/strict-Clippy qualification only; not physical GPU performance evidence
- result: pending exact-head completion

## Self-review

- exact head: pending after this task checkpoint commit
- method/reviewer: whole-diff self-review by implementing/coordinating agent
- material findings: pending final changed-file review
- verdict: pending

## Independent review

- required: yes before protected integration unless repository policy classifies this non-production evidence path otherwise
- exact head: pending
- method/auditor: not self-claimed
- material findings: pending
- verdict: pending

## PR and closeout

- PR: #468 `test(graphics): record wgpu vs Bevy engine bake-off`
- changed-file review: pending final exact-head review
- unresolved review threads: pending
- related/superseded PRs: none found at allocation
- protected auto-merge: not enabled by this task
- merge commit/result: pending repository control plane
- ownership release: pending protected-main integration/readback

## Context checkpoint

```yaml
last_progress: physical matrix completed 90/90; evidence committed; branch reconciled with fresh main; PR #468 open
status: ready_for_review
branch: agent/graphics-engine-bakeoff-465
head_sha: beaa169095ad3f1ed781c4a60354ef4f7b152b14
pr: 468
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: branch_push
ci_check_generation: task-checkpoint-final
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: expected_windows_2025
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: exact_head_ci_and_independent_review
next_action: whole-diff self-review, exact-head CI readback, independent review and normal protected integration path
```
