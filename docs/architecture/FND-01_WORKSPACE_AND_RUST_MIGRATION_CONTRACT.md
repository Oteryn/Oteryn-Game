# FND-01: Workspace, Dependency and Existing-Rust Migration Contract

- Status: Candidate for owner acceptance
- Date: 2026-08-06
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Destination baseline: `blakinio/Oteryn-v2@cbc2150024d98bbdbfa9b1c17bc9b9df16bcd9f2`
- Source inventory revision: `blakinio/otclient@c923ad8a1dff17b4933a6110931b0823cec2c590`
- Exact source evidence: `docs/architecture/FND-01_RUST_SOURCE_INVENTORY.md`
- Applies to: `FND-01`, `VSL-02` and the one atomic destination migration/workspace pull request

## 1. Purpose and authority

This candidate defines the Rust workspace that may be created by the controlled native-client migration. It classifies every current source member, proposes the minimum consumer-backed destination graph and fixes dependency, build, release-role, target, CI and machine-enforcement policy.

It does not move code, create Cargo members, implement `protocol-oteryn`, freeze public identifiers, authorize gameplay transport/admission or create an authoritative server runtime. Physical migration remains owned by `VSL-02`. The graph remains a candidate until explicit owner acceptance.

## 2. Binding architecture

- ADR-0001: one native Rust client, one authoritative Rust server and one project-owned `protocol-oteryn`.
- ADR-0002: Rust ownership moves to `blakinio/Oteryn-v2` after `FND-01` and `VSL-02` through one atomic destination migration/workspace PR.
- ADR-0005: production world/content/asset formats remain separately owned; synthetic fixture packs are not production formats.
- ADR-0007: later E2E uses one shared three-tier platform with exact evidence and no hidden retry-until-green.
- ADR-0008: `protocol-canary` is `REFERENCE_ONLY` and cannot enter destination Cargo membership, binaries, negotiation, fallback, translation or packaging.
- ADR-0011: the migrated client may launch in `pre-native-protocol`, but gameplay entry fails before gameplay credential consumption, Game Session binding or endpoint connection.
- `FND-ID-01` owns public identifier meaning/representation.
- `FND-02` owns native framing, schema, gameplay transport, limits, TLS, sequencing and compatibility.
- `FND-04` owns Game Session credentials, routing/admission and character leases.

## 3. Exact source inventory

The exact 26-member source graph, direct dependencies, direct consumers, third-party dependencies, root policy and non-member paths are recorded at:

```text
docs/architecture/FND-01_RUST_SOURCE_INVENTORY.md
```

That evidence proves the source cannot migrate unchanged because:

- application and technical-login tests directly depend on Canary;
- `game-session` contains `CanaryCurrent` entry policy;
- account/directory/entry concerns are fragmented across tightly coupled crates;
- client domain/simulation are projection and render-snapshot contracts, not server authority;
- asset/resource packages implement synthetic fixtures;
- source protocol/transport ceilings cannot become native limits by inertia;
- several tested members have no product consumer.

Any source change after the inventory SHA invalidates affected rows until `VSL-02` reconciles the selected cutover SHA.

## 4. Proposed initial destination workspace

```text
apps/client

crates/foundation
crates/diagnostics
crates/client-runtime
crates/platform-contracts
crates/platform-client
crates/identity
crates/client-domain
crates/client-simulation
crates/input-actions
crates/input-platform
crates/renderer
crates/synthetic-assets
crates/test-support

tests/security/auth
tests/pre-native-client

tools/architecture-check
tools/synthetic-asset-compiler
tools/synthetic-client-harness
```

Proposed package names:

```text
oteryn-client
oteryn-foundation
oteryn-diagnostics
oteryn-client-runtime
oteryn-platform-contracts
oteryn-platform-client
oteryn-identity
oteryn-client-domain
oteryn-client-simulation
oteryn-input-actions
oteryn-input-platform
oteryn-renderer
oteryn-synthetic-assets
oteryn-test-support
oteryn-identity-security-tests
oteryn-pre-native-client-tests
oteryn-architecture-check
oteryn-synthetic-asset-compiler
oteryn-synthetic-client-harness
```

