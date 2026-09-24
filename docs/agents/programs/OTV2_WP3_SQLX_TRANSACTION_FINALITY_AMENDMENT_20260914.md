# WP3 SQLx transaction finality/custody amendment

Coordinator: #162. Programme: #364. Existing sole material writer: #351 / Draft
PR #356.

## State

```yaml
allocation_id: OTV2-WP3-SQLX-TRANSACTION-FINALITY-20260914
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 8dfae3b9455673feff1745b9f124b786f93fcacc
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-sqlx-transaction-finality-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: c41aecc2ad146520608f4018471859f94b0e58b2
source_wp3_tree: 59586fe0ed6b7ed1b513052c0b6f5bdb21e4c0f5
source_blocker: 5664639334
m05_authority: 5663306427
pool_return_finality_application: 5664373819
architecture: WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1_REVISION_3
risk: HIGH
```

This is one prospective, symbol-bounded dependency allocation for the **same**
canonical #351/#356 writer. It grants no source mutation until its complete
protected lifecycle and a fresh #162 application. It does not mark #356 Ready,
authorize #356 Merge Queue entry, or claim M05/WP3 completion.

## Exact-source reconciliation

The allocation is bound to canonical #356 head
`c41aecc2ad146520608f4018471859f94b0e58b2`, tree
`59586fe0ed6b7ed1b513052c0b6f5bdb21e4c0f5`, and protected
`main@8dfae3b9455673feff1745b9f124b786f93fcacc`.

At that exact source:

- `Transaction<'c, DB>` privately stores
  `MaybePoolConnection<'c, DB>` and `open: bool`;
- ordinary `commit(mut self)` and `rollback(mut self)` await the applicable
  `DB::TransactionManager` operation, set `open = false` only after success,
  consume the wrapper, and return only `Result<(), Error>`;
- on an ordinary finalization error, `?` leaves `open = true`, and ordinary
  `Transaction::drop` calls `DB::TransactionManager::start_rollback`;
- Drop only starts or queues rollback. It does not prove rollback completion,
  pool return, close, or descendant finality;
- `MaybePoolConnection` is already a `#[doc(hidden)]` public pool export with
  public `Connection` and `PoolConnection` variants;
- `Pool::try_begin()` returns `Transaction<'static, DB>` created from an owned
  `MaybePoolConnection::PoolConnection`; and
- protected pool-finality allocation #610 already supplies the separately
  awaited terminal/no-evidence pool-return seam required after transaction
  finalization.

These facts confirm blocker `5664639334`. Revision 3 requires the M05 root to
retain physical pooled-connection custody after commit or rollback finalization,
then explicitly await the protected pool-return/close finality seam before a
readiness or recovery-generation transition. Ordinary consuming transaction
finalization cannot provide that custody.

## Exact prospective production dependency lease

The **only writable production dependency source** is:

`vendor/sqlx-core-0.9.0/src/transaction.rs`

Within that file, authority is limited to the minimum M05-specific,
`#[doc(hidden)] pub` consuming finalization seam needed to:

1. operate on the owned/top-level `Transaction<'static, DB>` family used by
   `Pool::try_begin()`;
2. await the existing `DB::TransactionManager::commit` or `rollback` future to
   completion;
3. return the same owned `MaybePoolConnection<'static, DB>` together with the
   exact `Result<(), Error>`, including when finalization fails; and
4. prevent ordinary Transaction Drop from losing that connection before the
   M05 caller observes it.

The preferred API placement is an implementation restricted by type/lifetime to
`Transaction<'static, DB>`, with separate M05 commit and rollback siblings (or
one equivalently narrow operation selector). It must not be available through
the ordinary borrowed `Transaction<'c, DB>` implementation used for nested
savepoints. The returned `MaybePoolConnection` variant remains explicit so the
M05 root must match `PoolConnection` and reject/fail closed on `Connection`;
`'static` alone is not permission to assume pooled custody.

The return type must preserve connection custody and finalization outcome as
separate values, for example the exact semantic shape:

```text
(same MaybePoolConnection<'static, DB>, Result<(), Error>)
```

Equivalent safe typed wiring is permitted only when it retains both values on
every completed success and error path. The seam does not normalize the SQLx
error. The M05 root separately performs its bounded `DurabilityError`
normalization while it still owns the returned connection, then finishes or
fences the queued rollback obligation as applicable and explicitly awaits the
pool return/close finality seam.

### Ordinary success and error semantics

On successful commit or rollback, the M05 seam must set the ordinary `open`
state to false exactly as the corresponding existing method does. Extracting
the connection must not cause `Transaction::drop` to start rollback after a
successful finalization.

On a commit or rollback error, the seam must mirror ordinary behavior before it
returns custody: `open` remains logically true and
`DB::TransactionManager::start_rollback` is invoked exactly as ordinary Drop
would invoke it. Because the connection is then deliberately extracted rather
than lost through Drop, the implementation must safely prevent a second
`start_rollback` from Transaction Drop without misrepresenting the failed
operation as successful. The returned result remains the original finalization
error, and the same connection remains available to the M05 root to finish the
obligation and perform bounded return/close finality.

No error path may silently discard, Drop, detach, return, or close the
connection before the root receives it. A commit/rollback error is not terminal
pool finality and is not readiness evidence.

