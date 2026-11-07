# VSL-02: Exact Rust Client Migration and Cutover Contract

- Status: implementation-ready candidate
- Date: 2026-08-06
- Coordination ID: `OTV2-RUST-CLIENT-CUTOVER-20260806`
- Destination: `blakinio/Oteryn-v2`
- Destination contract base: `9034bd4bfa491eac6a898b29bc8151c94a4c2b89`
- Source: `blakinio/otclient/oteryn-client`
- Selected source commit: `c923ad8a1dff17b4933a6110931b0823cec2c590`
- Selected subtree tree: `c0928dafca6df19ff11d7901e503ed85a5199439`
- Governing workspace contract: `docs/architecture/FND-01_WORKSPACE_AND_RUST_MIGRATION_CONTRACT.md`
- Source reconciliation: `docs/migration/VSL-02_SOURCE_RECONCILIATION.md`
- Machine path map: `docs/migration/rust-client-path-map.json`
- Machine provenance/rollout plan: `docs/migration/rust-client-provenance-plan.json`

## 1. Purpose

This contract defines the only permitted migration of the Rust client into the canonical Oteryn-v2 Rust workspace.

It fixes:

- one immutable source commit and subtree tree;
- one exact source-to-destination mapping;
- one atomic destination implementation PR;
- one root Cargo workspace and lockfile;
- truthful cross-repository provenance;
- disposition-specific transformation evidence;
- no Canary, gameplay transport, Game Session or speculative native protocol;
- destination-first ownership and a later source-only marker;
- single-writer rollback with no zero-canonical or dual-writable interval.

This task does not move code. Its merge authorizes the atomic destination migration task. A separate owner-authorized source task is required after destination merge.

## 2. Binding architecture

- ADR-0002 makes Oteryn-v2 the destination for the Rust client/server/shared workspace.
- ADR-0008 makes `protocol-canary` permanently `REFERENCE_ONLY` for this migration.
- ADR-0011 requires a launchable `pre-native-protocol` client and fail-closed gameplay entry.
- FND-01 fixes the exact initial 19-member workspace and all source dispositions.
- FND-ID-01 owns final public identifiers.
- FND-02 owns native protocol, framing and gameplay transport.
- FND-04 owns Game Session credentials, routing, admission and leases.

VSL-02 may select migration tooling and dependencies required to implement FND-01. It may not capture later protocol, runtime, admission or durable-state contracts.

## 3. Exact source and drift rule

```text
repository: blakinio/otclient
commit: c923ad8a1dff17b4933a6110931b0823cec2c590
subtree: oteryn-client/
subtree tree: c0928dafca6df19ff11d7901e503ed85a5199439
root manifest blob: 037013e8e4a762a65f0f2a30f7761ee14725a3fc
root lockfile blob: 2143408c12c50132883890f0821278320a331fde
```

The implementation imports only from the selected commit, never from mutable later `main`.

Source `main` may advance outside `oteryn-client/**`. Before opening the atomic PR and again before merging it, the current default branch must resolve `oteryn-client/` to tree `c0928daf...`.

A changed subtree, new Rust-subtree PR or new active ownership claim stops migration. A newer source requires an owner-approved VSL-02 amendment, new commit/tree, regenerated manifests and fresh audit.

## 4. Freeze and conflicting source programmes

The administrative freeze starts when VSL-02 merges:

```text
blakinio/otclient/oteryn-client/**
```

It forbids normal feature, refactor, dependency, protocol, formatting and documentation work in that subtree. Legacy work outside it is unaffected.

Two source programmes are historical only:

1. `OTC2-20260803-playability-p2-canary-world-protocol`
   - Canary code/evidence remains source-only;
   - it cannot block omission of `protocol-canary`.
2. `oteryn-client/docs/agents/tasks/active/OTC2-20260805-native-protocol-single-version-completion.md`
   - its dual-protocol/native-runtime plan is superseded by ADR-0008, ADR-0011 and FND-01;
   - it cannot be resumed for Oteryn-v2;
   - it is archived as `SUPERSEDED_REFERENCE_ONLY` by the later source marker.

