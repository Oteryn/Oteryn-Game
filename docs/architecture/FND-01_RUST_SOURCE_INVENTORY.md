# FND-01 Rust Source Workspace Inventory

- Status: Evidence for `FND-01`
- Inventory date: 2026-08-06
- Source repository: `blakinio/otclient`
- Source revision: `c923ad8a1dff17b4933a6110931b0823cec2c590`
- Source path: `oteryn-client/`
- Root manifest blob: `037013e8e4a762a65f0f2a30f7761ee14725a3fc`
- Root lockfile blob: `2143408c12c50132883890f0821278320a331fde`
- Crates tree: `c2a5426f764bab0f3a89de3e5a03e88a7f111c20`
- Governing contract: `docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md`

## 1. Evidence rules

This document records the exact source state inspected by `FND-01`. It is evidence, not destination architecture by itself.

- `PROVEN` statements are derived from files at the exact source revision above.
- Source names and dependency edges do not become accepted destination names or edges automatically.
- Any later source change invalidates only the affected inventory rows until `VSL-02` reconciles the selected cutover revision.
- Reverse consumers list direct workspace consumers only; CI, internal unit tests and external repositories are recorded separately.
- Third-party dependencies list direct manifest declarations, not the complete transitive lock graph.

## 2. Root workspace policy

The source workspace contains 26 members and fixes:

```text
edition = 2024
rust-version = 1.94
resolver = 3
license = MIT
repository = https://github.com/blakinio/otclient
one root Cargo.lock
unsafe_code = forbid
unused_must_use = deny
strict Clippy deny policy
```

The pinned toolchain is Rust `1.94.0` with `clippy`, `rustfmt` and target `x86_64-pc-windows-msvc`.

The source CI has:

- one Windows workspace lane running locked metadata, format, Clippy, tests and architecture validation;
- one Ubuntu cargo-deny lane using `--all-features` against the Windows target graph;
- no Linux shared/server target lane;
- no product-default release-closure negative check.

## 3. Exact member and dependency inventory

Dependency notation:

- `N:` normal dependency;
- `W:` Windows-target normal dependency;
- `D:` development dependency;
- `3P:` direct third-party dependency.