### Safe extraction and ordinary behavior preservation

A minimum private storage refactor inside `transaction.rs` is permitted only if
needed for safe extraction from a type implementing Drop. A safe takeable
representation, such as an optional private connection field with an exact
internal accessor, is acceptable when all ordinary methods preserve their
existing externally observable behavior. Unsafe field moves are not permitted
when a bounded safe representation suffices.

Any private representation change must preserve exactly:

- `Transaction::begin`, including rollback-on-incomplete/failed begin intent;
- ordinary `Transaction::{commit, rollback}` results and state transitions;
- `Deref`, `DerefMut`, and `AsMut<DB::Connection>` behavior;
- ordinary Drop/start-rollback behavior;
- borrowed transaction and nested-savepoint behavior; and
- all non-M05 SQLx behavior.

Internal access to an unexpectedly absent connection must fail closed and must
not fabricate success. It must not add a panic to a path that can be reached by
ordinary safe API use.

The new finalization future is intentionally not an SQLx-owned
cancellation-surviving service. The Revision-3 root owns and retains the future
and its obligation until it completes. Cancelling or dropping that M05 future
is not commit, rollback, return, close, or successor-generation evidence. This
allocation does not authorize a completion channel, task registry, waiter,
history, retry loop, or detached finalizer inside SQLx.

## Read-only evidence and exclusions

The following are explicitly **read-only** with zero mutation authority:

- `vendor/sqlx-core-0.9.0/src/pool/maybe.rs`;
- `vendor/sqlx-core-0.9.0/src/pool/mod.rs`;
- `vendor/sqlx-core-0.9.0/src/pool/connection.rs` beyond its already-active
  #610 scope;
- every TransactionManager implementation; and
- PostgreSQL protocol source.

No mutation or re-export is needed in `pool/maybe.rs` or `pool/mod.rs` because
`MaybePoolConnection` and both variants are already downstream-nameable.

This allocation grants no generic Transaction API redesign, new stable public
pool/transaction contract, savepoint semantic change, retry/autocommit policy,
new SQLx test file, Cargo/lock/workflow change, PostgreSQL/rustls/Tokio change,
six-file Durability implementation, #335/WP4/WP5/Server Seam authority,
production/deployment/credential/live-data authority, or external-repository
write.

`vendor/sqlx-core-0.9.0/OTERYN_PROVENANCE.md` may be prospectively written only
for the exact authored transaction-finality symbols, pinned-source identity,
and focused proof record. Downstream registered-root integration proof remains
under protected #606 authority in
`apps/game-server/tests/durability_postgres.rs`; this allocation neither
duplicates nor transfers that custody.

No new or modified `vendor/sqlx-core-0.9.0/tests/**` path is authorized. Focused
proof should live in `transaction.rs` where feasible. If exact implementation
proves another path or symbol is inseparable, the writer must stop before
mutation with:

`SHARED_LEASE_REQUIRED = <path> :: <symbol/resource> :: <reason>`

## Required focused proof after activation

On one exact successor of canonical #356, the SAME writer must prove at least:

- a successful top-level commit returns `Ok(())` plus the exact owned pooled
  connection, without ordinary Drop rollback;
- a successful top-level rollback returns `Ok(())` plus the exact owned pooled
  connection;
- commit error and rollback error each return the original `Err` plus the exact
  same connection and invoke `start_rollback` with the ordinary semantics,
  without double invocation or custody loss;
- the caller can distinguish/reject a borrowed `Connection` variant, and normal
  borrowed/nested/savepoint use cannot be mistaken for the owned M05 pooled
  root path;
- cancellation of the M05 finalization future is not classified as success and
  does not authorize an `R`/`T`, readiness, or successor-generation transition;
- after extraction, the registered game-server M05 root can retain the
  `PoolConnection`, bounded-normalize the SQLx result, and call the already
  active #610 pool-return seam to observe terminal returned-to-idle,
  retired/closed, or no-evidence finality;
- ordinary `Transaction::commit`, `rollback`, Drop, begin, dereference,
  savepoint, and non-M05 SQLx tests remain unchanged; and
- all read-only pool and TransactionManager implementation files remain
  byte-identical.

The focused seam is custody/observability only. It does not prove SQL semantics,
M05, the Revision-3 root equation, complete rollback processing, or WP3.

## Protected lifecycle

```text
this one-document prospective allocation
-> exact-head producer review and repository checks
-> genuinely independent HIGH exact-head review with P0=P1=P2=0
-> explicit human-owner authorization for the exact allocation candidate
-> governed native FULL Merge Queue
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> fresh #162 main/head/open-PR/task/path-custody reconciliation
-> explicit SAME #351/#356 application of this exact transaction.rs seam
-> focused RED/GREEN implementation and M05 composition qualification
```

Protected integration alone does not activate source authority. Direct merge,
generic auto-merge, bypass, force/rebase, no-op/retrigger commits, replacement
workers/branches/PRs, and protection weakening are forbidden.

```text
ALLOCATION_STATE = NOT_ACTIVE
SOURCE_AUTHORITY = PROSPECTIVE_ONLY
M05_PROOF = NOT_CLAIMED
WP3_COMPLETE = NO
MERGE_AUTHORITY = REPOSITORY_CONTROL_PLANE_ONLY
```
