# TEST-FUZZ-1

```yaml
task_id: TEST-FUZZ-1
title: Bounded fuzz and round-trip tests for every protocol-oteryn decoder
mode: REPAIR
status: validated
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/happy-davinci-ijz3wy-test-fuzz-1
pr: null
base_sha: ac3f7fbc6279687c71d6ac3ec43eb27383451003
owner: Claude integrator
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
owned_paths:
  - crates/test-support/src/property.rs
  - crates/test-support/src/lib.rs
  - crates/protocol-oteryn/Cargo.toml
  - crates/protocol-oteryn/src/
  - Cargo.lock
  - docs/agents/tasks/TEST-FUZZ-1.md
public_contracts: []
depends_on: []
blocks: [TEST-PROP-1]
external_repositories: []
registry: no
plan_row: owner request for decoder fuzz tests and pure-rule property tests
```

## Problem

The `protocol-oteryn` decoders read untrusted client and server bytes. Their
tests cover golden vectors and chosen bounds, but nothing feeds them
arbitrary, truncated or mutated input. A panic, an unbounded allocation from
a length prefix or a non-canonical round trip would go unnoticed.

The custody rule forbids new third-party crates, so `proptest` and
`cargo-fuzz` are not options.

## Fix

- `crates/test-support/src/property.rs` adds a std-only helper:
  - `Rng`, a seeded generator;
  - `for_each_case`, which runs a property and prints a replay seed on failure;
  - `fuzz_decoder`, which feeds seed mutations and random bytes and fails on any panic;
  - `fuzz_round_trip`, which checks `decode(encode(decode(x))) == decode(x)` for every accepted input.
- `crates/protocol-oteryn/src/decoder_fuzz_support.rs` holds the shared harness. Each
  wire module keeps its own generators:
  - `decoder` checks that every seed decodes, every truncation is safe, and
    garbage such as a `u32::MAX` length prefix with a tiny buffer is refused.
  - `codec` adds a generated-value round trip.
- Tests sit next to their module: in the existing `*_tests.rs` file, or in a
  sibling `<module>_fuzz_tests.rs` mounted with `#[cfg(test)] #[path]` when
  the module keeps its tests inline. No single fuzz file.
- `oteryn-test-support` becomes a workspace-internal dev-dependency of
  `oteryn-protocol-oteryn`; the only `Cargo.lock` change is that edge.

## Coverage

There are 83 fuzz tests. Together they reach all 103 public `decode_*` and
`parse_*` entry points in `crates/protocol-oteryn/src`; some tests drive
several decoders of one wire family.

## Findings

No decoder panicked, allocated from an unchecked length or accepted the
garbage samples.

One canonicalization was confirmed. `encode_actor_cooldowns` sorts entries by
`(kind, id)`, while `decode_actor_cooldowns` keeps wire order. Entry order
carries no meaning, so the cooldown round trip compares sorted entries. The
wire format is unchanged.

## Running heavier

The defaults are 256 cases per property. The whole `--lib` suite runs in about
0.1 s on Molehill.

- `OTERYN_PROPERTY_CASES=<n>` raises the case count, up to 1,000,000.
- `OTERYN_PROPERTY_SEED=<u64>` replays a run.

A failure prints both variables plus the per-case `Rng::new` seed.

## Validation

- `cargo fmt --all -- --check`, `git diff --check`.
- `cargo clippy -p oteryn-protocol-oteryn -p oteryn-test-support --all-targets -- -D warnings`.
- `cargo test -p oteryn-protocol-oteryn`: 380 passed.
- `cargo test -p oteryn-test-support`: 13 passed.
- `--lib fuzz` with seeds 11, 222, 3333, 44444 and with 5,000 cases: 83 passed each time.
