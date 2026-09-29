# WP3 SQLx pool return/finality amendment

Coordinator: #162. Programme: #364. Existing sole material writer: #351 / Draft
PR #356.

## State

```yaml
allocation_id: OTV2-WP3-SQLX-POOL-RETURN-FINALITY-20260914
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 43f0128f7e11253cca19b40b5cc3703b46b35916
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-sqlx-pool-finality-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: f5b275cca1adb3404ee51a5b8f02f3ee000348ee
source_checkpoint: 5663349424
narrowing_evidence: 5663531035
blocking_pre_pr_review: 5663595517
independent_high_review_finding: 5663752252
superseded_preparation_note: 5656948788
architecture: WP3-V2-ROOT-OWNED-BOUNDED-PGPOOL-V1_REVISION_3
risk: HIGH
```

This is a prospective symbol-bounded amendment for the **same** canonical
#351/#356 writer. It grants no source mutation until independent exact-head HIGH
review, exact-head repository checks, protected Merge Queue integration,
protected-main readback, and an explicit fresh #162 same-writer activation. It
creates no second pool, owner, writer, branch, PR, ledger, or recovery policy.

## Reconciliation result

The older read-only preparation note `5656948788` concluded
`NO_ADDITIONAL_POOL_SYMBOL_REQUIRED`. That conclusion is superseded for M05
implementation custody by the later exact-source proof in checkpoint
`5663349424`.

At canonical #356 head `f5b275cca1adb3404ee51a5b8f02f3ee000348ee`:

- doc-hidden `PoolConnection::return_to_pool()` returns
  `Future<Output = ()>` even though its private path obtains a boolean that
  distinguishes successful return from retirement/close;
- `Floating<DB, Live<DB>>::return_to_pool()` returns `true` only after its
  viable path synchronously calls `release()`;
- `PoolInner::release()` then pushes the idle connection, releases the permit,
  and updates `num_idle` synchronously before returning;
- every `false` path awaits its existing graceful or hard close before the
  boolean is returned; and
- ordinary `PoolConnection::return_to_pool()` discards that completed boolean.

The exact missing M05 seam is therefore the erased completed disposition in
`connection.rs`, not a missing transition or signal in `PoolInner::release`.
Revision 3 requires the root to retain an explicit finality obligation until it
is terminal before transferring charged `T -> R` custody or admitting a
successor `T`. Pool counters, permits, wrapper destruction, and spawned Drop
cleanup cannot replace that explicit result.

The blocking pre-PR review `5663595517` is accepted. `inner.rs` is read-only
evidence and receives no prospective mutation authority. The smallest lawful
response is one Oteryn/M05-specific sibling/helper in `connection.rs` exposing
the already-existing completed disposition. No #356 source authority is active
from this allocation alone.

## Exact prospective source lease

The **only writable production dependency source** is:

`vendor/sqlx-core-0.9.0/src/pool/connection.rs`

Within that file, authority is limited to:

- one Oteryn/M05-specific terminal-disposition type with exactly
  `RETURNED_TO_IDLE` and `RETIRED_CLOSED`;
- one explicitly **`#[doc(hidden)] pub`** sibling on `PoolConnection` which
  follows the current private return path and returns a minimal fail-closed
  shape such as `Option<terminal-disposition>` (or an equivalently unambiguous
  typed `Result`); and
- a minimal refactor of the existing doc-hidden
  `PoolConnection::return_to_pool() -> Future<Output = ()>` solely when needed
  to share that helper while preserving its output and behavior exactly.

The callable visibility is mandatory: the M05 consumer is in the separate
`game-server` crate, so a private SQLx sibling is insufficient. The exact
`#[doc(hidden)] pub` visibility makes only this bounded seam
downstream-callable; it does not establish a stable or general-purpose SQLx
pool API.

`RETURNED_TO_IDLE` may be produced only after the existing
`Floating<DB, Live<DB>>::return_to_pool()` has returned `true`. That boolean is
already produced only after synchronous `Floating::release()` and
`PoolInner::release()` return, including idle publication, permit release, and
the `num_idle` update.

`RETIRED_CLOSED` may be produced only after the current `false` path has
completed its existing graceful or hard close await. The helper must not infer
retirement from an error, timeout, task-spawn acceptance, permit/counter change,
or wrapper destruction before that existing close path completes.

The existing `self.live == None` path is a third, **non-terminal/no-evidence**
result. It must yield neither `RETURNED_TO_IDLE` nor `RETIRED_CLOSED`. This
includes every repeated invocation after a prior call has taken the live
connection. Absence of a live connection proves neither prior idle publication
nor prior close completion, so the sibling must return `None` (or the exact
equivalent typed fail-closed result) without fabricating a terminal
disposition. M05 must retain or fail the root finality obligation on that
result; it may not release `R`/`T` custody, publish readiness, or admit a
successor `T`.

Ordinary `PoolConnection::return_to_pool() -> Future<Output = ()>` and ordinary
Drop behavior must remain behaviorally unchanged. The allocation does not
authorize changing `take_and_close`, `Drop for PoolConnection`, close-on-drop,
or the private `Floating::{close,close_hard,release}` semantics. It does not
authorize turning the M05-specific outcome into a stable generic SQLx pool API.

### Read-only evidence with no mutation authority

`vendor/sqlx-core-0.9.0/src/pool/inner.rs`, including
`PoolInner::release`, is read-only. It receives **no** write, signalling,
observability, state, or call-plumbing grant. Its current synchronous ordering is
the proof boundary consumed by the `connection.rs` disposition:

1. push the idle connection;
2. release the permit; and
3. increment `num_idle` before returning.

