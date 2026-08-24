# Oteryn Game — First Durability Topology Decision Packet

- Date: 2026-08-24
- Issue: `#94`
- Preparation task: `OTV2-20260824-prep-durability-topology`
- Allocation PR: `#118`
- Allocation merge: `58459c275ba62714741e6794b92d8935b140a37c`
- Exact preparation base: `main@58459c275ba62714741e6794b92d8935b140a37c`
- Mode: `PREPARATION / DECISION PACKET`
- Runtime/DDL/migration/dependency/Cargo/registry/workflow/production authority: **NONE**
- Topology decision: **FROZEN FOR A LATER EXACT IMPLEMENTATION ALLOCATION**
- Implementation release verdict: **BLOCKED_ON_OWNER_DECISION**

## 1. Purpose and terminal handoff

This packet completes the Issue #94 preparation gate. It selects one first-increment Rust/PostgreSQL topology, one migration/client stack, one game-owned migration ledger, one runtime-to-durable handoff shape, one isolated test-database strategy and the exact paths/serialized leases that a later implementation allocation would need.

It does **not** grant `OTV2-IMPL-DURABILITY` write authority. The implementation lane remains blocked because accepted DUR-03 requires finite hard ceilings for amplification-prone durable transaction resources and the current registry contains no DUR-03-owned entries. The worker does not invent those numbers.

Final handoff from this preparation task is therefore:

```text
TOPOLOGY_PREPARATION = COMPLETE
TOPOLOGY_DECISION = FROZEN
OTV2-IMPL-DURABILITY = BLOCKED_ON_OWNER_DECISION
```

## 2. Verified base and governing facts

`PROVEN`: `docs/architecture/FOUNDATION_PROGRAMME_CURRENT_STATUS.md` classifies FND-03/FND-04/GAME-CHAR/GAME-ITEM as implemented and DUR-01/DUR-02/DUR-03/ANL-01 as accepted, lifecycle-closed and not implemented.

`PROVEN`: ADR-0004 selects PostgreSQL and a separate game-owned logical database `oteryn_game`.

`PROVEN`: DUR-01 requires PostgreSQL native `uuid` for accepted UUIDv7 identities and `numeric(20,0)` for persisted full-range FND-02 `CommandId` values.

`PROVEN`: DUR-02 requires one authoritative immutable game migration history, dedicated migration execution, production runtime credentials without routine DDL, startup compatibility checks that fail closed, database-visible single-winner migration exclusion, durable acknowledgement, crash-safe audit/outbox coupling and evidence-driven isolation/locking.

`PROVEN`: DUR-03 requires stable TransactionId/OperationId semantics, one PostgreSQL durable linearization point, PREPARE -> durable COMMIT/CLASSIFY -> RECONCILE, ambiguous-outcome classification and bounded atomic participants/effects.

`PROVEN`: ANL-01 defines EventId, OperationId and TransactionId as strong UUIDv7 identities and applies DUR-01 PostgreSQL `uuid` representation when stored in the game database.

`PROVEN`: root `Cargo.toml` uses Rust `1.94`; `apps/game-server/Cargo.toml` currently has no PostgreSQL client/migration dependency; current runtime composition is game-server-local.

`PROVEN`: current `workspace-boundaries.toml` forbids package-name fragment `persistence`; there is no demonstrated immediate consumer outside `apps/game-server` that requires a new persistence crate.

`PROVEN`: `RESOURCE_LIMITS_REGISTRY.json` has no `DUR03-*` entry. The separately completed Issue #93 packet covers Ability/Interaction/AI/Movement semantic-work limits and explicitly does not authorize arbitrary reuse of Foundation envelopes as other semantic limits.

## 3. First-increment topology decision

### Decision

Use a **game-server-local Durability module**, not a new workspace crate:

```text
apps/game-server
├── src/durability/**
├── src/bin/oteryn-game-migrate.rs
├── migrations/**
└── tests/** Durability PostgreSQL integration support
```

### Why no dedicated crate

A new crate would introduce workspace topology and policy mutation without a concrete second immediate consumer. The accepted Durability semantics are consumed first by the game-server Foundation/Domain seams, so a local module is the smallest architecture that preserves ownership without speculative abstraction.

A future extraction requires evidence of a real second immediate consumer or a separately accepted service/database authority boundary. It is not an implementation-worker convenience decision.