The initial workspace deliberately excludes:

```text
services/game-server
apps/oteryn-studio
crates/protocol-canary
crates/protocol-core
crates/protocol-oteryn
crates/transport
crates/game-session
persistence, world, content, scripting and server-runtime crates
```

Later gates may add only members with an immediate accepted consumer and owned contract.

## 5. Production client and synthetic harness separation

`oteryn-client` is the production-shaped pre-native shell. It must not declare optional normal dependencies on client simulation or synthetic fixture packages merely because a feature disables them by default.

Deterministic foundation evidence uses a separate non-release executable:

```text
tools/synthetic-client-harness
```

The harness may consume client domain/simulation, input, renderer, synthetic assets and test support. It cannot access live Identity/Game Gateway/gameplay endpoints, consume credentials, enter release packaging or claim protocol/server/native-E2E compatibility.

## 6. Proposed members and immediate acceptance

| Destination member | Immediate consumer/purpose | Observable acceptance |
|---|---|---|
| `apps/client` | production-shaped pre-native shell | Windows build, deterministic launch/shutdown and explicit gameplay-unavailable state |
| `foundation` | lower client layers | cancellation, monotonic time and explicitly local generation tests |
| `diagnostics` | runtime, Platform client and tests | bounded/redacted diagnostics |
| `client-runtime` | app and pre-native tests | application-owned Tokio runtime, deterministic lifecycle/shutdown, no gameplay adapter |
| `platform-contracts` | Platform client, Identity, runtime and auth tests | bounded display/account-directory values with provisional client references and no gameplay routes |
| `platform-client` | Identity/runtime and auth tests | asynchronous cancellation-safe account/directory HTTP boundary only |
| `identity` | runtime and auth tests | PKCE, browser/loopback callback, cancellation, stale-operation and redaction tests |
| `client-domain` | client simulation and synthetic harness | typed client command/event and stale-session tests |
| `client-simulation` | synthetic harness | deterministic client event application and immutable snapshots |
| `input-actions` | adapter, app and harness | framework-neutral semantic input tests |
| `input-platform` | app and harness | Windows/winit target-isolation evidence |
| `renderer` | app and harness | surface/device/resource lifecycle and bounded recovery |
| `synthetic-assets` | fixture compiler and harness | deterministic project-owned fixture packs/decode; no production claim |
| `test-support` | tests and harness | deterministic fixtures, clocks and safe diagnostics |
| `tests/security/auth` | Identity/Platform boundary | exact negative and lifecycle security cases |
| `tests/pre-native-client` | ADR-0011 | no protocol adapter, gameplay credential, Game Session binding, endpoint connection or false success |
| `architecture-check` | complete workspace | locked metadata checked against root machine policy |
| `synthetic-asset-compiler` | fixture pipeline | deterministic compilation, path safety and provenance |
| `synthetic-client-harness` | non-release deterministic evidence | bounded domain/simulation/input/renderer scenarios without live services |

## 7. Per-member source dispositions

Closed vocabulary: `MIGRATE_AS_IS`, `MIGRATE_AND_RENAME`, `MERGE`, `SPLIT`, `REWRITE`, `REFERENCE_ONLY`, `DROP`.