## 5. Atomic destination package

Use branch:

```text
migrate/rust-client-cutover-c923ad8
```

and one PR to `blakinio/Oteryn-v2:main`.

That PR must contain together:

1. the exact 19-member workspace;
2. every FND-01 migrate/merge/split/rename/rewrite;
3. omission of every reference-only path;
4. a launchable Windows pre-native client;
5. a separate non-release synthetic harness;
6. root lockfile, toolchain, rustfmt, cargo-deny and boundaries policy;
7. executable dependency/release-role checks;
8. destination Windows/Linux Rust workflows;
9. finalized provenance, path map and dependency delta;
10. disposition-specific transformation evidence;
11. production closure negatives;
12. source-marker and rollback handoff.

Forbidden delivery:

```text
copy source -> merge -> remove Canary later -> repair workspace later
```

No invalid intermediate destination may become canonical.

## 6. Exact workspace

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

No additional Cargo member is allowed. In particular, do not create or alias:

```text
protocol-canary
protocol-core
protocol-oteryn
transport
game-session
game-server
session-lease
persistence
production world/content/scripting crates
```

## 7. Mapping rules

`rust-client-path-map.json` is normative and covers all 26 source members, non-member configs, fixtures and exclusions.

- every source member has one disposition;
- every derived file records source blob and destination path;
- every destination file is source-derived or destination-authored;
- merged/split code lands directly in final modules;
- no committed import/staging duplicate tree;
- no source governance, manifest, lockfile or workflow becomes destination authority.

The technical-login suite is `REFERENCE_ONLY` with no destination path. `tests/pre-native-client/**` is independently authored from accepted negative scenarios.

## 8. Permanent exclusions

Source-only evidence:

```text
oteryn-client/crates/protocol-canary/**
oteryn-client/crates/protocol-core/**
oteryn-client/crates/transport/**
oteryn-client/contracts/canary/**
oteryn-client/docs/architecture/**
oteryn-client/docs/agents/**
oteryn-client/AGENTS.md
```

No utility-module copy may bypass these dispositions. Protocol/transport reuse belongs only to FND-02.

No build output, binary, debug symbol, cache, secret, environment file or proprietary Tibia/CipSoft asset enters the destination.

## 9. Provenance

The migration does not preserve cross-repository Git ancestry. It uses immutable provenance:

```text
docs/migration/rust-client-provenance.json
docs/migration/rust-client-path-map.json
docs/migration/rust-dependency-delta.json
```

Every source-derived file records:

```text
source_repository
source_commit
source_path
source_blob_sha
destination_path
disposition
transformation
license
verification
```

Rules:

- no fabricated co-authorship or ancestry claim;
- rewritten files are not called byte-identical;
- applicable notices are preserved;
- destination glue/rewrites are destination-authored;
- proprietary references do not authorize asset import;
- final squash message records coordination ID, source commit and subtree tree;
- source subtree/history remains available as evidence.

## 10. Dependency policy

Retain during cutover:

```text
Rust 1.94.0
edition 2024
resolver 3
Tokio =1.51.4
```

Tokio normal features:

```text
io-util, net, rt-multi-thread, sync, time
```

`macros` and `test-util` are test-only and cannot enlarge production closure.

Replace source blocking `ureq` with:

```toml
reqwest = {
  version = "=0.13.4",
  default-features = false,
  features = ["form", "json", "rustls", "stream"]
}
```

Forbidden initial capabilities:

```text
blocking, cookies
compression features
multipart, socks, system-proxy
native-tls, native-tls-vendored
http2, http3
```

Required HTTP behavior:

- explicit rustls backend;
- no automatic redirects or proxy discovery;
- no cookie store or transparent decompression;
- mandatory certificate and hostname validation;
- no production custom CA/pin in migration;
- bounded request/body/header/chunk/total response sizes;
- explicit connect/request/total deadlines;
- application-Tokio-owned cancellation and shutdown;
- no leaked workers/tasks/secrets;
- redacted diagnostics/errors.

