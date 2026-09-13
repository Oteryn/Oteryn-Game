# Oteryn provenance — Tokio 1.53.1

## Immutable source

- Package: crates.io `tokio` 1.53.1.
- Download URL: `https://static.crates.io/crates/tokio/tokio-1.53.1.crate`.
- SHA-256: `202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`.
- Upstream repository commit: `75fef53d0a8590c2d1dbb63672aa7b7d1ef51155`.
- Upstream license: MIT, retained verbatim in `LICENSE`.

The complete package was extracted without normalizing untouched files. The delta
manifest records every archive file SHA-256 and the complete authored path set.
The root patch selects this exact package version; it does not change Tokio's
version or existing feature selection.

## Post-amendment RED and exact stop boundary

The first archive-import attempt at `53f9ad90b3cead9ad5a25d026910f0a6fcc00ff3`
failed before compiling the intended test because crates.io's normalized manifest
has automatic test discovery disabled. That setup failure receives no RED credit.
The corrected manifest registered the focused test, and distinct RED commit
`45da01b13b848785ad7fe068c6e100b7cc3eebe5` reached Tokio test compilation and
failed only with `E0432`: `tokio::task` has no `spawn_blocking_owned`,
`BlockingOwner`, `BlockingOwnerConfig`, or `OwnedSpawnError` surface.

Implementation then stopped before GREEN source because the protected authored
allowlist is insufficient by one exact source path:

`SHARED_LEASE_REQUIRED = vendor/tokio-1.53.1/src/task/mod.rs`

The required symbol is the `cfg_rt!` public re-export list currently containing
only `pub use blocking::spawn_blocking` (lines 280–281 in the exact package).
The authorized implementation location `src/task/blocking.rs` is a private module;
there is no public path by which SQLx can name an owned spawn function, owner
trait/configuration, or bounded denial error unless `src/task/mod.rs` re-exports
the new surface. SQLx cannot call a private module, and changing an unrelated
public Tokio type to smuggle the API would broaden semantics rather than remove
this need. No source file, including the listed Tokio owner/drop surfaces, was
modified after this discovery.

Smallest next amendment: authorize only the re-export statement in
`vendor/tokio-1.53.1/src/task/mod.rs` for the Oteryn owned-blocking API implemented
in already-listed `src/task/blocking.rs`. This does not authorize changes to
ordinary `spawn_blocking`, features, numeric limits, or any other task module.
After protection/application, resume the required owner queue/task/worker RED to
GREEN matrix. `TLS_BLOCKING_OWNER` remains unproven, so TLS composition,
PostgreSQL accounting, the shared include-only target, and real TLS/PG evidence
remain OPEN.

## Window5 protected task-export amendment checkpoint

Protected PR #417 and Work application #351 comment `5575953751` authorized the
single missing `src/task/mod.rs` re-export at protected
`main@feb6db96bd2fc93813cb120b874c61085f6dde45`. The amended RED at
`45da01b13b848785ad7fe068c6e100b7cc3eebe5` is preserved.

This checkpoint adds the fallible `spawn_blocking_owned` surface and a distinct
owned queue. A concrete generic `Cell<T, S>` charge is acquired before its Box
allocation and moved outside the allocation during final harness destruction so
release follows actual Cell deallocation. Owner-queue storage is separately
charged, finite, and never uses the ordinary blocking `VecDeque`; an owned-created
worker requires explicit nonzero stack configuration, reserves stack and concrete
bookkeeping before `thread::Builder::spawn`, and holds the reservation across idle
keep-alive until its exit. Ordinary `spawn_blocking` continues through the
unchanged upstream entry point.

Focused controls prove insufficient-balance denial, checked overflow, funded
execution, dropped-handle retention while work is running, idle worker retention,
and final shutdown release. The full amendment gate is nevertheless **not yet
claimed proven**: queue-full pre-admission ordering, queued abort, forced OS-spawn
failure, loom concurrency, SQLx ledger adaptation, complete TLS phase composition,
and actual TLS-positive/PostgreSQL 17.6 qualification remain OPEN. No PostgreSQL
source or shared test target was changed.

`TLS_BLOCKING_OWNER = NOT_PROVEN` at this intermediate GREEN checkpoint; WP3 is
not accepted or ready for integration.

## Window6 owned-Tokio prerequisite completion

Starting from intermediate-GREEN `6438a8c0a168b722672ab4e48bffc83687da28a3`,
owner-queue admission now checks the configured queue while holding the pool lock
before constructing `BlockingTask<F>`, reserving the generic task Cell, or calling
`task::unowned_oteryn`. A full owner queue therefore returns `QueueFull` without a
new owner reservation and cannot enqueue work on Tokio's ordinary blocking queue.