| Source member | Disposition | Destination/rule |
|---|---|---|
| `apps/client` | `REWRITE` | retain useful Windows shell/render composition; remove Canary/W7 entry wiring; expose pre-native state |
| `crates/account-session` | `MERGE` | client-local account context moves into `platform-contracts` under a non-canonical name |
| `crates/app-runtime` | `MIGRATE_AND_RENAME` | `client-runtime`; retain app-owned Tokio orchestration/shutdown, remove W7 policy |
| `crates/asset-decode` | `MERGE` | `synthetic-assets` |
| `crates/asset-types` | `MERGE` | `synthetic-assets` |
| `crates/asset-runtime` | `MERGE` | `synthetic-assets` |
| `crates/diagnostics` | `MIGRATE_AS_IS` | `diagnostics` |
| `crates/foundation` | `REWRITE` | retain clocks/cancellation/technical generations; remove or rename future-authoritative `SessionGeneration` semantics |
| `crates/game-domain` | `MIGRATE_AND_RENAME` | `client-domain`; explicitly client projection only |
| `crates/simulation-core` | `MIGRATE_AND_RENAME` | `client-simulation`; explicitly non-authoritative client state |
| `crates/input-actions` | `MIGRATE_AS_IS` | `input-actions` |
| `crates/input-platform` | `MIGRATE_AS_IS` | `input-platform`; app/harness become named consumers |
| `crates/game-session` | `SPLIT` | safe non-secret selection/lifecycle logic into `platform-contracts`/`client-runtime`; all credentials, Canary profile, routing and admission deferred |
| `crates/identity` | `REWRITE` | retain PKCE/browser/loopback security; remove gameplay-session/protocol coupling and unmanaged blocking I/O |
| `crates/platform` | `SPLIT` | non-gameplay validated values into `platform-contracts`; async account/directory I/O into `platform-client`; source `ureq`, ticket issuance and gameplay routing are not migrated as production behavior |
| `crates/protocol-canary` | `REFERENCE_ONLY` | source history only |
| `crates/protocol-core` | `REFERENCE_ONLY` | reconsider under `FND-02` |
| `crates/renderer` | `MIGRATE_AS_IS` | `renderer`; root metadata/target policy rewritten |
| `crates/renderer-resource` | `SPLIT` | backend-neutral generation/cache logic into renderer; synthetic adapters/tests remain fixture/harness-only |
| `crates/test-support` | `MIGRATE_AS_IS` | `test-support` |
| `crates/transport` | `REFERENCE_ONLY` | reconsider under `FND-02`; no pre-native production consumer |
| `crates/world-directory` | `MERGE` | display/selection values into `platform-contracts`; source host/port routes and canonical-looking IDs are not retained as public contracts |
| `tests/integration/technical-login` | `REFERENCE_ONLY` | rewrite only non-Canary scenarios into auth/pre-native tests |
| `tests/security/auth` | `REWRITE` | consolidated Platform/Identity boundaries |
| `tools/architecture-check` | `REWRITE` | root policy data replaces hard-coded aspirational matrix |
| `tools/asset-compiler` | `MIGRATE_AND_RENAME` | `synthetic-asset-compiler` |

Every source member appears exactly once. Reference-only code is not copied into destination Cargo membership, product paths or release packaging. Selective later reuse requires the owning gate and explicit provenance.

## 8. Non-member dispositions

| Source subsystem | Disposition | Destination rule |
|---|---|---|
| `assets/test-fixtures` | `MIGRATE_AND_RENAME` | `tests/fixtures/synthetic-assets`, with project-owned/synthetic provenance |
| `contracts/canary` | `REFERENCE_ONLY` | source history only |
| source architecture/agent docs | `REFERENCE_ONLY` | provenance links only; destination policy is authoritative |
| source root manifest | `REWRITE` | one destination root manifest using accepted FND-01 membership |
| source lockfile | `REWRITE` | one destination lockfile plus direct/transitive dependency delta; source blob retained as evidence |
| source toolchain | `REWRITE` | Rust 1.94.0 with accepted Windows/Linux targets |
| source rustfmt | `REWRITE` | edition 2024, width 100 and repository LF normalization |
| source cargo-deny | `REWRITE` | Windows-default, Linux-shared and supplemental all-feature graphs |
| source Rust CI | `REWRITE` | product/target matrix and release-closure negatives |
| source protocol fixtures | `REFERENCE_ONLY` | no Canary fixture enters destination contracts |

## 9. Provisional identifier safeguards

Before `FND-ID-01`, retained source types must not use names that imply accepted global/cross-boundary identity.

Required semantic renaming:

```text
AccountSessionId         -> ClientAccountContextId
DirectoryRevision        -> ClientDirectoryGeneration
GameplayChannelId        -> DirectoryChannelRef
source WorldId           -> DirectoryWorldRef
source CharacterId       -> DirectoryCharacterRef
source SessionGeneration -> ClientSessionEpoch or another explicitly local name
```

