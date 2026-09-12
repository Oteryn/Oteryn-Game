# WP3 runtime-owner finality and owned-thread spawn decision

- Decision ID: `WP3-RUNTIME-OWNER-FINALITY-AND-SPAWN-V1`
- Status: candidate; not accepted until independently reviewed and protected
- Date: 2026-09-12
- Governing issues: #162, #351
- Existing implementation lineage: Draft PR #356,
  `agent/sqlx-driver-budget-351@2e37c9025d19e9a9f7c1b4dbc13e36f70dd58fdb`
- Findings resolved by this decision: `3993253945`, `3993253957`
- Related protected repair: `3993253965` remains the required external-join
  physical-thread finality implementation and is not weakened here.

## Decision timing

1. **Must decide now? YES.** The #351 writer cannot truthfully repair the final
   `Arc<BudgetOwner>` debit or owned-thread name/spawn packet without selecting a
   representation. Field reordering and post-allocation charging are invalid.
2. **Blocked downstream work.** WP3 final TLS/PostgreSQL resource qualification,
   WP4/#335 activation, WP5 composition, and Server Seam #247 all remain blocked
   on terminal protected WP3.
3. **Harder later.** Allowing raw owner `Arc` clones or arbitrary thread-name
   callbacks to remain in the owned path would bake early release and
   unaccounted spawn allocations into every future blocking owner.
4. **Evidence that can supersede this decision.** A Rust/Tokio API that natively
   exposes fallible preallocation and post-deallocation custody, a proven
   allocation-free native spawn primitive, or measured implementation evidence
   showing this representation cannot preserve upstream behavior may justify a
   reviewed successor.
5. **Deliberately not decided.** No production numeric limit, allocator overhead
   policy, scheduler redesign, general Tokio naming policy, SQLx public contract,
   or non-WP3 runtime ownership API is selected.

## Problem

The current concrete `Arc<BudgetOwner>` stores its own Arc-control allocation
reservation inside `BudgetOwner`. On final Arc destruction, the inner value is
dropped before the Arc allocation is deallocated, so the reservation releases
too early. Moving the field or placing the reservation in the SQLx wrapper does
not help because Tokio-held owner clones can outlive that wrapper.

The owned Tokio blocking-worker path also invokes an arbitrary `thread_name`
callback and uses Rust 1.94 `std::thread::Builder::spawn`. The callback allocates
an unbounded `String`; standard spawn then creates private CString/thread/packet/
closure backing before native spawn. Their exact capacity cannot be discovered
or charged afterward without violating reservation-before-allocation. A guessed
allowance is not acceptable.

## Constraints

- Reuse the existing caller-supplied `ResourceBudget`; no second ledger.
- The final owner-control debit outlives actual Arc control-block deallocation.
- All Tokio-owned strong references participate in one finality protocol; no raw
  strong Arc bypass may coexist.
- Preserve ordinary upstream owner-free `spawn_blocking` and its configured
  naming behavior.
- Owned worker/stack/thread-slot custody remains live until an outside-thread
  `JoinHandle` observes native termination. Timed shutdown is not termination.
- No private-layout guess, post-allocation catch-up, scheduler semantic change,
  or silently reduced resource coverage.

## Options considered

### 1. Store/reorder a reservation in `BudgetOwner` or `BlockingJobOwner`

Rejected. Rust drops the value before deallocating the enclosing Arc allocation,
and SQLx wrapper lifetime is shorter than Tokio's last owner clone.

### 2. Preserve arbitrary thread names and estimate Rust spawn backing

Rejected. The callback's `String` allocation is unbounded before invocation and
Rust 1.94's private spawn packet allocations cannot be reconstructed truthfully
from a caller after allocation.

### 3. Create a custom native thread implementation now

Rejected for this slice. It expands platform, cancellation, panic, TLS/runtime,
and maintenance risk beyond the smallest WP3 repair and would fork ordinary
Tokio scheduling behavior.

### 4. Concrete finality handle plus owner-aware precharged owned-spawn contract

Selected. It keeps ordinary Tokio unchanged while making the exceptional owned
path explicit and fail-closed.

## Decision

### A. Runtime owner finality representation

The Oteryn-owned blocking path uses one private, concrete, non-allocating erased
owner handle whose strong references all refer to the same `Arc<BudgetOwner>`
allocation. Creation reserves the exact Arc allocation before `Arc::new`. The
reservation remains in the concrete inner value, but raw `Arc<dyn BlockingOwner>`
clones are forbidden after conversion to the private handle.

Every handle clone/drop participates in a single concrete finality protocol.
The last handle consumes its strong reference using the Rust 1.94
`Arc::into_inner` final-owner guarantee. Exactly one concurrent final consumer
receives the inner `BudgetOwner`; the Arc control block has been deallocated
before `into_inner` returns that value when no Weak references exist. Only that
consumer then destroys the returned owner and releases the pre-acquired control
debit. Non-final consumers release nothing.

