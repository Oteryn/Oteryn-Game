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
- when the target is not yet allocated on the candidate or its PR diff, record an explicit `NOT_APPLICABLE` result rather than claiming PostgreSQL E2E PASS.

Ordinary Rust-relevant candidates run the target automatically; deleting or renaming it cannot convert that evidence into a skip. Both upstream scope and target classification enumerate the immutable comparison of exact base/head SHAs, retaining before/after PR identity checks. A transient A-to-B-to-A PR movement cannot substitute another revision's files. GitHub returns at most 300 files for a comparison: larger PRs, missing file arrays or a count mismatch make scope explicitly incomplete and select FULL. The downstream PG target classifier still rejects an incomplete comparison, so uncertain target evidence cannot yield a green gate. PR identity movement remains a hard failure.

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

Neutral Markdown is limited to README/CHANGELOG/CONTRIBUTING and docs Markdown, with AGENTS and migration exclusions. Rust omission also requires a separate reviewed snapshot including the server tree: a later server change may introduce a document input, so stale document-consumer assumptions select FULL. Other dedicated contract/architecture workflows remain unchanged. Mixed material surfaces select FULL; accompanying neutral task documentation does not invalidate an otherwise proven server change. Cross-surface renames select FULL.

Issue #309 re-audits the document-consumer snapshot after five server test additions/changes since #297: `authority_invariants.rs`, `durability_postgres.rs`, `server_ci_qualification.rs`, `support/authority_matrix.rs` and `support/authority_recovery.rs` under `apps/game-server/tests/`. The new tests consume in-memory fixtures, PostgreSQL records and Cargo-built/current test executables, not documentation. No server production, Cargo, migration or build input changed in that review range. The reviewed snapshot adoption restores existing PR documentation eligibility; it does not extend the neutral path family, change always-required gates or alter post-merge/Merge Queue routing. A later consumer edit still selects FULL until separately reviewed; snapshots must never update automatically merely to preserve savings. Regression fixtures bind the independently reviewed input contract rather than require every future tree to remain unchanged. Actual hosted qualification and savings remain on the governing Issue.

Missing classifier, malformed metadata or enumeration select explicit FULL outputs. The aggregate requires successful classification, strict boolean outputs and success for every selected predicate; missing/cancelled/failed/selected-skipped results fail closed. Scope, classifier job, aggregate and evidence-job execution are pinned and mutation-tested.

After #285 protected-main integration proved canonical ownership, rust.yml loses only its redundant PR trigger. Existing main/manual triggers and full Merge Queue qualification remain intact. Actual hosted skip/run and runner/wall-time benchmark evidence belongs on Issue #283/PR #297; staged implementation integration alone does not complete benchmark acceptance.

The protected `main` ruleset requires only the stable `game-gate` context. Individual sub-gates are intentionally composed behind it so applicable path-proportional jobs may be skipped without creating missing required-status deadlocks.

## Current Merge Queue gate

`.github/workflows/merge-group-gate.yml` validates the exact synthetic merge-group head and requires candidate/governance validation, dependency review, CodeQL, Linux workspace, real PostgreSQL17.6, Windows production client plus deterministic simulation and supply-chain checks before emitting `game-gate`.

Issue #285 activates exactly the workflow blob preapproved by integrated #284. PostgreSQL runs in an unconditional job with a pinned17.6 service, verifies the synthetic head, requires the durability test target and executes it. Windows verifies that same head before its production client and simulation tests. The aggregate requires success from PostgreSQL and Windows; skipped, missing, cancelled or failed results reject integration. The complete queue workflow is pinned by executable policy; intentional changes require a separately reviewed protected-base pin rotation. Current exact-head execution and protected-main integration evidence are recorded on Issue #285/PR #296; source presence alone is not execution evidence. The queue regression suite runs through the existing canonical governance regression driver.

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

### Protected-main post-merge lanes (#304)

Standalone `.github/workflows/rust.yml` runs on every push to main, without path filters, and on manual dispatch. Policy and supply chain always run. Issue #311 extends the existing protected-main adapter to omit Linux workspace, PostgreSQL 17.6 and Windows/SIM only for proven neutral documentation. Manual dispatch remains FULL. Merge Queue remains FULL and its workflow and PR `game-gate` are unchanged.

Only a normal push to protected `refs/heads/main` can omit runtime lanes. The lane job verifies the exact already-protected event SHA, obtains full Git history, checks before/after ancestry, and enumerates the complete tree diff locally (including both rename sides, without the API's 300-file cap). It runs the existing #283 classifier and Cargo metadata from that protected revision, including the reviewed all-consumer snapshot for documentation. It does not trust PR labels/body or the push event's capped commits array. This is post-integration protected code, unlike the PR classifier's untrusted candidate.

| Post-merge input | Standalone lanes |
|---|---|
| Neutral documentation with both reviewed consumer snapshots | Policy + supply chain; Linux, PostgreSQL and Windows/SIM not applicable |
| Proven server-only with matching reviewed consumer snapshot | Linux + PostgreSQL + policy + supply chain |
| Client/shared/simulation, mixed material surfaces | FULL, including Windows production/SIM |
| Cargo/toolchain/build/workflow/control-plane/unknown/incomplete | FULL |
| Manual dispatch; malformed event, missing ancestry/metadata, classifier failure | FULL |

The #283 reverse dependency closure and reviewed consumer snapshot retain their conservative semantics; stale snapshots select FULL. The classifier writes to a fresh private output file. Only complete canonical `rust/windows` pairs `false/false`, `true/false` or `true/true` are published; failure, partial, duplicate, malformed or contradictory output becomes explicit FULL. Linux/PostgreSQL require successful classification and both exact `false` outputs to omit execution; Windows retains its successful exact-`false` fallback. Policy and supply chain remain independent of classification. A subsequent push does not cancel an earlier post-merge run. The real golden command remains unconditional inside selected Windows. Canonical repository-policy validation executes real-Git adapter and actual shell-output fixtures and checks the reviewed workflow pin. Actual timing and run evidence live in Issues #304 and #311; replay or projected savings do not substitute for observed hosted decisions.

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