Each retained type states that it is client-local or producer-opaque, grants no authority, carries no durable/global uniqueness claim and does not commit wire/database representation. Client-domain entity/item handles remain session-local projection handles. Server/shared public crates cannot depend on them before `FND-ID-01`.

## 10. Platform and Identity boundary

### 10.1 Values exposed to the client

`platform-contracts` may expose bounded account context, world/character display metadata, availability and selection references. It must not expose:

- gameplay host or port;
- gameplay endpoint URI;
- protocol profile/adapter selection;
- Game Session credential or bearer material;
- admission/routing token;
- node/container/orchestrator identity.

If current producer DTOs contain route fields, `platform-client` may validate and discard them inside the I/O adapter for source compatibility evidence; they cannot enter `platform-contracts`, application state, logs or telemetry. `FND-04` later defines authoritative channel binding and routing.

### 10.2 I/O model

The source blocking `ureq` adapter is `REFERENCE_ONLY` and is not a permitted production migration outcome.

`platform-client` exposes asynchronous cancellation-safe operations scheduled on the application-owned Tokio runtime. It must not create a global runtime, unmanaged thread or block the window/event/frame thread. Every request has explicit body/header/time limits, deadline, cancellation, deterministic shutdown and secret redaction.

`VSL-02` selects and records the exact audited async HTTP/TLS dependency, feature set and certificate-root policy. That library choice cannot weaken this boundary.

Identity callback I/O follows the same cancellation/deadline/ownership rules. Identity may retain PKCE, system-browser and loopback security behavior but cannot depend on gameplay protocol, transport, Game Session credentials or admission state.

## 11. Root Cargo and dependency policy

The destination root owns exactly one:

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
rustfmt.toml
deny.toml
workspace-boundaries.toml
```

Root policy:

```text
edition = 2024
rust-version = 1.94
resolver = 3
license = MIT
repository = https://github.com/blakinio/Oteryn-v2
publish = false
```

- Members inherit package metadata, workspace dependencies and lints.
- No member owns another lockfile.
- Direct third-party dependencies are declared once and pinned to reviewed exact versions during migration.
- Paths remain inside the workspace.
- Git dependencies/unknown registries/wildcards are forbidden by default.
- Multiple transitive versions require a narrow owner/reason/removal exception.
- `cargo metadata --locked` is mandatory.
- The destination records a dependency delta against source lockfile blob `2143408c12c50132883890f0821278320a331fde`.
- Rustfmt uses edition 2024, width 100 and LF-normalized repository text.

Minimum lints:

```text
unsafe_code = forbid
unused_must_use = deny
clippy::all = deny
clippy::dbg_macro = deny
clippy::expect_used = deny
clippy::panic = deny
clippy::todo = deny
clippy::unimplemented = deny
clippy::unwrap_used = deny
```

Local relaxation requires a path-scoped reviewed reason.

## 12. Closed workspace dependency policy

Categories:

```text
app
foundation
diagnostics
client-runtime
platform-contracts
platform-client
identity
client-domain
client-simulation
input
input-platform
renderer
test-fixture
test-support
test
tool
```

### 12.1 Complete normal category allowlist

| Source | Allowed normal target categories |
|---|---|
| `app` | `foundation`, `diagnostics`, `client-runtime`, `input-platform`, `renderer` |
| `foundation` | none |
| `diagnostics` | `foundation` |
| `client-runtime` | `foundation`, `diagnostics`, `identity`, `platform-contracts`, `platform-client` |
| `platform-contracts` | `foundation` |
| `platform-client` | `foundation`, `diagnostics`, `platform-contracts` |
| `identity` | `foundation`, `diagnostics`, `platform-contracts`, `platform-client` |
| `client-domain` | `foundation` |
| `client-simulation` | `foundation`, `client-domain` |
| `input` | none |
| `input-platform` | `input` |
| `renderer` | `foundation`, `diagnostics` |
| `test-fixture` | `foundation` |
| `test-support` | `foundation`, `diagnostics` |
| `test` | product categories, `test-fixture`, `test-support` as narrowed below |
| `tool` | foundation/client-test categories as narrowed below |

### 12.2 Exact package narrowing

```text
oteryn-client
  -> foundation, diagnostics, client-runtime, input-platform, renderer

