# WP3 SQLx operation-owner propagation amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / PR #356. Protected prerequisite: #429.

## State

```yaml
allocation_id: OTV2-WP3-SQLX-OPERATION-OWNER-PROPAGATION-20260908
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 7508a72705ab6cba95a33dd59571eca3e94b91cb
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-sqlx-operation-owner-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: d2a71ac931f9407f720fa41c447690ce390b8cf5
source_wp3_tree: 44dbd2c6545f44718971204c6cbfe69256e5eeec
risk: HIGH
```

This is an allocation-only follow-up for the SAME canonical #351/#356 worker. It creates no replacement worker and grants no present source-mutation authority. Material application requires independent exact-head review, canonical exact-head checks, normal FULL Merge Queue, protected-main readback and a fresh Work custody/overlap check.

## Why this amendment is required

Protected #429 supplied the exact rustls owner-installation + ALPN/protocol custody authority. The canonical #356 worker then normally merged protected `main@7508a72705ab6cba95a33dd59571eca3e94b91cb` and stopped before semantic rustls or SQLx mutation at `d2a71ac931f9407f720fa41c447690ce390b8cf5`.

The preflight proved that the production SQLx PostgreSQL TLS call chain currently contains no accepted operation owner:

```text
PgConnectOptions::connect
-> PgConnection::establish
-> PgStream::connect
-> MaybeUpgradeTls
-> maybe_upgrade
-> sqlx_core::net::tls::handshake
-> tls_rustls::handshake
-> ClientConnection::new
```

`TlsConfig` currently carries only TLS policy/input fields, and PostgreSQL `maybe_upgrade` constructs it without any `ResourceBudget`. The only current `ResourceBudget` objects in #356 are focused-test/certificate-loader values. Creating a new ledger in TLS, using a global/thread-local registry, or falling back to ordinary unowned construction would violate the accepted same-ledger/no-fallback contract.

The accepted #351 implementation plan already resolves the ownership architecture: WP3 must **expose a driver reservation capability that the existing B writer can bind to the same executor/active-slot ledger**. WP3 must not mint B's ledger. After protected driver delivery, B/WP4 separately binds the capability to its actual active-slot owner under B's own allocation.

Therefore the smallest structurally correct fix is a separate **caller-supplied, call-scoped owner-aware PostgreSQL connection path**. The caller supplies the existing `Arc<dyn ResourceBudget>` explicitly. That exact Arc is propagated before TLS construction and retained for the lifetime of the owner-bound connection. Ordinary SQLx connection/pool behavior remains unchanged and owner-free.

## Owner source and lifecycle decision

The owner source for this WP3 capability is an explicit function argument supplied by the future B/WP4 composition (or by controlled WP3 tests). It is not derived from `PgConnectOptions`, a connection URL, TLS policy, a global, a runtime default or a newly created driver budget.

The owner lifetime is:

```text
caller-owned accepted ResourceBudget Arc
-> owner-aware PgConnection establish entry
-> owner-aware PgStream connect
-> owner-aware MaybeUpgradeTls/maybe_upgrade
-> owner-aware SQLx-core rustls handshake
-> protected #429 rustls ClientConnection owner-aware constructor
-> retained private PgStream/PgConnection owner identity
-> later already-owned driver/TLS accounting cells
-> connection drop, or a separately protected explicit charged-owner transfer
```

The owner-aware connection is operation-bound for this capability. It must not be silently returned to or created through the ordinary unowned `PgPool` / `ConnectOptions::connect` path. A future B composition may choose a legitimate registered shared/pool owner only through separate protected authority and proof; this amendment does not invent or authorize one.

Dropping the caller's original Arc while an owner-bound connection still exists must not release the connection's owner identity. The driver retains its own Arc until all backing attributed to that connection is destroyed or explicitly transferred under separately proven ownership.

## Exact additional authored lease after protected application

### 1. Existing SQLx-core accounting path: `vendor/sqlx-core-0.9.0/src/net/tls/mod.rs`

Authority is limited to adding a separate owner-aware TLS dispatch entry point/helper, for example `handshake_with_resource_budget`, that accepts the existing `TlsConfig`, `WithSocket` value and caller-supplied `Arc<dyn ResourceBudget>`.

Requirements:

- ordinary `TlsConfig` layout and ordinary `handshake(...)` behavior remain unchanged;
- do not add the operation owner as a persistent/default field on ordinary `TlsConfig`;
- the owner-aware entry point must dispatch only through the configured and qualified rustls path; an unsupported/unqualified runtime/TLS backend fails closed rather than discarding the owner;
- no TLS mode/version/certificate/hostname policy change;
- no new budget, numeric maximum, global owner or fallback.

### 2. Existing SQLx-core accounting path: `vendor/sqlx-core-0.9.0/src/net/tls/tls_rustls.rs`

Authority is limited to a resource-owner-aware handshake sibling/helper that receives the same caller-supplied Arc and composes already-protected SQLx/rustls ownership seams.

It may:

- construct/reuse the existing SQLx `BlockingJobOwner` from the same `ResourceBudget` for already-authorized certificate-loader custody;
- call the protected #429 owner-aware rustls client constructor using the same owner identity;
- retain the same owner for later already-authorized TLS custody cells as mechanically required.

Ordinary `handshake(...)` remains behaviorally unchanged and owner-free. Do not route owner-aware denial into ordinary `ClientConnection::new` or ordinary unowned certificate loading.

This amendment does **not** add new rustls session/cache, key-exchange, ECH/configuration or crypto authority. If completing the owner-aware TLS path reaches a still-unallocated rustls/config/crypto/session symbol, stop before mutation with exact `SHARED_LEASE_REQUIRED` evidence.

### 3. New PostgreSQL capability surface: `vendor/sqlx-postgres-0.9.0/src/connection/establish.rs`

Authority is limited to a separate owner-aware `PgConnection` establish/connect capability accepting `Arc<dyn ResourceBudget>` from the caller, plus minimum private factoring required to share the unchanged establishment logic.

Requirements:

- ordinary `PgConnection::establish(options)` remains unchanged and owner-free;
- the owner-aware sibling must never manufacture a budget;
- owner-aware and ordinary paths must preserve identical authentication/startup/database semantics except for resource ownership and fail-closed resource denial;
- the owner-aware path passes the exact Arc into the owner-aware stream/TLS path and preserves it through the resulting connection.

### 4. New PostgreSQL capability surface: `vendor/sqlx-postgres-0.9.0/src/connection/stream.rs`

Authority is limited to:

- a separate owner-aware `PgStream::connect` sibling/helper taking the same Arc;
- minimum private storage/accessor needed to retain the owner identity in an owner-bound `PgStream` / `PgConnection` through connection lifetime for later already-owned driver accounting;
- ordinary `PgStream::connect` continues with no owner and unchanged behavior.

The private owner field/custody must not itself reserve arbitrary bytes. It retains identity/capability; actual allocations debit through the existing `ResourceBudget` interface at their own authorized boundaries.

### 5. Existing PostgreSQL accounting path: `vendor/sqlx-postgres-0.9.0/src/connection/tls.rs`

Authority is limited to an owner-aware `MaybeUpgradeTls`/`maybe_upgrade` sibling or equivalent minimum factoring that receives the same Arc and invokes the owner-aware SQLx-core TLS entry point.

Preserve all current `PgSslMode` semantics and SSLRequest protocol behavior. In particular:

- `Allow`/`Disable` and a legitimate server refusal remain the same transport-policy outcomes as upstream;
- if TLS upgrade is selected/accepted, resource-owner denial or owner-aware TLS failure must not fall back to ordinary unowned TLS;
- no TLS downgrade, retry loop or authentication change;
- ordinary `MaybeUpgradeTls` / `maybe_upgrade` remain owner-free and unchanged.

### 6. PostgreSQL public binding surface: `vendor/sqlx-postgres-0.9.0/src/lib.rs`

Authority is limited to a doc-hidden or narrowly documented re-export of the **existing** `sqlx_core::net::resource_budget::ResourceBudget` trait, only if needed so the future B/WP4 consumer can implement/bind the existing owner interface through the already-used PostgreSQL/umbrella SQLx surface.

Do not define a second PostgreSQL owner trait, wrapper policy, numeric budget or resource architecture. If the existing umbrella `sqlx::postgres` re-export makes an additional lib change unnecessary, leave this file byte-identical.

### 7. Focused driver test surface

