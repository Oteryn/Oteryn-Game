# WP3 runtime-owner finality and owned-thread spawn decision

- Decision ID: `WP3-RUNTIME-OWNER-FINALITY-AND-SPAWN-V1`
- Status: candidate repaired after independent review; not accepted until fresh
  exact-head review and protected integration
- Date: 2026-09-12
- Governing issues: #162, #351
- Existing implementation lineage: Draft PR #356,
  `agent/sqlx-driver-budget-351`
- Finding resolved by this candidate: `3993253945`
- Finding still unresolved: `3993253957`
- Independent review evidence: PR #579 comment `5644888166`
- Related protected repair: `3993253965` remains the required external-join
  physical-thread finality implementation and is not weakened here.

## Decision timing

1. **Must decide now? YES for runtime-owner finality.** The #351 writer cannot
   truthfully repair the final `Arc<BudgetOwner>` debit without selecting a
   concrete final-owner representation. Field reordering and post-allocation
   charging are invalid.
2. **Owned-thread metadata finality is not decided.** Independent review proved
   that successful native join is not sufficient to prove destruction of every
   Rust thread metadata/name backing allocation because `std::thread::Thread`
   handles may escape and outlive join.
3. **Blocked downstream work.** WP3 final TLS/PostgreSQL resource qualification,
   WP4/#335 activation, WP5 composition, and Server Seam #247 remain blocked on
   terminal protected WP3.
4. **Harder later.** Allowing raw owner `Arc` clones or treating join as a final
   owner for all thread metadata would bake early release into future blocking
   owners.
5. **Evidence that can supersede the thread blocker.** A reviewed mechanism that
   proves custody until the last escaping `std::thread::Thread` metadata owner is
   destroyed, a pinned-toolchain/runtime mechanism that makes such escape
   impossible, or another explicitly reviewed fail-closed representation may
   resolve `3993253957`.
6. **Deliberately not decided.** No production numeric limit, allocator overhead
   policy, scheduler redesign, general Tokio naming policy, SQLx public contract,
   production deployment behavior, or custom native-thread runtime is selected.

## Problem

The current concrete `Arc<BudgetOwner>` stores its own Arc-control allocation
reservation inside `BudgetOwner`. On final Arc destruction, the inner value is
dropped before the Arc allocation is deallocated, so the reservation releases
too early. Moving the field or placing the reservation in the SQLx wrapper does
not help because Tokio-held owner clones can outlive that wrapper.

The owned Tokio blocking-worker path also invokes an arbitrary `thread_name`
callback and uses Rust 1.94 `std::thread::Builder::spawn`. The callback allocates
an unbounded `String`; standard spawn then creates private CString/thread/packet/
closure backing before native spawn. Their exact capacity cannot be charged
after allocation without violating reservation-before-allocation.

Independent review additionally proved that **join is not a complete finality
boundary for Rust thread metadata**. Code running on the worker can call
`std::thread::current()`, clone the returned `std::thread::Thread`, and retain or
return that handle beyond successful native join. That handle retains Arc-backed
thread metadata/name state. Therefore a combined name/control reservation cannot
truthfully be released merely because an outside thread joined the native worker.

## Constraints

- Reuse the existing caller-supplied `ResourceBudget`; no second ledger.
- The final owner-control debit outlives actual Arc control-block deallocation.
- All Tokio-owned strong references participate in one finality protocol; no raw
  strong Arc bypass may coexist.
- Preserve ordinary upstream owner-free `spawn_blocking` and its configured
  naming behavior.
- Owned worker/stack/thread-slot custody remains live until an outside-thread
  `JoinHandle` observes native termination. Timed shutdown is not termination.
- Any thread metadata/name debit that can remain retained by an escaping
  `std::thread::Thread` handle must remain charged beyond join unless a reviewed
  final-owner mechanism proves later destruction.
- No private-layout guess, post-allocation catch-up, scheduler semantic change,
  silent omission, or silently reduced resource coverage.

## Options considered

### 1. Store/reorder a reservation in `BudgetOwner` or `BlockingJobOwner`

Rejected. Rust drops the value before deallocating the enclosing Arc allocation,
and SQLx wrapper lifetime is shorter than Tokio's last owner clone.

### 2. Preserve arbitrary thread names and estimate Rust spawn backing

Rejected. The callback's `String` allocation is unbounded before invocation and
Rust 1.94's private spawn packet allocations cannot be reconstructed truthfully
from a caller after allocation.

### 3. Create a custom native thread implementation now

Not selected in this candidate. It expands platform, cancellation, panic,
TLS/runtime, and maintenance risk beyond the smallest WP3 repair. It remains a
possible **future architecture candidate** only if a bounded reviewed design is
required to resolve `3993253957`.

### 4. Concrete finality handle for `Arc<BudgetOwner>`

Selected for `3993253945`. It keeps ordinary Tokio unchanged while providing an
exact final-owner boundary for the owner Arc control allocation.

### 5. Fixed owned-worker name + precharged spawn contract + release at join

