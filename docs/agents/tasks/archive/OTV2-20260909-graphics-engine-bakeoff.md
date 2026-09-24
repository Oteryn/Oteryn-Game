# OTV2-20260909-graphics-engine-bakeoff

```yaml
task_id: OTV2-20260909-graphics-engine-bakeoff
title: Qualify Oteryn graphics engine backend
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/graphics-engine-bakeoff-465
pr: 468
base_sha: 0ca0f6d257d6eb982c4ff9d05bc9a84e44ba48da
head_sha: d16e83c170e059bcdd7ebdba8b5b4171e72f5d94
final_head_sha: d16e83c170e059bcdd7ebdba8b5b4171e72f5d94
final_head_frozen_at: null
owner: chatgpt-gpt5.6-sol
created_at: 2026-09-09T10:02:53Z
updated_at: 2026-09-09T14:23:50Z
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

`final_head_frozen_at` remains `null` rather than inventing a retrospective timestamp. The exact final source head is established by immutable PR/review/check evidence and is recorded above post-merge under the repository closeout rule.

## Outcome

Produce reproducible evidence for Issue #465 comparing a custom Rust/wgpu presentation backend with a Bevy challenger using the same benchmark-only Oteryn render workload. The task does not select or activate a production engine by itself.

**Measured bounded verdict: `ADOPT_CUSTOM_WGPU`.** Use the project-owned Rust/`wgpu` renderer as the foundation for the next native-client rendering slices. Do not adopt Bevy as the primary client engine/rendering ownership layer on the measured evidence. This verdict does not freeze asset-container, atlas/array, streaming, animation-schema, final UI or World Bundle decisions and grants no live deployment authority.

## Architecture and source of truth

- `PROVEN`: production workspace remained Rust 1.94.0 with direct `wgpu = 30.0.0`; the production `apps/client`, root Cargo dependency graph and `crates/renderer` were not mutated by the experiment.
- `PROVEN`: `GRAPHICS_APPEARANCE_ANIMATION_AND_SEASONALITY_HORIZON_NOTE.md` defers permanent renderer texture/container choices until representative evidence exists.
- `PROVEN`: the isolated workspace pins Rust 1.95.0 only for the Bevy comparison and resolves custom `wgpu 30.0.0` plus Bevy 0.19.1's separate `wgpu 29.0.4` graph.
- `PROVEN`: exact final source head is `d16e83c170e059bcdd7ebdba8b5b4171e72f5d94` and changes exactly 23 dedicated task/workflow/experiment paths.
- `PROVEN`: PR #468 integrated through native FULL Merge Queue as `4d06bad1c0d21f2237df290865be55d8f7ed4f02`; protected `main` readback matched that commit exactly.
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

- final source head: `d16e83c170e059bcdd7ebdba8b5b4171e72f5d94`
- Agent Governance run `34345091288`: **SUCCESS**.
- Architecture Semantic Audit run `34345021473`: **SUCCESS**.
- Merge Gate run `34345091270` / #1933: **SUCCESS**.
- FULL Merge Queue run `34362373199` / #138 on integrated candidate `4d06bad1c0d21f2237df290865be55d8f7ed4f02`: **SUCCESS**, including supply chain, dependency review, CodeQL actions/python, Linux workspace, Windows client, governance and real Durability PostgreSQL harness.

## Self-review

- exact head: `d16e83c170e059bcdd7ebdba8b5b4171e72f5d94`
- method/reviewer: whole-diff implementing/coordinating-agent review, persisted as PR COMMENT review `5155516366`
- material findings: **0**
- verdict: **PASS**

The earlier self-review on `4cf789b...` remained applicable to experiment bytes because the two later commits imported only unrelated Atlas workflow/test paths; the final `d16e83c...` review revalidated the exact 23-path candidate and current-main drift.

## Independent review

- required: **NO** under the protected risk policy for this isolated non-production benchmark/evidence delivery; it changes no authentication/session/protocol/persistence/value/production authority and weakens no safety gate.
- automated architecture/governance checks are supporting evidence and are not relabeled as an independent human/agent review.

## PR and closeout

- PR: #468 `test(graphics): record wgpu vs Bevy engine bake-off`
- changed-file review: **PASS**, exactly 23 dedicated paths
- unresolved review threads: **0**
- related/superseded PRs: none
- integration: native FULL Merge Queue #138 / Actions `34362373199`
- merge commit/result: `4d06bad1c0d21f2237df290865be55d8f7ed4f02`
- protected-main readback: `main == 4d06bad1c0d21f2237df290865be55d8f7ed4f02`
- ownership release: this archive move removes the active task lock when the bounded closeout PR becomes protected

## Context checkpoint

```yaml
last_progress: PR #468 integrated through FULL Merge Queue #138 and protected-main readback matched 4d06bad1c0d21f2237df290865be55d8f7ed4f02
status: completed
branch: agent/graphics-engine-bakeoff-465
head_sha: d16e83c170e059bcdd7ebdba8b5b4171e72f5d94
pr: 468
final_head_sha: d16e83c170e059bcdd7ebdba8b5b4171e72f5d94
final_head_frozen_at: null
ci_trigger_source: pull_request_and_merge_group
ci_check_generation: terminal
ci_checks_for_current_head: 3
ci_run_ids:
  - 34345091288
  - 34345021473
  - 34345091270
  - 34362373199
ci_job_ids: []
runner_assignment_state: terminal_success
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: protect this bounded archive move, then close Issue #465 as completed
```