Within the already-owned `vendor/sqlx-postgres-0.9.0/**` subtree, the worker may add the previously planned focused resource-budget test module/file needed to prove this owner-aware connection propagation. The existing serialized `apps/game-server/tests/durability_postgres.rs` lease remains include-only and must not be used until the TLS gate is actually proven.

No Game/B runtime source is granted by this amendment.

## Explicit exclusions

No new authority is granted for:

- `PgConnectOptions` fields, URL parsing, ordinary `ConnectOptions::connect`, ordinary `PgPool` creation, pool policy or pooling an operation-bound owner connection;
- Game/B production source, Foundation, migrations, registry, workflows, rulesets, deployment, Platform or external repositories;
- new root dependency declarations solely to expose the owner interface;
- a new resource ledger/trait, per-connection default budget, hidden pool allowance or whole-slot reservation;
- session/cache, key-exchange, ECH/configuration, verifier/crypto internals or any other rustls path not already protected;
- substantial PostgreSQL decoder/cache accounting until the complete TLS gate is proven;
- WP4/WP5/G0/Server Seam activation.

If implementation proves another SQLx path/symbol is materially required just to propagate the caller-supplied owner from the explicit connection entry to `tls_rustls::handshake`, stop before mutation with exact `SHARED_LEASE_REQUIRED = path :: symbol :: reason`. Do not silently place the owner in `PgConnectOptions` or widen ordinary pool behavior.

## Required RED/GREEN and lifetime proof

Before resuming #429 ALPN work, prove at minimum:

1. **RED**: the owner-aware PostgreSQL/TLS call cannot compile or cannot reach the TLS constructor before this propagation exists; no test-only global/thread-local injection is accepted.
2. **GREEN identity**: one caller-supplied Arc reaches SQLx-core TLS and the protected rustls owner-aware constructor as the same owner identity; no newly constructed ledger appears.
3. **Retention**: dropping the caller's original Arc after owner-aware connection creation does not destroy/release the retained connection owner; connection drop releases only after all charged connection-owned backing is gone.
4. **Denial**: an unfunded owner fails before the first newly authorized owner-aware ALPN/TLS allocation and does not retry through ordinary unowned TLS/connect.
5. **Ordinary control**: existing `ConnectOptions::connect`, `PgConnection::establish`, `PgStream::connect`, `MaybeUpgradeTls`, SQLx-core ordinary TLS handshake and ordinary rustls client construction remain behaviorally unchanged and owner-free.
6. **Transport policy**: SSL mode selection, server S/N response, TLS version/certificate/hostname verification and protocol bytes remain unchanged.
7. **No pool leak**: the owner-aware capability has no implicit ordinary-pool path and no owner stored in reusable `PgConnectOptions`.

After these propagation controls pass, continue the protected #429 owner-installation + ALPN/protocol RED/GREEN using the same Arc. Only if that seam is GREEN may the worker proceed through already-protected #427 ClientHello and #425 decoded-owner surfaces. Session/cache and configuration/crypto remain separate OPEN cells.

## Acceptance boundary after this amendment

This amendment only makes the existing operation owner reachable; it does not prove complete TLS or WP3.

Remaining mandatory cells after propagation include:

- #429 owner-aware ALPN/protocol construction and custody;
- #427 ClientHello emit and #425 decoded-owner RED/GREEN;
- session/cache ownership;
- configuration/key-exchange/ECH/crypto ownership;
- complete TLS phase/lifetime composition and one feasible funded positive;
- actual TLS-positive integration evidence;
- actual final-candidate PostgreSQL 17.6 driver test under protected #422 classifier;
- independent high-risk whole-diff review;
- canonical exact-head CI, normal FULL Merge Queue and protected-main readback.

Only terminal/protected WP3 releases WP4 #335. WP5 and Server Seam #247 remain frozen.

## Integration lifecycle

```text
this allocation-only amendment
-> independent exact-head review
-> canonical exact-head checks
-> normal FULL Merge Queue
-> protected-main readback
-> fresh Work custody check
-> apply to SAME #351/#356 worker
-> owner-propagation RED/GREEN
-> protected #429 ALPN owner seam
-> protected #427/#425 ClientHello + decoded owner cells
-> remaining session/config/crypto + complete TLS
-> actual TLS-positive + PostgreSQL17.6 evidence
-> whole-diff review + canonical CI/MQ
-> protected WP3 release
-> only then WP4 #335 resume
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