Direct source `ureq` and direct Platform native-tls usage are removed. Any remaining transitive native-tls occurrence requires explicit lockfile-delta explanation.

All other retained direct dependencies remain exact-pinned unless an accepted merge/split removes them. No broad opportunistic upgrade is allowed.

## 11. Product behavior

The production `oteryn-client` must:

- build for `x86_64-pc-windows-msvc`;
- launch a real application window/shell;
- initialize only permitted Identity/account-directory boundaries;
- clearly show native gameplay unavailable;
- never advertise Canary or protocol selection;
- fail before gameplay credential request/consumption;
- fail before Game Session binding;
- fail before gameplay endpoint DNS resolution/connection;
- never present synthetic success as gameplay;
- shut down without task/worker leakage.

Directory contracts contain no gameplay host, port, endpoint, protocol selector, admission token or Game Session credential.

## 12. Synthetic harness

`oteryn-synthetic-client-harness` is a separate non-release executable and the consumer for client domain, simulation, input, renderer, synthetic assets and test support.

It:

- uses project-owned deterministic fixtures;
- has no Identity, Platform or gameplay network capability;
- runs bounded deterministic scenarios;
- never enters production installer/package manifests;
- is not a production feature;
- makes no protocol/server/shared-E2E claim.

The production app cannot normally or at build time depend on the harness, client simulation, synthetic assets or test support.

## 13. Transformation evidence

### `MIGRATE_AS_IS`

Public-item inventory, retained tests, metadata-only semantic diff and source/destination blobs.

### `MIGRATE_AND_RENAME`

Package/API/symbol rename table, retained or stronger tests and proof of ownership-name-only change.

### `MERGE`

Source-to-final-module map, duplicate type/error/limit reconciliation, retained tests and no cycle.

### `SPLIT`

Every public item classified retained/rewritten/deferred/dropped, one owner per retained item, deferred gameplay semantics absent and no hidden source-crate shim.

### `REWRITE`

Preserved scenarios, discarded assumptions and destination tests proving the new boundary.

### `REFERENCE_ONLY`

Exact source path/SHA, zero destination copy/member and zero release inclusion.

## 14. Workspace enforcement

The atomic PR creates one root:

```text
Cargo.toml
Cargo.lock
rust-toolchain.toml
rustfmt.toml
deny.toml
workspace-boundaries.toml
```

The boundaries policy lists every member, package, category, allowed internal edge, target role and release role.

Architecture validation consumes it with `cargo metadata --locked` and rejects:

- missing/duplicate/unregistered members;
- normal/dev/build cycles;
- forbidden names/features/dependencies;
- external path, Git or unknown registry sources;
- hidden Canary/protocol/transport code;
- production synthetic/test/tool dependencies;
- provisional identifier violations;
- root metadata/toolchain/lockfile drift;
- release closure mismatch.

## 15. Atomic PR validation

### Source/provenance

- selected commit reachable;
- selected subtree tree exact;
- current source default branch has same subtree tree;
- no new Rust-subtree PR/task;
- machine documents parse and satisfy schemas;
- all source blobs/paths exist;
- every destination file is classified;
- excluded/binary/secret/proprietary content absent.

### Ubuntu workspace policy

```text
cargo metadata --locked --format-version 1
cargo fmt --all --check
cargo run --locked -p oteryn-architecture-check -- workspace .
```

Plus governance, cycle, source restriction, lock and dependency-delta validation.

### Shared Linux

Compile, Clippy and test the named portable selection for `x86_64-unknown-linux-gnu`. No Linux desktop-client claim.

### Windows pre-native client

- default production build and strict Clippy;
- launch, visible pre-native state and shutdown;
- PKCE/callback/directory negatives;
- proof of no gameplay credential/endpoint operation;
- renderer lifecycle evidence;
- production closure audit.

### Windows harness

Build/run deterministic domain, simulation, input, renderer and synthetic-asset scenarios with no network capability.

### Security/supply chain