## 4. Exact proposed implementation paths

A later `OTV2-IMPL-DURABILITY` allocation should own only the paths it actually exercises from this set.

### Runtime-facing Durability module

```text
apps/game-server/src/durability/mod.rs
apps/game-server/src/durability/db.rs
apps/game-server/src/durability/schema.rs
apps/game-server/src/durability/admission_journal.rs
apps/game-server/src/durability/transactions.rs
apps/game-server/src/durability/reconcile.rs
apps/game-server/src/durability/audit_outbox.rs
```

Responsibilities are fixed as follows:

- `mod.rs` — public game-server-local Durability boundary and typed error/result surface;
- `db.rs` — `PgPool` construction, least-privilege runtime connection configuration and typed SQLx conversion boundary;
- `schema.rs` — embedded migration set plus **read-only** runtime schema/migration compatibility validation; no startup DDL;
- `admission_journal.rs` — concrete durable adapter for the existing FND admission/reconnect journal seam and current authority/fence reconstruction;
- `transactions.rs` — TransactionId/OperationId receipts, idempotency, transaction classification and atomic mutation/evidence coordinator primitives;
- `reconcile.rs` — known-commit / known-abort / ambiguous result classification and restart-safe reconciliation;
- `audit_outbox.rs` — common ANL-compatible immutable durable event + crash-safe publication state written in the same owning PostgreSQL transaction where required.

### Migration execution and ledger

```text
apps/game-server/src/bin/oteryn-game-migrate.rs
apps/game-server/migrations/**
apps/game-server/build.rs
```

`apps/game-server/migrations/**` is the **only authoritative game migration source/history** for `oteryn_game` in this first boundary. Migration versions are positive monotonically increasing SQLx versions and merged/released migration contents are immutable.

`apps/game-server/build.rs` exists only to make stable Rust rebuild when the embedded migrations directory changes, as required by SQLx `migrate!()` behavior on stable Rust.

### DB integration tests

```text
apps/game-server/tests/durability_postgres.rs
apps/game-server/tests/support/postgres.rs
```

The implementation may split the test file by responsibility only if the allocation enumerates those additional paths before write; it may not move test ownership into another crate by convenience.

## 5. Selected Rust PostgreSQL/migration stack

### Selected: SQLx 0.9.0

Freeze this dependency shape for the first increment:

```toml
sqlx = {
  version = "=0.9.0",
  default-features = false,
  features = [
    "runtime-tokio",
    "tls-rustls-ring-webpki",
    "postgres",
    "migrate",
    "macros",
    "uuid",
  ],
}
```

`PROVEN` external compatibility evidence:

- SQLx `0.9.0` declares `rust-version = "1.94.0"`, exactly matching the workspace Rust floor;
- SQLx provides Tokio runtime support, PostgreSQL, built-in connection pooling, migrations and PostgreSQL UUID integration;
- `migrate!()` requires `macros` + `migrate` and embeds the resolved migration set into the binary;
- SQLx migration state records applied checksums and reports `VersionMismatch` when an already-applied migration differs from the current source;
- SQLx `Migrator` defaults migration locking to `true`, and PostgreSQL migration locking is database-visible; disabling locking is forbidden for this allocation;
- SQLx documentation warns that CRLF/LF changes alter migration hashes and recommends forcing `.sql` checkout to LF or an equivalent explicit hashing policy.

Primary source evidence:

- `https://docs.rs/crate/sqlx/0.9.0/source/Cargo.toml.orig`
- `https://docs.rs/sqlx/0.9.0/sqlx/macro.migrate.html`
- `https://docs.rs/sqlx/0.9.0/sqlx/migrate/enum.MigrateError.html`
- `https://docs.rs/crate/sqlx-core/0.9.0/source/src/migrate/migrator.rs`

### Viable alternative considered: tokio-postgres stack

`tokio-postgres 0.7.18` is an async PostgreSQL client and declares Rust `1.85`, so Rust 1.94 compatibility is not the blocker. A production-equivalent stack would additionally require a pool such as `deadpool-postgres` and a separate migration solution such as `refinery` or custom migration code.

Primary evidence:

- `https://docs.rs/crate/tokio-postgres/0.7.18/source/Cargo.toml.orig`
- `https://docs.rs/deadpool-postgres/latest/deadpool_postgres/`
- `https://docs.rs/refinery/latest/refinery/`

It is **rejected for the first increment** because it introduces separate client, pool and migration lifecycle surfaces while SQLx supplies one compatible stack and one migration error/checksum/locking model. The rejection is about first-increment complexity and supply-chain surface, not correctness impossibility.

### Supply-chain posture

SQLx is crates.io-published and MIT OR Apache-2.0, both allowed by current `deny.toml`. The later implementation PR must still regenerate `Cargo.lock` and pass exact-head `cargo-deny`; this packet does not pre-approve transitive dependency state that has not yet been resolved by Cargo 1.94.

## 6. Migration ledger and immutable-history mechanics

The implementation uses SQLx `migrate!()` over `apps/game-server/migrations` for one embedded migration set shared by the dedicated migration executor and read-only runtime compatibility logic.

Binding rules:

1. one migration version maps to one immutable SQL artifact;
2. a merged/released migration is never edited, removed, renumbered or silently reinterpreted;
3. correction is a new forward migration unless a separately accepted recovery plan explicitly requires another mechanism;
4. SQLx checksum/version mismatch, missing applied migration, dirty state or unsupported ahead/behind state fails closed;
5. migration locking remains enabled;
6. the dedicated migration binary may execute migrations using the migration principal;
7. ordinary game-server startup never calls migration `run`, `undo`, `skip`, force/drop or any authoritative DDL path;
8. production runtime credentials have only the DML/read permissions required by accepted runtime operations;
9. migration history remains Game-owned; Platform never becomes a second migration writer.

To make checksum identity platform-stable, the later implementation allocation must include root `.gitattributes` with:

```gitattributes
*.sql text eol=lf
```

The allocation must also include `apps/game-server/build.rs` with a stable-Rust `cargo:rerun-if-changed=migrations` directive so newly added migrations cannot be silently omitted from a rebuilt embedded migration set.

## 7. Dedicated migration execution and startup compatibility

The dedicated command is the separate Cargo binary:

```text
oteryn-game-migrate
```

It is the only first-increment application binary allowed to call SQLx migration execution APIs. It uses a migration-only database credential supplied outside the repository.

Adding a second package binary requires `apps/game-server/Cargo.toml` to set:

```toml
default-run = "oteryn-game-server"
```

so existing `cargo run -p oteryn-game-server -- --smoke` behavior remains unambiguous.

Ordinary `oteryn-game-server` startup performs a read-only compatibility check before enabling any Durability-backed capability. Unsupported migration state returns a stable fail-closed startup/dependency error. It does not repair schema state and never escalates to migration credentials.

## 8. Runtime async boundary

The runtime/durable boundary is frozen as:

```text
FND-03 logical writer
    -> PREPARE / validate / reserve under current fences
    -> enqueue one bounded persistence request
    -> yield writer lane

async Durability worker / PgPool
    -> DB transaction
    -> COMMIT or known ABORT or AMBIGUOUS classification

new normalized authoritative completion input
    -> revalidate current authority/fences
    -> RECONCILE runtime projection
```

Binding rules:

- no database connect/acquire/query/transaction/network wait is performed while the FND-03 logical writer lane is held;
- PREPARE is not durable success;
- a durable PostgreSQL commit is the first-increment durable linearization point;
- a stale completion from an old runtime ownership generation cannot mutate current runtime authority;
- a committed durable result outranks stale runtime checkpoint/projection state;
- runtime memory and PostgreSQL are not distributed 2PC participants; reservation + fencing + one durable commit + idempotent reconciliation provide safety.

## 9. TransactionId / OperationId and ambiguous commit

ANL-01 and DUR-03 remain authoritative:

- `TransactionId` is one UUIDv7 per logical atomic durable mutation and is allocated before the first commit attempt;
- physical retry never creates a new TransactionId for the same logical intent;
- `OperationId` is used when one logical workflow spans multiple durable transactions or resumes asynchronously/across processes or GameSessions;
- `CommandRef` remains distinct ingress identity;
- the same TransactionId with conflicting intent is an integrity conflict;
- planned output identities remain stable across physical retry;
- ambiguous commit freezes the materialized candidate mutation/evidence/output set until durable state classifies it.

