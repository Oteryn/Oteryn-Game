# WP3 Tokio I/O registration owner amendment

Coordinator: #162. Programme: #364. Existing sole material writer: #351 / Draft PR #356.

## State

```yaml
allocation_id: OTV2-WP3-TOKIO-IO-REGISTRATION-OWNER-20260912
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-tokio-io-registration-owner-351
worker_branch: agent/sqlx-driver-budget-351
source_decision: OTV2-WP3-SQLX-CONNECT-SOCKET-OWNER-20260912
source_protected_merge: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
source_blocker: SHARED_LEASE_REQUIRED_TOKIO_IO_REGISTRATION
risk: HIGH
```

This is a prospective symbol-bounded amendment for the SAME canonical #351/#356 material writer. It grants no source mutation until protected integration, protected-main readback and a fresh #162 activation. It creates no second Tokio/runtime owner.

## Why this amendment is needed

Protected #583 proves that owner-aware PostgreSQL TCP and UDS establishment converge on the same retained Tokio registration allocation:

```text
TCP: TcpStream constructor path
 -> PollEvented
 -> Registration
 -> Handle::add_source
 -> RegistrationSet::allocate
 -> Arc::new(ScheduledIo)

UDS: UnixStream constructor path
 -> PollEvented
 -> Registration
 -> Handle::add_source
 -> RegistrationSet::allocate
 -> Arc::new(ScheduledIo)
```

`RegistrationSet::deregister` clones the registration Arc into `pending_release`; later driver release removes it from the registration set. Therefore SQLx socket drop or initial `Registration` teardown is too early to release the registration backing debit.

## Exact prospective symbol lease

**Paths are not authority by themselves. Only the symbols listed below become writable after later explicit #162 activation. Every unlisted symbol in every listed file remains read-only and requires a new `SHARED_LEASE_REQUIRED` amendment.**

### `vendor/tokio-1.53.1/src/net/tcp/stream.rs`

Permitted existing symbols, only for owner propagation into the common registration constructor while preserving the owner-free path:

- `TcpStream::connect_addr`;
- `TcpStream::connect_mio`;
- `TcpStream::new`.

Permitted new private owner-aware counterparts if the implementation needs them:

- `TcpStream::connect_addr_oteryn_owned`;
- `TcpStream::connect_mio_oteryn_owned`;
- `TcpStream::new_oteryn_owned`.

`TcpStream::connect<A: ToSocketAddrs>` is **not writable by E2** because hostname resolution is E3. Ordinary public TCP connect behavior remains unchanged.

### `vendor/tokio-1.53.1/src/net/unix/stream.rs`

Permitted existing symbols, only for owner propagation into the same registration constructor:

- `UnixStream::connect`;
- `UnixStream::connect_addr`;
- `UnixStream::connect_mio`;
- `UnixStream::new`.

Permitted new private owner-aware counterparts if required:

- `UnixStream::connect_oteryn_owned`;
- `UnixStream::connect_addr_oteryn_owned`;
- `UnixStream::connect_mio_oteryn_owned`;
- `UnixStream::new_oteryn_owned`.

No other Unix socket/listener/datagram symbol is allocated.

### `vendor/tokio-1.53.1/src/io/poll_evented.rs`

Permitted existing symbols, only to route the owner-aware stream constructor to `Registration` while leaving ordinary callers unchanged:

- `PollEvented::new`;
- `PollEvented::new_with_interest`;
- `PollEvented::new_with_interest_and_handle`;
- `PollEvented::into_inner` only if needed to preserve owner-aware deregistration/finality.

Permitted new private helpers:

- `PollEvented::new_oteryn_owned`;
- `PollEvented::new_with_interest_and_handle_oteryn_owned`.

Readiness polling, read/write I/O, process re-registration and unrelated `PollEvented` behavior are not allocated.

### `vendor/tokio-1.53.1/src/runtime/io/registration.rs`

Permitted type surface:

- `Registration` fields only to the minimum extent required to retain the E2 owner/finality capability associated with its exact `ScheduledIo` allocation.

Permitted existing methods:

- `Registration::new_with_interest_and_handle`;
- `Registration::deregister`.

Permitted new private helper:

- `Registration::new_with_interest_and_handle_oteryn_owned`.

Readiness polling and unrelated methods are not allocated.

### `vendor/tokio-1.53.1/src/runtime/io/registration_set.rs`

Permitted existing types only to the minimum field/type changes required by registration custody:

- `RegistrationSet`;
- `Synced`;
- the existing `linked_list::Link` implementation only if the owner-aware registration handle replaces the current Arc handle and the link implementation must follow that exact replacement.

Permitted existing methods:

- `RegistrationSet::new` only if owner-aware retained handle type initialization requires it;
- `RegistrationSet::allocate`;
- `RegistrationSet::deregister`;
- `RegistrationSet::shutdown`;
- `RegistrationSet::release`;
- `RegistrationSet::remove`.

Permitted new private owner/finality handle type and methods are limited to the registration allocation itself, for example an `OterynOwnedScheduledIo`-equivalent wrapper that:

- preserves direct access/pointer identity to the same `ScheduledIo`;
- carries one charge/finality capability across every strong retained registration owner;
- releases only after the final charged registration backing is actually deallocated;
- introduces no independently growing registry/history/queue.

The exact helper name may differ, but no generic reusable Tokio owner framework is authorized by this allocation.

### `vendor/tokio-1.53.1/src/runtime/io/driver.rs`

Permitted existing symbols only:

