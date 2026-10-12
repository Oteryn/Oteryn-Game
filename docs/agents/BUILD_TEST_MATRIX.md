# Build and test matrix

Status: active hot-path validation contract. Machine implementations own exact routing; this document records the current selection semantics and evidence boundaries.

Canonical native E2E architecture: `docs/architecture/ADR-0007-native-end-to-end-test-platform.md` (`QA-E2E-01`).

## Machine authority

Use the exact candidate/tree and current protected machine implementations:

- PR lane classifier: `tools/repository/classify_pr_test_lanes.py`;
- PR routing contract: `tools/repository/validate_pr_routing_contract.py` plus focused tests under `tools/repository/test_classify_pr_test_lanes.py`;
- repository-policy/fan-in checks: `tools/repository/validate_repository_policy.py` and `validate_repository_policy_core.py`;
- PostgreSQL/SIM routing regressions: `test_validate_pr_gate_pg_sim.py` and `test_validate_merge_group_pg_sim.py`;
- PR aggregate: `.github/workflows/merge-gate.yml`;
- Merge Queue aggregate: `.github/workflows/merge-group-gate.yml`;
- protected-main post-merge lanes: `.github/workflows/rust.yml`.

If this summary conflicts with those protected machine contracts, fail closed and reconcile the documentation; do not weaken the machine gate.

## Selection principles

- Validate proportionally to changed paths and risk.
- Focused checks may run during implementation; required qualification applies to the frozen exact head.
- Historical/parent-head results never substitute for current exact-head evidence.
- `game-gate` is the stable protected status and succeeds only after every selected required predicate succeeds.
- Unknown, malformed, incomplete, special-mode or unclassified evidence fails closed to the broader lane.
- Hidden retry-until-green is forbidden; physical attempts and cleanup outcomes stay visible.
- Environment startup is not E2E success; compilation is not native presentation proof.

## Pull-request gate

`.github/workflows/merge-gate.yml` runs on every PR to `main`.

The pull-request gate is light: heavy qualification runs once, in the Merge Queue
(D431). Always-required evidence includes:

- exact PR/base/head identity and trusted-base risk classification;
- exact-head routing-contract validation against candidate Cargo/tree state;
- PR metadata, agent governance and repository policy;
- Dependency Review;
- when the Rust lane is selected: Rust policy/metadata/fmt/production closure, supply chain and
  `Merge gate / Rust changed-crate Clippy and unit tests` (strict Clippy `--all-targets` and
  `--lib`/`--bins` tests of the changed workspace crates; when the scope transport is incomplete
  (over 300 files or 32 KiB of records), the changed paths are recovered from the exact base/head
  Git trees as the trusted-base classifier does; unrecoverable enumeration, workspace-wide Rust
  inputs or no attributable crate select every workspace crate);
- aggregate `Merge gate / validate` and final `game-gate`.

CodeQL, full Linux/PostgreSQL, Windows client/input/SIM and installer lifecycle,
Atlas fullworld, node boot and Server Seam execute in the Merge Queue on the exact
synthetic candidate. The PR gate contains no heavyweight jobs or `full-ci` switch.
All label-only and ordinary metadata edits re-run the exact-head PR gate in an
isolated concurrency group without cancelling a running qualification; retargets
still require canonical qualification.

Fast Rust selection on the PR:

| Proven PR surface | Fast PR evidence |
|---|---|
| Unconsumed auxiliary inputs: neutral docs, agent governance, standalone workflows and non-Cargo offline tooling | no Rust product lanes; always-required governance/security/routing and any dedicated workflow remain |
| Auxiliary file referenced by exact-candidate Cargo package source/build input | route as the consuming package, then apply normal reverse Cargo closure |
| Audited Atlas fullworld producer/self-test surface | always-required fast PR checks and any applicable dedicated workflow; add Rust checks only if another consumer selects them |
| Server-only, including server-consumed auxiliary inputs | changed-crate Clippy/unit tests + Rust policy/supply chain |
| Client/shared/simulation, including their consumed auxiliary inputs | changed-crate Clippy/unit tests + Rust policy/supply chain |
| Canonical routing controls (`merge-gate.yml`, `merge-group-gate.yml`, `rust.yml`), `.github/actions/**`, `tools/repository/**`, `docs/migration/**`, Cargo/toolchain/build inputs, unknown/mixed/incomplete evidence | conservative workspace fast Rust checks + policy/supply chain |

Physical server qualification runs only in Merge Queue. It runs node boot and Server Seam against the real Platform (`Merge Queue / Node boot against the real Platform`, `Merge Queue / Server Seam over TCP+TLS`). These run when a change can reach the shipped server's boot, admission, transport, durability or reconnect paths:

