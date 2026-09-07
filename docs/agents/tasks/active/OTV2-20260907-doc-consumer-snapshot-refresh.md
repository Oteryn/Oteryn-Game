# OTV2-20260907-doc-consumer-snapshot-refresh

```yaml
task_id: OTV2-20260907-doc-consumer-snapshot-refresh
title: Refresh audited neutral-document consumer snapshot
mode: REPAIR
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: ci/doc-consumer-snapshot-current-refresh-w6
issue: 375
pr: null
base_sha: a6f69427d663539c6a8e23f166e69147b66ec078
integration_main_sha: 686cf85cac3e7f66a473527c198705466dc5c3bd
successor_base_sha: 15164c38a2775e45eaff4001fddddbabf4b63ab6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Game-W6
created_at: 2026-09-07T09:00:59Z
updated_at: 2026-09-07T09:10:10Z
execution_policy: continuous_progress
owned_paths:
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/tasks/active/OTV2-20260907-doc-consumer-snapshot-refresh.md
public_contracts: []
depends_on:
  - issue: 308
    state: first_wave_frozen_and_released
blocks: []
cross_repository_coordination_id: Oteryn/Oteryn#142-W6-GAME
external_repositories: []
```

## Outcome

Restore the existing conservative neutral-document routing only for the newly reviewed protected Game tree. This repair changes one production snapshot constant and its independent regression evidence. It does not change the classifier algorithm, workflow fan-in, required checks, product code or runtime behavior.

## Architecture and source of truth

- `PROVEN` — protected admission `main` is `a6f69427d663539c6a8e23f166e69147b66ec078`; live Issue #375 owns this repair.
- `PROVEN` — prior audited source `b9b1a4317858bffc25ad6af3cffcf7b5eff93445` has non-server digest `9f7aff4dc25c9c6561b77ea73342b675eeccb1d008ab9d1fbdbd504618ec5ab8` over 63 records and all-consumer digest `f8eed774249df64a5a64612b4a169a73bac093a7bcbfb21e59ea0e06dd2ddc26` over 124 records.
- `PROVEN` — admission `main` retained the exact same 63-record non-server digest and advanced the all-consumer digest to `742350c55587ab94d652e27a4196308f350afaf5140ed3633d33d5d165e807b6` over 132 records.
- `PROVEN` — while the first draft was published, protected `main` advanced normally through AI #369 and Ability #370 to `728f25461d5a2b029ed60f7db4b14151d31776d7`. The branch was merged up before final freeze.
- `PROVEN` — intermediate `main` at `728f254…` still had the identical 63-record non-server digest and all-consumer digest `051473d37842a816e9378c9769fd28cf9c7ddb49fb5483d02b3841d73c2cd403` over 132 records.
- `PROVEN` — from the prior audited source through that intermediate main, the selected source delta was exactly 20 `apps/game-server` Rust paths, +15,005/-7, eight added and twelve modified. Cargo, build inputs and non-server package trees were unchanged. The final server increment was six modified server Rust paths, +206/-6.
- `PROVEN` — independent lexical scans of both the original 14-path delta and the six-path upstream increment found no filesystem, document, Markdown or workspace-boundary reads. The only original broad-`open` match was Tokio `Builder::new_current_thread`, which is not file access.
- `PROVEN` — before final integration, protected `main` advanced once more through native-client #368 to `686cf85cac3e7f66a473527c198705466dc5c3bd`. Its selected-input delta is exactly two modified non-server Rust paths, `apps/client/src/windows_shell.rs` and `crates/identity/src/lib.rs`, +130/-17. Neither adds a filesystem, document, Markdown or workspace-boundary read.
- `PROVEN` — final reviewed `main` has 63 non-server records at `669052b19bf8d067d949623ba566e087b43def40b2661f8c3feb16f3d62de180` and 132 all-consumer records at `dc9381615fb7b1f7b06533ac6f6ffebb6d274b8d799894fe54882bf69f638472`. From the original audited tree this is exactly 22 selected Rust paths, +15,135/-24; Cargo/build inputs remain unchanged.
- `DERIVED` — the audited server, client and identity changes do not introduce a neutral-document consumer; adopting both current digests restores the already designed routing contract without changing its algorithm or modeled inputs.
- `PROVEN` — later consumer-tree advances remain safe: either digest mismatch selects FULL. Optimization freshness may be lost, but safety and required checks remain; this candidate will not chase unrelated future merges absent an actual safety/correctness conflict.

