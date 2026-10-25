# CI-RUNNER-C

```yaml
task_id: CI-RUNNER-C
title: Route PR rust_fast to Molehill with a hosted fallback
mode: BUILD
status: in_review
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-cirunner
pr: 2385
owner: standalone CI-RUNNER-C worker
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-gate.yml
  - tools/repository/test_validate_pr_gate_pg_sim.py
  - docs/agents/tasks/CI-RUNNER-C.md
  - .github/workflows/merge-authority-audit.yml (EXPECTED_MERGE_GATE_BLOB rotation only)
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
```

## Goal and owner decision

PROVEN: the owner selected option C, self-hosted Rust checks with hosted fallback,
and authorized workflow and repository-tool edits for this task. All four matrix
legs of `merge-gate.yml` / `rust_fast` use Molehill only when the repository
variable `OTERYN_SELF_HOSTED_RUST` is `on`, the PR head belongs to this repository,
and the actor is not `dependabot[bot]`. Otherwise they use `ubuntu-24.04`.
The existing matrix, ledger key inputs and aggregate gate semantics are preserved.

## Runner setup and operation

PROVEN (owner-supplied setup): Molehill has four self-hosted Linux x64 runners
under a dedicated user, managed by systemd services. `runner-install.sh` lives
outside this repository on Molehill. The runner labels are `self-hosted`, `linux`,
`x64`, `oteryn-molehill`. Their `.env` supplies
`OTERYN_CI_TARGET_ROOT=/home/oteryn-ci/ci-target`, `CARGO_HOME`, `RUSTUP_HOME`, and a
PATH containing rustup. Rust 1.99.1, clippy and rustfmt are preinstalled.
This task does not install or modify runners or live repository settings.

Set the repository Actions variable `OTERYN_SELF_HOSTED_RUST` to `on` to enable
Molehill for eligible PRs. Turn it off or delete it to restore hosted routing when
the PC is offline. GitHub does not automatically fall back when an opted-in
self-hosted runner is unavailable; queued runs may need cancellation and a rerun
after changing the variable.

Each self-hosted runner uses its own `$OTERYN_CI_TARGET_ROOT/$RUNNER_NAME/merge-gate`.
A runner executes one job at a time, so concurrent legs or PRs never share build
artifacts or the nextest JUnit report the job-result ledger reads; a shared
directory would let one PR's report be recorded for another head. An unset or
relative root, or a runner name that is not a plain path segment, fails explicitly.
Self-hosted runs retain warm artifacts instead of restoring the protected-main
Cargo cache or normalizing checkout mtimes. Cargo validates source fingerprints
against the fresh checkout. Setup removes the previous nextest JUnit report, and
the result ledger reads the report from the selected target directory.

## Merge-authority pin

