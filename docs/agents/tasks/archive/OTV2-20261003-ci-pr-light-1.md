# OTV2-20261003-ci-pr-light-1

```yaml
task_id: OTV2-20261003-ci-pr-light-1
title: Run heavy qualifications once in the Merge Queue
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/ci-pr-light-1
pr: 1694
issue: 1622
base_sha: cee17c6
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Claude worker for control plane session_013KJX6mv8LQveCKKXYgAX94, sole writer; owner decision D431 1a+c, approved directly by the owner in the worker session
created_at: 2026-10-03
updated_at: 2026-10-03
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/repository/test_classify_pr_test_lanes.py
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/tasks/archive/OTV2-20261003-ci-pr-light-1.md
public_contracts: []
depends_on: [merge-authority-audit EXPECTED_MERGE_GATE_BLOB rotation PR]
blocks: []
external_repositories: []
```

## Outcome

PROVEN: every heavy PR job already has an exact Merge Queue equivalent in the
unchanged `merge-group-gate.yml` (parity table in PR #1694). The PR gate keeps
fast checks plus changed-crate strict Clippy and `--lib`/`--bins` unit tests;
CodeQL, Rust Linux/PostgreSQL, Windows, Atlas fullworld, node boot and Server
Seam run on a PR only with the `full-ci` label and are then required. `game-gate`
name, PR reporting and the live head/base fence are preserved; other label events
use the isolated non-cancelling concurrency group. No `workflow_dispatch` on the
gate. Validator digests and the aggregate regression test are repinned to the
new shape. The merge-authority audit merge-gate blob pin is rotated in a separate
owner-authorized PR that must merge first.

## Validation

python tools/agents/validate_governance.py: pass
python -m unittest discover -s tools/agents/tests: OK
python tools/repository/validate_repository_policy.py: pass
python tools/repository/test_validate_pr_gate_pg_sim.py: pass
python tools/repository/test_validate_merge_group_pg_sim.py: pass
python tools/repository/test_classify_pr_test_lanes.py: pass
python tools/repository/test_main_job_applicability.py: OK

## Review and closeout

Independent review is REQUIRED on the frozen head; the control plane triggers it.
Merge result: squash merge of #1694, pending audit rotation, review and Merge Queue
at authoring. The exact frozen head is recorded in the FREEZE_SHA entry outside Git.