oteryn-identity-security-tests
  -> foundation, diagnostics, platform-contracts, platform-client, identity, test-support

oteryn-pre-native-client-tests
  -> foundation, diagnostics, client-runtime, platform-contracts, platform-client, identity, test-support

oteryn-architecture-check
  -> no workspace package

oteryn-synthetic-asset-compiler
  -> synthetic-assets

oteryn-synthetic-client-harness
  -> foundation, client-domain, client-simulation, input-actions, input-platform,
     renderer, synthetic-assets, test-support
```

No workspace build dependency is accepted initially.

Product crates may depend on `test-support` and `synthetic-assets` only as dev dependencies. Tests/tools may normally depend only on their exact package allowlists. Dev edges participate in cycle detection.

### 12.3 Forbidden edges and names

Permanent:

- any local package/feature/dependency named `protocol-canary` or unrestricted Canary compatibility;
- external path dependencies and workspace cycles;
- `foundation` to any workspace package;
- client domain/simulation to Tokio, network, SQL, Platform I/O, renderer implementation, winit or wgpu;
- renderer to Identity, Platform, protocol, transport or mutable simulation implementation;
- input actions to winit/OS APIs;
- production normal/build dependency on tests, tools, test support or fixtures.

Before later gates:

- production app to gameplay protocol, transport or Game Session/admission packages;
- server/shared public code to provisional client references;
- production client/renderer to synthetic asset packages.

## 13. Machine-readable enforcement

`workspace-boundaries.toml` records every member, path, category, exact package edge allowlist, release role, source restriction and release-closure exclusion.

Minimum shape:

```toml
schema_version = 1
source_inventory_revision = "c923ad8a1dff17b4933a6110931b0823cec2c590"

