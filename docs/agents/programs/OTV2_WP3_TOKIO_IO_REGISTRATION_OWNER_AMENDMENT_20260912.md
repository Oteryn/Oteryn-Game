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
source_wp3_head: e88c7cf175e068e798d5aaed1556ac0b8951558a
source_decision: OTV2-WP3-SQLX-CONNECT-SOCKET-OWNER-20260912
source_protected_merge: 489e3e390a1bce1ce3439c66521ab75f8a826cd8
source_blocker: SHARED_LEASE_REQUIRED_TOKIO_IO_REGISTRATION
risk: HIGH
```

This is a prospective, path-bounded amendment for the SAME canonical #351/#356 material writer. It does not activate source mutation by itself and does not create a second Tokio/runtime owner.

## Why this amendment is needed

Protected PR #583 proves that both accepted owner-aware PostgreSQL connect families reach the same retained Tokio reactor registration allocation before SQLx `BufferedSocket` custody:

```text
TCP: TcpStream::connect -> TcpStream::new -> PollEvented -> Registration -> RegistrationSet::allocate
UDS: UnixStream::connect/connect_addr -> UnixStream::new -> PollEvented -> Registration -> RegistrationSet::allocate
```

`RegistrationSet::allocate` creates `Arc<ScheduledIo>`. Driver registration and pending-release custody can keep Arc backing alive after the SQLx socket wrapper and `Registration` begin teardown. Therefore E1 cannot truthfully close PostgreSQL socket accounting by releasing the debit at SQLx socket drop.

The accepted #583 decision requires E2 to cover both TCP and UDS, reserve before the actual `Arc<ScheduledIo>` allocation, retain the same already-established per-operation runtime-owner identity, and release only after actual final registration backing deallocation.

## Prospective writable scope after protection and explicit #162 activation

Only the exact SAME #351/#356 worker may receive this amendment. The prospective Tokio E2 lease is limited to the smallest symbols necessary within:

```text
vendor/tokio-1.53.1/src/net/tcp/stream.rs
vendor/tokio-1.53.1/src/net/unix/stream.rs
vendor/tokio-1.53.1/src/io/poll_evented.rs
vendor/tokio-1.53.1/src/runtime/io/registration.rs
vendor/tokio-1.53.1/src/runtime/io/registration_set.rs
vendor/tokio-1.53.1/src/runtime/io/driver.rs
vendor/tokio-1.53.1/src/runtime/io/scheduled_io.rs
vendor/tokio-1.53.1/tests/oteryn_resource_owner.rs
vendor/tokio-1.53.1/OTERYN_PROVENANCE.md
vendor/tokio-1.53.1/OTERYN_DELTA_MANIFEST.json
```

The worker must begin with a source-symbol preflight and use fewer paths when possible. Any additional Tokio path or symbol remains `SHARED_LEASE_REQUIRED`; this document is not blanket `vendor/tokio-1.53.1/**` authority.

Existing already-active #351 SQLx/Tokio blocking-owner paths remain governed by their own grants. This amendment neither revokes nor broadens those grants.

## Required E2 implementation contract

### 1. One exact operation owner

One owner-aware PostgreSQL establishment operation must establish/reuse exactly one concrete runtime-owner identity before DNS/TCP/UDS. E2 receives that same identity and must not create another `BudgetOwner`, `BlockingJobOwner`, queue owner or independent ledger.

Pointer identity is semantic for current owned Tokio queue accounting. The E2 representation must preserve the exact identity used by owner-aware blocking work and TLS loaders.

P1 `3993253945` final-owner correctness is a prerequisite for relying on this identity end-to-end. E2 work may be developed path-disjointly, but terminal composition cannot claim correctness until that finality proof is GREEN.

### 2. Prospective reservation before registration allocation

The owner-aware TCP/UDS path must reserve the exact charged registration backing before the relevant `Arc<ScheduledIo>` allocation. Post-allocation accounting is forbidden.

The proof must cover the actual requested layout/control backing for the pinned Tokio/Rust target rather than `size_of::<Arc<_>>()`, an arbitrary constant or allocator-overhead percentage.

If an exact requested layout cannot be derived at the allocation owner, stop with an evidence-backed architecture/scope blocker rather than guessing.

### 3. Custody follows every retained registration owner

The charge must survive every retained registration clone/state that can keep the `ScheduledIo` Arc backing alive, including:

- active registration set custody;
- socket/`Registration` lifetime;
- deregistration and pending-release transition;
- driver-held pending-release/list custody;
- cancellation/error paths;
- runtime shutdown and driver purge.

Release is permitted exactly once and only after the final charged Arc backing is actually deallocated. SQLx socket drop, deregistration request or logical pending-release enqueue is not sufficient evidence of deallocation.

No raw/Weak/ordinary Arc final-drop bypass may reproduce the owner-finality defect represented by `3993253945`.

### 4. TCP and UDS share the same owner-aware core

The accepted behavior-preserving route must cover both `TcpStream` and Unix `UnixStream` constructor/registration entrypoints. Do not solve TCP while silently leaving UDS unaccounted, and do not fail closed for UDS merely to avoid the common reactor boundary.

Platform-conditional compilation may keep Unix-only code absent on Windows; that is not authority to alter ordinary Tokio platform behavior.

### 5. Ordinary Tokio behavior remains unchanged

Owner-free TCP/UDS registration remains byte-for-byte/semantically equivalent apart from minimal shared plumbing required to select the owner-aware path. No global registration cap, global resource budget, DNS change, scheduler change, socket semantic change or production policy is introduced.

E2 accounting failure on an owner-aware path must reject before the charged allocation and leave registration/driver state unchanged.

## Required proof

Before this E2 cell may be considered GREEN, the SAME #351/#356 lineage must demonstrate on one coherent head:

1. TCP owner-aware registration succeeds when exactly funded and denies at max-minus-one before `Arc<ScheduledIo>` allocation.
2. UDS owner-aware registration exercises the same invariant on Unix.
3. registration-set/driver pending-release custody keeps the debit live after socket/`Registration` teardown begins.
4. final driver release/purge destroys the charged backing before exactly one debit release.
5. register failure, deregister failure/cleanup, cancellation and runtime shutdown do not leak or double-release the debit.
6. every E2 custody clone preserves the same operation-owner identity; no second runtime-owner Arc is minted.
7. ordinary owner-free TCP and UDS tests remain unchanged and pass.
8. source/API census shows the owner-aware E2 path has no alternate uncharged `ScheduledIo` allocation/finality route.
9. focused Tokio tests, Rust 1.94 formatting/check/strict Clippy required by the affected crate, root composition checks and repository governance pass.
10. a later complete SQLx TLS/PG qualification treats E2 as one component only; E2 success does not claim DNS E3 or std-thread `3993253957` closure.

## Explicit exclusions

This amendment grants no authority for:

- DNS hostname resolution or `lookup_host` / generic resolver redesign;
- `ARCHITECTURE_BLOCKED_DNS_RESOLVER_PREALLOCATION` E3;
- std private thread spawn-packet/name accounting `3993253957`;
- Tokio blocking pool/task paths except where separately already authorized by prior protected amendments;
- Tokio scheduler, timers, process, signal, unrelated UDP/listener/pipe/named-pipe registrations;
- resource registry or numeric production maxima;
- Cargo/lock/workflow/protection changes;
- Foundation, WP4/#335, WP5, Server Seam #247;
- production/deployment/live data/external repositories.

If implementation evidence proves one additional symbol is inseparable from the common registration finality path, finish all legal work first and return exactly:

```text
SHARED_LEASE_REQUIRED = path :: symbol/resource :: reason
```

No silent scope widening.

## Lifecycle

```text
this docs-only candidate
-> exact-head repository checks
-> genuinely independent HIGH-risk exact-head review, P0/P1/P2=0
-> native FULL Merge Queue + real merge_group game-gate
-> protected-main readback
-> fresh #162 overlap/custody readback
-> explicit SAME #351/#356 E2 activation only
-> source-symbol preflight on the canonical material branch
-> focused RED/GREEN E2 implementation
-> later complete WP3 composition qualification
```

Protected integration alone does not activate E2. Direct merge, generic auto-merge, bypass, force/rebase, no-op/retrigger and protection weakening are forbidden.

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`

Refs #162 #351 #356 #364 #583.