Queued cancellation retains the task charge while the cancelled task remains in
the owner queue. The worker removes it, runs the cancellation destruction path,
and only final Cell deallocation releases the charge. A deterministic owner
control forces the OS-worker spawn failure branch after worker reservation; that
branch removes and destroys the just-enqueued task, releases the never-created
worker backing exactly once, retains the still-real owner queue backing, and
releases that queue backing only with runtime destruction.

The pinned crate's existing Loom surface is `src/runtime/tests/loom_blocking.rs`.
It does not expose the new private owned queue/task/worker path, and adding a new
unit-test registration there or a new feature/source route is outside the focused
test allowlist. No new route was invented merely to claim Loom coverage. The
allocated `tests/oteryn_resource_owner.rs` surface instead supplies deterministic
multi-threaded concurrent admission and exact reservation/release equality, in
addition to queue-full and cancellation races. Ordinary upstream
`spawn_blocking` remains a separate passing control.

These controls close the remaining Tokio-only prerequisite. SQLx ledger
adaptation, fail-closed non-Tokio dispatch, complete TLS phase composition, real
TLS-positive evidence and PostgreSQL 17.6 qualification remain OPEN.
`TLS_BLOCKING_OWNER = NOT_PROVEN` until those SQLx/TLS cells complete.

## Window13 distinct operation-owner repair

Review P1 `3947483202` correctly identified that the original single retained
`OwnedQueue` made a Tokio runtime permanently reject every later operation owner.
The owner path now maintains one separately allocated and charged linked queue
node per owner identity. Each node owns its finite task backing and worker-lifetime
state; neither the node nor its task storage is charged to another operation, and
the ordinary blocking queue remains untouched. Worker wakeups search only the
matching identity and custody for every owner remains live through its worker's
idle lifetime and runtime shutdown.

The focused distinct-owner control submits completed jobs from two independently
created owners on one current-thread runtime, proves both retain nonzero custody
simultaneously, and proves each returns independently to zero at shutdown. The
existing queue-full, cancellation, forced-spawn-failure, overflow and ordinary
path controls remain green. The additional queue-node reservation is included in
the forced-spawn-failure exact-once count.

This dispositions P1 as **PROVEN/FIXED** for the Tokio prerequisite only. The
caller-supplied SQLx operation-owner propagation required by protected #430 has
not yet been implemented, so #429 ALPN work and all later TLS/PG cells remain
OPEN and no WP3 completion is claimed.

## E2 registration owner implementation — 2026-09-13

Allocation: `OTV2-WP3-TOKIO-IO-REGISTRATION-OWNER-20260912`, activated on
canonical #351/#356 by #162 comment 5655057468; dispatch 5655229270.
Implementation base: `db1257a27f2c1c131331866da79fbc4151e72f32`.
This is a bounded source candidate, **not E2 GREEN or terminal WP3**.
Agent identity: A4 implementation worker (not the operation-owner identity).

### Allocation, identity and finality proof

The doc-hidden public TCP entrypoint accepts `std::net::SocketAddr` only.
The public Unix entrypoint preserves the ordinary path/abstract-address
validation family. Both pass the supplied `Arc<dyn BlockingOwner>` unchanged
through their private constructors, `PollEvented`, `Registration`, and
`Handle::add_source` into `RegistrationSet::allocate`. No owner, ledger,
resolver, task, or registry is minted. Ordinary constructors pass `None`.
Owned entrypoints require `rt`, where the existing capability is available;
owner-free net-only compilation is preserved. TCP preserves the ordinary
connect family's WASIp1 exclusion.

`RegistrationHandle` exclusively encapsulates `Arc<RegistrationBacking>`.
The backing contains the unchanged inline `ScheduledIo` and, with `rt`, an
optional existing `OterynCharge`. Reservation occurs before constructing the
Arc, before list insertion, and before OS registration. Shutdown rejection
also occurs before reservation. The charged request is
`size_of::<ArcLayout>()`, where the local `repr(C, align(2))` mirror contains
two `AtomicUsize` counters followed by the concrete backing. This includes
actual payload alignment/padding and both Arc counters; it is not handle
size or a guessed allocator overhead. Allocator implementation overhead is
outside the requested-layout claim.

Primary pinned proof: rust-lang/rust tag `1.94.0`,
`library/alloc/src/sync.rs`, Git blob
`4180fe91cb558bb12044ddeb09c4c96d7202bab5`, independently fetched through the
GitHub file API and corroborated by the raw file's Git hash. `ArcInner`
(lines 377–401) is `repr(C, align(2))` with strong/weak atomic usize counters
and data; `Arc::new` allocates that structure with `Box::new`.
`Arc::into_inner` (1218–1246) suppresses ordinary Arc drop, atomically consumes
one strong reference, and on the unique final reference reads the payload,
drops the implicit Weak, then returns it. `Weak::drop` (3443–3495) deallocates
the backing when its count reaches zero. No Weak, Arc clone, Arc raw pointer,
or ordinary Arc final-drop capability escapes the private E2 handle. All
strong-handle drops call `into_inner`; unlike `try_unwrap(...).ok()`, concurrent
final drops cannot bypass the unique returned-payload path.

