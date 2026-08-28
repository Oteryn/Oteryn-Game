# ADR-0008: `protocol-canary` is reference-only migration evidence

- Status: Accepted
- Date: 2026-08-05
- Decision owners: Oteryn project owner and Oteryn v2 architecture programme
- Decision gate: binding constraint for `FND-01` and `VSL-02`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Related: ADR-0001, ADR-0002, `FOUNDATION_DECISION_BACKLOG.md`, `GLOBAL_ARCHITECTURE_DECISION_REGISTER.md`

## Context

The existing native Rust client under `blakinio/otclient/oteryn-client` currently contains a `crates/protocol-canary` workspace member. That adapter was created for compatibility with historical Canary/Tibia packet families and with an earlier dual-protocol client direction.

Oteryn v2 has since accepted a different target architecture:

- one native Rust client;
- one authoritative Rust game server;
- one project-owned gameplay protocol, `protocol-oteryn`;
- classic and modern gameplay variants implemented as ruleset/content policy rather than wire-protocol forks;
- no Canary/Tibia compatibility adapter in the target runtime.

The owner confirmed that `protocol-oteryn` is designed as Oteryn's native protocol rather than as an evolution, wrapper or translation of `protocol-canary`, and that Canary compatibility is not wanted in production.

`FND-01` must classify every existing Rust-client crate or subsystem. Without an explicit fixed disposition, an inventory agent could incorrectly preserve `protocol-canary` as a destination workspace member, optional production feature or fallback path.

## Decision

### 1. Fixed `FND-01` classification

The existing source subsystem:

```text
blakinio/otclient/oteryn-client/crates/protocol-canary
```

has the binding primary migration classification:

```text
REFERENCE_ONLY
```

`FND-01` and `VSL-02` may refine how evidence is retained, but they may not promote this subsystem to a target runtime component without an owner-approved ADR that explicitly supersedes ADR-0001, ADR-0002 and this decision.

### 2. No production runtime presence

`protocol-canary` must not be present in the Oteryn v2 production runtime graph.

It must not become:

- a production Cargo workspace member required by the client or server;
- a direct or transitive dependency of `protocol-oteryn`;
- a direct or transitive dependency of gameplay domain or simulation crates;
- an enabled or optional production client adapter;
- a server listener, compatibility endpoint or packet family;
- a protocol negotiation candidate;
- a fallback after authentication, ticket redemption, Game Session issuance or connection failure;
- a translation layer between Oteryn domain messages and the native wire protocol;
- a source of canonical Oteryn identifiers, message names, packet layout or gameplay semantics.

There is no production `ForceCanary`, `Auto` dual-protocol selection or silent downgrade mode in Oteryn v2.

### 3. Permitted reference use

Pinned Canary-related material may be used only as bounded evidence for:

- understanding existing client behavior and migration risk;
- identifying protocol-neutral client/domain boundaries worth preserving;
- regression fixtures that demonstrate intentional incompatibility or removal;
- verifying that the destination build and dependency graph contain no Canary runtime edge;
- historical provenance and audit of the source-client migration;
- comparison of semantic actions and events where the behavior is independently validated and not copied as the native wire contract.

Reference use does not make Canary behavior normative for Oteryn.

### 4. Retention location and build isolation

The preferred destination disposition is to exclude `protocol-canary` source code from the migrated canonical product tree.

When `VSL-02` establishes that retaining selected files is necessary for reviewable evidence, they must be placed in an explicitly non-production evidence or migration location that:

- is not a Cargo workspace member;
- is not reachable from production features or target binaries;
- is excluded from release packaging;
- has exact source repository, commit, path and transformation provenance;
- retains applicable copyright and license notices;
- cannot be enabled by a runtime flag, Cargo feature or environment variable;
- is covered by an architecture check preventing dependency edges into product crates.

Fixtures should be minimal and synthetic where possible. Proprietary packets, credentials, account data or assets must not be retained.

### 5. Native protocol independence

