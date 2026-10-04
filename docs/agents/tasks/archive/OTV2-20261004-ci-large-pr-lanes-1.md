# OTV2-20261004-ci-large-pr-lanes-1

```yaml
task_id: OTV2-20261004-ci-large-pr-lanes-1
title: "Path-selected lanes for large PRs"
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-large-pr-lanes-1-20261004
issue: 1622
pr: 1778
head_sha: "exact frozen head in the FREEZE_SHA entry to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA entry to the control plane"
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/archive/OTV2-20261004-ci-large-pr-lanes-1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D561 3a: a large PR runs only the fast, path-selected lanes; the heavy suite runs
in the Merge Queue.

- PROVEN: the heavy PR jobs (CodeQL, Rust Linux workspace, Rust Windows, Atlas fullworld, node boot,
  Server Seam) already run on a PR only with the `full-ci` label.
- PROVEN: the trusted-base classifier (`lanes`) and the routing-contract validator already recover
  an incomplete scope transport (over 300 files or 32 KiB) from the exact base/head Git trees
  (#621), so their lane selection is path-based for large PRs.
- PROVEN: `Merge gate / Rust changed-crate Clippy and unit tests` was the remaining consumer that
  treated an incomplete transport as workspace-wide, running Clippy and unit tests of every crate.
- Fix: that job now fetches the exact base commit and recovers the changed paths from the exact
  base/head tree diff when `ENUMERATION_COMPLETE=false`, the same recovery as the classifier.
- DERIVED: the 300-file scope limit stays, because the SHA-bound compare API lists at most 300
  files; the 32 KiB transport cap stays, because the records travel as an environment variable to
  the protected-base classifier (the Linux single-string limit is 128 KiB). Consumers no longer
  need either: recovery from exact trees has no file-count ceiling.

## Fail-closed cases

A malformed `ENUMERATION_COMPLETE`, an unfetchable base, a checkout that is not the exact head,
equal or invalid SHAs, an empty, non-UTF-8 or unsupported-status diff all select every workspace
crate. Workspace-wide Rust inputs and unattributable paths still select every crate. Scope still
fails on a moved head or base, a closed PR, a changed file count or an API error. The `full-ci`
label and `merge-group-gate.yml` are unchanged.

## Excluded scope

No change to `merge-group-gate.yml`, required check names, branch protection, the classifier or
the routing-contract validator.

## Validation

- `python tools/repository/test_validate_pr_gate_pg_sim.py`: all 30 merge-gate regressions pass,
  including the new `test_rust_fast_recovers_large_pr_from_exact_trees`; the chained
  merge-group suite stops locally only on its `pwsh` requirement (not installed here)
- `python tools/repository/test_classify_pr_test_lanes.py`: pass
- `python tools/repository/validate_repository_policy.py`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `git diff --check`: pass

## Closeout

- PR #1778; merge commit/result: squash merge of #1778.
- Review state: pending at freeze.
