# WP5 native admission source-readiness reconciliation

Coordinator: #162. Source readiness: #319. Programme: #364.

## Status

Path-disjoint control-plane reconciliation only. **NOT_ACTIVE**.

This checkpoint reconciles the protected source-ingestion allocation from #452
with the protected PostgreSQL target-registration amendment from #456 and the
current live dependency state. It does not allocate runtime, SQL/migration,
workflow, Cargo/workspace, Foundation composition, Platform/external-repository,
production or Server Seam write authority.

```yaml
reconciliation_id: OTV2-WP5-NATIVE-ADMISSION-SOURCE-READINESS-20260909
repository: Oteryn/Oteryn-Game
coordinator_issue: 162
source_issue: 319
programme_issue: 364
reconciliation_base_main_sha: 10ce3393a51dac14105b831040e1f4faa3ca565f
parent_source_allocation: "#452 / OTV2-WP5-NATIVE-ADMISSION-SOURCE-INGESTION-20260909"
postgres_target_registration: "#456 protected"
allocation_state: NOT_ACTIVE
platform_write_authority: FORBIDDEN
production_authority: FORBIDDEN
server_seam_authority: FORBIDDEN
```

## Why this reconciliation exists

The protected #452 allocation was correct when authored, but some of its
future-tense wording predates protected #456. In particular, #452 says S2 still
requires a separately reviewed amendment to #416 registering
`native_admission_source_postgres`.

That registration prerequisite is now satisfied by protected PR #456:

```text
native_admission_source_postgres
-> apps/game-server/tests/native_admission_source_postgres.rs
```

PR #456 merged as protected commit
`0f8f27758a67a50101744ef047d17c97b46325ad`. Its allocation remains target
registration only and `NOT_ACTIVE`.

This checkpoint therefore supersedes only the stale statement that target
registration itself is still pending. It does not supersede #452's accepted S1,
S2 or S3 semantics, resource envelope, source-authentication requirements,
material #416 routing order or custody rules.

## Fresh live evidence

### Game protected main

Protected Game main at reconciliation start:

```text
10ce3393a51dac14105b831040e1f4faa3ca565f
```

The current main remains a descendant of the protected #456 and #452 deliveries.
No current WP5 source-ingestion implementation PR was found that would replace
or overlap this reconciliation.

### WP3 / #356

The canonical WP3 PR remains the existing `agent/sqlx-driver-budget-351` line.
At the latest read during this reconciliation it is still `OPEN/DRAFT`, not
protected/released.

Recent canonical WP3 checkpoints materially improve the future S1 TLS footing:

- actual TLS 1.3 HelloRetryRequest execution and initial/replacement KX overlap
  are proven on the canonical line;
- AWS-LC KX/provider residency has advanced to proven focused evidence;
- owner-aware rustls handshake-span capacity accounting is now proven focused,
  including pre-allocation reservation, growth overlap, retained high-water
  custody and release after backing destruction.

The same canonical task still records `complete_tls_accounting: NOT_PROVEN`.
Remaining cells include generic reader/list/payload/message ownership,
transcript, decoded ClientHello/ServerHello, certificate/OCSP and successor-state
custody, compressed-certificate overlap, retained-session composition, complete
TLS witness, funded SQLx TLS-positive execution and PostgreSQL 17.6 qualification.

Therefore WP3 progress is useful evidence but **does not release S1**. S1 must
refresh the final protected WP3 graph after terminal protected integration and
prove every accepted `NSRC-*` lower-layer bound on that final graph before its
results can count.

### WP4 / #335

PR #335 remains the canonical WP4 durability line and is still
`NOT_READY_FOR_INTEGRATION`. Its protected schema/durability custody is therefore
not released to S2.

### Platform producer

Read-only Platform main at this reconciliation remains:

```text
13e207d5e826b2a24513664d7b8b6dc5d918f9b0
```

No open Platform PR found in the refreshed inventory implements the required
native security/trust producer family. The protected default branch still lacks
implementations named:

- `ReadAccountSecurityV1`;
- `ReadFreshSigningTrustV1`;
- `ReadRecoveryAccountSecurityV2`;
- `ReadRecoverySigningTrustV2`.

The external classification remains
`MISSING_CROSS_REPOSITORY_PLATFORM_PRODUCER`. No Platform mutation is authorized
or performed by this checkpoint.

## Reconciled stage gates

### S1 — authenticated transport and descriptor validation

State: **BLOCKED_DEPENDENCY / NOT_ACTIVE**.

Already satisfied prerequisites:

- #452 allocation is protected;
- #346/#349 wire codec is protected;
- accepted fixed-descriptor HTTPS/TLS 1.3 mTLS semantics and `NSRC-*` envelope
  remain authoritative;
- controlled independent test producer remains an allowed qualification source
  before the real Platform counterpart exists.

Still required before activation:

1. WP3 #356 terminally integrates through its required exact-head review,
   canonical CI, FULL Merge Queue and protected-main readback;
2. shared Cargo/TLS/dependency custody is explicitly released and then granted to
   S1 without active-owner overlap;
3. the final protected dependency graph demonstrably enforces every accepted
   certificate/parser/byte/work bound before hostile work can exceed the
   `NSRC-*` budget;
4. any required `apps/game-server/Cargo.toml`, workspace or later `lib.rs` change
   receives a separate serialized lease;
5. authenticated interoperability proof uses a separately authorized compatible
   producer or controlled independent test producer.

No current WP3 WIP checkpoint, including the new HRR/span proofs, is permission
to activate S1 early.

### S2 — durable descriptor/source-floor/pending-slot bootstrap

State: **BLOCKED_DEPENDENCY / NOT_ACTIVE**.

Reconciled prerequisite status:

```yaml
native_admission_source_postgres_target_registration: PROTECTED_SATISFIED_BY_456
wp3_driver_release: PENDING
wp4_schema_durability_release: PENDING
s1_descriptor_wire_semantics: PENDING_PROTECTED_PREDECESSOR
migration_number_selection: PENDING_FRESH_POST_WP4_READBACK
conditional_durability_hook_leases: PENDING
material_416_routing: INTENTIONALLY_DEFERRED_UNTIL_HELD_S2_TARGET_EXISTS
```

The protected #456 registration means S2 no longer needs another registration
amendment before it may eventually create the exact held test target.

The non-cyclic protected ordering remains:

```text
#456 target registration protected
-> WP3 + WP4 + S1 + exact custody gates become true
-> activate held S2 implementation
-> create apps/game-server/tests/native_admission_source_postgres.rs
-> separately serialize material #416 routing/policy/pin work
-> protect #416 routing + audit-pin rotation + canonical CI/MQ/readback
-> reconcile held S2 against protected routing
-> only then run/count real PostgreSQL 17.6 S2 evidence
-> review/CI/FULL MQ/protected readback for S2
```

Pre-routing PostgreSQL execution still cannot count as S2 acceptance evidence.
Target registration itself does not grant workflow, migration, runtime or
production authority.

### S3 — sealed Foundation publication/composition

State: **BLOCKED_AUTHORITY_AND_DEPENDENCY / NOT_ACTIVE**.

S3 remains blocked until all of the following are true:

- S1 protected;
- S2 protected;
- compatible separately authorized Platform producer is protected and
  independently current;
- WP4 is protected;
- fresh Foundation/composition/shared `lib.rs` custody has been read back and a
  sole-writer lease granted;
- #414 Character Authority and #415 runtime-scope assignment are ready for the
  required final composition without substituting for one another.

No test fixture, directory/catalogue record, caller-created fact, grant or stale
receipt may be promoted into producer authority to bypass this gate.

## Exact next resume triggers

The coordinator may resume material WP5 source-ingestion work only on a fresh
GitHub readback proving one of these transitions:

1. **S1 trigger:** #356 is terminally integrated on protected main and shared
   Cargo/TLS custody is released. Re-read the final TLS graph first; do not reuse
   WIP proof as final dependency qualification.
2. **S2 trigger:** S1 is protected, WP3 driver qualification is protected, #335
   is protected, current migrations are re-read and non-overlap plus exact
   durability-hook leases are proven. #456 target registration is already done.
3. **S3 trigger:** S1+S2 are protected and a separately authorized compatible
   Platform producer exists, followed by a fresh Foundation/composition custody
   readback.

If none of those triggers is true, only read-only reconciliation or additional
path-disjoint control-plane preparation is legal. Do not create replacement
WP3/WP4/Server-Seam workers and do not mutate Platform.

## Scope and validation

This reconciliation changes one new `docs/agents/programs/**` path only. It
creates no implementation lease and no deployment authority.

Required integration evidence:

- exact changed-file/diff review;
- independent exact-head review because this document changes interpretation of
  a security/durability activation prerequisite;
- exact-head repository checks selected by current policy;
- normal FULL Merge Queue;
- protected-main readback.

Runtime E2E: `NOT_APPLICABLE` — control-plane reconciliation only.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