| # | Source member | Declared role / public responsibility | Direct workspace and third-party dependencies | Direct workspace consumers |
|---:|---|---|---|---|
| 1 | `apps/client` | Windows renderer and W7 technical-login application shell | `N: diagnostics, foundation`; `W: account-session, app-runtime, game-session, identity, platform, protocol-canary, renderer, transport, world-directory, winit`; `D: test-support` | executable root; no workspace consumer |
| 2 | `crates/account-session` | one client-local non-secret `AccountSessionId` correlation type | none | app, app-runtime, game-session, identity, platform, world-directory; protocol-canary dev; both integration-test packages |
| 3 | `crates/app-runtime` | Tokio orchestration and deterministic shutdown for W7 technical login | `N: account-session, foundation, game-session, world-directory, tokio` | app; technical-login tests |
| 4 | `crates/asset-decode` | bounded synthetic-v1 RGBA8 normalization | `N: asset-runtime, asset-types`; `D: asset-compiler` | renderer-resource |
| 5 | `crates/asset-types` | synthetic-v1 asset schema and deterministic pack format | `3P: sha2` | asset-decode, asset-runtime, asset-compiler; renderer-resource dev |
| 6 | `crates/asset-runtime` | immutable bounded runtime for synthetic-v1 packs | `N: asset-types`; `D: asset-compiler` | asset-decode, renderer-resource |
| 7 | `crates/diagnostics` | bounded/redacted diagnostics and history | `N: foundation` | app, test-support |
| 8 | `crates/foundation` | cancellation, monotonic time and process/session/task generations | standard library only | app-runtime, diagnostics, game-domain, game-session, identity, platform, protocol-canary, renderer, renderer-resource, simulation-core, test-support, transport, app and both integration-test packages |
| 9 | `crates/game-domain` | protocol-neutral **client** command/event envelopes, session-scoped handles and values | `N: foundation` | simulation-core, protocol-canary |
| 10 | `crates/simulation-core` | deterministic **client** world projection and immutable render snapshots | `N: foundation, game-domain` | no direct workspace consumer; internal tests only |
| 11 | `crates/input-actions` | framework-neutral physical-input and semantic-action contracts | none | input-platform |
| 12 | `crates/input-platform` | bounded Windows/winit physical-input adapter | `N: input-actions`; `W: winit` | no direct workspace consumer; internal tests only |
| 13 | `crates/game-session` | W7 selection, one-shot credential and entry lifecycle including `CanaryCurrent` | `N: account-session, foundation, world-directory` | app, app-runtime, identity, platform, protocol-canary, technical-login tests |
| 14 | `crates/identity` | Authorization Code + PKCE, system browser and loopback callback validation | `N: account-session, foundation, game-session, platform, world-directory`; `3P: base64, getrandom, sha2, url` | app, both integration-test packages |
| 15 | `crates/platform` | strict Identity/Game Gateway HTTP and DTO boundary | `N: account-session, foundation, game-session, world-directory`; `3P: serde, serde_json, time, ureq, url` | app, identity, both integration-test packages |
| 16 | `crates/protocol-canary` | Canary compatibility adapter mapping wire data to current client contracts | `N: foundation, game-domain, game-session, protocol-core, transport, world-directory`; `D: account-session` | app, technical-login tests |
| 17 | `crates/protocol-core` | bounded binary readers/writers with little-endian primitives and `u16::MAX` ceiling | standard library only | protocol-canary |
| 18 | `crates/renderer` | Windows/wgpu surface lifecycle and bounded recovery | `N: foundation`; `W: pollster, wgpu` | app |
| 19 | `crates/renderer-resource` | generation-fenced synthetic texture upload/cache lifecycle | `N: asset-decode, asset-runtime, foundation`; `D: asset-types` | no direct workspace consumer; internal tests only |
| 20 | `crates/test-support` | deterministic shared diagnostics/foundation test helpers | `N: diagnostics, foundation` | app dev dependency |
| 21 | `crates/transport` | Tokio TCP full-duplex transport, framing boundary, queues, cancellation and shutdown | `N: foundation, tokio`; optional `blocking-baseline` feature | app, protocol-canary, technical-login tests |
| 22 | `crates/world-directory` | account/world/character/channel directory model, provisional identifiers and selection validation | `N: account-session` | app, app-runtime, game-session, identity, platform, protocol-canary and both integration-test packages |
| 23 | `tests/integration/technical-login` | cross-crate W7 login/entry integration tests | `N: account-session, app-runtime, foundation, game-session, identity, platform, protocol-canary, transport, world-directory`; `3P: serde_json, time, url` | test package |
| 24 | `tests/security/auth` | Identity/Platform authentication security tests | `N: account-session, foundation, identity, platform, world-directory`; `3P: serde_json, time, url` | test package |
| 25 | `tools/architecture-check` | hard-coded package-category and dependency-graph validator | `3P: serde_json` | CI/workspace policy command |
| 26 | `tools/asset-compiler` | deterministic compiler for synthetic-v1 fixture packs | `N: asset-types`; `3P: cap-fs-ext, cap-std, serde_json` | asset-decode/asset-runtime dev tests; standalone tool |

## 4. Direct third-party dependency inventory

| Dependency | Source use | FND-01 migration treatment |
|---|---|---|
| `base64` | PKCE/state encoding | retain only in `identity`; exact version and features revalidated by `VSL-02` |
| `getrandom` | Identity entropy | retain only in `identity`; OS CSPRNG failure remains fail-closed |
| `sha2` | PKCE and synthetic pack hashes | retain in narrowly scoped owners; not a substitute for signed production asset manifests |
| `url` | Identity and Platform URL validation | retain in Identity/Platform boundaries |
| `serde` / `serde_json` | Platform DTOs, tests, architecture tool and synthetic compiler | retain only in I/O/tool/test boundaries; not in foundation/client-domain hot paths |
| `time` | RFC3339 producer timestamps and tests | retain only in Platform/test boundaries; monotonic runtime timing remains separate |
| `ureq` | concrete blocking Platform HTTP adapter | do not automatically migrate as production I/O; retained as reference until `VSL-02` selects an audited cancellation-safe implementation consistent with the client runtime |
| `tokio` | client runtime and gameplay transport | retain for `client-runtime`; source gameplay transport usage remains reference-only until `FND-02` |
| `winit` | Windows event/input integration | retain behind Windows-target boundaries |
| `wgpu` | Windows renderer | retain provisionally at the source-compatible version for migration evidence; no cross-platform/performance claim follows |
| `pollster` | synchronous renderer initialization helper | retain provisionally only if initialization remains outside frame-critical execution; revalidate during migration |
| `cap-std` / `cap-fs-ext` | synthetic fixture compiler filesystem sandbox | retain only in non-release fixture tooling |