- `apps/game-server/src/**`, except `ability/`, `ai/`, `interaction/`, `combat.rs` and Content authoring;
- the native entry room Content, its WorldProject/v2 model and its activation;
- `apps/game-server/migrations/**`;
- `tools/qualification/**`;
- `vendor/**`;
- Cargo/toolchain inputs;
- `merge-gate.yml`.

They also run when the changed-file evidence is incomplete or malformed, or when the trusted-base classifier emits no selection. Only an explicit `false` skips them. Only the Merge Queue runs these physical qualifications, and its `game-gate` requires both to succeed whenever selected.

Reduced lanes are derived from the exact candidate tree. Cargo metadata owns package/reverse dependency closure; literal file/directory references from exact-candidate Cargo package sources attach non-Cargo files to their real consumers. Canonical product-CI workflows conservatively attach literal file and directory consumers too. Only explicitly audited routing-only directory predicates registered in the protected classifier may be omitted from consumer attachment; every unregistered or ambiguous workflow directory predicate/reference remains fail-closed/FULL. There is no historical document-consumer SHA or source-drift snapshot to refresh.

`docs/agents/evidence/**` is not blanket runtime material. Evidence that current Rust source/tests actually consume through paths such as `include_str!` / `include_bytes!` inherits the consuming package lane; unconsumed evidence remains auxiliary.

Cross product/auxiliary renames, symlinks/submodules, special modes, malformed consumer evidence and incomplete file enumeration fail closed. The classifier itself and canonical routing controls always self-qualify through FULL.

## Merge Queue gate

The canonical Merge Queue workflow is `.github/workflows/merge-group-gate.yml`; it validates GitHub's exact synthetic `merge_group` head.

| Exact queue classification | Selected jobs |
|---|---|
| Complete valid diff containing only Markdown under `docs/architecture/**` | candidate/governance, dependency review and CodeQL; heavy Rust/PostgreSQL/Windows/supply-chain may be unselected |
| Unconsumed auxiliary inputs, proven by protected-base exact-consumer routing on the synthetic candidate | always-required gates; heavy Rust/PostgreSQL/Windows/supply-chain may be unselected |
| Exact server-only consumer closure (`server` or `durability`), proven by that same classifier | Linux workspace, PostgreSQL 17.6 and supply chain plus always-required gates; Windows may be unselected |
| Everything else, including client/shared/simulation, canonical routing controls, mixed, special-mode, malformed or incomplete evidence | FULL Linux workspace, PostgreSQL 17.6, Windows client/input/SIM and supply chain plus always-required gates |

The physical server qualifications (`Merge Queue / Node boot against the real Platform`, `Merge Queue / Server Seam over TCP+TLS`) are selected separately on the exact queue diff. The selection uses the protected-base `server_qualification_required` classifier with the same path rules as the pull-request gate, so a batched candidate is re-qualified whenever any of its changes can reach the shipped server. It fails closed: anything but a proven-unrelated diff selects them.

Selected jobs must succeed. Only genuinely unselected jobs may be `skipped`; missing, failed, cancelled or selected-skipped evidence cannot qualify the candidate.

A PR-head PASS does not prove integration. Require the real `merge_group` aggregate `game-gate` SUCCESS and protected-main readback.

## Focused validation

| Change | Focused implementation checks | Frozen-head PR evidence |
|---|---|---|
| Agent governance/prompt/task docs | `python tools/agents/validate_governance.py` plus affected agent tests | governance/policy/security/routing + aggregate `game-gate`; runtime lanes according to trusted classifier |
| Repository/GitHub policy | `python tools/repository/validate_repository_policy.py` plus affected routing tests | FULL when policy/build/control paths select it |
| Architecture/contracts | governance plus applicable link/JSON/schema/semantic checks | always-required PR gates; runtime E2E may be `NOT_APPLICABLE` with an accurate reason |
| Rust/server code | affected package/tests + strict lint while editing | trusted server/full lanes above |
| Client/shared/simulation | affected package/platform tests | FULL Linux/PostgreSQL/Windows/SIM |
| Canonical routing/build/dependency inputs | repository-policy tests and workflow review | FULL |
| Standalone workflow / offline non-Cargo tool | its focused/dedicated checks | always-required PR gates; product lanes only when exact-candidate Cargo package or canonical product-CI consumers require them |

## Rust and platform evidence

