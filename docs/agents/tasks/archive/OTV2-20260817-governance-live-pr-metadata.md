# OTV2-20260817-governance-live-pr-metadata

```yaml
task_id: OTV2-20260817-governance-live-pr-metadata
title: Eliminate stale PR metadata dependence from Agent governance
mode: FIX
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: fix/governance-live-pr-metadata
pr: 320
base_sha: 65b5711fc56f80c0407ce462e83cdc973535636f
final_head_sha: 400b7de91221f1833a4bfe3e7cd0659f8b7a056d
merge_sha: 592ed69487c324e68d7ed50c3311e5682d85e894
owner: governance repair agent
created_at: 2026-08-17T09:22:00+02:00
completed_at: 2026-08-17T09:49:00+02:00
cross_repository_coordination_id: OTERYN-GOV-LIVE-PR-METADATA-20260817
owned_paths_released:
  - .github/workflows/agent-governance.yml
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy.py
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/tasks/active/OTV2-20260817-governance-live-pr-metadata.md
```

## Outcome

Completed and merged. Pull-request metadata validation is self-healing for ordinary title/body edits without weakening exact-head controls.

- `Agent governance` handles `opened`, `reopened`, `synchronize`, and `edited`, resolves the PR number plus expected event head, then fetches live title/body/state/base/head repository/head SHA from the GitHub API before checking out code.
- A live head mismatch fails closed.
- `Merge gate / validate`, the protected aggregate status, also handles `edited`, so metadata-only changes automatically invalidate/recompute the required status without a dummy commit or manual `workflow_dispatch`.
- `workflow_dispatch` remains only the existing exact-head break-glass path for standalone Agent governance; it was not added to the aggregate merge gate.
- The trusted-base merge-gate implementation remained unchanged outside the five-line activity-type trigger block; canonical scope and aggregate-job hashes stayed pinned.

## Regression evidence

A deliberate metadata-only edit of PR #320 was performed while the head remained exactly `400b7de91221f1833a4bfe3e7cd0659f8b7a056d`.

Fresh runs created by that `edited` event:

- Agent governance run `32006480653` / `Agent governance / validate`: PASS.
- Merge gate run `32006480651` / `Merge gate / validate`: PASS.
- The previous in-progress merge-gate generation `32006435686` was cancelled by same-PR concurrency, proving stale validation is superseded.
- Merge-gate sub-jobs PASS: exact PR scope, live metadata governance, repository policy, dependency review, CodeQL Actions, CodeQL Python, Rust policy/metadata, Linux build + strict Clippy + tests + synthetic harness, Windows production build + strict Clippy + visible smoke + synthetic harness, and cargo-deny supply chain.
- Merge Authority Audit run `32006435680`: PASS on the exact final head.
- Architecture Semantic Audit run `32007114564`: PASS on the exact final head after ready-for-review transition.
- Review submissions: 0; unresolved review threads: 0.
- Final pre-merge compare: `behind_by=0` against live `main`.

## Merge and live protection readback

PR #320 was squash-merged with expected head `400b7de91221f1833a4bfe3e7cd0659f8b7a056d` to merge commit `592ed69487c324e68d7ed50c3311e5682d85e894`.

The owner explicitly authorized the bounded break-glass path if required. No live ruleset mutation was ultimately necessary: GitHub accepted the exact-head merge directly after all required checks passed. Post-merge readback confirmed `Protect main` remained active and unchanged with:

- `require_code_owner_review: true`;
- required status `Merge gate / validate` with strict policy;
- `bypass_actors: []`;
- `current_user_can_bypass: never`;
- squash as the only allowed merge method.

Therefore there was no temporary protection reduction to restore.

## Cross-repository audit

Read-only audit found no equivalent stale title/body snapshot dependency requiring the same change in:

- `blakinio/Oteryn-Platform`;
- `blakinio/Otheryn`;
- `blakinio/otclient`.

No cross-repository writes were performed. Existing manual workflows were not changed merely because they use `workflow_dispatch`.

## Acceptance criteria

- [x] Normal PR metadata validation fetches live PR state.
- [x] Metadata edits automatically trigger Agent governance.
- [x] Metadata edits automatically trigger the protected aggregate Merge gate.
- [x] Stale event head versus live head fails closed.
- [x] Existing title/body/base/same-repository checks remain enforced.
- [x] No `pull_request_target` or privileged PR-code execution was introduced.
- [x] Canonical merge-gate critical-job hashes remain unchanged.
- [x] Metadata-only regression passed end-to-end on an unchanged head.
- [x] Exact-head independent deterministic merge-authority audit passed.
- [x] Exact-head full aggregate CI passed.
- [x] No review-thread/requested-change blocker remained.
- [x] Main drift was zero at merge.
- [x] Live `Protect main` readback remained unchanged after merge.

## Final checkpoint

```yaml
checkpoint_status: completed
final_head_sha: 400b7de91221f1833a4bfe3e7cd0659f8b7a056d
merge_sha: 592ed69487c324e68d7ed50c3311e5682d85e894
findings:
  - stale PR body/title event snapshots no longer require manual workflow_dispatch recovery
  - metadata-only edits re-run both governance and the protected aggregate merge status
  - exact-head and same-repository fail-closed boundaries remain intact
  - live Protect main ruleset was not modified during merge
next_action: none
```