The durable receipt state must distinguish at least:

```text
NOT_APPLIED
COMMITTED
TERMINAL_REJECTED (when persisted by owning domain)
AMBIGUOUS
CONFLICT
```

An unclassifiable result fails/holds. It never guesses a second TransactionId.

## 10. Audit/outbox atomicity

When the owning contract requires durable evidence, the same PostgreSQL transaction commits:

```text
authoritative mutation
+ revision / receipt / idempotency state
+ immutable ANL event evidence
+ crash-safe publication enqueue/checkpoint state
```

Publication remains at-least-once with stable EventId and exact immutable semantic content. Publisher retry never replays the gameplay mutation. Best-effort telemetry is not part of the mandatory transaction.

## 11. Isolated non-production PostgreSQL test lifecycle

The later implementation must add a real PostgreSQL DB-E2E lane; mocks alone cannot close Durability.

Required lifecycle:

1. CI/local test receives a **non-production admin URL** through test-only environment configuration; no credential is committed;
2. test support creates a uniquely named database using prefix `oteryn_game_test_` and a run-unique suffix;
3. migration executor applies the exact embedded game migration ledger using a migration test role;
4. runtime tests connect through a separate runtime-role credential that lacks routine DDL;
5. tests exercise migration compatibility, migration interruption/locking, authority fencing, lost response, ambiguous commit, crash/restart reconstruction and audit/outbox behavior;
6. test cleanup drops the unique test database through the admin path even after ordinary test failure where cleanup is still possible;
7. any URL/database name that does not satisfy the explicit non-production test guard is rejected before create/drop/migrate.

The PostgreSQL service image/tag/digest is an OPS/test-environment selection and is **not** promoted to production architecture by this packet. Before the implementation PR claims DB-E2E reproducibility, that PR must pin the exact non-production CI image/digest it actually tested.

## 12. Exact serialized shared-path lease required later

The later implementation cannot self-assign these paths. One coordinator lease must explicitly include the exercised subset:

```text
Cargo.toml
Cargo.lock
apps/game-server/Cargo.toml
apps/game-server/src/lib.rs
.github/workflows/rust.yml
.gitattributes
```

Reasons:

- root `Cargo.toml` — exact SQLx workspace dependency;
- `Cargo.lock` — resolved supply-chain state;
- `apps/game-server/Cargo.toml` — SQLx consumption + `default-run`;
- `apps/game-server/src/lib.rs` — compose `pub mod durability`;
- `.github/workflows/rust.yml` — real isolated PostgreSQL DB-E2E lane/service;
- `.gitattributes` — stable SQL migration bytes/checksums across Windows/Linux.

`workspace-boundaries.toml` is **not required** for this topology because no new crate/member/edge is introduced.

`apps/game-server/src/main.rs` is **not required** for the first increment because the migration executor is a separate `src/bin` binary and normal server startup remains fail-closed except for separately allocated server-seam work.

## 13. DUR-03 hard-max closure

Current classification follows the same discipline as the completed #93 packet: because no exact Durability implementation child plan has yet merged, a required semantic resource cannot be declared `NOT_APPLICABLE_TO_FIRST_SLICE` merely by this preparation worker.

| ID | DUR-03 dimension | Classification | Unit to register | Why it blocks |
|---|---|---|---|---|
| `DUR03-RL-01` | touched ItemInstances per logical transaction | `OWNER_DECISION_REQUIRED` | ItemInstances / transaction | bounds participant/effect materialization and locking |
| `DUR03-RL-02` | immediate location/custody lines | `OWNER_DECISION_REQUIRED` | location/custody lines / transaction | bounds location/custody fan-out and conservation proof |
| `DUR03-RL-03` | value/account lines | `OWNER_DECISION_REQUIRED` | value lines / transaction | bounds non-item/value conservation work |
| `DUR03-RL-04` | transform inputs + outputs | `OWNER_DECISION_REQUIRED` | transform participants/outputs / transaction | bounds type-changing/multi-output amplification |
| `DUR03-RL-05` | container expansion/reachable work exercised during commit | `OWNER_DECISION_REQUIRED` | items/edges or deterministic work units / transaction | bounds recursive containment/capacity validation |
| `DUR03-RL-06` | workflow participants/custody effects | `OWNER_DECISION_REQUIRED` | participants/effects / transaction or workflow step | bounds multi-party/custody fan-out |
| `DUR03-RL-07` | mandatory audit event/payload contribution | `OWNER_DECISION_REQUIRED` | events and aggregate bytes / transaction | ANL envelope byte limits do not define DUR-03 semantic event-count/amplification limits |
| `DUR03-RL-08` | retry/reconciliation work | `OWNER_DECISION_REQUIRED` | attempts or deterministic work units / transaction/window | bounds deadlock/serialization/ambiguous-outcome recovery amplification |

