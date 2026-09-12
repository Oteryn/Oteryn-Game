# Build and test matrix

Status: active repository baseline; update this matrix whenever executable workspace or merge-gate behavior changes.

Canonical native E2E architecture: `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` (`QA-E2E-01`).

## Selection principles

- Validate proportionally to changed paths and risk.
- Cheap focused checks run during implementation; heavy checks run at coherent package/final head.
- Exact-head required checks cannot be replaced by historical or parent results.
- `game-gate` is the stable protected-branch status; on pull requests it may succeed only when internal `Merge gate / validate` proves every applicable sub-gate.
- Rust/workspace validation is dependency-aware and conservative but cannot be bypassed by changing CI/workspace policy itself.
- Environment startup alone is not successful E2E.
- Hidden retry-until-green is forbidden; every physical attempt and cleanup outcome remains visible.
- A headless system scenario does not prove native-client presentation, and an instrumented client does not prove the exact production binary.

## Current pull-request merge gate

`.github/workflows/merge-gate.yml` runs on every pull request to `main` without workflow-level path filters.

Always-required sub-gates:

- exact PR identity and protected-base risk classification;
- PR metadata, agent-governance and repository-policy validation;
- GitHub Dependency Review with `high` severity as the failure threshold;
- CodeQL for repository Python and GitHub Actions code;
- internal aggregate `Merge gate / validate`;
- final stable status `game-gate`.

For full-risk changes, the same merge gate additionally requires:

- Rust policy/metadata validation;
- exact-head Linux workspace build, strict Clippy, tests and synthetic harness;
- a pinned PostgreSQL 17.6 service plus deletion-safe routing for `oteryn-game-server --test durability_postgres` inside the required Linux job;
- exact-head Windows production-client build, strict Clippy, smoke and synthetic harness;
- deterministic Windows `oteryn-simulation-determinism` golden fixtures inside the required Windows job;
- `cargo-deny` advisory/license/ban/source validation.

The PostgreSQL test target is present on protected main after terminal-replacement PR #252. The canonical Linux job uses these fail-closed rules:

- when `apps/game-server/tests/durability_postgres.rs` exists on the exact candidate, run it against PostgreSQL 17.6;
- when the exact PR removes or renames that target, fail the required Linux job;
- when exact base/head target observations prove the target is absent on both revisions, record an explicit `NOT_APPLICABLE` result rather than claiming PostgreSQL E2E PASS.

Ordinary Rust-relevant candidates run the target automatically; deleting or renaming it cannot convert that evidence into a skip. Upstream scope uses the immutable base/head comparison; larger-than-300-file comparisons, missing arrays or count mismatches select FULL. The downstream PostgreSQL target classifier instead inspects the exact commit trees at base and head, requires complete valid tree evidence, and verifies the checkout's target blob. It does not use the capped comparison as target-presence authority. Both boundaries retain before/after PR identity checks, and uncertain/mismatched target evidence fails closed.

The required governance job executes the focused PG/SIM regressions, including both real classifiers against controlled GitHub responses and job/step failure-tolerance/skip families. The complete Linux and Windows evidence jobs are pinned by SHA256 using the existing canonical-job validation pattern. Future intentional job changes must update their reviewed pins; preserving command strings while inserting an early successful exit cannot pass policy.

### Trusted-base risk lanes (#283)

The lane job checks out and verifies the exact protected base, then runs its classifier and pinned Cargo1.94 metadata there. Candidate labels/body/code do not determine selection. Every local normal/dev/build/optional/target-specific dependency participates in reverse closure. The implementation PR runs FULL because its protected base lacks the classifier; candidate-classifier observation in Rust policy is diagnostic only and cannot alter lane selection.

| Proven surface | Required Rust lanes |
|---|---|
| Neutral root/documentation Markdown | none; all always-required checks still run |
| Server-only, including durability/migrations/reconnect | Linux workspace + real PG17.6 + strict Clippy, policy and supply chain |
| Client, shared or simulation | full Linux/PG + Windows production/SIM + policy and supply chain |
| Control plane, dependencies/build inputs, unknown/mixed/incomplete evidence | full set |

Server-only Windows/SIM omission also requires the reviewed SHA256 snapshot of all non-server workspace package trees and root Cargo/toolchain/build inputs. Cargo alone does not model include macros, symlinks or runtime file reads. Current reviewed consumers do not read server inputs; any consumer-tree/dependency change disables the optimization until a reviewed classifier update adopts its new input contract. Symlinks/submodules select FULL. This deliberately conservative snapshot may reduce savings after unrelated consumer changes; it never silently assumes their new input dependencies are safe.

