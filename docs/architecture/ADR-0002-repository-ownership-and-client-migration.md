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

### 4. Status of the old repository

After successful migration:

- `blakinio/otclient` remains available as history and migration/reference evidence;
- the migrated `oteryn-client` path is marked as moved or otherwise prevented from becoming a second active product line;
- new Oteryn v2 client development occurs only in `blakinio/Oteryn-v2`;
- removing or archiving unrelated legacy OTClient content is a separate decision.

### 5. Protocol ownership

`protocol-oteryn` has one canonical schema and fixture owner in `blakinio/Oteryn-v2`.

Client and server adapters may have different implementation modules, but they consume the same versioned contract and are validated together. Rust memory layout, unstable serializer output or duplicated hand-written message definitions must not become the implicit wire contract.

### 6. Legacy protocol handling

`protocol-canary` is not part of the target Oteryn v2 runtime.

During migration it may be retained temporarily only as bounded reference or migration evidence. It must not:

- become a dependency of `protocol-oteryn`;
- shape the new gameplay domain model;
- remain an enabled production adapter in the Oteryn v2 client;
- require the Rust game server to implement Canary/Tibia packet compatibility.

Its removal or isolation must be explicit in the migration plan and acceptance evidence.

### 7. Platform remains external

This repository consolidation applies to the native Rust gameplay stack only.

`blakinio/Oteryn-Platform` remains a separate repository and owner of portal, Identity, OAuth/PKCE, Game Login Tickets, Game Gateway and World Registry. Platform code is not moved into the Rust workspace by this ADR.

## Consequences

### Positive

- client/server protocol changes can be delivered atomically in one PR;
- shared identifiers and fixtures have one owner;
- cross-component E2E tests can run on one exact commit;
- dependency direction is easier to enforce;
- the legacy Canary adapter can be removed without preserving a permanent cross-repository compatibility layer.

### Costs

- the client migration requires provenance and history handling;
- the new workspace will contain both desktop-client and server build concerns;
- CI must support at least Windows client and Linux server targets;
- ownership and path leases must prevent unrelated agents from changing shared crates concurrently.

## Rejected alternatives

### Keep the client permanently in `blakinio/otclient`

Rejected because it creates continuous cross-repository coordination for protocol and shared-type changes.

### Duplicate the client into both repositories

Rejected because it creates two sources of truth and an immediate drift risk.

### Move the Platform into Oteryn v2

Rejected because Platform is a separate control-plane boundary with different technology, deployment and security responsibilities.

## Not performed by this ADR

- no code has yet been moved;
- no root Cargo workspace has been created;
- no exact migration source commit has been selected;
- no final crate graph has been accepted;
- no `protocol-oteryn` runtime has been implemented.

## Required follow-up

1. Accept the workspace and dependency contract.
2. Create a dedicated cross-repository migration programme with one task/branch/PR per written repository.
3. Pin and migrate the exact client source revision.
4. Validate the migrated client before freezing the old location.
5. Implement and validate the shared `protocol-oteryn` contract in the destination workspace.
