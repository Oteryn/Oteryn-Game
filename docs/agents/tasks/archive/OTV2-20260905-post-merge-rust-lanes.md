# OTV2-20260905-post-merge-rust-lanes

```yaml
task_id: OTV2-20260905-post-merge-rust-lanes
title: Scope protected-main post-merge Rust lanes
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
issue: 304
pr: 305
base_sha: 187c6b83c6945d79aabef2c5730c3ddba13fcab1
head_sha: 865443ca75e980ab4d216dc843960ff0a92f805b
final_head_sha: 865443ca75e980ab4d216dc843960ff0a92f805b
owner: null
created_at: 2026-09-05T11:33:03Z
updated_at: 2026-09-05T13:25:28Z
execution_budget_minutes: 120
large_budget_reason: Full hosted CI, independent review, protected integration and actual post-merge measurements
owned_paths:
  - .github/workflows/rust.yml
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_post_merge_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260905-post-merge-rust-lanes.md
public_contracts: []
depends_on:
  - issue: 283
    state: completed
external_repositories: []
```

## Outcome and authority

Owner explicitly authorizes this single bounded task and normal protected integration/readback. Live Issue #304/PR #305 own mutable status and final evidence. #283 is DONE and released its allocation. No new programme or framework. One isolated writable checkout; workflow/classifier/regressions are coupled and implemented serially, with independent read-only deep review on the stable candidate.

## Acceptance criteria and excluded scope

Only a verified protected-main push with complete Git before/after ancestry and server-only proof may omit Windows/SIM. Reuse #283 graph, all dependency kinds and audited consumer snapshot. Full fallback for unknown/mixed material inputs, missing/invalid event or history, Cargo/build/control plane, changed consumers and classifier failures. Manual always FULL; Linux/PG/policy/supply chain always run. Remove dormant SIM job while preserving actual Windows golden execution. Preserve PR gate, full Merge Queue, ruleset and required status byte/semantic contracts. No product/runtime/schema, protected settings, secret, remote host or cross-repository mutation.

## High-risk authority/recovery qualification

NOT_APPLICABLE to runtime authority/recovery matrices: this task changes standalone CI selection, not session/controller/persistence behavior. CI trust boundary and negative families are explicitly covered by the real-Git adapter fixtures and reviewed workflow pin.

## Validation

Fresh RED: missing-adapter assertion on original protected code, preserved in remote commit `52a7852be5e1313d5c0c2c75f9200dbc308594c1`. Minimal GREEN adds only the post-merge adapter and uses existing classifier semantics. A second RED proves the missing workflow regression contract before its addition.

Independent review on `5183ef36f71960505ed7d33250b7bae818122e7b` verified one P2: branch-wide concurrency still replaces pending runs despite `cancel-in-progress: false`. Fresh RED rejected that group before minimal removal of the concurrency block. Regression now forbids cross-run cancellation of either manual or push qualification; full workflow pin family rejects reintroduction. The repair does not change classification or evidence commands.

Focused checks: new real-Git fixtures cover protection/event/SHA bindings, disconnected or absent before SHA, full 301-file enumeration, bad/truncated diff, dependency/client/shared/SIM changes, cross-consumer rename, special file modes, stale consumer snapshot and actual shell fallback. Existing #283 PR classifier/aggregate/mode/job mutation families remain required, along with existing PR/MQ PG/SIM regressions and repository/governance checks.

Exact-head full GitHub CI, whole-diff self-review and one independent deep review are required before normal Merge Queue integration. Actual post-merge hosted evidence and timing are required after readback; clearly distinguish observed runs from historical replay/inferred savings. No synthetic runtime changes solely to manufacture a benchmark. Final SHA and CI/review evidence stay external; no self-referential metadata commits.

## Self-review and independent review

Whole-diff author review is mandatory. Independent deep review is required because GitHub Actions execution/selection is control-plane risk. Verified findings are repaired test-first with a family sweep; cosmetic changes do not independently demand another review.

## PR and closeout

Normal protected Merge Queue only, then exact protected-main tree/workflow/ruleset readback and actual post-merge measurement. Delete task branch and release ownership on terminal acceptance. Task archival follows terminal merge and release, preserving historical evidence.

## Context checkpoint

The implementation and measured benchmark are accepted; the historical allocation above is released. No implementation next action remains. Issue #304 final closure is conditional on this terminal archive's normal protected integration/readback and applicable post-merge CI.

## Terminal implementation and benchmark evidence

PR #305 integrated the independently reviewed implementation through full Merge Queue at `6295e4079a53cc95a3021e5c34b9004b2e9bd50c`. Its actual control-plane post-merge run `33964871201` correctly executed FULL: 14.1500 allocated runner-minutes and 8.5833 wall-minutes, PostgreSQL 124/124 and Windows SIM 7/7.

The owner subsequently authorized exactly one permanent server-only validation prerequisite. PR #306 added only `apps/game-server/tests/server_ci_qualification.rs`, covering the actual executable's smoke success and fail-closed default/near-miss CLI invocations without changing runtime behavior. The test remains in the repository. Exact reviewed head `a2b5dcc01fb81f2fa6e0275e3b2a0e8a58568e72` passed focused qualification, negative sensitivity, Clippy, format, regression families, self-review and independent review. Required PR run `33967983424` and full Merge Queue `33968249032` passed. It merged at `62590071b7e47e3221af0e180c73bbc7cdf37c31`, tree `65e4af400821a2c34ad27d727336d3ecd099ca1f`, identical to the reviewed candidate.

Actual protected-main push run `33968711222` completed successfully on 2026-09-05, 13:22:31-13:25:28 UTC: **2.9500 wall-minutes, 5.9667 allocated runner-minutes**. Trusted classifier job `101313293874` selected server-only. Linux `101313293879` passed including the permanent tests; PostgreSQL `101313293853` passed 124/124 against real 17.6; Windows/SIM `101313326088` was skipped with no runner or steps. Five allocated job durations total 358 seconds; skipped-job placeholder timestamps are excluded. The complete protected before/after diff contains only the reviewed test addition. Classifier/Cargo trust inputs, PR/MQ workflows, required fan-in and ruleset semantics are unchanged; MQ and manual dispatch remain FULL, with no bypass actors.

Compared with actual server FULL baselines `33962958617` and `33961442773`, allocated runner time decreased by 54.85% and 58.28% respectively. These are separate observed hosted samples, not a controlled A/B experiment, a future performance guarantee or billed-cost measurements. Author and independent reviewer both verified the actual GitHub evidence.

Exact run/job timestamps, comparison method, safety readback and final lifecycle evidence are authoritative in [Issue #304](https://github.com/Oteryn/Oteryn-Game/issues/304#issuecomment-5552136169). Do not add self-referential head/readiness commits to this archived packet.
