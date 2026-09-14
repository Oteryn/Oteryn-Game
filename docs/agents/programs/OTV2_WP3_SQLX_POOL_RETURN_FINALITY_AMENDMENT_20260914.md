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
- `Floating::return_to_pool()` returns `true` only after its viable path calls
  `release()`, and returns `false` after pool-close, lifetime-expiry,
  `after_release` rejection/error, ping failure, and the associated close or
  hard-close awaits;
- `PoolInner::release()` privately publishes the connection to the idle queue
  before releasing the pool permit and updating the idle count;
- `PoolConnection::drop()` spawns return or close cleanup and exposes no
  completion/finality witness to an M05 owner; and
- close-on-drop applies a timeout and then runs pool maintenance, so wrapper
  destruction, task spawn, permit/counter observation, or timeout completion is
  not proof that the connection returned to idle or reached terminal close.

Revision 3 requires successful maintenance to await final return/ping before the
charged `T -> R` transfer and readiness publication. It also retains failed,
reaped, closed, or cancelled connection generations in `R` or `T` until all
connection-attributable descendants are final, and forbids a successor `T` while
any prior retirement tail is non-final. The current public seam erases the
return-versus-close disposition and the Drop seam does not expose tail
completion. Polling `try_acquire`, reading pool counters/permits, assuming
`return_to_pool().await` means ready, or treating spawned Drop work as complete
cannot prove that contract.

Therefore the old preparation conclusion is reconciled as **stale after the
newer M05 source proof**. The smallest lawful response is this protected
allocation; no #356 source authority is active from the proof alone.

## Exact prospective source lease

Paths are not blanket authority. Only the symbols and minimum inseparable wiring
listed below become writable after explicit post-protection activation. Every
other symbol remains read-only.

### `vendor/sqlx-core-0.9.0/src/pool/connection.rs`

Permitted existing symbols:

- `PoolConnection::return_to_pool`;
- `Floating<DB, Live<DB>>::return_to_pool`;
- `PoolConnection::take_and_close` and `Drop for PoolConnection` only for the
  minimum completion-witness plumbing that preserves their existing behavior;
- existing `Floating::{close,close_hard,release}` call sites only to propagate
  the exact already-completed terminal disposition; their ordinary close
  semantics are not otherwise writable.

Permitted minimum private or doc-hidden representation:

- one exact return-finality outcome with only the states needed to prove
  `RETURNED_TO_IDLE` versus `RETIRED_CLOSED` after the corresponding operation
  actually completes; and
- one bounded, non-blocking completion witness for an explicitly owner-aware M05
  return/retirement path when finality must outlive the wrapper or calling
  future.

The exact helper/type name is implementation-local. The seam must not expose a
new stable general-purpose SQLx pool API. Cancellation before first poll,
cancellation during ping/return/close, and Drop-spawned cleanup must retain a
truthful path to eventual disposition or remain explicitly non-final; they may
not report success from spawn acceptance or wrapper destruction.

### `vendor/sqlx-core-0.9.0/src/pool/inner.rs`

Permitted existing symbol:

- `PoolInner::release`, only to make successful idle publication observable at
  the exact existing publication boundary.

Permitted minimum inseparable close/finality observability:

- only the private signalling/state needed to complete the corresponding
  connection-level witness after actual idle publication or terminal
  close/hard-close completion.

No acquire, connect, reaper, maintenance, semaphore, queue, sizing, parent-pool,
or policy behavior is writable except the minimum unchanged call plumbing
strictly required by the two symbols above. If terminal close cannot be observed
within these exact files/symbols without another path, the worker must stop with
an exact `SHARED_LEASE_REQUIRED` result rather than infer finality.

### Focused tests and provenance

Only the minimum focused SQLx-core tests required to prove this seam may be
added or updated within the existing SQLx-core pool test surface. If a new test
file is necessary, it must be under
`vendor/sqlx-core-0.9.0/tests/` and named only for Oteryn pool-return finality.
`vendor/sqlx-core-0.9.0/OTERYN_PROVENANCE.md` may receive only the exact authored
symbol, source, and proof record for this amendment.

No game-server test, workflow, helper manifest, Cargo, or lockfile path is added
by this allocation.

