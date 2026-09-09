# WP5 native admission source-ingestion / bootstrap allocation

Coordinator: #162. Source readiness: #319. Programme: #364.

## Status

Prospective Game-only source-readiness allocation. **NOT_ACTIVE**.

This document prepares exact future ownership and activation gates for the missing
Game-side authenticated Platform security/trust source ingestion. It creates no
runtime, SQL/migration, workflow, Foundation-consumer, production, Platform or
external-repository write authority. It does not release #247 or change custody of
#351/#356, #335, the current Foundation workers, Cargo/workspace files or any
existing shared composition path.

```yaml
allocation_id: OTV2-WP5-NATIVE-ADMISSION-SOURCE-INGESTION-20260909
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
allocation_base_main_sha: b26395edff3dde1ebcc155ab70758520d780884c
allocation_state: NOT_ACTIVE
worker_launch: NONE_UNTIL_EXPLICIT_WORK_APPLICATION
accepted_source_contract: FND-NATIVE-SOURCE-EVIDENCE-V1
accepted_recovery_contract: FND-RECOVERY-SOURCE-TRANSPORT-V2
resource_envelope: NATIVE-SOURCE-RESOURCE-ENVELOPE-V1
wire_codec_delivery: "#346/#349 protected and released"
character_source_allocation: "#414 protected; NOT_ACTIVE"
scope_assignment_allocation: "#415 protected; NOT_ACTIVE"
dedicated_postgres_routing_allocation: "#416 protected; NOT_ACTIVE"
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
server_seam_authority: FORBIDDEN
```

## Fresh live source evidence

The preparation readback used protected Game
`main@b26395edff3dde1ebcc155ab70758520d780884c` and read-only Platform
`main@5b272a8246ebb050a5b0522d2dce86e7ef3c57ce`.

### Game

- `apps/game-server/src/admission_evidence.rs` already contains the bounded inert
  request/response codec for all four accepted operations and is terminally
  delivered by #346/#349. It intentionally authenticates no peer, installs no
  descriptor, persists no source floor and constructs no Foundation authority.
- `apps/game-server/src/foundation/fnd04_verifier.rs` exposes the sealed consumer
  capability `Fnd04EvidenceAuthority`; that interface is a consumer boundary, not
  an owning producer.
- #414, #415 and #416 already protect prospective Character Authority,
  Channel-scope assignment and dedicated PostgreSQL CI-routing allocations. They
  remain `NOT_ACTIVE` and are not duplicated here.

### Platform — read only

At the exact Platform revision above:

- `app/GameAuth/Tickets/RedeemedGameLoginTicket.php` returns integer
  `identityId` / `canaryAccountId`, security generation and redeem time;
- `app/GameAuth/Tickets/RedeemGameLoginTicket.php` transactionally locks the
  ticket, Identity and Canary binding and rejects disabled/generation-mismatched
  state before consuming the ticket;
- `app/Identity/Actions/RevokeIdentityGameAuthorizations.php` transactionally
  increments `game_auth_generation` and revokes native OAuth bindings;
- `routes/internal.php` exposes ticket redeem and Canary-account login-context
  routes only;
- default-branch code search finds no `ReadAccountSecurityV1`,
  `ReadFreshSigningTrustV1`, `ReadRecoveryAccountSecurityV2` or
  `ReadRecoverySigningTrustV2` implementation.

Therefore the current external classification remains
`MISSING_CROSS_REPOSITORY_PLATFORM_PRODUCER`, not `WIRE_EXISTING_SOURCE`.
Platform has useful current security/revocation state, but the inspected protected
revision does not expose the independently current native AccountId/security/trust
producer required by the accepted Game contract. This is source evidence only,
not permission to modify Platform.

## Non-reopening rules

This allocation consumes, without changing:

- `FND-NATIVE-SOURCE-EVIDENCE-V1` selected fixed-descriptor HTTPS/TLS 1.3 mutual
  service authentication, strict typed request/response binding, source ordering,
  conservative freshness and fail-closed bootstrap;
- `FND-RECOVERY-SOURCE-TRANSPORT-V2` for the recovery operation family;
- `NATIVE-SOURCE-RESOURCE-ENVELOPE-V1`, including its already registered `NSRC-*`
  resource ceilings;
- the existing five-second accepted evidence-age boundary;
- #346/#349 wire encoding/decoding behavior.

No worker may substitute bearer grants, directory/catalogue data, caller-created
facts, prior receipts, expected fences, a local clock refresh, direct Platform DB
reads, endpoint discovery or trust-on-first-use for authenticated source evidence.

## Bounded staged allocations

The stages below are serial ownership packets, not concurrent active leases. Work
must refresh protected main, active PRs/tasks and exact custody before activating
any one of them.

### S1 — authenticated transport and descriptor validation