Only partially acceptable. A fixed name and prospective precharge may solve the
**preallocation** side of the owned-spawn problem, but independent review proved
that join alone does not prove destruction of all Arc-backed Rust thread
metadata/name state. Therefore the previous claim that one combined spawn debit
may be released after join is rejected.

No implementation may treat this option as resolving `3993253957`.

## Decision

### A. Runtime owner finality representation — selected

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
the final owner-control debit cannot release until all of those concrete owner
handles are gone.

### B. Owned-thread spawn preallocation — bounded prerequisite only

The exceptional Oteryn-owned blocking path may use a fixed ASCII name such as
`oteryn-owned-blocking` and may use a private owner-aware **precharged** spawn
contract only if every controlled allocation is prospectively bounded from the
pinned Rust 1.94 implementation. Ordinary Tokio workers must continue to use the
upstream callback unchanged.

A prospective spawn charge must be split by actual lifetime/finality instead of
being treated as one join-released packet. Stack/thread-slot/native-worker cells
whose destruction is actually proven by external join may follow the existing
`3993253965` join boundary. Rust thread metadata/name/control cells that may be
retained by an escaping `std::thread::Thread` handle **must not** be released at
join.

This candidate does not provide a truthful mechanism for observing the final
destruction of every escaping `std::thread::Thread` metadata owner. The standard
library's cloneable `Thread` handle is outside the current private owner-finality
domain, and this candidate does not authorize a stdlib patch, custom thread
runtime, permanent accounting leak, or semantic restriction on worker code.

Accordingly:

- `3993253957` remains **OPEN**;
- the #351/#356 material writer must not implement or claim a join-finalized
  thread-metadata/name debit from this document;
- any proposed final-owner mechanism, deliberate permanent-charge policy, custom
  native-thread path, or other representation is a new material architecture
  decision requiring explicit review/authority before implementation; and
- terminal WP3 integration remains blocked until that decision is resolved and
  independently qualified.

## Required proof for the selected owner-finality decision

1. Max-minus-one rejects before the owner Arc allocation.
2. SQLx wrapper drop while Tokio owner handles remain does not release the Arc
   control debit.
3. Concurrent last-handle drops elect exactly one final releaser after Arc
   control-block deallocation; no raw/Weak bypass exists.
4. Task, queue, worker and owned-to-ordinary handoff preserve the same concrete
   owner identity and do not allocate for identity comparison.
5. Existing ordinary `spawn_blocking`, naming, scheduling and owner-free behavior
   remain unchanged.

## Required evidence before any successor closes `3993253957`

A successor architecture must include a focused proof where code executing on an
owned worker obtains and retains a clone of `std::thread::current()` across native
return and successful outside-thread join. The evidence must demonstrate that the
charged thread metadata/name backing remains accounted for until its **actual**
final owner is gone, or must establish another reviewed representation that
truthfully avoids or contains that backing without post-allocation catch-up.

A test that observes only worker return, `JoinHandle::join`, thread-cap release,
or destruction of the native OS thread is insufficient.

## Risks and mitigations

- **Pinned-toolchain coupling:** prospective spawn footprint data is
  Rust-version-sensitive. Mitigation: bind any later implementation to pinned
  source and fail closed on toolchain/layout drift.
- **Owner-finality protocol misuse:** one escaped owner Arc/Weak could invalidate
  release. Mitigation: private constructors/types, exhaustive clone/drop tests,
  and whole-surface audit before integration.
- **Thread metadata escape:** a cloneable `std::thread::Thread` may outlive join.
  Mitigation in this candidate: do not claim the cell resolved; keep
  `3993253957` as an explicit architecture blocker.
- **Availability pressure:** permanently retaining a charge would preserve safety
  but can change long-term resource availability. This candidate does not adopt
  that policy implicitly; it would require a separate reviewed decision.

## Player and producer impact

The selected owner-finality repair prevents early release of the owner Arc debit.
The unresolved thread-metadata cell remains fail-closed at the architecture
boundary rather than being hidden by a join-based accounting shortcut. No player
or producer behavior is authorized to change until the remaining blocker is
resolved and the canonical implementation is protected.

## Integration and authority boundary

This candidate now records **one selected material decision and one explicit
material blocker**:

- `3993253945`: concrete owner Arc finality representation selected, subject to
  fresh exact-head independent review and protected integration;
- `3993253957`: unresolved because join cannot prove destruction of all escaping
  Rust thread metadata/name backing.

The companion BufferedSocket amendment for `3993253930` remains a separate,
already-reviewed-as-coherent allocation candidate in this same PR.

If PR #579 is later independently accepted and protected, Work may separately
activate only the exact scopes that are actually resolved and non-overlapping.
It must not activate a `3993253957` implementation from this document. Terminal
WP3/#356 remains held until a successor architecture resolves that one material
thread-metadata finality decision.

WP4/#335, WP5, Server Seam #247, production, deployment, credentials, PKI,
secrets, external repositories, branch protection and required checks remain
held. `MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`.
