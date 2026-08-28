# Build and test matrix

Status: active repository baseline; update this matrix whenever executable workspace or merge-gate behavior changes.

Canonical native E2E architecture: `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` (`QA-E2E-01`).

## Selection principles

- Validate proportionally to changed paths and risk.
- Cheap focused checks run during implementation; heavy checks run at coherent package/final head.
- Exact-head required checks cannot be replaced by historical or parent results.
- `Merge gate / validate` is the stable protected-branch PR check and may succeed only when every applicable sub-gate succeeds.
- Rust/workspace validation is path-proportional but cannot be bypassed by changing CI/workspace policy itself.
- Environment startup alone is not successful E2E.
- Hidden retry-until-green is forbidden; every physical attempt and cleanup outcome remains visible.
- A headless system scenario does not prove native-client presentation, and an instrumented client does not prove the exact production binary.

## Current pull-request merge gate

`.github/workflows/merge-gate.yml` runs on every pull request to `main` without workflow-level path filters.

Always-required sub-gates:

- PR metadata, agent-governance and repository-policy validation;
- GitHub Dependency Review with `high` severity as the failure threshold;
- CodeQL for repository Python and GitHub Actions code;
- final aggregate `Merge gate / validate`.

When Rust/workspace-relevant paths change, the same merge gate additionally requires:

- Rust policy/metadata validation;
- Linux workspace build, strict Clippy, tests and synthetic harness;
- Windows production-client build, strict Clippy, smoke and synthetic harness;
- `cargo-deny` advisory/license/ban/source validation.

The protected `main` ruleset requires only the stable aggregate `Merge gate / validate` context. Individual sub-gates are intentionally composed behind it so path-proportional jobs may be skipped without creating missing required-status deadlocks.

## Current focused validation

| Change | Focused validation | Exact-head PR validation |
|---|---|---|
| Agent governance/prompt/task docs | `python tools/agents/validate_governance.py` | `Merge gate / governance` → `Merge gate / validate` |
| Repository/GitHub policy | `python tools/repository/validate_repository_policy.py` | governance + dependency review + CodeQL + applicable Rust jobs → aggregate gate |
| Architecture/contracts only | governance validator plus applicable link/JSON/schema checks | always-required merge-gate subchecks; runtime E2E may be `NOT_APPLICABLE` with reason |
| Rust/workspace/client code | package-focused tests while editing | full path-triggered Rust policy/Linux/Windows/supply-chain merge-gate set |
| GitHub workflow affecting Rust validation | repository-policy validation plus workflow review | full Rust merge-gate set because merge-gate/rust workflow paths are Rust-validation-sensitive |

## Current Rust workspace commands