Purpose: turn the already-protected inert wire codec into an authenticated,
bounded private source client without granting Foundation owning authority.

Prospective implementation paths:

```text
apps/game-server/src/native_admission_source/mod.rs               # new
apps/game-server/src/native_admission_source/descriptor.rs        # new
apps/game-server/src/native_admission_source/http1_mtls.rs        # new
apps/game-server/tests/native_admission_source_transport.rs        # new
```

Read-only dependencies:

```text
apps/game-server/src/admission_evidence.rs
apps/game-server/src/foundation/fnd04_verifier.rs
apps/game-server/src/foundation/mod.rs
```

S1 may not edit those read-only dependencies. A later serialized export/composition
lease for `apps/game-server/src/lib.rs` is required before S1 can become production
reachable; this document deliberately does **not** allocate that shared path while
Server Seam/shared composition custody is unresolved.

S1 must enforce the registered source profile exactly:

- one configured producer descriptor and four fixed operation mappings;
- HTTPS only, TLS 1.3 mutual authentication, HTTP/1.1 only;
- no redirects, proxy/discovery, HTTP/2, compression, AIA/CRL network fetch,
  automatic retries or hidden connection pool;
- exact authenticated service identity and exact configured Game client identity;
- `NSRC-HTTP-HEADERS`, `NSRC-HTTP-BODY`, `NSRC-HTTP-CHUNKS`,
  `NSRC-JSON-SHAPE`, `NSRC-SOURCE-AUTHORITY`, `NSRC-DESCRIPTOR`,
  `NSRC-TLS-CERTS`, `NSRC-TLS-VERIFY`, `NSRC-INFLIGHT`, `NSRC-QUEUE`,
  `NSRC-ACTIVE-BUFFERS`, `NSRC-PENDING-PUBLICATION`, `NSRC-PROJECTION`,
  queue/connect/handshake/exchange/publish deadlines and all aggregate accounting;
- capacity is owned end-to-end through publication/reconciliation; timeout or
  cancellation cannot detach uncounted work or make a slot reusable early.

S1 activation prerequisites:

1. a separately authorized compatible producer or controlled independent test
   producer exists for authenticated interoperability proof;
2. selected TLS/HTTP implementation can demonstrably enforce every accepted
   certificate/parser/byte/work bound before hostile work exceeds the budget;
3. required Cargo/workspace and `lib.rs` shared paths have explicit serialized
   Work leases and no #351/#356, Server Seam or other active-owner overlap;
4. no production endpoint, certificate, key or secret is selected by this packet.

If prerequisite 2 cannot be proven with the current dependency graph, return a
bounded implementation finding; do not weaken `NSRC-*` or silently accept an
unbounded lower layer.

### S2 — durable descriptor / source-floor / ambiguity bootstrap

Purpose: persist authenticated descriptor revision, accepted source high-water,
fixed pending-slot identities and restart reconciliation needed for fail-closed
non-rollback source registration.

Prospective new owner/test paths after WP4 release:

```text
apps/game-server/src/durability/native_admission_source.rs
apps/game-server/tests/native_admission_source_postgres.rs
apps/game-server/migrations/<NEXT>_native_admission_source.sql
```

Conditional serialized hook paths, **not currently leased**:

```text
apps/game-server/src/durability/mod.rs
apps/game-server/src/durability/db.rs
apps/game-server/src/durability/schema.rs
```

S2 activation prerequisites:

1. WP3 PostgreSQL driver qualification is terminal/protected and its shared Cargo /
   test-target custody released;
2. WP4/#335 is terminal/protected and Work has read back the then-current schema;
3. Work selects `<NEXT>` from that protected schema and proves no migration/path
   overlap with #415 or any other durability writer;
4. #416's dedicated PostgreSQL routing prerequisite is protected and materially
   activated for this exact target before hosted PG evidence counts;
5. S1 exact descriptor/wire semantics are protected or a serialized predecessor.

S2 semantics:

- absence of a row is not proof of a fresh store;
- first initialization requires an independently authorized fresh-store provenance
  record;
- descriptor revision, source revision/decision floors and fixed slot custody are
  monotonic/nonrollback and survive restart;
- equal source revision is exact semantic replay only and never refreshes time;
- lower/unknown revision, missing/contradictory checkpoint or unprovable restored
  high-water fails closed;
- an ambiguous publication owns its original slot/operation until authoritative
  reconciliation; restart cannot mint replacement capacity;
- durable historical floors/denials/receipts are not deleted to meet an in-memory
  cache limit.

No Platform schema, production DB, B/#335 migration or existing #415 assignment
schema is editable under S2.

### S3 — sealed Foundation publication / consumer composition

Purpose: bind authenticated S1 observations plus S2 durable non-rollback state to
the existing sealed Foundation evidence/publication seams.

