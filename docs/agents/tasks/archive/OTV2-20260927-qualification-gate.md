---
task_id: OTV2-20260927-qualification-gate
title: Require the physical server qualifications in the merge gate when they apply
mode: IMPLEMENT
status: completed-on-merge
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 52b983fa
branch: agent/qualification-gate-20260927
issue: 162
jira: KAN-13
allocation_comment: 5859647569
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/node-boot-qualification.yml
  - .github/workflows/gameplay-server-seam.yml
  - tools/repository/classify_pr_test_lanes.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/archive/OTV2-20260927-qualification-gate.md
---

# Require the physical server qualifications in the merge gate when they apply

Owner decision (this session): a red node boot must block merging, but the physical qualifications must not run for changes that cannot affect them.

## Defect

The node boot and Server Seam qualifications ran as standalone, path-filtered workflows outside `game-gate`, so neither blocked auto-merge nor the merge queue. #1010 merged with a red node boot as a result. Their narrow path filters also skipped changes that reach the shipped server; #998, for example, changed durability without running node boot.

## Outcome

- **Classifier.** `server_qualification_required` in the trusted-base classifier selects the physical qualification from the complete changed-file evidence, including rename sources. The classifier emits it as `server_qualification`. It fails closed on incomplete or malformed evidence.
- **Selected paths.** The lane runs when a change touches:
  - `apps/game-server/src/**`, except `ability/`, `ai/`, `interaction/`, `combat.rs` and Content authoring;
  - the native entry room Content, its WorldProject/v2 model and its activation;
  - `apps/game-server/migrations/**`;
  - `tools/qualification/**`;
  - `vendor/**`;
  - Cargo/toolchain inputs;
  - `merge-gate.yml`.
- **Merge gate.** Two new jobs, `Merge gate / Node boot against the real Platform` and `Merge gate / Server Seam over TCP+TLS`, run when the lane is selected. They check out the exact head and the pinned Platform producer. `Merge gate / validate` requires both to succeed unless the lane is explicitly `false`; an older classifier that emits nothing therefore still requires them. `game-gate` is unchanged.
- **Standalone workflows.** Both standalone workflows keep manual `workflow_dispatch` only, so a PR never runs them twice.
- **Pins and tests.** The repository policy pins the new job blocks and the changed `lanes` and `validate` blocks, and requires the new job names. Classifier and aggregate regressions cover both selection and fail-closed behaviour.

Excluded: no merge-group change and no change to the required-status configuration. `game-gate` remains the only required check.
