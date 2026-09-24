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

Always-required evidence includes:

- exact PR/base/head identity and trusted-base risk classification;
- exact-head routing-contract validation against candidate Cargo/tree state;
- PR metadata, agent governance and repository policy;
- Dependency Review and CodeQL;
- aggregate `Merge gate / validate` and final `game-gate`.

Current runtime selection:

| Proven PR surface | Runtime evidence |
|---|---|
| Unconsumed auxiliary inputs: neutral docs, agent governance, standalone workflows and non-Cargo offline tooling | no Rust product lanes; always-required governance/security/routing and any dedicated workflow remain |
| Auxiliary file referenced by exact-candidate Cargo package source/build input | route as the consuming package, then apply normal reverse Cargo closure |
| Audited Atlas fullworld producer/self-test surface | dedicated Atlas producer/consumer gate; add Rust lanes only if another path selects them |
| Server-only, including server-consumed auxiliary inputs | Linux workspace + PostgreSQL 17.6 + policy/supply chain |
| Client/shared/simulation, including their consumed auxiliary inputs | FULL: Linux/PostgreSQL + Windows client/input/SIM + policy/supply chain |
| Canonical routing controls (`merge-gate.yml`, `merge-group-gate.yml`, `rust.yml`), `.github/actions/**`, `tools/repository/**`, `docs/migration/**`, Cargo/toolchain/build inputs, unknown/mixed/incomplete evidence | FULL |

Reduced lanes are derived from the exact candidate tree. Cargo metadata owns package/reverse dependency closure; literal file/directory references from exact-candidate Cargo package sources attach non-Cargo files to their real consumers. Canonical product-CI workflows attach auxiliary inputs through literal file references; a bare directory literal is only routing/glob evidence and does not by itself prove that every file below that directory is a build consumer. There is no historical document-consumer SHA or source-drift snapshot to refresh.

`docs/agents/evidence/**` is not blanket runtime material. Evidence that current Rust source/tests actually consume through paths such as `include_str!` / `include_bytes!` inherits the consuming package lane; unconsumed evidence remains auxiliary.

Cross product/auxiliary renames, symlinks/submodules, special modes, malformed consumer evidence and incomplete file enumeration fail closed. The classifier itself and canonical routing controls always self-qualify through FULL.

## Merge Queue gate

The canonical Merge Queue workflow is `.github/workflows/merge-group-gate.yml`; it validates GitHub's exact synthetic `merge_group` head.

| Exact queue classification | Selected jobs |
|---|---|
| Complete valid diff containing only Markdown under `docs/architecture/**` | candidate/governance, dependency review and CodeQL; heavy Rust/PostgreSQL/Windows/supply-chain may be unselected |
| Everything else, including agent-governance/docs, mixed, special-mode, malformed or incomplete evidence | FULL Linux workspace, PostgreSQL 17.6, Windows client/input/SIM and supply chain plus always-required gates |

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
- `oteryn-input-platform` package tests on `x86_64-pc-windows-msvc`;
- `oteryn-simulation-determinism` golden tests on the same target.

Windows smoke exits before renderer construction; it is shell/package evidence, not renderer or gameplay E2E. Affected renderer/UI work still needs its named native fixtures/platform tests.

## Protected-main post-merge

`.github/workflows/rust.yml` runs on protected-main pushes and manual dispatch. Policy and supply chain remain independent; runtime omission is allowed only after successful protected classification.

| Protected-main input | Standalone runtime lanes |
|---|---|
| Proven unconsumed auxiliary inputs | runtime lanes not applicable; policy/supply chain remain |
| Proven server-only or server-consumed auxiliary input | Linux + PostgreSQL + policy/supply chain |
| Client/shared/simulation or their consumed auxiliary inputs | FULL |
| Cargo/toolchain/build/canonical-routing/unknown/incomplete | FULL |
| Manual dispatch or classifier/evidence failure | FULL |

Post-merge routing does not replace PR or Merge Queue qualification and cannot retroactively prove a skipped pre-merge gate.

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