No paths are allocated now. Candidate shared surfaces include
`foundation/fnd04_verifier.rs`, publication/recovery seams and the game-server
composition root, all of which require a fresh exact readback and sole-writer
lease after their current owners are released.

S3 cannot activate until S1 + S2 are protected, the compatible Platform producer
is independently current, and WP4 is protected. It must not be smuggled into
#351/#356, #335, #414/#415 producer work or Server Seam.

S3 qualification must prove at least:

- wrong peer/root/client scope and wrong operation/account/key/purpose reject;
- malformed/duplicate/oversized JSON and numeric/identity overflow reject;
- lower revision and changed equal-revision replay reject without re-aging;
- expired/future/uncertain source time fails closed;
- newer denial/untrust fences admission before stale allow/trust can be adopted;
- lost response, source restart, Game restart and restored-backup uncertainty
  preserve/reconcile high-water without namespace reset;
- actual authenticated transport + real PostgreSQL publication are exercised;
- Foundation lookup remains nonblocking on HTTP/SQL and observes only acknowledged
  sealed projection state.

Mocks/fixtures may prove local behavior but never actual producer availability.

## External counterpart dependency — no Platform allocation

The smallest still-missing external package must be implemented under separate
explicit `Oteryn/Oteryn-Platform` authority. This Game packet requests only the
accepted counterpart behavior:

1. canonical immutable native `AccountId` subject binding;
2. independently authenticated current account/security observations for fresh
   and recovery operations with durable monotonic observation revision,
   decision identity, authoritative observation time/uncertainty and exact replay;
3. current signing/trust-key observations for fresh and recovery fixed scopes,
   including durable profile/key rotation/revocation ordering;
4. all relevant disable/revoke/generation/trust mutations serialized into the
   source ordering so a post-issuance security change can invalidate admission;
5. exact version/operation/source/request binding and fail-closed not-found /
   unavailable / unauthorized / unsupported outcomes;
6. compatible mutual-TLS service identity/client authorization and durable
   producer-side non-rollback/bootstrap behavior;
7. exact cross-repository golden + authenticated interoperability evidence on the
   final producer and consumer revisions.

No Platform branch, Issue, PR, route, schema, service, credential, certificate or
production change is authorized here.

## Relationship to other WP5 allocations

- **#414 Character Authority** owns canonical AccountId <-> CharacterId/world and
  lifecycle bootstrap. An `allowed` security observation or AccountId alone may
  not mint character ownership.
- **#415 Channel assignment** owns Game runtime Channel assignment/fencing. A
  directory, grant or receiving node cannot self-assign.
- **#416 PostgreSQL routing** is the control prerequisite for dedicated WP5 PG
  targets; its workflow/policy paths remain separately serialized and are not
  edited here.
- this allocation owns only authenticated Platform security/trust ingestion,
  nonrollback source bootstrap and later sealed consumer composition.

WP5 readiness requires all relevant source owners to compose; none substitutes
for another.

## Activation / integration order

```text
this allocation protected
-> keep S1/S2/S3 NOT_ACTIVE while their exact gates remain false
-> external Platform counterpart separately authorized + implemented/reviewed
-> WP3 protected/released
-> WP4 protected/released
-> #416 prerequisite protected/materially exercised
-> activate S1 only with exact shared-path/Cargo leases
-> exact-head review + canonical CI/MQ + protected readback
-> activate S2 with selected next migration and dedicated PG target
-> actual PostgreSQL17.6/restart/rollback qualification + review/CI/MQ/readback
-> activate S3 only after fresh Foundation/composition custody readback
-> authenticated producer/consumer interoperability + nonrollback composition
-> compose with #414 Character Authority and #415 Channel assignment
-> fresh WP5 readiness evaluation
```

A review/CI/Merge-Queue pending state is a checkpoint. It is not permission to
activate the next stage before the predecessor is protected and read back.

## Current blocker disposition

Game-side preparation in this document is independently mergeable because it
changes one new documentation path only. Material source mutation remains blocked
by real dependencies/authority, principally:

- missing separately authorized compatible Platform native producer;
- unreleased WP3/WP4 shared Cargo/PostgreSQL/durability custody;
- shared `lib.rs`/Foundation composition custody;
- #416 material routing activation for future dedicated PG targets.

Those are activation blockers, not reasons to weaken source authentication or
reuse fixtures as authority.

## Validation for this allocation PR

Documentation/control-plane only:

- full changed-file/diff review;
- governance and repository whitespace/policy validation as selected by the
  repository gate;
- independent exact-head review if required by current control-plane/risk policy;
- exact-head `game-gate`, normal FULL Merge Queue and protected-main readback.

Runtime E2E: `NOT_APPLICABLE` — this PR allocates future work only and changes no
runtime, source, transport, database, workflow, Platform or production behavior.