The source root also declares exact workspace versions for `pollster`, `serde_json`, `sha2`, `tokio` and `wgpu`. `VSL-02` records every retained, removed or newly introduced direct dependency and compares the final destination lockfile with this inventory.

## 5. Non-member source inventory

| Source path | Proven content | Inventory consequence |
|---|---|---|
| `contracts/` | only `contracts/canary/` | all content is Canary reference evidence, not destination product contract material |
| `assets/` | only `assets/test-fixtures/` | fixtures may migrate only under explicit synthetic/test naming and provenance |
| `docs/architecture/` | source-client dual-protocol/Canary-first architecture | discovery evidence only; accepted Oteryn-v2 ADRs supersede conflicting direction |
| `docs/agents/` and local `AGENTS.md` | source repository programme/governance | history/reference only; destination governance is authoritative |
| `Cargo.toml` | 26-member client-only graph | rewritten by the atomic destination migration according to accepted FND-01 membership |
| `Cargo.lock` | exact source dependency graph | retained by source history and recorded by blob; destination lockfile is regenerated and audited |
| `rust-toolchain.toml` | Rust 1.94.0, Windows target only | destination adds Linux shared validation while retaining Windows client target |
| `rustfmt.toml` | edition 2024, width 100, native line endings | destination retains edition/width and normalizes repository text to LF |
| `deny.toml` | Windows all-features graph, documented duplicate exceptions | rewritten for product-realistic Windows default, Linux shared and supplemental all-features graphs |
| `.github/workflows/rust-client.yml` | Windows workspace plus Ubuntu supply-chain jobs | rewritten into destination product/target matrix and release-closure checks |

## 6. Open source work at inventory time

Open PRs visible at the inventory point:

- `blakinio/otclient#23` — C++/Lua/OTUI login-shell prototype;
- `blakinio/otclient#48` — operational official Tibia Linux analysis;
- `blakinio/otclient#97` — legacy client asset-download integrity work.

None changes the `oteryn-client` Rust workspace at the inspected heads. They are not silently included or discarded. `VSL-02` must record each terminal cutover disposition and repeat the exact source-path comparison at the selected cutover SHA.

## 7. Inventory conclusions

### PROVEN

- The source graph is not suitable for unchanged migration because production application and integration tests depend on Canary.
- The account/directory/entry stack is split across five tightly coupled product crates plus application orchestration.
- Several workspace members have no direct product consumer and exist as tested producers/spikes.
- Client domain/simulation contracts are explicitly client projection models.
- Asset and renderer-resource code uses synthetic fixture formats.
- Current protocol and transport ceilings cannot be assumed to satisfy the native protocol contract.

### DERIVED

- The destination must consolidate trivial/tightly coupled session-directory contracts while separating value contracts from Platform HTTP I/O.
- Client projection, simulation and synthetic assets need an explicit non-release consumer if retained.
- The production client and deterministic synthetic harness should be separate executables/dependency closures, not feature variants of one release binary.
- Source protocol/transport code is valuable evidence but should be reconsidered under `FND-02`, where native framing and limits are owned.

## 8. VSL-02 refresh procedure

Before physical migration, `VSL-02` must:

1. pin the exact source cutover SHA;
2. compare `oteryn-client/` at that SHA against this inventory revision;
3. regenerate the member, edge, third-party and non-member inventories for every changed path;
4. reconcile open PRs and active tasks;
5. update the source-to-destination path map and provenance manifest;
6. reject migration if any unclassified member, dependency, fixture or contract remains.
