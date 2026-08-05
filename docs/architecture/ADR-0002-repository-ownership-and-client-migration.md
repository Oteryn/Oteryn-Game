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
3. preserve useful file history using an approved history-preserving mechanism where practical;
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
→ coordinated destination migration and source freeze/cutover
→ canonical root workspace bootstrap or completion around the migrated client
→ FND-ID-01 foundation identifiers
→ FND-02 protocol-oteryn
→ FND-03 runtime
→ FND-04 admission and character lease
```

`FND-01` must inspect the exact current client workspace and classify every existing crate and subsystem. It must not create a duplicate client architecture from an illustrative target tree.

After `FND-01` is accepted, `VSL-02` becomes the next mandatory gate. No canonical client/server identifier, protocol, runtime or admission contract may be frozen against a destination workspace that does not contain the exact canonical client source and its accepted migration disposition.

The destination bootstrap may be part of the coordinated migration package or may immediately follow it as a separate task. In either case:

- the destination must not establish a competing placeholder client before cutover;
- the source SHA, open pull requests and post-audit changes must be reconciled again at cutover;
- shared crates and dependency boundaries must be created around the migrated client and immediate server consumers, not around speculative empty layers;
- `protocol-canary` may be retained only as bounded evidence or isolated migration support and cannot become part of the target runtime graph;
- the source repository must be frozen or clearly marked non-canonical only after the destination build, tests, provenance and rollback evidence pass.

This sequencing prevents the `FND-01` inventory from becoming stale while later contracts are designed and preserves the single-repository purpose of this ADR.

### 5. Status of the old repository

After successful migration:

- `blakinio/otclient` remains available as history and migration/reference evidence;
- the migrated `oteryn-client` path is marked as moved or otherwise prevented from becoming a second active product line;
- new Oteryn v2 client development occurs only in `blakinio/Oteryn-v2`;
- removing or archiving unrelated legacy OTClient content is a separate decision.

### 6. Protocol ownership

`protocol-oteryn` has one canonical schema and fixture owner in `blakinio/Oteryn-v2`.

Client and server adapters may have different implementation modules, but they consume the same versioned contract and are validated together. Rust memory layout, unstable serializer output or duplicated hand-written message definitions must not become the implicit wire contract.

### 7. Legacy protocol handling

`protocol-canary` is not part of the target Oteryn v2 runtime.

During migration it may be retained temporarily only as bounded reference or migration evidence. It must not:

- become a dependency of `protocol-oteryn`;
- shape the new gameplay domain model;
- remain an enabled production adapter in the Oteryn v2 client;
- require the Rust game server to implement Canary/Tibia packet compatibility.

Its removal or isolation must be explicit in the migration plan and acceptance evidence.

### 8. Platform remains external

This repository consolidation applies to the native Rust gameplay stack only.

`blakinio/Oteryn-Platform` remains a separate repository and owner of portal, Identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry. Platform code is not moved into the Rust workspace by this ADR.

## Consequences

### Positive

- client/server protocol changes can be delivered atomically in one PR;
- shared identifiers and fixtures have one owner;
- cross-component E2E tests can run on one exact commit;
- dependency direction is easier to enforce;
- the legacy Canary adapter can be removed without preserving a permanent cross-repository compatibility layer;
- foundation contracts are designed against the actual canonical client rather than a stale source inventory or placeholder destination tree.

### Costs

- the client migration requires provenance and history handling;
- the new workspace will contain both desktop-client and server build concerns;
- CI must support at least Windows client and Linux server targets;
- ownership and path leases must prevent unrelated agents from changing shared crates concurrently;
- open client pull requests and changes after the FND-01 inventory require explicit cutover reconciliation before migration.

## Rejected alternatives

### Keep the client permanently in `blakinio/otclient`

Rejected because it creates continuous cross-repository coordination for protocol and shared-type changes.

### Duplicate the client into both repositories

Rejected because it creates two sources of truth and an immediate drift risk.

### Bootstrap a new canonical client shell before migrating the existing client

Rejected because it would create a competing architecture, make the FND-01 inventory stale and force later identifiers/protocol/runtime contracts to reconcile two client foundations.

### Delay migration until after protocol and runtime contracts

Rejected because it recreates the cross-repository drift and coordinated-version problem that this ADR is intended to remove.

### Move the Platform into Oteryn v2

Rejected because Platform is a separate control-plane boundary with different technology, deployment and security responsibilities.

## Not performed by this ADR

- no code has yet been moved;
- no root Cargo workspace has been created;
- no exact migration source commit has been selected;
- no final crate graph has been accepted;
- no `protocol-oteryn` runtime has been implemented.

## Required follow-up

1. Accept `FND-01`, including the exact client inventory, migration disposition and target dependency boundaries.
2. Immediately accept `VSL-02` and create the dedicated cross-repository migration programme with one task/branch/PR per written repository.
3. Pin the cutover source SHA, reconcile open PRs and post-inventory changes, then migrate the client with provenance, history, validation and rollback evidence.
4. Freeze or mark the old client location non-canonical only after the destination proves its exact build and tests.
5. Bootstrap or complete the canonical root workspace around the migrated client and immediate server/shared consumers.
6. Continue with `FND-ID-01`, `FND-02`, `FND-03` and `FND-04` in that destination workspace.