The implementation must remain safe Rust unless a separately reviewed proof
shows that private type erasure requires `unsafe`. If `unsafe` or another path is
required, the worker stops for exact authority. No Weak owner reference and no
raw strong Arc may escape the finality domain. Pointer identity for per-owner
queue selection must remain stable and must not allocate.

This representation is compatible with the already accepted external-join
worker repair: task, queue, and worker handles keep the same concrete owner alive;
the final control debit cannot release until all of them, including the externally
joined worker custody, are gone.

### B. Owned worker naming and spawn representation

The exceptional Oteryn-owned blocking path no longer accepts or invokes the
runtime's arbitrary `thread_name: Fn() -> String` callback. It uses the fixed
ASCII name `oteryn-owned-blocking` as the only V1 owned-worker name. Ordinary
Tokio workers continue to use the upstream callback unchanged.

The owned path must call a private **owner-aware precharged spawn contract**. The
contract is fallible and accepts an already-live reservation acquired before any
owned name or Rust spawn-packet allocation. Its required charge is not a magic
constant: it is the checked sum produced by one private Rust-1.94-bound footprint
function for exactly:

- the fixed name byte storage and terminator;
- the concrete closure/environment payload used by this owned worker;
- Rust thread metadata/control and packet layouts that the implementation can
  prove from the pinned toolchain source; and
- the configured worker stack and already accepted native worker bookkeeping,
  without double counting existing charges.

If any required Rust 1.94 private allocation has no truthful finite preallocation
bound available to the implementation, spawn returns resource-unavailable before
invoking `std::thread::Builder::spawn`; it must not guess, charge afterward, omit
the cell, or fall back to the ordinary unowned path. This is an intentionally
fail-closed owner-aware contract, not permission to patch the standard library.

The reservation transfers into the outside-thread `JoinHandle` custody selected
by the `3993253965` repair. It remains live through the name, spawn packet, native
thread, owned-to-ordinary handoff, and actual native termination. Only an
outside-thread successful join may release it. Failed spawn destroys all created
backing before release. Shutdown timeout retains or deliberately leaks the charge
with the still-running thread; timeout completion never releases it.

## Required implementation proof

1. Max-minus-one rejects before the owner Arc allocation; SQLx wrapper drop while
   Tokio clones remain does not release; concurrent last-handle drops elect one
   final releaser after control-block deallocation; no raw/Weak bypass exists.
2. The owned path never calls the arbitrary thread-name callback; ordinary
   workers still do. The fixed name is observed where the platform exposes it.
3. Spawn-footprint checked arithmetic rejects overflow and insufficient budget
   before any owned spawn allocation. Every charged cell is source-linked to the
   pinned Rust 1.94 implementation; an unprovable cell causes fail-closed denial.
4. Spawn failure, task cancellation, panic/error, detached completion, and normal
   shutdown release exactly once after their backing is destroyed.
5. Worker and spawn charge remain live at `before_stop`, through native return,
   and until another thread joins. `thread_cap=1` cannot admit a second physical
   owned worker while the first has not been externally joined.
6. Owned-to-ordinary handoff retains original custody. Bounded inline reaping
   grows no uncharged map/history. Timed shutdown never releases a live thread.
7. Existing ordinary `spawn_blocking`, naming, scheduling, and owner-free behavior
   remain unchanged.

## Risks and mitigations

- **Pinned-toolchain coupling:** the spawn footprint is Rust-version-sensitive.
  Mitigation: bind it to Rust 1.94 source, test every cell, and fail closed on
  toolchain/layout drift rather than silently retaining an obsolete formula.
- **Finality protocol misuse:** one escaped Arc/Weak could invalidate release.
  Mitigation: private constructors/types, exhaustive clone/drop tests, and a
  whole-surface audit before integration.
- **Reduced owned-worker configurability:** the exceptional path has one fixed
  name. This affects diagnostics only; ordinary Tokio naming is preserved.
- **Availability under unknown backing:** owned spawn may deny rather than run.
  This is the accepted safe outcome until a truthful bound exists.

## Player and producer impact

Players receive fail-closed admission rather than memory-budget bypass or a
nominal slot while an old physical thread still exists. Producers accept a
pinned-toolchain internal contract and one fixed diagnostic thread name in the
owned path, avoiding a custom cross-platform thread runtime and preserving the
ordinary Tokio API.

## Integration and authority boundary

This candidate selects semantics; it grants no implementation authority by
itself. After independent HIGH-risk exact-head review and protected Merge Queue
readback, Work must confirm the existing #351 paths/symbols cover the concrete
repair and explicitly reactivate the same #351/#356 writer. Any additional path,
`unsafe` representation, standard-library patch, or inability to prove a spawn
cell returns to an exact architecture/shared-lease boundary.

WP4/#335, WP5, Server Seam #247, production, and external repositories remain
held. `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