Neutral Markdown is limited to README/CHANGELOG/CONTRIBUTING and docs Markdown, with AGENTS and migration exclusions. Rust omission uses a separately reviewed protected-main document-consumer baseline plus a bounded baseline-to-current drift proof over root build inputs and all workspace package roots. The proof does not infer safety from a finite absence-of-marker scan: added/deleted/type-changed inputs select FULL, and modified Rust is admitted only when every changed line is blank or an ordinary non-doc `//` comment. Doc comments (`///` and `//!`) are attributes and macro-visible, so they select FULL along with scalar constants/functions, other executable lines and any unparsed or uncertain drift. Other modified workspace input types also select FULL. Because only changed lines are evaluated, unchanged audited consumers do not invalidate later ordinary-comment/blank-only edits. Cargo/build-input changes, special modes, unavailable or malformed baseline/Git evidence also select FULL. The historical broad all-consumer digest remains a regression/fallback contract, but live neutral-doc freshness no longer requires repinning it after unrelated proven-safe source edits. Other dedicated contract/architecture workflows remain unchanged. Mixed material surfaces select FULL; accompanying neutral task documentation does not invalidate an otherwise proven server change. Cross-surface renames select FULL.

Issue #309 re-audits the historical document-consumer snapshot after five server test additions/changes since #297: `authority_invariants.rs`, `durability_postgres.rs`, `server_ci_qualification.rs`, `support/authority_matrix.rs` and `support/authority_recovery.rs` under `apps/game-server/tests/`. The new tests consume in-memory fixtures, PostgreSQL records and Cargo-built/current test executables, not documentation. No server production, Cargo, migration or build input changed in that review range. The historical snapshot adoption restored existing PR documentation eligibility without extending the neutral path family, changing always-required gates or altering post-merge/Merge Queue routing. #580/#581 supersede only neutral-document freshness: later harmless source drift may remain eligible through the bounded proof, while actual or uncertain consumer drift still selects FULL. Snapshots must never update automatically merely to preserve savings. Regression fixtures bind independently reviewed input contracts. Actual hosted qualification and savings remain on the governing Issue.

Missing classifier, malformed metadata or enumeration select explicit FULL outputs. The aggregate requires successful classification, strict boolean outputs and success for every selected predicate; missing/cancelled/failed/selected-skipped results fail closed. Scope, classifier job, aggregate and evidence-job execution are pinned and mutation-tested.

After #285 protected-main integration proved canonical ownership, rust.yml lost its redundant PR trigger while retaining main/manual triggers. The rollout evidence on Issue #283/PR #297 is historical; current Merge Queue selection is described separately below. Staged implementation integration alone does not complete benchmark acceptance.

The protected `main` ruleset requires only the stable `game-gate` context. Individual sub-gates are intentionally composed behind it so applicable path-proportional jobs may be skipped without creating missing required-status deadlocks.

## Current Merge Queue gate

The inspected `.github/workflows/merge-group-gate.yml` at `main@663bd35a5196a925fc6eb0318381ad0b97f4cc2c` is pinned to blob `c59b30fde7538e738346eec03a602081dc4ac2d6`. It validates the exact synthetic candidate and always requires candidate/governance, dependency review and CodeQL before publishing `game-gate`.

| Exact queue classification | Selected additional jobs |
|---|---|
| Complete, valid diff only of Markdown paths under `docs/architecture/`, with eligible object modes for non-deleted changed paths (`architecture-docs`) | Rust Linux, PostgreSQL, Windows and supply-chain jobs may be skipped |
| Other, mixed, special-mode, malformed or incomplete classification (`full`) | Linux workspace, real PostgreSQL 17.6, Windows client/SIM and supply chain |

The inline queue classifier reads exact base/head Git evidence, including both sides of renames through `--no-renames`; it is not the PR/post-merge consumer-snapshot classifier. Its `architecture-docs` path predicate does not establish that no runtime consumer reads those documents. Do not describe it as a consumer-closure proof or extend the exception through this documentation. A relevant document-input dependency requires owning control-plane review of the admission assumption; this matrix does not repair or authorize routing changes.

The aggregate accepts only coherent `true/true` or `false/false` selections. Selected jobs must succeed; only unselected jobs may report `skipped`. Missing/failed/cancelled mandatory or selected evidence cannot qualify the candidate. For FULL, PostgreSQL verifies the synthetic head and requires the durability test target; Windows verifies that same head before its build/smoke/SIM commands.

Issue #285/PR #296 records the earlier full-queue rollout, not a claim that the current pinned workflow has unconditional runtime jobs. Workflow or pin changes require their own reviewed control-plane change. Source presence and a docs-only queue PASS are not runtime execution evidence.

## Current focused validation

| Change | Focused validation | Exact-head PR validation |
|---|---|---|
| Agent governance/prompt/task docs | `python tools/agents/validate_governance.py` | `Merge gate / governance` → `Merge gate / validate` → `game-gate` |
| Repository/GitHub policy | `python tools/repository/validate_repository_policy.py` | governance + dependency review + CodeQL + applicable Rust jobs → aggregate gate |
| Architecture/contracts only | governance validator plus applicable link/JSON/schema checks | always-required merge-gate subchecks; runtime E2E may be `NOT_APPLICABLE` with reason |
| Rust/workspace/client code | package-focused tests while editing | conservative trusted-base lanes above; selected predicates must all succeed |
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
- deletion-safe conditional `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres` against pinned PostgreSQL 17.6 when the target is allocated on the exact PR head;
- `cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness`;
- Windows release build for `oteryn-client` on `x86_64-pc-windows-msvc`;
- Windows strict client Clippy and `--smoke` launch;
- `cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc` for Rust-relevant pull requests;
- `cargo-deny check --all-features` through the pinned cargo-deny action.

