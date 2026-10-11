# OTV2-20261011-ci-rust-speed

```yaml
task_id: OTV2-20261011-ci-rust-speed
title: Shard the Merge Queue Rust Linux workspace job and remove duplicated test runs
mode: GOVERNANCE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: 4ab138b90ae28e2a526fe94acd46ecd2addaabb6
branch: claude/ci-rust-speed-20261011
pr: 1990
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: CI speed worker under the CI lead (single writer of this branch)
created_at: 2026-10-11
updated_at: 2026-10-11
execution_policy: continuous_progress
owned_paths:
  - .github/workflows/merge-group-gate.yml
  - apps/game-server/Cargo.toml
  - tools/repository/check_duplicate_test_sources.py
  - tools/repository/plan_rust_test_shards.py
  - tools/repository/test_check_duplicate_test_sources.py
  - tools/repository/test_validate_merge_group_pg_sim.py
  - tools/repository/validate_repository_policy_core.py
  - docs/agents/tasks/active/OTV2-20261011-ci-rust-speed.md
  - docs/agents/tasks/archive/OTV2-20261011-ci-rust-speed.md
public_contracts: []
depends_on: [merge-authority-audit pin rotation of EXPECTED_MERGE_GROUP_GATE_BLOB]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The Merge Queue job `Merge Queue / Rust Linux workspace` ran one serial step of
40-50 minutes. It is now a six-leg matrix under the same job key and name
(`lint`, `lib-content`, `lib-rest`, `integration-heavy`, `integration-rest`,
`build-smoke`), so `game_gate` still reads one `needs.rust_linux.result`, which
is success only when every leg succeeds. The expected wall-clock time is about 13-16
minutes on a warm protected-main cache (DERIVED from the measured per-step times
below). The work is also smaller: 25 thin PostgreSQL wrapper test targets that
re-ran library unit tests and aggregate case files are now `test = false`.
A new CI guard fails when a test-bearing source compiles into more than one
test harness outside an explicit, shrink-only allowlist.

## Architecture and source of truth

- PROVEN, from the measured MQ job 114513480431 (run 38154056379, warm cache):
  - setup took about 4 minutes;
  - `build --all-targets` took 15m33s, almost all of it `oteryn-game-server`;
  - clippy took 7m47s;
  - test execution took 1265 s: library 430-438 s (`content::` 183 s, `gameplay_transport` 154 s, other ~93 s) and integration 788 s (the 25 wrappers 189 s, the kept targets 599 s);
  - the smoke rebuild took 1m54s and doctests about 10 s.
- PROVEN: across the two measured runs, one `cargo test --workspace` run made 30193 test executions of 4377 unique test names. Without the 25 wrappers it makes 9547 executions of the same 4377 names: every name the wrappers ran also runs elsewhere, so no coverage is lost.
- PROVEN: `test = false` removes a `[[test]]` target from `cargo test`, `--tests` and `--all-targets`. `cargo test --test <name>` still builds and runs it for focused local work.
- DERIVED: each leg selects whole-workspace targets with the protected-main toolchain and flags. The resolved features therefore match the cache producer and each other.
- DERIVED: the union of the legs is the old `build --all-targets` + `clippy` + `test` + harness + smoke:
  - libtest `content::` and `--skip content::` partition the library tests;
  - the planner partitions the tested integration targets from `cargo metadata`;
  - `--bins` and `--doc` cover binary unit tests and doctests;
  - `build-smoke` builds the examples and benches.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: CI job layout and test-target selection only. No production
mutation, session/lease/generation fence, PREPARE/COMMIT, controller or
persisted recovery evidence is touched.

## Acceptance criteria

- [x] The same job key and name, aggregated by `game_gate`, with no check removed or weakened (see Outcome).
- [x] Unique test names are unchanged, 4377 before and after (Architecture).
- [x] The duplicate-source guard and the shard planner fail closed, with regressions (13 tests).
- [x] Governance pins rotated consistently: the validator's `EXPECTED_MERGE_GROUP_GATE_BLOB`, its rust_linux fragments and the pg-sim regression `APPROVED` blob, all set to `cb48993d2f1f2aed34a472e66437e82b91f5d2c0`.

## Excluded scope

- `.github/workflows/merge-authority-audit.yml` still pins the old MQ gate blob `a9e9fcb9…`. Rotating that pin needs a separate protected audit-pin PR with an owner exception, as for #1943 before #1944. Until then the protected-base merge authority audit fails on this PR. That failure is expected.
- The post-merge `rust.yml` Linux lane, the PR gate, nightly workflows and the ten allowlisted `#[path]` copy targets are unchanged. Gating those copies at the source is a follow-up.

## Implementation / findings

- Workflow:
  - `lint` keeps the policy/fmt, dependency-closure and clippy steps, then runs the guard on clippy's JSON messages. The guard reads each test harness's rustc dep-info.
  - The test legs compile only what they run.
  - `build-smoke` builds bins, examples and benches, runs `--bins` and `--doc`, then the harness and the smoke.
- `plan_rust_test_shards.py`:
  - Seven named heavy targets go to `integration-heavy`; every other tested integration target goes to `integration-rest`, so a new target always lands in a shard.
  - It fails on an unknown heavy name, an empty shard, or an example or bench that opts into `cargo test`.
- `check_duplicate_test_sources.py`:
  - The allowlist names the ten `#[path]` copy targets, limited to `apps/game-server/src/`, plus the library's revision-sequencer fixture.
  - Unused entries are reported as stale, so the list can only shrink.

## Validation

- `python tools/repository/validate_repository_policy.py`: pass
- `python tools/repository/test_validate_merge_group_pg_sim.py`: pass (approved blob, policy, Atlas/World bundle fan-in and 28 workflow mutations; its four PowerShell native canaries need `pwsh`, absent on the validation host, and run in CI unchanged)
- `python tools/repository/test_check_duplicate_test_sources.py`: OK (13 tests)
- `python tools/repository/plan_rust_test_shards.py` on real `cargo metadata --no-deps`: pass (heavy 7, rest 50 targets; 25 wrappers `test = false`, disjoint, union = all tested targets)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- actionlint 1.7.7 on `.github/workflows/merge-group-gate.yml`: pass
- `git diff --check`: pass
- `cargo fmt/clippy/test -p oteryn-game-server`: left to CI. The manifest change only sets `test = false`; local disk and the PC WSL build host could not hold a full workspace build.

## Self-review

- exact head: the frozen head recorded on the PR
- method/reviewer: implementing agent, whole-diff review
- material findings: none open; residual risks are recorded in the PR body (wrapper glue no longer compiled in CI; six concurrent legs)
- verdict: ready for independent review

## Independent review

- required: YES. This changes a required Merge Queue gate and its pinned contract.
- exact head: pending (dispatched by the lead)
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: the 7 source paths plus this record, all within owned paths
- unresolved review threads: none at authoring
- related/superseded PRs: none; depends on the audit-pin rotation PR
- protected auto-merge: not enabled
- merge commit/result: pending; recorded later as `squash merge of #1990`
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: final authoring commit with archived record
status: completed
branch: claude/ci-rust-speed-20261011
head_sha: null
pr: 1990
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: 1
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
owner_action_required: audit-pin rotation exception for EXPECTED_MERGE_GROUP_GATE_BLOB
blocker: protected-base merge authority audit pins the old MQ gate blob
next_action: lead dispatches independent review and the audit-pin rotation PR
```