The intrusive list continues targeting the exact inline `ScheduledIo`.
`addr_of!` preserves its backing provenance; `offset_of!(RegistrationBacking,
io)` recovers its containing allocation in `Link::from_raw`. The list's
ManuallyDrop custody is reconstructed as a handle, never an escaping Arc.
The active list, pending-release vector, returned shutdown vector and socket
all retain the same charge. List links are detached before list ownership
ends; driver event pointers remain protected by that existing custody.
Moving the returned `ScheduledIo` occurs only after all strong registration
owners end, so no registration/readiness borrow or linked-list pinning
obligation remains. Final handle drop first obtains the payload after Arc
backing deallocation, then destroys `ScheduledIo`, then drops its charge.

OS registration failure retains the existing unconditional list removal
before returning the error; both local and list references now use the same
finality handle. `Handle::deregister_source` still records the OS result and
unconditionally queues cleanup before returning that result, including its
error branch. No readiness or `ScheduledIo` code changed. These OS-error
branches are source-audited; deterministic OS-error injection is not claimed.

### Focused qualification and remaining gates

Tests are external integration-crate callers in the sole allocated test
file; all new function names begin `oteryn_io_registration_owner_` and helper
items are nested inside those functions. The allocator witness is likewise
nested: only that test's thread arms it from owner reservation. It records
actual allocation layout and calls `System::dealloc` before recording death;
owner release asserts that death, exact byte equality and one release.
The full-feature Linux x86_64 test observed a 384-byte request. The value is
profile-specific evidence; the implementation computes each concrete layout.
The same witness checks retained readiness-waker destruction and concurrent
socket/runtime final teardown. Source proof, rather than timing sampling,
supplies the universal concurrent-finality claim.

| Amendment proof | Status on this source candidate |
| --- | --- |
| 1 TCP exact funding / max-minus-one | PROVEN by allocator-observed external test; denial has no tracked allocation or debit release. |
| 2 UDS exact funding / max-minus-one | UNKNOWN at runtime: retained tests compile, but this executor denies AF_UNIX creation with EPERM before Tokio runs. |
| 3 Pending-release custody | PROVEN for TCP socket drop/cancellation and subsequent live driver turn; common-core source covers retained handles. |
| 4 Backing destruction then one release | PROVEN by pinned source plus allocator/waker witness, including shutdown with surviving socket and concurrent teardown. |
| 5 Failure/cancellation/shutdown | PROVEN for TCP cancellation, failed connect, registration after shutdown and normal cleanup. OS register-error/deregister-error cleanup is DERIVED from unchanged unconditional branches, not injected runtime evidence. UDS runtime cases remain UNKNOWN. |
| 6 Same owner identity | PROVEN source census: supplied Arc moves into one existing charge; no second owner construction. |
| 7 Ordinary owner-free regressions | TCP PROVEN (16 existing tests); UDS runtime UNKNOWN under the same AF_UNIX restriction. |
| 8 Allocation/finality census | PROVEN source: one registration backing allocation, private Arc field, one clone implementation, one list reconstruction, one into_inner release path; no Weak or alternate owned constructor path. |
| 9 Toolchain/composition/governance | Rust 1.94 focused strict Clippy, net-only check, game-server all-target composition check, formatting and repository governance/policy pass locally. Exact candidate hosted qualification remains required. |
| 10 Excluded debts | PROVEN unchanged: no E3/DNS or std-thread 3993253957 closure; final-owner 3993253945 remains an end-to-end prerequisite. |
| 11 External entrypoints | PROVEN external integration-crate compilation of TCP and Unix APIs; TCP calls execute locally, Unix runtime remains blocked. |

Initial focused tests were RED because the owned APIs did not exist. Native
Rust 1.94 subsequently passed the TCP focused tests and existing TCP/owner
regressions. The complete E2 run is deliberately **not reported green**:
Unix tests fail at native socket creation (`EPERM`) and are neither ignored
nor weakened. The root composition check passes with pre-existing SQLx
warnings; no SQLx or consumer source was changed. Independent exact-head
review and the separately allocated hosted qualification step remain required.
This append does not close Q01–Q75, DNS E3, std-thread accounting, or WP3.

The charged-resource claim is limited to each registration Arc/control/payload
backing and custody of its debit. Existing pending-release vector capacity,
shutdown-vector capacity, driver/runtime allocations and socket/provider
resources are not newly charged by this cell and must retain their separate
composition proofs; this cell establishes no universal runtime resource bound.
