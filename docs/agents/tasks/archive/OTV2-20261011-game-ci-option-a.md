# OTV2-20261011-game-ci-option-a

```yaml
task_id: OTV2-20261011-game-ci-option-a
title: Reserve full canonical Game qualification for Merge Queue
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: da4e66982290b557d1d45fd008906d8c81bb6bb3
branch: codex/game-ci-option-a-reconciled-20261011
pr: 2001
head_sha: "final frozen head in PR #2001 metadata"
final_head_sha: "final frozen head in PR #2001 metadata"
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
  - docs/architecture/CLIENT-INSTALLER-0_WINDOWS_CLIENT_INSTALLER_CONTRACT_CANDIDATE.md
  - apps/client/installer/ci-installer.ps1
  - docs/agents/tasks/archive/OTV2-20261011-game-ci-option-a.md
public_contracts: [CLIENT-INSTALLER-0]
depends_on: [1997, 1990, 2000]
blocks: []
external_repositories: []
```

## Outcome

This is the final-authoring successor of #1998 after protected main integrated
#1990 as da4e66982290b557d1d45fd008906d8c81bb6bb3. Both previous candidates
remain preserved. The owner explicitly instructed this task to execute review
and protected integration. Source and approval-pin successors require their
new exact-head qualification and independent review, then native Merge Queue
and terminal main readback. Archive placement does not claim completion before
that terminal proof. The canonical replacement is #2001; resolve its frozen head from live metadata.

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
is derived from `merge_group.head_sha` before the release build. The installer
script retains installation, reinstall, concurrency and uninstall checks; only
its caller comment is updated. Routing regression and policy mutation checks
protect that coverage.

The already-integrated #1990 Linux matrix remains intact: lint, library Content,
other libraries, heavy integration, other integration, and build/binary/doctest
smoke legs. The accepted aggregate still requires every applicable leg. Its
duplicate-source guard and registered targets remain inherited from main. MQ
PostgreSQL, Windows, Atlas, CodeQL, node boot and Server Seam coverage remain
binding. Dedicated content/interop/scale workflows retain their existing checks.
No global claim that every specialized workflow is fast is made.

## Protected approval and review packet

#1997 already integrated owner-authorized audit approval of the fast PR gate
55bc76801a86ec2104b280b09ce39d7ea4be6d45 and the installer-bearing MQ gate
22e7688cce7a19ed48d27ac1bf44bbeab8537fd0. #1990 subsequently integrated
the independently qualified Linux matrix and made #1998 conflict. This successor
is authored from actual protected main with both optimizations preserved.

The separate owner-authorized approval-pin successor #2000 changes only the audit's
expected MQ literal from 22e7688cce7a19ed48d27ac1bf44bbeab8537fd0 to
d7cfbf90b2b81c43f92c010660f41d242a3cf840 (integrated Linux shards plus installer).
The PR approval, audit logic, gate requirement and protection rules stay intact.
Its deliberate self-modification rejection follows the existing explicitly
owner-authorized rotation exception with independent review and normal game-gate
and MQ qualification. Exact allocation, frozen heads and terminal proof are
recorded on coordination Issue #1622.

The installer-preserving implementation completed independent review without new
findings. Review of the matrix reconciliation then raised P2 comment 4243217459:
CLIENT-INSTALLER-0 still named the retired PR caller. Its CI-placement clauses now
explicitly supersede that part of D854 under the owner's option A instruction;
all package/lifecycle/signing/distribution contracts stay binding. The task
records the affected contract and updates the script's caller comment only.
Workflow, executable PowerShell and approval-pin bytes are unchanged by this
contract repair. The replacement returns to AUTHORING, freezes the complete successor and
qualifies that exact head. Live PR review/checks govern the final candidate.
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
candidate from #2001 metadata; rerun checks on that exact head.
Rollback is a reviewed reversal of option A and its approval-pin rotations,
preserving unrelated protected-main changes such as #1990.

## Next action

Qualify the final frozen source and pin successors, complete required independent
review, integrate the pin through protected Merge Queue, refresh the source audit
and integrate the source. Close out only after the real MQ aggregate (including
Windows installer lifecycle) and protected-main readback. No #1990 branch or
queue mutation belongs to this task.
