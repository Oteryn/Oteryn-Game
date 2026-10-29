---
name: oteryn-build-test
description: Build, format, lint and test Oteryn-Game changes locally before a push - the exact cargo, clippy, rustfmt, PostgreSQL, Python tool and architecture checks per changed path, and the pitfalls that turned CI red after a push. Use before every push, when a local build or test fails, or when choosing what to run for a change.
---

# Local checks before a push

The selection table is `docs/agents/CONTEXT_ROUTING.md`, *Local checks by
changed path*; this skill adds what that table does not say. CI runs the full
workspace, Windows client and E2E lanes; run only what your paths need, but
run all of it, and fix every failure of one run in one push.

## Order (cheapest first)

1. `git merge origin/main` (see `steward`), then `cargo fmt --all`.
2. Generated content: `tools/merge-driver/regen.sh` if anything under
   `content/`, `imports/` or a pinned input changed (`oteryn-content-world`).
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
   when you want every failing target in one run.
7. Governance, when `docs/agents/`, `tools/agents/`, `.claude/` or `AGENTS.md`
   changed: `python3 tools/agents/validate_governance.py` and
   `python3 -m unittest discover -s tools/agents/tests`.
8. Python tools: the workflow for that `tools/` subtree shows its exact test
   command. Content-migration tests run from inside `tools/content-migration`.
   Lint with the pinned `ruff==0.16.1` (`requirements*.txt` of the tool), not
   whatever ruff is installed.
9. `git diff --check` and a read of the full diff.

## Formatting pitfalls

- Use `cargo fmt`. Do **not** run bare `rustfmt` on a file that declares
  modules (`mod x;`): it follows the declarations, formats the child files
  too, and defaults to an older edition. For a single generated file use
  `rustfmt --edition 2024 <file>`.
- After a merge, look for duplicated `mod` lines, imports or test bodies left
  by the additive merge driver before formatting; they show up as E0428 or as
  an fmt diff in CI.

## PostgreSQL tests

- CI pins PostgreSQL 18.6 and the harness refuses any other version. Never
  relax that check. Local setup: `CONTEXT_ROUTING.md` (docker recipe), or a
  shared local cluster if your machine has one.
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
- Builds and tests print a lot: send output to a file and read the tail and
  the `error`/`FAILED` lines, not the whole log.

## What not to do

- Do not push to "see what CI says". A push without the checks above usually
  costs a full CI cycle (15-60 minutes) and a second push.
- Do not skip, `#[ignore]`, weaken or delete a test to make a run pass.
- Do not add a new top-level `tests/*_postgres.rs` target (use the mounted
  directories, `oteryn-mechanic`).
- Do not add a third-party dependency without the custody review its
  `Cargo.toml` change triggers.
