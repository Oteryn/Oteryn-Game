# Local checks before a push

The selection table is `docs/agents/CONTEXT_ROUTING.md`, *Local checks by
changed path*; this file adds what that table does not say. CI runs the full
workspace, Windows client and E2E lanes; run only what your paths need, but
run all of it, and fix every failure of one run in one push.

## Order (cheapest first)

1. If the branch conflicts with `main` or is about to get `heavy-tests`, merge
   `origin/main` first (`ci-merge.md`, *Before every push*); then `cargo fmt --all`.
2. Generated content: `python3 tools/content-migration/regenerate_content.py --resolve`
   (same as `tools/merge-driver/regen.sh`; the python form is the one the
   session allowlist permits) if anything under
   `content/`, `imports/` or a pinned input changed (`content-world.md`).
3. Registry fragments: `python3 tools/contracts/registry_fragments.py`.
4. Architecture: `cargo run --locked -p oteryn-architecture-check -- workspace .`
   (needed for every new capability or client registration).
5. Clippy per changed package:
   `cargo clippy --locked -p <package> --all-targets --quiet -- -D warnings`.
   For `oteryn-gameplay-dispatch` and `oteryn-gameplay-entities` add
   `--all-features`; their `test-support` code only compiles with it and CI
   builds it.
6. Tests per changed package: `cargo test --locked -p <package> --quiet`
   (with `--all-features` for the two crates above). Add `--no-fail-fast`
   when you want every failing target in one run. Unit tests are where PR
   gates go red most often. CI runs them with cargo-nextest, one
   process per test, so a test that relies on state left by another passes
   locally under `cargo test` and fails in CI; `cargo nextest run -p <package>`
   reproduces CI when nextest is installed.
7. Governance, when `docs/agents/`, `tools/agents/`, `.claude/` or `AGENTS.md`
   changed: `python3 tools/agents/validate_governance.py` and
   `python3 -m unittest discover -s tools/agents/tests`.
8. Python tools: the workflow for that `tools/` subtree shows its exact test
   command. Content-migration tests run from inside `tools/content-migration`.
   Lint with the ruff version pinned in that tool's `requirements*.txt`,
   not whatever ruff is installed.
9. `git diff --check` and a read of the full diff.

## Jobs that only run after the merge

The PR gate does not run these (`ci-merge.md`, *Blind spot*). Run the
matching one locally when your change can reach it; commands are copied from
`.github/workflows/merge-group-gate.yml`, which wins if they differ.

- Integration shards (`integration-heavy`, `integration-rest`):
  ```sh
  cargo metadata --locked --no-deps --format-version 1 > /tmp/meta.json
  python tools/repository/plan_rust_test_shards.py /tmp/meta.json integration-rest > /tmp/shard.txt
  mapfile -t a < /tmp/shard.txt; test "${#a[@]}" -gt 0   # an empty plan would run everything
  cargo nextest run --locked --workspace "${a[@]}" --no-fail-fast
  ```
  Narrow with a test filter when only one target is affected.
- Node boot (needs the pinned `Oteryn/Oteryn-Platform` checkout named in the
  workflow, as `PLATFORM_SOURCE`):
  `bash tools/qualification/node_boot/run.sh` and look for
  `NODE_BOOT_RESULT=NODE_BOOT_PASS`. Run it for changes to node startup,
  configuration, thread or stack size, schedulers or the dispatch runtime.
- Smoke: `cargo run --locked -p oteryn-game-server -- --smoke`.

## Formatting pitfalls

- Use `cargo fmt`. Bare `rustfmt` on a file that declares modules (`mod x;`)
  follows the declarations, formats the child files too, and defaults to an
  older edition. For a single generated file use
  `rustfmt --edition 2024 <file>`.
- After a merge, check for duplicates left by the additive driver
  (`ci-merge.md`, *Merge conflicts*) before formatting.

## PostgreSQL tests

- CI pins PostgreSQL 18.6 and the harness refuses any other version; keep
  that check as it is. Local setup: the Docker recipe under the table in
  `CONTEXT_ROUTING.md`, *Local checks by changed path*, or the lane's own
  database on the PC. Where neither exists (Codex Cloud, a worker whose packet
  forbids Docker), leave PostgreSQL targets to CI and say so in the report.
- Export the admin URL into the environment without printing it
  (`set +x` first in scripts; never `echo` it or write it into a log, note or
  PR).
- Run one target or one filtered test locally. The full six-partition suite is
  CI's job (`heavy-tests`); running it locally costs about an hour of CPU.
- A lock timeout or deadline in one partition can be contention. Re-run that
  test once in isolation; if it fails again it is a real failure.

## Build hygiene

- One Rust build per checkout at a time; parallel builds of one target dir
  block on the cargo lock and look like a hang.
- Give each worktree its own `CARGO_TARGET_DIR`, and delete it once the
  branch is merged: target dirs are tens of GB.
- Check free disk space before a build. "No space left on device" mid-build
  leaves a corrupt incremental cache; delete the target dir and rebuild.
- In shell pipelines take the exit code of the command, not of `tail`/`tee`:
  `set -o pipefail`, or `${PIPESTATUS[0]}`.
