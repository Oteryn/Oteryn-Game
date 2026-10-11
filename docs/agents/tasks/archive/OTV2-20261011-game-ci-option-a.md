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
  - .github/workflows/merge-group-gate.yml
  - .github/workflows/rust.yml
  - tools/repository/validate_repository_policy_core.py
  - tools/repository/validate_pr_gate_pg_sim.py
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - tools/repository/test_classify_pr_test_lanes.py
  - tools/repository/test_classify_post_merge_lanes.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - docs/repository/GITHUB_GOVERNANCE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/tasks/archive/OTV2-20261011-game-ci-option-a.md
public_contracts: []
depends_on: [1997]
blocks: []
external_repositories: []
```

## Outcome

Implementation is prepared in #1998. The owner explicitly instructed this task
to execute review and protected integration. Hosted qualification, independent
review of the repaired frozen heads and protected integration remain pending.
Archive placement follows the final-authoring convention; this record reaches
main only if #1998 merges. Queue acceptance is not terminal integration proof.

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

Independent review on the original #1998 head found that its removed Windows
PR job was the sole installer caller (review comment 4243099652). The repair
moves the existing release-identity, hash-verified Inno Setup, unsigned installer
lifecycle and artifact steps into the exact-candidate MQ Windows job. Identity
is derived from `merge_group.head_sha` before the release build. The unchanged
installer script retains installation, reinstall, concurrency and uninstall
checks. Routing regression and policy mutation checks protect that coverage.

The existing MQ Linux/PostgreSQL qualification remains intact. #1990 is not a
dependency: its independent optimization has unresolved review findings. Any
later integration of that PR must reconcile its MQ/policy changes with this
accepted installer coverage. Dedicated content/interop/scale workflows retain
their existing coverage; no global claim that every specialized workflow is fast
is made.

## Protected approval and review packet

#1997 separately rotates the audit's approved PR-gate blob from
`f2a6a873901c1aa4ae674cdf5d4619427cd25e33` to
`55bc76801a86ec2104b280b09ce39d7ea4be6d45`, and its MQ approval from the
unintegrated #1990 preapproval `cb48993d2f1f2aed34a472e66437e82b91f5d2c0`
to `22e7688cce7a19ed48d27ac1bf44bbeab8537fd0` (current main MQ plus installer migration).
No audit logic or protection rule changes. This rotation must merge before
#1998 can satisfy the protected-base audit. The audit intentionally rejects
self-modification; #1997 follows the existing explicit owner-authorized rotation
procedure with independent deep review and the normal exact-candidate game-gate
and protected Merge Queue. Owner authorization and exact frozen heads are
recorded on coordination Issue #1622.

Both repaired heads completed independent review without new findings. A final
documentation reconciliation updates the active build/test matrix to describe the
reviewed PR/MQ/main behavior; workflow bytes and approved pins are unchanged.
#1998 returns to AUTHORING for that coherent documentation delta, then freezes
and qualifies the successor head. Live PR review/checks govern that final head.
One GitHub Codex review is dispatched per frozen head without duplicate paid-review triggers.
Review preservation of exact-candidate coverage (including installer lifecycle),
full-ci retirement, metadata cancellation/live fences, `--no-run` semantics,
protected-main cache provenance and the narrowed PG/SIM caller.

## Validation

- `python tools/repository/validate_repository_policy.py`: pass
- `python tools/repository/test_classify_post_merge_lanes.py`: pass
- `PATH=/tmp/oteryn-ci-pwsh:$PATH python tools/repository/test_validate_pr_gate_pg_sim.py`: pass; seven placement tests plus existing MQ PG/Windows, PR routing and content-routing regressions
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK, 60 tests
- `/tmp/game-ci-actionlint/actionlint -shellcheck= -pyflakes= .github/workflows/merge-gate.yml .github/workflows/merge-group-gate.yml .github/workflows/rust.yml`: pass
- `git diff --check`: pass

PowerShell 7.6.6's release archive checksum was verified against its published
SHA256 file. Native failure-propagation canaries ran; they were not skipped.
Full Rust/Windows/integration execution belongs to repository CI.

Final remote commit identity cannot be embedded in itself. Resolve the frozen
candidate from #1998 metadata; rerun deterministic checks on that exact head.
Rollback is a reviewed reversal of #1998 and both approval pins in #1997.

## Next action

Qualify both repaired frozen heads, complete independent review, integrate #1997
through the protected Merge Queue, then refresh the protected-base audit and
integrate #1998. Close out only with the real merge-group aggregate and protected
main readback. No #1990 mutation or queue submission belongs to this task.