- `Driver::shutdown`;
- `Driver::turn` only to the minimum extent necessary to preserve/release pending registration custody at its existing release point;
- `Handle::add_source`;
- `Handle::deregister_source`;
- `Handle::release_pending_registrations`.

No other `Driver`, `Handle`, scheduler, metrics, poll-loop, io-uring, timer, signal or process behavior is allocated.

### Explicitly read-only: `vendor/tokio-1.53.1/src/runtime/io/scheduled_io.rs`

No `ScheduledIo` field or method mutation is authorized by this amendment. Protected source already exposes the exact allocation at `RegistrationSet::allocate`; E2 must first solve custody at the registration owner/finality layer above it. If implementation proves a `ScheduledIo` mutation is inseparable, stop and return:

```text
SHARED_LEASE_REQUIRED = vendor/tokio-1.53.1/src/runtime/io/scheduled_io.rs :: exact symbol/resource :: reason
```

### Focused test/provenance surface

Only these non-production additions/updates are prospectively permitted:

- `vendor/tokio-1.53.1/tests/oteryn_resource_owner.rs` — add focused test functions whose names begin `oteryn_io_registration_owner_`; existing unrelated tests remain read-only;
- `vendor/tokio-1.53.1/OTERYN_PROVENANCE.md` — append E2 source/proof provenance only;
- `vendor/tokio-1.53.1/OTERYN_DELTA_MANIFEST.json` — append/update only entries corresponding to the exact E2 symbols above.

No other Tokio path or symbol is allocated.

## Required E2 implementation contract

### One exact operation owner

One owner-aware PostgreSQL establishment operation must establish/reuse exactly one runtime-owner identity before DNS/TCP/UDS. E2 receives that same identity and must not create another `BudgetOwner`, `BlockingJobOwner`, queue owner or independent ledger.

P1 `3993253945` final-owner correctness remains a prerequisite for end-to-end terminal composition. E2 may be developed path-disjointly but may not claim final correctness until that prerequisite is GREEN.

### Reserve before registration allocation

The owner-aware TCP/UDS path must reserve the exact charged registration backing before `Arc::new(ScheduledIo)` in `RegistrationSet::allocate`. Post-allocation catch-up is forbidden. The proof must account for the actual requested Arc/control backing on the pinned target, not `size_of::<Arc<_>>()`, a magic constant or allocator-overhead percentage.

### Custody through final backing death

The charge must survive all retained owners that can keep the registration backing live, including active registration-set custody, socket/`Registration`, `pending_release`, deregistration cleanup and shutdown. Release is permitted exactly once and only after the final charged backing is actually deallocated.

No raw/Weak/ordinary final-drop bypass may reproduce the finality defect represented by `3993253945`.

### TCP and UDS share one core

Both accepted owner-aware connect families must reach the same owner-aware `PollEvented`/`Registration`/`RegistrationSet` core. E2 may not solve only TCP, silently leave UDS unaccounted or disable UDS.

### Ordinary Tokio unchanged

Owner-free Tokio TCP/UDS registration remains semantically unchanged. E2 introduces no global registration cap, DNS policy, scheduler change, production limit or second resource ledger. Owner-aware accounting failure rejects before the charged allocation and leaves registration/driver state unchanged.

## Required proof

Before E2 can be GREEN, the SAME #351/#356 lineage must prove on one exact head:

1. TCP owner-aware registration succeeds when exactly funded and denies at max-minus-one before `Arc<ScheduledIo>` allocation.
2. UDS owner-aware registration proves the same invariant on Unix.
3. registration-set/driver pending-release custody keeps the debit live after socket/`Registration` teardown begins.
4. final release/shutdown destroys the charged backing before exactly one debit release.
5. registration failure, deregistration cleanup, cancellation and runtime shutdown do not leak or double-release.
6. every custody transfer preserves the same operation-owner identity and no second runtime owner is minted.
7. ordinary owner-free TCP/UDS regressions pass unchanged.
8. source/API census proves no alternate owner-aware uncharged registration allocation/finality path.
9. focused Tokio tests, Rust 1.94 fmt/check/strict Clippy as applicable, root composition checks and repository governance pass.
10. E2 success does not claim DNS E3 or std-thread `3993253957` closure.

## Explicit exclusions

No authority for:

- DNS hostname resolution, `TcpStream::connect<A: ToSocketAddrs>`, `lookup_host` or resolver redesign;
- E3 `ARCHITECTURE_BLOCKED_DNS_RESOLVER_PREALLOCATION`;
- std private thread-spawn/name accounting `3993253957`;
- Tokio blocking pool/task paths except separately protected prior grants;
- `scheduled_io.rs` under this allocation;
- UDP/listeners/named pipes/process/signal/timers/io-uring/scheduler redesign;
- resource registry or production numeric maxima;
- Cargo/lock/workflow/protection changes;
- Foundation, WP4/#335, WP5, Server Seam #247;
- production/deployment/live data/external repositories.

Any newly proven inseparable symbol remains fail-closed:

```text
SHARED_LEASE_REQUIRED = path :: symbol/resource :: reason
```

## Lifecycle

```text
this docs-only candidate
-> exact-head repository checks
-> genuinely independent HIGH-risk exact-head review, P0/P1/P2=0
-> native FULL Merge Queue + real merge_group game-gate
-> protected-main readback
-> fresh #162 overlap/custody readback
-> explicit SAME #351/#356 E2 activation only
-> focused RED/GREEN E2 implementation
-> later complete WP3 composition qualification
```

Protected integration alone does not activate E2. Direct merge, generic auto-merge, bypass, force/rebase, no-op/retrigger and protection weakening are forbidden.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

Refs #162 #351 #356 #364 #583.