Canonical workspace/toolchain inputs are root `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `deny.toml` and `workspace-boundaries.toml`.

Current selected Linux evidence includes:

- locked workspace metadata and formatting/policy checks;
- workspace build, strict Clippy and tests;
- `oteryn-game-server --test durability_postgres` against pinned PostgreSQL 17.6 when the exact target is allocated/present;
- synthetic client harness and server bootstrap smoke where selected;
- dependency closure and `cargo-deny` supply-chain checks.

Current selected Windows evidence includes:

- exact release `oteryn-client` build and strict client Clippy;
- smoke against that exact release artifact plus synthetic harness;
- unsigned installer build, install/reinstall, concurrency and uninstall checks using the existing lifecycle script and hash-verified Inno Setup; release identity is derived from the exact MQ SHA before the client build;
- `oteryn-input-platform` package tests on `x86_64-pc-windows-msvc`;
- `oteryn-simulation-determinism` golden tests on the same target.

Windows smoke exits before renderer construction; it is shell/package evidence, not renderer or gameplay E2E. Affected renderer/UI work still needs its named native fixtures/platform tests.

## Protected-main post-merge

`.github/workflows/rust.yml` checks policy/metadata and produces a shared Linux
compilation cache on protected `main`. Complete auxiliary-only diffs may skip
cache warming; server/client/control or uncertain inputs select it. Manual
dispatch on protected main also selects cache warming. No cache job or save is
allowed on an unprotected ref.

The selected cache producer builds and runs strict Clippy, compiles workspace
tests with `cargo test --no-run`, then runs bounded synthetic/native smoke before
trimming and saving the cache. It does not execute the full workspace,
PostgreSQL or Windows test suites after merge. Supply-chain qualification remains
in PR/MQ; dedicated workflows retain their accepted checks.

Compilation can still be substantial. Cache contents are a build optimization;
PR/MQ exact-candidate qualification and real merge-group `game-gate` remain the
integration evidence.

## Required additions as owning layers appear

Do not invent tests for nonexistent product layers. Add bounded evidence when an owning implementation introduces the seam, including:

- parser property/fuzz tests for untrusted protocol/content input;
- canonical/golden protocol fixtures and malformed/adversarial corpora;
- persistence migration/concurrency/rollback/restart tests;
- shared failure-scenario tests for clock/dependency loss/stale generation/overload;
- multichannel isolation/recovery/soak scenarios;
- targeted sanitizer/Miri-equivalent proof where it adds evidence beyond `unsafe_code = "forbid"`.

## QA-E2E-01 tiers

| Tier | Purpose | Does not prove |
|---|---|---|
| Tier 1 — headless system E2E | deterministic Platform → Gateway → protocol → server → PostgreSQL using production transport/schema | native UI/rendering/final packaging |
| Tier 2 — instrumented native-client E2E | real Rust client networking/input/reconciliation/UI/rendering with bounded test observation | exact production-default binary behavior |
| Tier 3 — production-binary smoke E2E | exact release client/server artifacts without in-process test adapter | broad fault/concurrency/exhaustive gameplay |

Use the smallest sufficient tier set. A supported native-client journey cannot be marked `PROVEN` from Tier 1 alone. A native window rendering synthetic fixtures is component qualification unless the full Tier-2 journey contract is actually exercised.

## Mandatory E2E evidence

Every counted attempt records exact artifact/revision identities, protocol/ruleset/content/world/migration revisions, scenario/tier/topology/seed/clock/fault profile, ordered phases and first divergence, required client/server/Platform/persistence/audit evidence, cleanup result and retained artifact hashes.

Canonical phases are environment, identity, world discovery, Gateway, Game Session, transport, admission, character lease, world entry, gameplay, persistence, audit/outbox, client presentation and cleanup. A non-applicable phase needs a scenario-defined reason.

## High-risk acceptance

| Area | Minimum additional evidence |
|---|---|
| Protocol/framing | limits/negative sequencing/replay/downgrade/golden fixtures + Tier 1 and native-client journey when supported |
| Character lease/relog | double-login, stale session/writer, crash/recovery, cross-channel misuse, exact final offline state |
| Inventory/loot/market | idempotency, concurrency, rollback, conservation/no-duplication and audit/outbox reconciliation |
| Multichannel runtime | two-channel isolation, shared-world service behavior, channel failure and revision compatibility |
| Persistence/migrations | isolated migration, compatibility/rollback, concurrent mutation, dependency-loss and restart E2E |
| Client renderer/UI | named platform/hardware/scene, Tier 2 interaction and relevant recovery, Tier 3 release smoke |
| Assets/updater | provenance, signatures/hashes, traversal/decompression limits, rollback and release smoke |
| Platform/admission | exact producer contract/service revision, expiry/replay/revocation, Gateway routing and cross-world/channel misuse |

## Stability and documentation-only rule

Repeated-run result is `PASS | UNSTABLE | FAIL | BLOCKED | NOT_EVALUATED` for one fixed comparison cell; repairing the environment creates a new population and does not rewrite historical evidence.

A documentation-only change does not by itself prove runtime behavior. It may record runtime E2E as `NOT_APPLICABLE` only when runtime behavior is unchanged and the governing scenario accepts that classification. Always-required governance/security/routing gates still apply, and current machine routing decides whether heavy product lanes are selected.
