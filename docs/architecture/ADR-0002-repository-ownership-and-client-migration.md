# ADR-0002: Repository ownership and native Rust client migration

- Status: Accepted foundation
- Date: 2026-08-05
- Decision owners: Oteryn project
- Applies to: `blakinio/Oteryn-v2`, with migration input from `blakinio/otclient`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`

## Context

The native Rust client currently exists under `blakinio/otclient/oteryn-client`. The target Oteryn v2 product also requires a new authoritative Rust game server and one project-owned gameplay protocol, `protocol-oteryn`.

Keeping the client, server and shared wire/domain contracts in separate repositories would require coordinated version bumps and cross-repository rollout for every protocol change. It would also make it easier for client and server representations to drift or for two competing implementations of shared types to appear.

The current client provides useful foundations, including Tokio-based asynchronous execution, TCP transport, protocol-neutral boundaries, `protocol-core` and a legacy `protocol-canary` adapter. It does not currently contain an implemented `protocol-oteryn` crate.

## Decision

### 1. One canonical Rust repository

`blakinio/Oteryn-v2` is the canonical repository for:

- the native Rust client;
- the authoritative Rust game server;
- shared domain and identifier types;
- `protocol-oteryn` schemas, codecs and golden fixtures;
- transport and Platform integration clients used by the Rust stack;
- cross-client/server E2E tests.

The existing native Rust client will be migrated from:

```text
blakinio/otclient/oteryn-client
```

into the Oteryn v2 workspace, initially targeting:

```text
blakinio/Oteryn-v2/apps/client
```

### 2. Target repository shape

The initial target shape is:

```text
Oteryn-v2/
├── apps/
│   └── client/
├── services/
│   └── game-server/
├── crates/
│   ├── protocol-core/
│   ├── protocol-oteryn/
│   ├── transport/
│   ├── platform-contracts/
│   ├── platform-client/
│   └── shared domain/runtime crates
├── content/
└── tests/
    └── e2e/
```

Exact crate names and membership remain subject to the workspace and dependency contract. This ADR fixes ownership and direction, not the complete Cargo graph.

### 3. Migration requirements

The migration task must:

1. pin the exact source commit in `blakinio/otclient`;
2. preserve copyright, license and provenance records;
3. preserve useful source history through an approved traceability mechanism compatible with repository merge policy;
4. record all included and excluded paths;
5. prove the migrated client builds and tests on the exact destination head;
6. prevent simultaneous active development of two canonical client copies;
7. provide a rollback path before the old location is frozen;
8. update cross-repository documentation after the destination PR merges.

A plain untracked copy with no source revision or provenance record is not acceptable.

### 4. Migration sequencing is mandatory

The workspace/dependency audit and the physical client migration are separate gates, but they must occur consecutively.

The required order is:

```text
FND-01 workspace, dependency and existing-Rust migration contract
→ VSL-02 exact client migration and cutover contract
→ freeze and reconcile the exact source revision
→ one atomic Oteryn-v2 destination migration PR
     ├── import the accepted client paths
     ├── apply every FND-01 crate/subsystem disposition
     ├── create or complete the canonical root workspace
     ├── enforce the accepted dependency boundaries
     ├── isolate or remove protocol-canary from the target runtime graph
     └── validate the complete destination head