For each row, the owner/serialized registry task must define an absolute hard maximum, failure behavior, allocation impact, client visibility and max/max+1 boundary tests. Generic FND/ANL transport/event envelope bounds may be inherited only when the resource is provably the exact same resource; they are not substitute numbers for these semantic dimensions.

### Blocker routing

Issue #93 is not blanket authority for DUR-03. The coordinator should open/use a **Durability-specific owner decision task** and then a serialized `RESOURCE_LIMITS_REGISTRY.json` mutation after owner acceptance. A later exact Durability child plan may explicitly exclude individual rows fail-closed; only then may those rows become `NOT_APPLICABLE_TO_FIRST_SLICE`.

Until every exercised row is `REGISTERED_EXACT` or explicitly excluded fail-closed by the exact implementation child plan:

```text
OTV2-IMPL-DURABILITY.write_authority = none
```

## 14. Required implementation tests and review

The future implementation allocation must require at least:

- UUIDv7/native-`uuid` and full-range CommandId round trips;
- migration fresh/up/compatibility, immutable-checksum mismatch and concurrent migration single-winner behavior;
- runtime-role DDL-negative tests and proof normal startup does not run migrations;
- schema ahead/behind/missing/changed/dirty fail-closed tests;
- isolated test database create/migrate/run/cleanup and non-production guard negatives;
- fresh admission/reconnect durable journal restart/fence tests;
- stale CharacterLease/runtime-scope/transaction precondition rejection;
- same TransactionId retry after known abort;
- lost-response retry returning original committed result;
- ambiguous-commit reconciliation without duplicate mutation;
- audit/outbox atomicity and restart-safe at-least-once publication evidence;
- item/value conservation tests only for operation classes actually enabled by the exact first slice and only after their hard maxima are accepted;
- PostgreSQL dependency loss/restart behavior;
- exact full workspace CI + `cargo-deny`;
- genuinely independent exact-head review because persistence, fencing, items/value and durable schema are high-risk surfaces.

A mock DB result is not terminal DB-E2E evidence.

## 15. Exact implementation allocation proposal

After the DUR-03 hard-max gate is closed, the coordinator may prepare `Oteryn: impl durability` only with an exact child Superpowers plan and a merged allocation that names:

```yaml
lane_id: OTV2-IMPL-DURABILITY
base_sha: <exact then-current main SHA recorded by the coordinator>
primary_owned_paths:
  - apps/game-server/src/durability/**
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/migrations/**
  - apps/game-server/build.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
serialized_shared_lease:
  - Cargo.toml
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/lib.rs
  - .github/workflows/rust.yml
  - .gitattributes
explicitly_not_owned:
  - workspace-boundaries.toml
  - apps/game-server/src/main.rs
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
production_database_authority: none
platform_external_repository_authority: none
independent_exact_head_review: required
real_postgresql_db_e2e: required
```

The literal `<exact then-current main SHA recorded by the coordinator>` above is an allocation schema field, not a placeholder value for this packet: repository policy requires that SHA to be taken from GitHub at the future allocation event and forbids pre-selecting a stale future SHA now.

## 16. Final disposition

### Preparation result

`PASS`: topology, library/migration approach, migration ownership, async boundary, reconciliation semantics, test topology and future shared-path lease are explicit.

### Implementation-release result

`BLOCKED_ON_OWNER_DECISION`: DUR-03 numeric hard ceilings are absent and may not be invented by the implementation worker.

### Safe next action

Create/complete a Durability-specific hard-max owner decision + serialized registry task. Once every resource exercised by the exact first implementation child plan is `REGISTERED_EXACT` or explicitly excluded fail-closed, the coordinator may create the exact `OTV2-IMPL-DURABILITY` allocation described above.