`merge-authority-audit.yml` pins the approved `merge-gate.yml` blob. As in earlier
CI changes (#2312, #2319), this PR rotates exactly `EXPECTED_MERGE_GATE_BLOB` to the
new blob; nothing else in the audit changes. The protected-base audit run on this PR
still reads the old pin from `main` and fails by design until the change lands.

## Security and excluded scope

PR code executes on the owner's machine when self-hosting is enabled. Fork PRs
and dependabot are excluded from that route. A dedicated runner user is an
operational boundary, not a sandbox for arbitrary PR code. The owner controls
which same-repository contributions are trusted for this execution environment.

No change to other jobs, permissions, timeouts, `.github/repository-policy.json`,
vendor files, Merge Queue qualification, or `game-gate` semantics is authorized.
High-risk authority/recovery qualification and mechanic reference parity are
NOT_APPLICABLE: this task changes CI runner selection, not game runtime behavior.

## Acceptance criteria

- The exact opt-in expression pins the hosted default, label set and exclusions.
- Cache restore and source-mtime normalization retain their hit condition and run
  only on GitHub-hosted runners.
- Self-hosted target setup rejects invalid roots and removes stale JUnit reports;
  ledger report lookup follows `CARGO_TARGET_DIR` with the hosted default intact.
- Required offline repository/governance checks pass.

## Measurements after rollout

PROVEN (live runs, 2026-10-24): #2385 merged, four runners `molehill-game-1..4`
registered with label `oteryn-molehill`, and `OTERYN_SELF_HOSTED_RUST=on`.
Durations are job execution times of the `rust_fast` legs, read from the
GitHub Actions job records; queue time is excluded.

| `rust_fast` leg | Hosted `ubuntu-24.04` (earlier PRs) | Molehill cold, #2386 run 40095302354 | Molehill warm, #2387 run 40097838921 |
|---|---:|---:|---:|
| Clippy | 5-9 min | 8.5 min | ~4.5 min |
| Tests 1/3 | 5-9 min | 13.9 min | ~4.3 min |
| Tests 2/3 | 5-9 min | 11.0 min | ~4.8 min |
| Tests 3/3 | 5-9 min | 12.7 min | ~4.6 min |
| Whole merge gate | not re-measured | ~15 min | ~9 min |

The cold run is the first build in each runner's empty target directory, so it
is slower than hosted with its restored Cargo cache. Once each runner's target
is warm, every leg is about 2x faster than hosted. The two PRs differ in changed
crates (durability/game-server versus entities), so the comparison is
indicative, not a controlled benchmark.

Decision: keep four runners and the variable `on`. The fallback is unchanged:
set `OTERYN_SELF_HOSTED_RUST=off` (or delete it) when Molehill is offline, then
cancel and rerun queued legs.

## Validation and self-review

Commands below use `python3` because this machine has no `python` executable.
Test fixtures use `TMPDIR="$PWD/.ci-runner-c-validation"` to keep temporary writes
inside the current worktree.

| Command | Result |
|---|---|
| `python3 -I tools/repository/validate_repository_policy.py` | PASS: 23 files, 68 workflows |
| `python3 -I tools/repository/validate_pr_gate_pg_sim.py` | PASS |
| `python3 -I -m unittest discover -s tools/repository -p test_validate_pr_gate_pg_sim.py` | PASS: 9 tests, including 2 new runner/target regressions |
| `python3 -I tools/repository/test_validate_merge_group_pg_sim.py` | PASS in the implementation lane (native failure canaries, 47 workflow mutations, fan-in and routing regressions); a later coordinator re-run stopped early because `pwsh` was not on PATH, and CI runs it |
| `python3 -I tools/repository/test_validate_game_atlas_semantic_search_triggers.py` | PASS: 6 tests |
| `python3 tools/repository/test_classify_quest_authoring_scope.py` | PASS: 30 tests |
| `python3 -I -m unittest discover -s tools/repository -p test_validate_coded_output.py` | PASS: 4 tests |
| `python3 -I tools/agents/validate_governance.py` | PASS: 22 required documents, 9 lanes; no task-card index update required |
| `python3 -m unittest discover -s tools/agents/tests -p test_meta_agent_policy_adoption.py` | PASS: 30 tests |
| `python3 -m unittest discover -s tools/agents/tests -p test_mechanic_cards.py` | PASS: 5 tests |
| `python3 -m unittest discover -s tools/agents/tests -p test_validate_governance_lifecycle.py` | PASS: 9 tests |
| `python3 -m unittest discover -s tools/agents/tests -p test_governance_lifecycle_discovery.py` | PASS: 2 tests |
| `python3 -m unittest discover -s tools/agents/tests -p test_governance_lifecycle_first_production_content_registry.py` | PASS: 4 tests |
| `python3 -m unittest discover -s tools/agents/tests -p test_qualify_monster_wiki_fields.py` | PASS: 5 tests |
| `python3 -m unittest discover -s tools/agents/tests -p test_enqueue_readiness.py` | PASS: 64 tests |
| `actionlint -shellcheck= -pyflakes= .github/workflows/merge-gate.yml` | PASS: Actions syntax/expression validation |
| `actionlint .github/workflows/merge-gate.yml` | FAIL: 9 pre-existing ShellCheck diagnostics, identical to HEAD (8 SC2016 in lanes, 1 SC2086 in unchanged ledger command); no new diagnostics |
| `git diff --check` | PASS |

A PyYAML comparison with `git show HEAD:.github/workflows/merge-gate.yml` passed:
every other job and workflow setting is identical, as are the `rust_fast` matrix,
permissions, timeout, job metadata and existing step names. The complete toggle
expression is pinned by the new regression. Whole-diff self-review against the
owner checklist found no new applicable defect. No enum variants, struct fields,
trait methods or Rust functions were added; Rust caller/dispatch, cargo/clippy,
PostgreSQL and rustfmt qualification are NOT_APPLICABLE.

## Unfinished

- `python3 tools/repository/test_agent_governance_pr_targets.py`,
  `python3 tools/repository/test_main_job_applicability.py`, and
  `python3 -I tools/repository/test_merge_authority_applicability.py` cannot pass:
  their JavaScript expression checks require `node`, which is absent from PATH.
  No tool was installed or downloaded.
- The full script `python3 tools/repository/test_validate_pr_gate_pg_sim.py`,
  `python3 tools/repository/test_classify_pr_test_lanes.py`,
  `python3 tools/repository/test_classify_content_routing.py`,
  `python3 tools/repository/test_classify_post_merge_lanes.py`,
  `python3 tools/repository/test_job_result_key.py`, and full
  `python3 -m unittest discover -s tools/agents/tests` await clarification of the
  owner's Git prohibition: these tests add/commit/checkout in disposable fixture
  repositories. They were not run through those Git mutations. Their existing
  assertions are unchanged. The focused PR-gate unittest class did run in full.
- Full actionlint retains the baseline ShellCheck findings listed above; fixing
  other jobs or changing ledger key arguments is outside this task's scope.