`protocol-oteryn` is designed from the accepted Oteryn contracts and target product requirements.

It must have its own:

- stable schema and canonical owner;
- framing and transport contract;
- revision and capability model;
- sequencing, `CommandId`, replay and idempotency behavior;
- snapshots, deltas and reconciliation model;
- resource limits and error vocabulary;
- client/server golden fixtures;
- downgrade-prevention and rollout contract.

It must not preserve Canary opcode numbering, packet branches, build-profile negotiation, RSA/XTEA assumptions or serializer behavior merely for familiarity.

Semantic ideas may be adopted only after independent evaluation against Oteryn's domain model and accepted as Oteryn behavior. Wire compatibility is neither a goal nor an acceptance criterion.

### 6. Migration and validation requirements

The atomic Oteryn-v2 destination migration/workspace PR required by ADR-0002 must prove on one exact head that:

1. the source `protocol-canary` subsystem is classified `REFERENCE_ONLY` in the migration manifest;
2. no production workspace member depends on it directly or transitively;
3. no production binary exposes Canary protocol selection, negotiation, fallback or listener behavior;
4. `protocol-oteryn` and protocol-neutral domain contracts do not import Canary definitions;
5. any retained evidence is outside product workspace membership and release packaging;
6. provenance and license treatment are complete;
7. architecture/dependency checks fail when a prohibited Canary edge is introduced.

The later source-marker PR in `blakinio/otclient` preserves the historical source repository and points to the canonical destination. It does not migrate Canary into the target runtime.

### 7. Failure behavior

When a client, server, Gateway or route offers only Canary/Tibia compatibility to an Oteryn v2 product build, the result is an explicit unsupported-revision or unsupported-product-path failure.

The system must not:

- silently choose Canary;
- issue an Oteryn Game Session for a Canary-only route;
- retry using a legacy adapter after partial native admission;
- reinterpret a native protocol failure as permission to downgrade.

## Consequences

### Positive

- the new protocol and domain model are not constrained by historical packet compatibility;
- the destination workspace has one protocol source of truth;
- protocol negotiation and admission remain simpler and downgrade-resistant;
- client and server implementation cannot drift into a permanent dual-protocol product;
- historical evidence remains available without becoming runtime authority;
- `FND-01` agents cannot classify the adapter inconsistently.

### Costs

- Oteryn v2 cannot connect to Canary/Tibia-compatible servers;
- comparison with historical behavior requires separate fixtures or reference tooling;
- migration must remove or isolate source-client code and tests that assume dual-protocol selection;
- some reusable client abstractions may require extraction from Canary-specific modules before migration.

## Rejected alternatives

### Retain Canary as an optional production adapter

Rejected because an optional adapter still becomes a supported compatibility surface, preserves downgrade and testing complexity and can influence the domain model.

### Translate `protocol-oteryn` through `protocol-canary`

Rejected because it would make the legacy adapter an implicit canonical intermediate representation and prevent independent native protocol evolution.

### Keep dual-protocol `Auto` selection during transition

Rejected for the Oteryn v2 destination runtime. Migration and historical validation do not require a production fallback mechanism.

### Delete all historical evidence immediately

Rejected because the pinned source repository remains useful for provenance, migration audit and behavioral comparison. Evidence is retained at the repository/reference boundary, not in the production runtime graph.

## Non-goals

This ADR does not:

- select the final `protocol-oteryn` schema or IDL;
- implement client or server codecs;
- move client source;
- create the root Cargo workspace;
- modify `blakinio/otclient`;
- classify unrelated client crates or subsystems;
- supersede the requirement to reconcile the merged Platform native contract in `FND-02`.

## Acceptance invariant

Future work complies with this ADR only when:

> `protocol-canary` exists solely as pinned historical or migration evidence, is unreachable from every Oteryn v2 production binary and dependency graph, and `protocol-oteryn` is designed and implemented as the project's independent native gameplay protocol with no compatibility fallback or translation through Canary.