Every other SQLx pool file and symbol is also read-only. If actual M05
implementation proves another dependency path or symbol is required, the worker
must stop before mutation with a new exact
`SHARED_LEASE_REQUIRED = <path> :: <symbol/resource> :: <reason>` result.

### Focused proof and provenance

Focused proof may be added inside the same leased `connection.rs` file where
feasible. `vendor/sqlx-core-0.9.0/OTERYN_PROVENANCE.md` may receive only the
exact authored-symbol, pinned-source, and proof record for this amendment.

No new or modified `vendor/sqlx-core-0.9.0/tests/**` file is prospectively
authorized. If the exact current manifest/source later proves that another test
path is required and runnable, stop for a new exact shared lease rather than
seizing it. Protected #606 separately owns the registered-root M05/E02
integration-test authority in
`apps/game-server/tests/durability_postgres.rs`; this allocation does not
duplicate or transfer that custody.

## Required invariants

1. `RETURNED_TO_IDLE` is observable only after the current private return path
   has returned `true`, which is already after synchronous idle publication,
   permit release, and `num_idle` update.
2. `RETIRED_CLOSED` is observable only after the current `false` path has
   completed its existing graceful or hard close await.
3. The two outcomes are mutually exclusive and describe only the completed
   disposition of one explicit M05 return obligation.
4. No-live and repeated invocation produce the third non-terminal/no-evidence
   result and can never produce either terminal success disposition. M05
   retains or fails the obligation and may not release `R`/`T` custody or admit
   a successor `T` on that result.
5. The Revision-3 M05 root owns and retains that explicit future/obligation
   until terminal. Dropping or cancelling the explicit root-owned future cannot
   be reinterpreted as success.
6. No SQLx-owned cancellation-surviving waiter, history, completion channel,
   registry, or signalling collection is pre-authorized. This allocation does
   not move root finality ownership into SQLx.
7. Ordinary `PoolConnection::return_to_pool()` and Drop behavior, including its
   existing no-live path and maintenance behavior, ping,
   lifetime expiry, `after_release`, close/hard-close, minimum-connection
   maintenance, permit accounting, queue ordering, and pool policy remain
   behaviorally unchanged.
8. No additional pool capacity, retry, reconnect, maintenance loop, timer poll,
   numeric limit, or recovery generation is introduced. M05 remains the owner
   of Revision-3 demand, recovery, and finality retention policy.
9. The completed disposition is evidence, not permission to publish root
   readiness or to convert a retired connection into ready state. M05 alone
   performs the accepted custody transfer and gates successor generations.

## Required focused proof

On one exact #356 successor, the SAME writer must prove at least:

- viable ping followed by the current synchronous release path yields exactly
  `RETURNED_TO_IDLE`, and never yields it before `PoolInner::release()` returns;
- pool-closed, max-lifetime, `after_release = false`, `after_release` error, and
  ping-error paths yield exactly `RETIRED_CLOSED` only after their current close
  or hard-close await completes;
- the two outcomes are mutually exclusive and terminal for one explicitly
  retained M05 finality future;
- a first no-live call and every repeated call after the live value has already
  been taken yield the third non-terminal/no-evidence result and never either
  terminal success disposition; M05 retains/fails the root obligation without
  releasing `R`/`T` custody or admitting a successor `T`;
- cancellation or dropping that explicit future is not reported as successful
  return or close, and M05 retains the obligation rather than depending on an
  SQLx-owned completion channel;
- ordinary unit-returning `return_to_pool()`, including its existing no-live
  path and maintenance behavior, and ordinary Drop/close-on-drop spawning,
  timeout, maintenance, and permit behavior remain unchanged; the new sibling
  cannot reinterpret that ordinary no-live behavior as finality evidence;
- `inner.rs`, pool policy, counters, permits, and queue implementation remain
  byte-identical; and
- the separately authorized #606 M05 PostgreSQL 17.6 tests subsequently show no
  successor recovery generation begins while a prior explicit return/close
  obligation is non-terminal.

Focused proof qualifies only this seam. It does not by itself prove M05, the
Revision-3 root equation, complete pool/root accounting, or WP3 completion.

## Explicit exclusions

No `inner.rs` mutation; no SQLx-owned cancellation-surviving waiter/history or
completion channel; no generic pool redesign; no new stable pool contract; no
pool size, acquire timeout, idle timeout, max lifetime, min/max connection,
reaper, maintenance, fairness, semaphore, permit, counter, or queue-policy
change; no numeric limit or retry/recovery policy; no new SQLx-core test file;
no Cargo or lockfile mutation; no workflow/B01 or protected-control change; no
PostgreSQL protocol/decoder/SASL/TLS/rustls/Tokio change; no #335/WP4, WP5,
Server Seam #247, production, deployment, credential, live-data, or
external-repository authority.

This allocation does not reopen PROVEN frozen-inventory cells and does not claim
M05 or WP3 complete. The existing six-file M05 Durability lease and protected
#606 integration-test scope remain distinct and unchanged. Protected integration
of this document does not activate the pool seam.

## Lifecycle

```text
this one-document prospective allocation
-> exact-head repository checks
-> genuinely independent HIGH exact-head review with P0=P1=P2=0
-> owner-authorized native FULL Merge Queue
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> fresh #162 main/head/open-PR/task/path-custody reconciliation
-> explicit SAME #351/#356 activation of this exact connection.rs seam
-> focused RED/GREEN implementation and M05 composition qualification
```

Protected integration alone does not activate source authority. Direct merge,
generic auto-merge, bypass, force/rebase, no-op/retrigger commits, replacement
workers/branches/PRs, and protection weakening are forbidden.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
