# PIN-SCOPE-1

```yaml
task_id: PIN-SCOPE-1
title: Scope the World pin inputs_digest to the World crates' Cargo inputs
mode: REPAIR
status: ready
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-10
pr: null
base_sha: 37bcdf52771a4f68992aec4c3ca5b0cdb28936ce
owner: Claude integrator
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
owned_paths:
  - tools/world-bundle-compiler/src/cargo_inputs.rs
  - tools/world-bundle-compiler/src/cargo_inputs_tests.rs
  - tools/world-bundle-compiler/src/main.rs
  - tools/world-bundle-compiler/tests/pin_check.rs
  - content/world/pins/oteryn.json
  - content/world/pins/README.md
  - docs/architecture/reviews/OTERYN_GAME_ARCH_WORLD_CONTENT_SERVE_PACKETS_2026-10-04.md
  - docs/agents/tasks/PIN-SCOPE-1.md
public_contracts: []
depends_on: []
blocks: []
external_repositories: []
registry: no
plan_row: integrator-found merge-conflict hotspot (owner approved)
```

## Problem

The root `Cargo.toml` and `Cargo.lock` enter the World pin `inputs_digest` as
whole git blobs. Every PR that adds a workspace crate or bumps any dependency
therefore moves the pin, and any two such PRs conflict on
`content/world/pins/oteryn.json` even when neither touches the World.

## Fix

- The two files enter the digest (domain `oteryn:world-bundle/inputs/v2`) as the
  SHA-256 of a World-scoped form, built in `cargo_inputs.rs`:
  - `Cargo.lock`: the lock version and one line per package in the dependency
    closure of `oteryn-world-bundle-compiler` and `oteryn-world-bundle`, over
    every lock edge. A path package outside the World crates in that closure,
    and an unknown lock form, are refused.
  - root `Cargo.toml`: every statement except comments, the `[workspace]`
    member lists and `[workspace.dependencies]` entries naming no package of
    the closure. Profiles, patches, lints and `[workspace.package]` count as
    written.
- `INPUT_PATHS` and `WORLD_BUNDLE_INPUT_PATHS` are unchanged, so the World lane
  still runs on any `Cargo.toml` or `Cargo.lock` change.
- An unmerged index path is refused.
- The pin's `inputs_digest` is rotated once. The bundle digest is unchanged.

The owner approved this rotation. No integrity check is removed: a change to
any package the World crates build still moves the pin.

## Validation

- `cargo +1.99.1 fmt --all -- --check`, `git diff --check`.
- `cargo +1.99.1 clippy --locked -p oteryn-world-bundle-compiler --all-targets -- -D warnings`.
- `cargo +1.99.1 test --locked -p oteryn-world-bundle-compiler`, including the
  `cargo_inputs` unit tests and the new `pin_check` case.
- `pin-refresh` then `pin-check` rc=0 on the exact head.
- `python3 tools/repository/validate_repository_policy.py`.

## Handoff

The integrator publishes this for exact-head CI after FLOW-1.