## Required invariants

1. `RETURNED_TO_IDLE` is observable only after the existing idle-queue
   publication has succeeded at `PoolInner::release`; it must not be inferred
   from ping success alone.
2. `RETIRED_CLOSED` is observable only after the selected graceful or hard-close
   future has completed. A timeout, task spawn, dropped wrapper, permit change,
   maintenance call, or counter observation is not terminal close evidence.
3. Every explicit M05 return/retirement attempt produces at most one terminal
   outcome. Cancellation and Drop races cannot lose, duplicate, or reverse the
   outcome.
4. The existing connection is never simultaneously published idle and reported
   retired/closed. The outcome makes that mutual exclusion testable.
5. Ordinary `PoolConnection` Drop behavior, close-on-drop behavior, ping,
   lifetime expiry, `after_release`, minimum-connection maintenance, permit
   accounting, queue ordering, and pool policy remain behaviorally unchanged.
6. No additional pool capacity, retry, reconnect, maintenance loop, timer poll,
   numeric limit, or recovery generation is introduced. M05 remains the owner
   of Revision-3 demand/recovery policy.
7. The seam is allocation-bounded. Any completion representation must have
   explicit finite ownership and must not create an unbounded waiter/history
   collection or lose finality when the original future/handle is dropped.
8. The returned outcome is observability, not permission to convert a retired
   connection into ready state. M05 alone performs the accepted `T -> R`
   transfer and gates successor generations.

## Required focused proof

On one exact #356 successor, the SAME writer must prove at least:

- viable ping followed by actual idle publication yields exactly
  `RETURNED_TO_IDLE` and permits a ready checkout only after publication;
- pool-closed, max-lifetime, `after_release = false`, `after_release` error, and
  ping-error paths yield exactly `RETIRED_CLOSED` after the applicable close or
  hard-close completes;
- cancellation before first poll, during ping, after ping/before publication,
  and during close cannot fabricate a terminal outcome or lose the eventual
  completion obligation;
- ordinary Drop and close-on-drop keep their current spawning, timeout,
  maintenance, and permit behavior while a separately instrumented finality
  witness (where used by the explicit owner-aware path) resolves only at the
  truthful boundary;
- delayed return, delayed close, concurrent Drop/observer, and pool shutdown do
  not publish both outcomes or release generation custody early;
- idle-queue publication happens before permit release exactly as today;
- ordinary owner-free SQLx pool tests and behavior remain unchanged; and
- M05 configured PostgreSQL 17.6 tests subsequently demonstrate that no new
  recovery generation begins until every prior `R`/`T` return/close witness is
  terminal, including delayed-tail and cancellation cases.

Focused unit tests qualify only this seam. They do not by themselves prove M05,
the Revision-3 root equation, complete pool/root accounting, or WP3 completion.

## Explicit exclusions

No generic pool redesign; no new public stable pool contract; no pool size,
acquire timeout, idle timeout, max lifetime, min/max connection, reaper,
maintenance, fairness, semaphore, permit, or queue policy change; no Cargo or
lockfile mutation; no workflow/B01 or protected-control change; no PostgreSQL
protocol/decoder/SASL/TLS/rustls/Tokio change; no #335/WP4, WP5, Server Seam
#247, production, deployment, credential, live-data, or external-repository
authority.

This allocation does not reopen PROVEN frozen-inventory cells and does not claim
M05 or WP3 complete. The existing six-file M05 Durability lease remains distinct
and unchanged. It does not activate this pool seam until the lifecycle below is
complete.

## Lifecycle

```text
this one-document prospective allocation
-> exact-head repository checks
-> genuinely independent HIGH exact-head review with P0=P1=P2=0
-> owner-authorized native FULL Merge Queue
-> real merge_group game-gate SUCCESS
-> protected-main readback
-> fresh #162 main/head/open-PR/task/path-custody reconciliation
-> explicit SAME #351/#356 activation of this exact pool seam
-> focused RED/GREEN implementation and M05 composition qualification
```

Protected integration alone does not activate source authority. Direct merge,
generic auto-merge, bypass, force/rebase, no-op/retrigger commits, replacement
workers/branches/PRs, and protection weakening are forbidden.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