[workspace]
required_prefix = "oteryn-"
forbid_external_path_dependencies = true
forbid_git_dependencies = true
forbid_cycles = true
forbidden_package_names = ["oteryn-protocol-canary"]
forbidden_feature_names = ["protocol-canary", "canary"]
```

`architecture-check` reads this policy and `cargo metadata --locked`; it must not duplicate a future aspirational crate catalogue in code.

Validation fails for unregistered/duplicate members, undeclared edges, cycles, external/Git/unknown sources, forbidden names/features, test/tool/fixture leakage, protocol/transport/Game Session edges in the pre-native app, synthetic leakage into production or metadata/toolchain/lockfile drift.

## 14. Features, release roles and targets

Features are additive capabilities, not product identities, environments, protocol selectors or security bypasses. There is no Canary feature and no empty native-protocol feature/crate. The synthetic harness is a separate package, not a production-client feature.

Initial targets:

```text
x86_64-pc-windows-msvc
x86_64-unknown-linux-gnu
```

Release/validation roles:

1. `client-windows-pre-native`
   - Windows default client graph;
   - no gameplay protocol/transport/Game Session, client simulation, synthetic fixture or test/tool dependency;
   - explicit gameplay-unavailable state.

2. `synthetic-client-harness`
   - separate Windows non-release executable;
   - deterministic client foundation evidence;
   - no live service/credential capability.

3. `shared-linux-validation`
   - portable shared/test/tool selection on Linux;
   - proves Windows dependency isolation;
   - no Linux desktop-client or GameNode claim.

`--all-features` is supplemental evidence only.

## 15. CI matrix

Required lanes:

- **Workspace policy / Ubuntu:** pinned toolchain, locked metadata, machine-boundary validation, package/source/cycle/root-lock checks, format and governance.
- **Shared Linux:** compile, Clippy and tests for the named portable selection on `x86_64-unknown-linux-gnu`.
- **Windows pre-native client:** default build/tests, deterministic launch/shutdown, renderer lifecycle, ADR-0011 negative entry evidence and production dependency-closure audit.
- **Windows synthetic harness:** separate harness build/run with deterministic fixture scenarios and no live network capability.
- **Security/supply chain:** Windows-default and Linux-shared advisory/license/source checks, supplemental all-features, auth/redaction tests, Dependency Review and CodeQL.

The production lane fails if `oteryn-client` has a normal/build dependency on:

```text
protocol-canary
protocol-core
protocol-oteryn
transport
game-session
client-domain
client-simulation
synthetic-assets
test-support
any tests or tools package
```

Later gates may change only entries they own. Canary remains permanently forbidden unless a new owner-approved ADR reverses ADR-0008.

## 16. Protocol, transport, renderer and fixtures

The initial workspace contains no gameplay protocol or transport member. Bounded reader/writer patterns, Tokio task ownership, queue/cancellation/shutdown behavior and malformed/saturation tests remain source evidence for `FND-02`. Any reuse must be revalidated against native framing, limits, endianness, TLS, sequencing, replay and error contracts.

Synthetic assets use only project-owned fixtures, remain absent from production closures, cannot claim ADR-0005 formats/identifiers and must be removable without changing product contracts.

Renderer production code retains surface/device/backend-neutral resource lifecycle. Source resource code coupled to synthetic assets is split so the adapter remains test/harness-only.

## 17. Crate evolution criteria

A new crate requires a distinct trust/security boundary, target/release role, heavy-dependency isolation, stable public contract with at least two immediate consumers, or executable/tool/platform/FFI ownership.

Split when value contracts mix with I/O, target-specific code leaks downward, production depends on fixtures/tests or client projection is confused with server authority.

Merge when a crate owns one trivial type, has one consumer without a meaningful boundary, mirrors an aspirational layer or always changes with another crate.

No empty crate, future placeholder or one-type convenience crate is accepted.

## 18. Canonical supporting locations

```text
docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json
docs/contracts/RESOURCE_LIMITS_REGISTRY.json
docs/contracts/FOUNDATION_ERROR_VOCABULARY.md
docs/contracts/FOUNDATION_FAILURE_SCENARIOS.md
workspace-boundaries.toml
```

`VSL-02` produces:

```text
docs/migration/rust-client-provenance.json
docs/migration/rust-client-path-map.json
docs/migration/rust-dependency-delta.json
```

These record source SHA/range, paths, transformations, exclusions, licenses, dependency changes and final destination merge without false cross-repository ancestry claims.

## 19. VSL-02 handoff

After owner acceptance and merge, `VSL-02` must:

1. compare source `main` with the inventory revision;
2. pin the cutover SHA and reconcile every changed path;
3. classify open PRs/tasks and establish source freeze;
4. produce provenance/path/dependency manifests;
5. plan one atomic destination PR implementing every disposition;
6. define rollback before source non-canonical marking;
7. validate exact Windows/Linux/release-role matrices;
8. prove pre-native closure and fail-closed entry;
9. merge destination first;
10. then create the source-marker PR.

No import-only destination PR or later workspace-cleanup destination PR is allowed.

## 20. Rejected alternatives

- unchanged source copy: imports Canary, fragmented session layers and source-shaped protocol limits;
- synthetic optional dependencies in production app: enlarges and risks the production graph;
- deleting all tested foundations: loses useful code with real test/tool consumers;
- empty server/native-protocol crates: no accepted consumer/contract;
- retaining source protocol/transport as production generic layers: limits and consumer remain unresolved;
- treating client simulation as server authority or synthetic packs as production assets: contradicts ownership.

## 21. Acceptance conditions

Ready for owner acceptance only when:

- all 26 source members/non-members are inventoried and disposed exactly once;
- the proposed graph is acyclic and every member has an immediate consumer;
- production and synthetic executables have separate closures;
- no provisional identifier resembles an accepted FND-ID-01 identity;
- gameplay routes/credentials cannot enter pre-native contracts;
- all Platform/Identity I/O is async, cancellation-safe and runtime-owned;
- machine policy has complete category and package allowlists;
- later gate ownership remains intact;
- independent audit finds zero open material findings;
- owner explicitly accepts the graph and dispositions.