→ squash-merge the destination PR
→ separate source-marker PR in blakinio/otclient
→ FND-ID-01 foundation identifiers
→ FND-02 protocol-oteryn
→ FND-03 runtime
→ FND-04 admission and character lease
```

`FND-01` must inspect the exact current client workspace and classify every existing crate and subsystem. It must not create a duplicate client architecture from an illustrative target tree.

After `FND-01` is accepted, `VSL-02` becomes the next mandatory gate. No canonical client/server identifier, protocol, runtime or admission contract may be frozen against a destination workspace that does not contain the exact canonical client source and its accepted migration disposition.

### 5. One atomic destination pull request

The client import and destination workspace consolidation are one atomic delivery unit in `blakinio/Oteryn-v2`. They must not be split into an import-only destination PR followed by a second workspace-consolidation destination PR.

The one destination PR must include, on one exact final head:

- the exact source SHA and any accepted source commit range;
- reconciliation and terminal disposition of open source PRs, active tasks and changes after the `FND-01` inventory;
- exact source-to-destination path mapping and exclusions;
- every per-crate/subsystem migration disposition accepted by `FND-01`;
- creation or completion of the root Cargo workspace around the migrated client;
- only immediate server/shared members with named consumers and acceptance evidence;
- the accepted toolchain, lockfile, features, lints, target matrix and dependency-boundary checks;
- isolation, reference-only retention or removal of `protocol-canary` so it is absent from the target production runtime graph;
- copyright, license, provenance and asset-rights records;
- destination build, test and governance evidence required by `FND-01` and `VSL-02`;
- rollback instructions that remain executable before the source repository is marked non-canonical.

Logical commits may separate import, workspace adaptation and validation for review, but the pull request is accepted or rejected as one unit. No intermediate destination `main` state may contain an imported client that is not yet governed by the accepted root workspace and dependency boundaries.

### 6. Provenance and source history under squash merge

Oteryn-v2 delivery policy requires squash merge. Therefore the migration must not claim that cross-repository source commits become destination `main` ancestry unless a separately owner-approved governance exception explicitly changes that policy.

The approved traceability baseline is:

- retain `blakinio/otclient` as immutable history and migration evidence;
- record the exact source repository, source SHA and relevant source commit range;
- retain a machine-readable provenance and path-mapping manifest in `blakinio/Oteryn-v2`;
- record included, transformed, merged, split, rewritten, reference-only and dropped paths;
- preserve copyright/license notices and links to the source commits and relevant PRs;
- record destination paths and the final destination merge commit;
- validate that the imported tree corresponds to the pinned source plus explicitly documented transformations.

This preserves reviewable history and provenance without making a false Git-ancestry claim. A stronger history-preserving mechanism may be used only if it remains compatible with repository policy and is explicitly accepted by `VSL-02`.

### 7. Source freeze and source-marker pull request

The source repository must enter a coordinated cutover hold at the exact revision selected by `VSL-02`. No uncoordinated source changes may merge after that freeze; any required change must either move the cutover SHA and revalidate the destination or be deferred to the destination repository after cutover.

After the atomic destination PR is squash-merged and its exact destination result is verified, a separate PR in `blakinio/otclient` must only:

- mark `oteryn-client` as moved or non-canonical;
- point to the exact destination repository path and merge commit;
- direct all new Oteryn v2 client work to `blakinio/Oteryn-v2`;
- preserve source history and migration evidence;
- close or redirect remaining source tasks and PRs according to the accepted cutover disposition.

The source-marker PR is required because each written repository has its own task, branch and PR. It is cross-repository closeout, not a second destination implementation phase, and it must not contain new Oteryn v2 client architecture or runtime work.

If the destination PR does not merge or is reverted before the source-marker PR, the source remains canonical. Rollback after the source marker requires the explicit `VSL-02` rollback procedure and coordinated correction in both repositories.

### 8. Status of the old repository

After successful migration and source-marker merge:

- `blakinio/otclient` remains available as history and migration/reference evidence;
- the migrated `oteryn-client` path is marked as moved or otherwise prevented from becoming a second active product line;
- new Oteryn v2 client development occurs only in `blakinio/Oteryn-v2`;
- removing or archiving unrelated legacy OTClient content is a separate decision.

### 9. Protocol ownership

`protocol-oteryn` has one canonical schema and fixture owner in `blakinio/Oteryn-v2`.

Client and server adapters may have different implementation modules, but they consume the same versioned contract and are validated together. Rust memory layout, unstable serializer output or duplicated hand-written message definitions must not become the implicit wire contract.

### 10. Legacy protocol handling

`protocol-canary` is not part of the target Oteryn v2 runtime.

During migration it may be retained temporarily only as bounded reference or migration evidence. It must not:

- become a dependency of `protocol-oteryn`;
- shape the new gameplay domain model;
- remain an enabled production adapter in the Oteryn v2 client;
- require the Rust game server to implement Canary/Tibia packet compatibility.

Its removal or isolation must be explicit in the migration plan and acceptance evidence.

### 11. Platform remains external

This repository consolidation applies to the native Rust gameplay stack only.

`blakinio/Oteryn-Platform` remains a separate repository and owner of portal, Identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry. Platform code is not moved into the Rust workspace by this ADR.

## Consequences

### Positive

- client import and workspace consolidation cannot drift across two destination PRs;
- `main` never contains an imported but architecturally unintegrated canonical client;
- client/server protocol changes can later be delivered atomically in one repository;
- shared identifiers and fixtures have one owner;
- cross-component E2E tests can run on one exact commit;
- dependency direction is easier to enforce;
- the legacy Canary adapter can be removed without preserving a permanent cross-repository compatibility layer;
- foundation contracts are designed against the actual canonical client rather than a stale source inventory or placeholder destination tree;
- provenance remains reviewable despite squash merge.

### Costs

- the destination migration PR is larger and requires disciplined internal commits and review evidence;
- the source freeze may pause client work while exact-head destination validation completes;
- the client migration requires explicit provenance and path mapping because cross-repository commit ancestry is not imported through squash merge;
- the new workspace will contain both desktop-client and server build concerns;
- CI must support at least Windows client and the Linux/shared targets accepted by `FND-01`;
- ownership and path leases must prevent unrelated agents from changing shared crates concurrently;
- open client pull requests and changes after the `FND-01` inventory require explicit cutover reconciliation before migration.

## Rejected alternatives

### Keep the client permanently in `blakinio/otclient`

Rejected because it creates continuous cross-repository coordination for protocol and shared-type changes.

### Duplicate the client into both repositories

Rejected because it creates two sources of truth and an immediate drift risk.

### Bootstrap a new canonical client shell before migrating the existing client

Rejected because it would create a competing architecture, make the `FND-01` inventory stale and force later identifiers/protocol/runtime contracts to reconcile two client foundations.

### Delay migration until after protocol and runtime contracts

Rejected because it recreates the cross-repository drift and coordinated-version problem that this ADR is intended to remove.

### Use two destination PRs: import first, workspace consolidation second

Rejected because it would place an incomplete or ambiguously governed client on destination `main`, allow the second PR to drift or stall and weaken rollback and exact-head validation.

### Claim full Git-history preservation through a squash merge

Rejected because squash merge does not import source commits as destination mainline ancestry. Provenance must be recorded truthfully through the retained source and migration manifest.

### Move the Platform into Oteryn v2

Rejected because Platform is a separate control-plane boundary with different technology, deployment and security responsibilities.

## Not performed by this ADR

- no code has yet been moved;
- no root Cargo workspace has been created;
- no exact migration source commit has been selected;
- no final crate graph has been accepted;
- no `protocol-oteryn` runtime has been implemented;
- no write to `blakinio/otclient` is authorized by this ADR alone.

## Required follow-up

1. Accept `FND-01`, including the exact client inventory, migration disposition and target dependency boundaries.
2. Immediately accept `VSL-02` and create the dedicated cross-repository migration programme with one task/branch/PR per written repository.
3. Freeze and reconcile the exact cutover source revision, open PRs, active tasks and post-inventory changes.
4. Deliver one atomic `blakinio/Oteryn-v2` destination PR containing the accepted import, workspace creation/completion, dependency enforcement, `protocol-canary` isolation, provenance, validation and rollback evidence.
5. Squash-merge and verify the exact destination result.
6. Deliver the separate `blakinio/otclient` source-marker PR and release source ownership only after the destination merge is immutable and validated.
7. Continue with `FND-ID-01`, `FND-02`, `FND-03` and `FND-04` in the canonical destination workspace.
