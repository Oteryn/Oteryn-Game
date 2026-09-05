# OTV2-20260905-post-merge-doc-lanes

```yaml
task_id: OTV2-20260905-post-merge-doc-lanes
title: Scope protected-main neutral-documentation runtime lanes
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/post-merge-doc-lanes
issue: 311
pr: null
base_sha: d89f063ea7ad7f1d8fa09688309c8d898fae856e
head_sha: null
final_head_sha: null
owner: Codex-postmerge-doc
created_at: 2026-09-05T15:28:43Z
updated_at: 2026-09-05T15:28:43Z
execution_budget_minutes: 120
large_budget_reason: Independent review, hosted FULL qualification, normal Merge Queue and natural postdeployment measurement
owned_paths:
  - .github/workflows/rust.yml
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_post_merge_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/active/OTV2-20260905-post-merge-doc-lanes.md
public_contracts: []
depends_on:
  - issue: 309
    state: integrated
external_repositories: []
```

## Outcome and authority

Live Issue #311 owns task status and exact head; programme #308 owns overall status and measured ranking. Predecessor #309 integrated through PR #310 and FULL Merge Queue run `33973968237`; its mutating allocation was explicitly released. Its natural docs benchmark remains separate pending evidence, not evidence of this task's success. One worker owns this branch/worktree; the coordinator owns publication, independent review and protected integration.

## Design and trust proof

Reuse the canonical classifier and existing protected-main adapter. A normal protected push, exact checkout, complete before/after ancestry, complete Git diff and safe tree modes remain prerequisites. Pass the existing reviewed all-consumer digest into classification. Accept only neutral docs `false/false` or server/durability `true/false`; other surfaces remain FULL. PR classification and reviewed snapshot values are unchanged.

Classification writes into a fresh private file. The shell publishes only byte-exact complete canonical rust/windows boolean pairs. Failed classifier execution, partial/empty/duplicated/unknown/malformed/contradictory output becomes explicit `true/true`. This validates the output wire; it does not introduce another classifier or source of risk authority. No candidate cache/artifact or historical execution is consumed as proof.

Linux and PostgreSQL skip only after successful classification with both booleans exactly false. Windows keeps its existing strict successful-false fallback. Policy/supply-chain jobs remain unchanged, independent and always enabled for existing push/manual events. Manual, malformed and unavailable classification remain FULL. No workflow-level path filters or protected-run cancellation are added.

## Acceptance criteria and excluded scope

- Neutral-doc modification and rename can omit all three runtime jobs while retaining policy/supply-chain and other always-required authority/security/governance workflows.
- Server plus neutral docs retains Linux/PostgreSQL; mixed material surfaces, stale/new consumers and uncertain inputs select FULL.
- Actual shell tests reject failed, incomplete or incoherent output and accept all three canonical pairs.
- Existing PR classifier, post-merge real-Git, policy and governance regressions pass; whole-diff author review and independent stable-head deep review are required.
- Fresh exact-head FULL PR qualification, normal FULL Merge Queue, protected-main readback and natural postdeployment docs/FULL measurements remain required.

No PR workflow/fundamental fan-in, Merge Queue workflow/blob, protected audit, ruleset, required status, cache, runtime/product/Cargo or oracle changes. Only the existing standalone Rust workflow pin rotates. No product/no-op benchmark probe or unrelated PR integration is authorized.

## Measured tradeoff

PROVEN baseline: natural docs main run `33973093609` on `bc9f5dac5642b56135cce31f91b9ed23e5258a70` allocated 888 seconds. Runtime jobs consumed 818 seconds (Linux 183, PostgreSQL 103, Windows 532); retained jobs consumed 70 seconds (classification 12, policy 21, supply chain 37). Another historical docs sample allocated 845 seconds. Source and method: Issue #311 comment `5552795267` and programme #308.

INFERENCE: docs runtime omission has positive expected allocated-time value while reviewed snapshots remain current. Linux/PostgreSQL now wait for classification on FULL/server runs; the measured 12-second classifier allocation indicates a possible serial startup penalty, not a precise wall-time regression. Queue delay, dependencies and the actual critical path affect the observed result. No controlled A/B, achieved saving, monthly projection or net critical-path gain is claimed. Natural postdeployment qualification remains UNKNOWN until observed.

## High-risk authority/recovery qualification

NOT_APPLICABLE to runtime session/lease/controller/persistence matrices: only CI selection changes. Protected-code provenance and fail-closed output handling are material control-plane risks requiring independent review. No authority expansion is introduced.

## Validation and review

Baseline post-merge suite passed. Fresh RED: a real-Git neutral-doc range returned FULL on the original adapter. Minimal adapter GREEN then reached a separate RED because failed metadata emitted only `windows=true`. Shell/output implementation reached the reviewed workflow-pin RED before deliberate pin rotation. Negative families cover malformed identity/event/history, both boolean results, docs rename, changed/new server and non-server consumers, mixed build/control/unknown inputs, all valid wire pairs, failed toolchain/metadata/classifier and invalid wire representations. Reviewed workflow mutation checks reject weakened per-job result/event/dependency guards.

Final local checks, exact candidate, self-review, independent review and hosted evidence are recorded on Issue #311. A commit cannot contain its own SHA: no self-referential checkpoint commit. Local GREEN does not establish completion or actual hosted savings.

## Context checkpoint

```yaml
last_progress: Fresh RED families followed by minimal adapter and output-wire implementation
status: validating
branch: ci/post-merge-doc-lanes
head_sha: null
pr: null
final_head_sha: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Publish the verified material candidate for independent exact-head review and hosted FULL qualification
```
