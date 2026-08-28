# ADR-0004: PostgreSQL and separate Platform/game data ownership

- Status: Accepted foundation
- Date: 2026-08-05
- Decision owners: Oteryn project
- Applies to: `blakinio/Oteryn-v2` and future coordinated persistence work in `blakinio/Oteryn-Platform`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`

## Context

Oteryn v2 needs durable storage for highly concurrent and security-sensitive state, including characters, inventory, progression, leases, market operations, houses, rewards, checkpointing and recovery. The Platform separately owns Identity, web sessions, CMS, payments, support and administrative data.

The historical Canary/Otheryn ecosystem uses MySQL/MariaDB-compatible schemas. The current Oteryn Platform is also configured for SQLite in local/test environments and MySQL/MariaDB-compatible connections for deployed and Canary integration use. That compatibility is useful during migration but must not define the greenfield game-server persistence model.

A single unrestricted shared schema would couple Laravel and the Rust server, allow accidental cross-domain writes and make migrations, rollback, backup and security ownership ambiguous.

## Decision

### 1. PostgreSQL is the target relational database

PostgreSQL is the selected relational database technology for:

- the new authoritative Rust game server;
- the target production Platform persistence after a separately planned migration;
- transactional state requiring strong constraints, revision fencing, idempotency and reliable concurrent mutation.

New Oteryn v2 persistence contracts and implementation must not introduce a new MariaDB-specific dependency.

The existing Platform may remain temporarily on its current database while the Rust game foundation is built. That temporary state is a migration constraint, not the target architecture.

### 2. Separate logical databases

Platform and game persistence use separate logical databases:

```text
PostgreSQL cluster
├── oteryn_platform
└── oteryn_game
```

They may initially run on one physical PostgreSQL cluster for operational simplicity. They remain separate boundaries with:

- separate database names;
- separate owners and runtime credentials;
- separate migration histories;
- separate backup/restore policies;
- independent connection pools and resource controls where supported;
- no unrestricted cross-database writes;
- no assumption that both databases always share one physical host.

The architecture must allow later movement to separate PostgreSQL clusters without changing domain ownership.

### 3. One primary owner per data set

Every persistent data set has exactly one semantic and migration owner.

#### Platform-owned examples

- Identity records and account bindings;
- password hashes, recovery and MFA state;
- OAuth clients, grants and web/launcher sessions;
- Game Login Ticket state;
- World Registry and route policy;
- CMS/news and portal preferences;
- administration, support, moderation workflow and audit;
- Oteryn Coins wallet, payment and Character Bazaar business records;
- notification and account-security metadata.

Platform schema and migrations are owned by `blakinio/Oteryn-Platform`.

#### Game-owned examples

- characters and durable progression;
- inventory, equipment, bank and depot;
- quest and achievement progress;
- guild/gameplay membership state where the game domain is authoritative;
- market gameplay state;
- houses, rent, access and authoritative house items;
- character leases and session-generation fencing;
- combat/PvP consequences that persist across channels;
- reward eligibility and anti-channel-hopping records;
- checkpoint, recovery and durable runtime revisions;
- transactional outbox and critical gameplay mutation audit/journal data.

Game schema and migrations are owned by `blakinio/Oteryn-v2`.

### 4. No shared-table free-for-all

No table may have two unrestricted application writers.

Cross-system operations use one of:

- a versioned private API owned by the semantic owner;
- a narrow operation-specific database principal and written contract during bounded migration;
- an asynchronous event/outbox integration with idempotent consumers;
- a read-only projection or allowlisted database view.

Direct SQL from Platform into arbitrary game tables is not the target mutation model. Direct SQL from the game server into Platform Identity, wallet or payment tables is prohibited.

### 5. No cross-database foreign keys

Platform-to-game identity links use stable identifiers and versioned contracts, not cross-database foreign keys.

For example, the game database may store an `AccountId` or Platform binding reference required for character ownership, but:

- the identifier representation must be frozen by the identity contract;
- browser/client input never proves ownership;
- Game Session admission provides the authoritative account/character relationship for entry;
- cross-system deletion, transfer or rebinding requires explicit orchestration and audit.

### 6. Portal reads of game data

Public and account-authorized portal features should prefer:

```text
Oteryn Platform
→ versioned game query API or projection
→ oteryn_game
```

A direct database read path is allowed only when explicitly contracted and must use:

- a read-only principal;
- allowlisted views or selected columns;
- no access to credentials, sessions, lease internals, private inventory or anti-abuse data;
- defined freshness and availability semantics;
- no mutation-capable ORM model hidden behind the read path.

Candidate public projections include character profiles, guild directory data, highscores and world/channel status. Exact fields remain subject to public-data contracts.

### 7. Portal mutations of game data

Operations such as character creation, rename, deletion, transfer or account binding must be executed through a game/domain-owned API or explicit migration contract.

The Platform authenticates and authorizes the user, but the game domain remains responsible for gameplay invariants, transaction boundaries and all dependent rows.

The Platform must not create a character by issuing an uncoordinated `INSERT` into game tables.

### 8. Game persistence requirements

The detailed Persistence v1 contract must build on PostgreSQL and define at least:

- schema and migration tooling;
- typed identifier representation;
- optimistic revision and session-generation fencing;
- idempotency keys and duplicate-command handling;
- inventory/ground-item transfer transaction boundaries;
- row/advisory locking policy where required;
- transaction isolation and retry policy;
- transactional outbox for cross-module/system effects;
- append-only audit records for critical mutations;
- checkpoint and crash-recovery semantics;
- backup, point-in-time recovery, restore testing, RPO and RTO;
- compatible rollout and rollback order.

This ADR does not mandate full event sourcing. The initial direction remains current-state tables plus revisions, idempotent commands, transactional outbox and bounded append-only critical audit history.

### 9. Redis is non-authoritative

Redis may be introduced for:

- cache;
- rate limiting;
- presence projection;
- transient routing/readiness information;
- short-lived coordination where loss semantics are explicit.

Redis is not the sole source of truth for:

- characters or progression;
- inventory, currency or market settlement;
- durable character leases without a separately accepted safety proof;
- houses or reward eligibility;
- Platform wallet/payment balances.

A Redis restart or eviction must not create item duplication, revive stale sessions or lose committed financial/game state.

### 10. Migration of Oteryn Platform

The target is for Platform production persistence to use PostgreSQL in its separate `oteryn_platform` database.

That migration requires a dedicated `blakinio/Oteryn-Platform` task and PR covering:

- Laravel PostgreSQL compatibility;
- schema and data conversion;
- SQL dialect/index/locking differences;
- exact cutover and rollback;
- backups and restore evidence;
- compatibility with any temporary Canary/MariaDB integration;
- removal of obsolete assumptions only after migration is proven.

This Oteryn v2 ADR does not authorize writes to Platform or a live database.

## Consequences

### Positive

- PostgreSQL provides a strong foundation for concurrent transactional gameplay state;
- Platform and game failures, migrations and restores are isolated;
- each repository owns its schema and migrations;
- least-privilege credentials are straightforward;
- later physical separation does not require changing semantic ownership;
- accidental portal writes cannot silently bypass game invariants.

### Costs

- temporary operation may involve both MariaDB and PostgreSQL during migration;
- cross-system features require APIs, projections or outbox integration rather than convenient table joins;
- two databases require coordinated observability, backups and disaster-recovery testing;
- Platform PostgreSQL migration is additional work before production standardization.

## Rejected alternatives

### One shared database and schema for Platform and game

Rejected because ownership, migration authority, security and failure isolation would be ambiguous.

### Keep MariaDB as the greenfield game database

Rejected because compatibility with the legacy stack is not a target constraint and PostgreSQL better matches the planned concurrency, constraints and transactional model.

### Separate databases but one shared administrator credential

Rejected because database separation without credential isolation does not provide a meaningful security boundary.

### Store authoritative gameplay state in Redis

Rejected because eviction/restart and weaker durable semantics are unacceptable for characters, items, leases and financial/game settlement.

## Not performed by this ADR

- no database is provisioned;
- no schema or migration code is created;
- no Platform database is converted;
- no production credential or network rule is changed;
- no final isolation level, partition scheme, backup target or lease storage is selected.

## Required follow-up

1. Accept the detailed Persistence v1 contract.
2. Freeze identifier representations before schema creation.
3. Define game migrations, transactions, fencing, outbox and recovery.
4. Create a separate Platform PostgreSQL migration programme.
5. Define public/query projections and cross-system mutation APIs.
6. Prove backup/restore and failure-path behaviour before production readiness.