The canonical root Cargo workspace exists and is enforced by `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `deny.toml` and `workspace-boundaries.toml`.

Current exact baseline uses Rust `1.94.0` and includes:

- `cargo +1.94.0 metadata --locked --format-version 1`;
- `cargo +1.94.0 fmt --all --check`;
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`;
- production dependency-closure negative checks for forbidden pre-native/runtime packages;
- `cargo +1.94.0 build --locked --workspace --all-targets` on Linux;
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings` on Linux;
- `cargo +1.94.0 test --locked --workspace`;
- `cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness`;
- Windows release build for `oteryn-client` on `x86_64-pc-windows-msvc`;
- Windows strict client Clippy and `--smoke` launch;
- `cargo-deny check --all-features` through the pinned cargo-deny action.

Post-merge/manual `.github/workflows/rust.yml` preserves the same current workspace baseline independently of the PR aggregate gate.

## Required additions as owning layers appear

Do not create speculative tests for nonexistent runtime layers. Add these when their owning implementation exists:

- parser property/fuzz tests for untrusted protocol/content inputs;
- canonical/golden protocol byte fixtures and malformed/adversarial corpora;
- deterministic simulation/replay tests;
- server target/feature builds and strict Clippy;
- persistence migration, concurrency, rollback and crash-recovery tests;
- shared foundation failure-scenario tests, including time/clock, dependency loss, stale generation and overload cases;
- multichannel integration, crash-recovery and soak scenarios;
- sanitizer/Miri or equivalent targeted undefined-behavior checks where they provide evidence beyond the workspace-wide `unsafe_code = "forbid"` baseline.

## `QA-E2E-01` execution tiers

| Tier | Purpose | Default placement | Does not prove |
|---|---|---|---|
| Tier 1 — headless system E2E | Broad deterministic Platform → Gateway → protocol → server → PostgreSQL coverage using production transport and schemas | focused PR gates, protected main, nightly fault/concurrency campaigns | renderer, UI interaction, final client packaging |
| Tier 2 — instrumented native-client E2E | Real Rust client networking, input, reconciliation, UI and rendering through a test-only bounded observation adapter | affected client-facing PRs, protected main journeys, nightly repeated populations | exact production-default binary behavior |
| Tier 3 — production-binary smoke E2E | Exact release-candidate client/server artifacts without the in-process test adapter | release candidate and named packaging/platform gates | broad fault, concurrency or exhaustive gameplay coverage |

A feature or programme selects the smallest sufficient set of tiers, but a supported user journey that includes native-client behavior cannot be marked `PROVEN` from Tier 1 alone. `VSL-01` completion requires the named `QA-E2E-01` evidence in ADR-0007.

## Mandatory E2E evidence

Every counted attempt records:

- exact client, server and Platform revisions or artifact hashes;
- protocol, ruleset, content, World Bundle and migration revisions;
- scenario, tier, topology, seed, clock mode and fault profile;
- ordered phase outcomes and the first divergence;
- client/server/Platform/persistence/audit evidence required by the scenario;
- cleanup status and retained artifact hashes.

Canonical phases are environment, identity, world discovery, Gateway, Game Session, transport, admission, character lease, world entry, gameplay, persistence, audit/outbox, client presentation and cleanup. Non-applicable phases require a scenario-defined reason.

## High-risk acceptance

| Area | Minimum additional evidence |
|---|---|
| Protocol/framing | limits, negative cases, sequencing, replay/downgrade, golden fixtures, Tier 1 client/server E2E and a native-client journey for supported client behavior |
| Character lease/relog | double-login, stale writer/session generation, crash/recovery, cross-channel misuse, exact final offline state |
| Inventory/loot/market | idempotency, concurrency, rollback, item/currency conservation, no-duplication failure paths, audit/outbox reconciliation |
| Multichannel runtime | two-channel isolation, shared-world services, channel failure, revision compatibility, multiclient evidence |
| Persistence/migrations | isolated migration tests, rollback/compatibility plan, concurrent mutation tests, dependency-loss and restart E2E |
| Client renderer/UI | named platform/hardware/scene, Tier 2 interaction and device-loss/recovery where relevant, Tier 3 release smoke |
| Assets/updater | provenance, signatures/hashes, traversal/decompression limits, rollback and exact production-binary smoke |
| Platform/admission | exact Platform contract/service revision, ticket/session expiry/replay/revocation, Gateway routing and cross-world/channel misuse |

## Stability classification

Repeated-run certification uses a fixed, exact comparison cell and minimum population:

- `PASS` — every counted attempt completes the journey and cleanup;
- `UNSTABLE` — mixed outcomes;
- `FAIL` — deterministic product failure or all usable attempts fail acceptance;
- `BLOCKED` — incomplete/inconsistent evidence, tampering, or unknown cleanup;
- `NOT_EVALUATED` — minimum population not reached.

A repaired runner or environment requires a new population. It does not rewrite the historical result.

## Documentation-only rule

A documentation-only final commit does not automatically require Rust build/test jobs when no Rust/workspace validation path is affected. It always requires the always-on merge-gate governance, dependency-review and CodeQL layers plus an accurate `NOT_APPLICABLE` reason for runtime E2E when runtime behavior is not changed.
