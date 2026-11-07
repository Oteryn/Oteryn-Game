# WP3 escaping error custody decision

Decision ID: `WP3-ERROR-EXTERNAL-BOX-CUSTODY-V1`

Status: **CANDIDATE / NOT ACCEPTED / NO SOURCE ACTIVATION**.

Repository: `Oteryn/Oteryn-Game`. Existing task: #351 / PR #356.
Protected basis: `448309c4042e3fb783d1ab9d7bb324f7ee977420`.
Inspected source: `f56896334d5c6b5d6e6419c21c56092b56b8f28e`.
Frozen inventory: [#162/5655890678](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5655890678), M03/M04.
Existing disposition: [#162/5656188739](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5656188739).
Owner handoff: [#162/5656281533](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5656281533).

## Problem and evidence

`vendor/sqlx-core-0.9.0/src/error.rs::Error::Database` owns a final
`Box<dyn DatabaseError>`. A reservation stored inside `PgDatabaseError` drops
before that outer Box is deallocated. Returning a naked owned Box through
`into_database_error`, `DatabaseError::into_error` or consuming downcasts can
also detach error backing from its funding. The TLS and Configuration Box
families have the same external-custody requirement.

This is a source-derived boundary, not an executed allocation measurement.
The accepted Revision-3 root, resource ceilings, R/T non-overlap, active-pass
semantics and M05 error normalization remain unchanged. The new decision is
limited to the public error representation and consuming API compatibility
needed to implement the already-required finality semantics.

## Options and recommendation

1. Retain naked owned extraction and use only interior charges. This cannot
   prove complete outer-Box deallocation before release. A forwarding wrapper
   inside another Box repeats the same problem. Returning `None` for an owned
   database error instead would silently lose real database classification.
2. Preserve legacy owner-free constructors and introduce explicit external
   error custody with custody-preserving consuming extraction. **Recommended.**
   This changes one public extraction return type but keeps ordinary error
   construction and borrowed database queries compatible.

The compatibility cost is deliberate and must be reviewed. Source compatibility
for callers explicitly requiring `Box<dyn DatabaseError>` from
`into_database_error()` is not claimed. No blanket compatibility claim applies
to third-party consumers of the fork.

## Candidate representation contract

All new core representation symbols and their colocated tests are confined to
`vendor/sqlx-core-0.9.0/src/error.rs`:

- Preserve existing owner-free `Error::{Database,Tls,Configuration}` variants,
  their payload types and constructors, and the raw `DatabaseError` trait API.
- Add `Error::Owned(OwnedError)` covering only Database, Tls and Configuration
  families, plus an inline typed resource-denial form retaining `BudgetError`.
- `OwnedError::{Database,Tls,Configuration}` retain `ErrorCustody<T>` for the
  applicable boxed trait object.
- `ErrorCustody<T: ?Sized>` contains a private Box followed by a private
  optional `ResourceReservation`. Complete Box destruction/deallocation must
  precede reservation release. The custodian is inline; any separately
  allocated enclosing object still needs its own external custody.
- Owned constructors consume already-established same-root reservations
  acquired before controlled allocation. No new ledger or runtime-owner
  identity is created. Caller-side proof covers final Box backing and its
  exclusively owned descendants; shared backing keeps its own final-owner
  charge, without duplicate charging or early transfer release.
- Expose only immutable payload views. No mutable source/payload extraction,
  `DerefMut`, `into_inner`, `into_parts`, raw pointers, charge detachment or
  unconditional conversion to naked Box is permitted. An owner-free-only
  compatibility conversion may return `Result<Box<T>, Self>` only if custody
  is absent; a present zero-byte reservation is not absence of custody.
- Do not implement `DatabaseError` for the custodian. Its existing mutable and
  consuming raw methods would reopen the escape. Keep them available on
  ordinary owner-free raw objects. Owned objects expose immutable underlying
  references and custody-preserving consuming methods.

The generic container's destruction-order claim does not prove arbitrary
external payload methods allocate within budget. Owned construction is used
only by the inspected, allocated producer graph with its own preallocation and
descendant-lifetime evidence.

## Borrowed and consuming API behavior

`Error::as_database_error` recognizes both database forms and returns the
underlying borrowed `dyn DatabaseError`. Preserve SQLSTATE, kind, constraint,
table, transient classification and concrete-type queries.

`Error::into_database_error` returns
`Option<ErrorCustody<dyn DatabaseError>>`. Legacy boxes move into an uncharged
custodian without another allocation; owned boxes retain their custody.
Neither real database form silently returns `None`.

`ErrorCustody<dyn DatabaseError>::{try_downcast,downcast,into_error}` preserve
the custodian through concrete-type extraction and standard-error conversion.
Successful downcast returns custody around the concrete Box; failed downcast
returns the original custodian. Standard-error downcasts for TLS/Configuration
must likewise retain custody. Wrong-type panicking downcasts must release
backing and custody in the correct order during unwind.

Transparent Display/source delegation must preserve the existing category text
and underlying borrowed source. Do not add a wrapper-source level that changes
concrete source inspection. Resource denial retains its borrowed typed source
without boxing, formatting or copying rejected data.

## Integration scope and exclusions

The sole additional imported core source path is `src/error.rs`. Existing
admitted paths may compose this representation without expanding their purpose:

- PostgreSQL `src/connection/stream.rs::PgStream::recv` and
  `src/error.rs::PgDatabaseError` producer construction;
- SQLx `src/net/tls/{mod,tls_rustls}.rs` owned bridges, including
  `certificate_read_error`, and admitted PostgreSQL TLS/configuration producers;
- PostgreSQL `src/copy.rs::PgCopyIn::abort`, only if owned errors reach its
  direct Database match: query borrowed classification and retain/rethrow the
  original complete error;
- existing allocated PostgreSQL/core tests and provenance records.

`apps/game-server/src/durability/schema.rs::is_missing_table` already uses
`as_database_error`; require compatibility proof, not a new consumer edit.
`PoolInner::connect` has a direct database-error retry match. Its owner-aware
no-hidden-retry disposition belongs to M05; this decision grants no pool
rewrite and does not establish that M05 is complete.

No rustls state/outbound/key/provider mutation, Tokio, Cargo, workflow, registry,
production-consumer, numeric ceiling, TLS policy, WP4/WP5, Server Seam or
external-repository scope is added. Saved/returned/cloned provider errors still
need the existing M03 backing proof. M05 still normalizes errors inside custody
and destroys them before releasing the corresponding ownership.

## Required qualification

1. Exact-bound positive and one-byte-short denial before error Box allocation;
   checked overflow and allocation-free typed denial.
2. Retain an extracted error after connection/producer/phase handles drop and
   prove its original R/T/active funding remains held.
3. Successful/failed database and standard-error downcasts, trait conversion,
   moves, wrong-type panic/unwind and final drop retain/release exactly once.
4. Inspect complete Box deallocation ordering. Payload destructor event tests
   alone do not prove outer allocation deallocation; supplement with a suitable
   allocator witness or exact standard-library source proof.
5. Compile-fail/API checks reject mutable replacement, naked charged extraction
   and unconditional raw-box conversion.
6. Preserve borrowed PgDatabaseError/SQLSTATE, source downcasts and Display;
   exercise schema `42P01`, applicable COPY classification and owner-free
   controls, including uncharged consuming extraction.
7. Prove shared diagnostic backing keeps its own final-owner debit with no
   premature release or double charge.
8. Run affected core/PostgreSQL targets explicitly, strict Clippy, exact-source
   provenance checks and configured PostgreSQL 17.6 qualification on the
   resulting canonical implementation head. Missing environment or skipped
   tests do not constitute runtime proof.

This document records no executed implementation tests or universal resource
bound. Review and qualification must distinguish source proof from execution.

## Decision timing, risks and future impact

- **Must decide now: YES.** The known M03/M04 escaping-error boundary cannot
  close with naked owned extraction and interior-only reservation lifetime.
- **Blocked work:** only implementation/qualification of this exact error
  boundary. All currently legal M02/M04 work continues.
- **Harder later:** more callers could depend on naked Box extraction, increasing
  the compatibility surface and risking premature funding reuse.
- **Superseding evidence:** a concrete safe representation preserving complete
  physical finality and consuming custody with a smaller compatibility delta.
- **Not decided:** resource byte values, Rev3 root choice, generic retry policy,
  unrelated provider/error internals or downstream activation.

Player-facing error classification and recovery must remain unchanged. The main
risks are lost SQLSTATE/source identity, early release through conversion, double
charging shared payloads and accidental retry changes. The focused proofs above
are required rather than relying on compile success or the new wrapper's name.

## Acceptance and activation

Require genuinely independent exact-candidate HIGH-risk architecture/resource/API
review with P0/P1/P2 and blocking evidence gaps all zero, required exact-head
repository checks, current authorized native Merge Queue integration, real
merge_group game-gate and protected-main readback. Then Work records acceptance
and explicitly activates only the exact amendment for the SAME #351/#356 writer
after fresh overlap/custody reconciliation. A candidate document or merged file
alone is not source activation. Preserve existing task history and frozen IDs.