Issue #308 remains the measured-impact programme. Released Issue #309 and its task packet remain historical first-wave evidence and are not reactivated or edited.

PR #377 passed exact-head FULL CI and independent review, entered the normal Merge Queue and had already integrated as protected main `15164c38a2775e45eaff4001fddddbabf4b63ab6` at `2026-09-07T09:22:52Z` before a later draft-conversion response was received. Direct main readback corrected the transient assumption that conversion had stopped the completed queue operation. Auto-deletion of the merged branch caused the subsequent update-ref attempt to return GitHub `422 Reference does not exist`; that was lifecycle evidence, not a candidate or gate failure. The merged `051473d3…` snapshot safely selects FULL against the already-integrated client/identity tree. This clean successor carries the authorized two-constant refresh from actual protected main without rewriting #377 history.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: no runtime session, lease, generation, controller, persistence, production or mutation authority changes. The trusted CI selection boundary is material and therefore retains fail-closed regressions, independent exact-head review, FULL candidate qualification and normal Merge Queue integration.

## Acceptance criteria

- [x] RED proves the newly reviewed digest remains FULL with `unreviewed-document-consumer-inputs` before the production constant changes.
- [x] GREEN changes only the production document-consumer snapshot constant and independent regression literals.
- [x] Historical/superseded all-consumer snapshots `f8eed774…`, `742350c5…` and `051473d3…` are explicitly rejected as stale after GREEN.
- [x] The superseded non-server snapshot `9f7aff4d…` is explicitly rejected as stale after GREEN.
- [x] Existing classifier, post-merge, governance and repository-policy regressions pass.
- [x] Whole-diff self-review found no unresolved material issue; independent deep review remains pending.
- [ ] Exact-head FULL CI, normal Merge Queue and protected-main readback pass.

## Excluded scope

No workflow, aggregate fan-in, ruleset, Merge Queue, branch protection, Cargo, runtime, product test, #308/#309/#311 lifecycle record, benchmark probe, no-op documentation PR, production, credential or external-repository change. No achieved time, token, API-cost or runner-cost saving is claimed from this snapshot repair.

## Implementation / findings

Focused RED on the initial test-only change failed at the expected neutral-document assertion with `rust=True`, `windows=True`, `surface=docs`, `reason=unreviewed-document-consumer-inputs`. After the two protected-main advances, second and third focused RED generations reproduced the same expected failure for the re-audited digest pairs against each preceding production snapshot. Final minimal GREEN updates the existing non-server and all-consumer constants. Existing negative fixtures continue to vary new and modified server/non-server records and require FULL.

## Validation

### Focused

- command/run: `python tools/repository/test_classify_pr_test_lanes.py`
- result: PASS after the recorded RED

### Component/integration

- command/run: both classifier suites, governance and repository policy
- result: PASS

### E2E

- scenario: `NOT_APPLICABLE` because this repair changes no product/runtime behavior; hosted classifier execution and exact-head FULL CI are required instead.
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pull request
- workflow/run/job: pending
- runner assignment: pending
- classification: FULL required because the candidate changes `tools/repository/`
- result: pending

## Self-review

- exact head: recorded externally after commit
- method/reviewer: Game-W6
- material findings: none
- verdict: PASS

## Independent review

- required: YES — material trusted CI selection snapshot
- exact head: pending
- method/auditor: one independent deep review after stable publication
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: PASS for the complete three-path candidate
- unresolved review threads: pending
- related/superseded PRs: #310 and #312 are integrated predecessors; #377 is the integrated stale-safe first refresh; none is resumed or rewritten
- protected auto-merge: pending normal Merge Queue
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: direct readback corrected the #377 integration race and a clean successor was prepared from protected main 15164c38
status: validating
branch: ci/doc-consumer-snapshot-current-refresh-w6
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: 0
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
next_action: rerun validation, freeze and publish the clean successor for exact-head review and FULL CI
```