- cargo-deny for Windows-default and Linux-shared graphs;
- supplemental all-features only;
- Dependency Review and CodeQL;
- redaction/oversize/truncation/path-traversal negatives;
- reqwest feature/client-builder policy checks;
- license/provenance review.

### Production closure negatives

Production normal/build closure cannot contain or alias:

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
any test or tool package
```

## 16. Destination merge gate

Merge only when:

- source commit/tree/current subtree are reverified;
- exactly 19 members exist;
- provenance/mapping/dependency records are final;
- diff is inside migration ownership;
- prohibited content is absent;
- all exact-head focused/Linux/Windows/harness/security gates pass;
- independent audit has zero material findings;
- no review, ownership or ordering hold remains;
- destination branch is current with `main`;
- rollback remains executable;
- squash merge is used.

The merge commit becomes the canonical Rust workspace commit.

## 17. Forward rollout

1. merge/archive VSL-02;
2. reverify selected source commit/tree/current subtree;
3. create atomic destination migration task/PR;
4. complete migration, workspace, CI, provenance and validation in that PR;
5. squash-merge destination implementation;
6. verify destination `main` and provenance;
7. create separately authorized source-marker task/PR;
8. mark source moved/non-canonical and archive both conflicting Rust tasks;
9. validate/squash-merge source marker;
10. release coordination ownership and proceed to FND-ID-01.

The source marker cannot merge before destination implementation.

## 18. Single-writer rollback

### Before destination merge

Close the destination PR. Source remains the single canonical but administratively frozen Rust source.

### After destination merge, before source marker

1. place destination Rust workspace under a **non-writable rollback hold**;
2. revert destination migration before dependent destination work remains;
3. record source as the single canonical Rust source;
4. lift or replace its migration freeze only through an owner-approved coordination amendment.

### After source marker

1. place destination Rust workspace under a **non-writable rollback hold**;
2. revert source marker; that merge transfers canonical write authority back to source;
3. revert destination migration before dependent destination work remains;
4. keep destination hold until it no longer claims canonical ownership;
5. update coordination records and reopen migration analysis.

The destination hold prevents a dual-writable interval while source authority is restored. Never leave both repositories non-canonical or both writable as canonical.

## 19. Source marker

A separate authorized source PR changes only ownership documentation/task lifecycle. Exact paths are in the reconciliation document.

It records:

- selected source commit/tree;
- destination PR/merge;
- finalized manifests;
- ownership-change date;
- no Canary migration claim;
- Canary task archived reference-only;
- nested native task archived superseded reference-only;
- rollback order and destination governance.

It preserves source subtree and history.

## 20. Completion and next gate

VSL-02 completes after independent audit, exact-head checks, squash merge and task archival.

It authorizes the atomic destination migration task, not source writes, partial bootstrap or native protocol implementation.

After destination migration and source marker are terminal:

```text
FND-ID-01 — Foundation Identifier Vocabulary
```

## 21. Rejected alternatives

- mutable latest source;
- unchanged 26-crate copy;
- false ancestry claim;
- import-first/clean-later;
- temporary Canary;
- empty native protocol;
- blocking `ureq`;
- broad dependency upgrades;
- whole-source-main freeze;
- resumable nested dual-protocol task;
- source marker before destination;
- development in both repositories;
- rollback without destination non-writable hold.

## 22. Acceptance checklist

- [x] exact source commit and subtree tree pinned;
- [x] source PRs and root/nested tasks reconciled;
- [x] all 26 members/non-members machine-mapped;
- [x] reference-only tests do not imply copying;
- [x] truthful provenance policy fixed;
- [x] atomic 19-member package fixed;
- [x] exact Tokio and reqwest/rustls policy fixed;
- [x] production/harness closures separate;
- [x] disposition-specific evidence fixed;
- [x] validation matrices fixed;
- [x] freeze/source-marker order fixed;
- [x] single-writer rollout and rollback fixed;
- [ ] final independent audit passes;
- [ ] exact-head checks pass;
- [ ] PR merges and task archives.
