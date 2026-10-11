# OTV2-20261011-game-ci-option-a

```yaml
task_id: OTV2-20261011-game-ci-option-a
title: Reserve full canonical Game qualification for Merge Queue
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 9080588de5abd4dfbf7980638657dbe8ebbb26cd
branch: codex/game-ci-option-a-20261011
pr: 1998
head_sha: "final frozen head in PR #1998 metadata"
final_head_sha: "final frozen head in PR #1998 metadata"
owner: Codex task under direct owner instruction, sole task-branch writer
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - .github/workflows/rust.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/test_classify_post_merge_lanes.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/tasks/archive/OTV2-20261011-game-ci-option-a.md
public_contracts: []
depends_on: [1990, 1997]
blocks: []
external_repositories: []
```

## Outcome

Implementation is prepared in draft #1998. Hosted qualification, the required
independent deep review and protected integration are pending. Archive placement
follows the final-authoring convention; this record reaches main only if #1998
merges. No merge/enqueue, protection or live-environment mutation was performed.

The owner selected option A: fast PR checks and full composed-candidate tests in
Merge Queue. Six full PR jobs and `full-ci` selection are removed. Exact scope,
trusted-base routing, fast Rust Clippy/unit tests, policy, supply chain, World
pin-check, isolated metadata/label fences and fail-closed `game-gate` remain.
This does not yet shorten the changed-crate Rust job itself.

Protected-main Rust builds and lints the merged tree, compiles tests with
`--no-run`, then runs bounded synthetic/native smoke before saving its cache.
Repeated workspace, PostgreSQL and Windows test execution is removed from main.
The namespace remains readable by the surviving PR and MQ consumers; writes
remain fenced to protected main. Compilation can still take substantial time.
Cache contents never replace test-result evidence on the exact MQ candidate.

MQ workflow bytes are unchanged by this task. #1990 owns its sharding/deduplication
change; serialize integration and reconcile any policy/test-file overlap after
that PR lands. Dedicated content/interop/scale workflows keep accepted coverage.
Migrating their unique checks requires a separate mapping into MQ; no global
claim that every specialized workflow is fast is made.

## Protected approval and review packet

Draft #1997 changes only the audit's approved PR-gate blob from
`f2a6a873901c1aa4ae674cdf5d4619427cd25e33` to
`55bc76801a86ec2104b280b09ce39d7ea4be6d45`. No audit logic or MQ pin is changed.
That separate rotation must merge before #1998 can satisfy protected-base audit.
The audit intentionally rejects modifications to itself; #1997 therefore needs
the existing explicit owner-authorized rotation procedure and independent deep
review. Its implementation head is `3c1b811fc896a29f023edc21a2822888d5266792`.

The active programme control plane owns review dispatch. This task supplies the
review packet rather than sending a competing paid-review trigger. Review both
draft PRs on their live frozen heads after deterministic checks: preservation of
exact-candidate MQ coverage, full-ci retirement, metadata cancellation/live fences,
`--no-run` semantics, protected-main cache provenance and narrowed PG/SIM caller.
No review result, hosted speedup or terminal integration is claimed.

## Validation

- `python tools/repository/validate_repository_policy.py`: pass
- `python tools/repository/test_classify_post_merge_lanes.py`: pass
- `PATH=/tmp/oteryn-ci-pwsh:$PATH python tools/repository/test_validate_pr_gate_pg_sim.py`: pass; seven placement tests plus existing MQ PG/Windows, PR routing and content-routing regressions
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK, 60 tests
- `/tmp/game-ci-actionlint/actionlint -shellcheck= -pyflakes= .github/workflows/merge-gate.yml .github/workflows/rust.yml`: pass
- `git diff --check`: pass

PowerShell 7.6.6's release archive checksum was verified against its published
SHA256 file. Native failure-propagation canaries ran; they were not skipped.
Full Rust/Windows/integration execution belongs to repository CI.

Final remote commit identity cannot be embedded in itself. Resolve the frozen
candidate from #1998 metadata; rerun deterministic checks on that exact head.
Rollback is a reviewed reversal of #1998 and its PR-gate approval pin.

## Next action

Control plane consumes the exact-head review packets, serializes #1990 and the
approved-blob rotation, and qualifies #1998 through protected Merge Queue. Owner
authorization for the concrete rotation must be recorded by the governing
control-plane procedure. Both PRs remain drafts; integration is not authorized
by green local checks alone.
