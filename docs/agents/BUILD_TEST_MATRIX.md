# Build and test matrix

Status: bootstrap; replace assumptions with discovered workspace commands as code is introduced.

Canonical native E2E architecture: `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` (`QA-E2E-01`).

## Selection principles

- Validate proportionally to changed paths and risk.
- Cheap focused checks run during implementation; heavy checks run at coherent package/final head.
- Exact-head required checks cannot be replaced by historical or parent results.
- Do not claim commands exist until their manifests/workflows are present.
- Environment startup alone is not successful E2E.
- Hidden retry-until-green is forbidden; every physical attempt and cleanup outcome remains visible.
- A headless system scenario does not prove native-client presentation, and an instrumented client does not prove the exact production binary.

## Current repository checks

| Change | Focused validation | Exact-head validation |
|---|---|---|
| Agent governance/prompt/task docs | `python tools/agents/validate_governance.py` | `agent-governance` workflow |
| Architecture/contracts only | governance validator plus link/JSON checks | documentation/governance workflow |
| GitHub workflow | syntax/review of permissions/triggers | workflow run on exact head |

## Planned Rust workspace checks

Once a root Cargo workspace exists, update this matrix with exact commands. Expected baseline, only after discovery:

- `cargo fmt --all -- --check`;
- product-realistic client, server and Studio target/feature Clippy builds with `-D warnings`;
- supplemental `--all-features` checks only where they do not create impossible mutually exclusive configurations;
- `cargo test --workspace` plus focused package tests and target-specific integration tests;
- `cargo metadata --locked` validation against the retained machine-readable workspace-boundary contract;
- dependency/advisory/license checks selected by accepted tooling;
- parser property/fuzz tests for untrusted protocol/content inputs;
- deterministic simulation/replay tests;
- platform-specific client build/runtime tests on exact named target triples;
- shared foundation failure-scenario tests, including time/clock, dependency loss, stale generation and overload cases;
- multichannel integration, crash-recovery and soak scenarios.

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

A documentation-only final commit does not require a nonexistent Rust build. It does require the governance/document validation selected by current workflows and an accurate `NOT_APPLICABLE` reason for runtime E2E.
