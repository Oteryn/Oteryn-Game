# MONK-SAVE-FLAKE-1

```yaml
task_id: MONK-SAVE-FLAKE-1
title: Wait for the root holder before the monk boundary observation
mode: REPAIR
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy
pr: null
base_sha: 1bace02133b37ff5cfd93460dca2131afb4e6939
owner: Codex MONK-SAVE-FLAKE-1 worker
created_at: 2026-10-22
updated_at: 2026-10-22
execution_policy: continuous_progress
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs
  - docs/agents/tasks/MONK-SAVE-FLAKE-1.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
registry: no
plan_row: owner-assigned MONK-SAVE-FLAKE-1 test-harness repair
```

## Problem and root cause

The integrator measured two failures in ten focused runs on clean main
`59785d76c`: `boundary_unknown_monk_save_keeps_lease_until_store_recovers`
occasionally returns `RootUnavailable` (reported PROVEN by EQUIP-RT-1 traces).
`drive_until` polls release first, then observes SAVES=2 and cancels save #2
while its `current_monk_fence -> current_session_at` read holds the root's sole
semantic connection. SQLx returns a dropped connection through a spawned task;
the next session observation can run before the holder becomes idle. Querying
the independent fixture pool for the revision does not establish that return.
DERIVED from SQLx's observed-return implementation: cancellation while the
floating holder is being returned can also retire it. A first passive-idle
barrier passed 12 runs, then timed out on run 13, so readiness must cover both
return and retirement rather than only polling for an existing idle connection.

## Module layout and fix

Keep this existing mechanic in `monk_save_postgres_tests.rs`, extending its
private `drive_until` helper once. Explicitly drop the pinned release future,
then request and await the same fixture root's existing `maintain_ready_once()`
before requiring `is_ready()`. Its five-second recovery budget waits for a
returned holder or replaces a retired one; an outer ten-second timeout and
assertion fail loudly with `root_identity()` and the maintenance result.
This reuses the existing Harness, release driver, readiness/identity APIs and
Tokio timeout; no copied fixture or new helper is introduced.
No new module registration, production API, enum, field or trait method is added.
No file grows by more than 200 lines. No registry/plan mutation is required by
the governance validator for this root-level repair card.

## Scope and acceptance

Test-harness scheduling only. Preserve every assertion, expected value and the
unavailable-store/recovered-store scenario. No assertion retry, guessed sleep,
production behavior change, admission/pool implementation change or dependency download.
High-risk authority/recovery qualification: NOT_APPLICABLE; only test ordering
changes, with the real release path and existing durable checks retained.

## Validation

All Rust commands ran through the queued Cargo wrapper with `+1.95.0` and
`CARGO_NET_OFFLINE=true`. Non-login shells preserve the wrapper PATH and its
PostgreSQL configuration. Preliminary direct-Cargo results without PostgreSQL
were discarded. A Python subprocess loop ran the following final-code tests
sequentially, stopping on any failure; no assertion retries were added.

- `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib --quiet -- boundary_unknown_monk_save_keeps_lease_until_store_recovers`: pass, 30/30 consecutive runs; each 1 passed, 0 failed, 0 ignored, 911 filtered out.
- `cargo +1.95.0 test --locked -p oteryn-gameplay-dispatch --lib --quiet`: pass, 3/3 consecutive runs; each 898 passed, 0 failed, 14 pre-existing ignored, 0 filtered out (74.39s, 67.62s, 67.58s).
- `cargo +1.95.0 fmt --all --check`: pass.
- `cargo +1.95.0 clippy --locked -p oteryn-gameplay-dispatch --all-targets --quiet -- -D warnings`: pass.
- `python3 tools/agents/validate_governance.py`: pass (22 required documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: pass (124 tests).
- `git diff --check`: pass.
- Whole-diff self-review against `~/oteryn-cache/CODEX-CHECKLIST.md`: pass;
  original assertions/expected values unchanged, no public API added, no new
  enum variants/fields/trait methods, and workspace-wide `drive_until` search
  confirms its sole caller was updated. Independent review is not required
  for this test-harness-only scheduling repair; integration remains separate.

## Handoff

Implementation and local validation are complete. The integrator committed the
change on `claude/happy-davinci-ijz3wy` over current `main` and publishes it for
exact-head CI; archive follows the merge.