### Client and architecture evidence boundaries

At the inspected revision, `tools/architecture-check/src/lib.rs` validates workspace-local edges and release-role closure. Additional external registry/git and transitive closure evidence is needed when a foundation promises framework/platform/GPU neutrality. The host-default production `cargo tree` checks do not by themselves cover every target/feature combination. Bind such claims to exact package identities, manifests, lockfile, target and feature selection.

The Windows jobs run client build/Clippy, shell smoke, synthetic harness and simulation-determinism tests. They do not execute all client, input-platform or renderer dependency test suites on Windows. Affected implementation slices must run their named platform-specific package tests and native fixtures; compilation is not test execution. The shell smoke uses `cargo run` without `--release`, separately from the release build, so it is not a test of the exact release artifact. The inspected shell also exits smoke before renderer construction.

The dedicated architecture semantic workflow selects explicit profiles, not arbitrary architecture Markdown. Record its semantic verdict and selected checks separately from workflow conclusion. On #560 head `db502e473bda60f7cdea2498f5138705c2cf0bea`, run `34562444300` / job `103147774477` passed its dispatcher tests but reported `SEMANTIC_AUDIT_NOT_APPLICABLE` with no UI profile/checks. This was workflow success, not semantic UI PASS or independent KEEP. New or changed coverage requires an allocated implementation change, not reinterpretation of an old green status.

### Protected-main post-merge lanes (#304)

Standalone `.github/workflows/rust.yml` runs on every push to main, without path filters, and on manual dispatch. Policy and supply chain always run. Issue #311 extended the protected-main adapter to omit all runtime lanes for proven neutral documentation, alongside the existing server-only Windows omission. Manual dispatch remains FULL. This post-merge classifier does not determine Merge Queue selection; the current queue exception is described above.

Only a normal push to protected `refs/heads/main` can omit runtime lanes. The lane job verifies the exact already-protected event SHA, obtains full Git history, checks before/after ancestry, and enumerates the complete tree diff locally (including both rename sides, without the API's 300-file cap). It runs the existing #283 classifier and Cargo metadata from that protected revision, including the same reviewed document-consumer baseline and bounded protected drift proof used by neutral-document PR routing. It does not trust PR labels/body or the push event's capped commits array. This is post-integration protected code, unlike the PR classifier's untrusted candidate.

| Post-merge input | Standalone lanes |
|---|---|
| Neutral documentation with successful bounded document-consumer proof | Policy + supply chain; Linux, PostgreSQL and Windows/SIM not applicable |
| Proven server-only with matching reviewed consumer snapshot | Linux + PostgreSQL + policy + supply chain |
| Client/shared/simulation, mixed material surfaces | FULL, including Windows production/SIM |
| Cargo/toolchain/build/workflow/control-plane/unknown/incomplete | FULL |
| Manual dispatch; malformed event, missing ancestry/metadata, classifier failure | FULL |

The #283 reverse dependency closure and reviewed non-server snapshot retain their conservative server-only semantics. Neutral-document routing instead uses the reviewed protected-main document-consumer baseline plus bounded drift proof described above; proof failure is explicit FULL. The classifier writes to a fresh private output file. Only complete canonical `rust/windows` pairs `false/false`, `true/false` or `true/true` are published; failure, partial, duplicate, malformed or contradictory output becomes explicit FULL. Linux/PostgreSQL require successful classification and both exact `false` outputs to omit execution; Windows retains its successful exact-`false` fallback. Policy and supply chain remain independent of classification. A subsequent push does not cancel an earlier post-merge run. The real golden command remains unconditional inside selected Windows. Canonical repository-policy validation executes real-Git adapter and actual shell-output fixtures and checks the reviewed workflow pin. Actual timing and run evidence live in Issues #304, #311 and #580; replay or projected savings do not substitute for observed hosted decisions.

The Linux/PostgreSQL dependency on classification introduces a serial startup cost for FULL/server runs. The measured docs baseline `33973093609` allocated 888 seconds: runtime jobs 818 seconds, classification 12 seconds, policy 21 seconds and supply chain 37 seconds. These are separate observed job durations, not a controlled A/B or achieved savings from #311. A roughly 12-second classification dependency can delay Linux/PostgreSQL eligibility; queue overlap and the workflow critical path determine its actual wall-time effect. Natural post-deployment docs and FULL measurements are required before claiming net savings.

## Required additions as owning layers appear

Do not create speculative tests for nonexistent runtime layers. Add these when their owning implementation exists:

- parser property/fuzz tests for untrusted protocol/content inputs;
- canonical/golden protocol byte fixtures and malformed/adversarial corpora;
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

A native window rendering and interacting with labelled synthetic UI/world fixtures is useful component qualification, not automatically Tier 2. Tier 2 retains the real journey and production-contract requirements of ADR-0007; Tier 3 retains exact release-artifact identity. Early UI foundation/native-fixture work need not wait for full server E2E, but it must not borrow an E2E tier label or erase required phases with ad hoc `NOT_APPLICABLE` claims.

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
