# WP3 SQLx connect/socket owner amendment

Coordinator: #162. Programme: #364. Existing sole material writer: #351 / Draft PR #356.

## State

```yaml
allocation_id: OTV2-WP3-SQLX-CONNECT-SOCKET-OWNER-20260912
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 06c06a69d8d5affaf07049bad07f45b83df89392
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-connect-socket-owner-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: e88c7cf175e068e798d5aaed1556ac0b8951558a
source_review_finding: 3993253930
supersedes_incomplete_allocation: OTV2-WP3-SQLX-BUFFERED-SOCKET-OWNER-20260912
superseding_review: PR_579_review_5186018568
review_repair: PR_583_review_5186127343
control_plane_evidence:
  - issue_162_comment_5645053755
  - issue_162_comment_5645200220
risk: HIGH
```

This is a prospective correction for the SAME #351/#356 writer. It grants no current source mutation. The older BufferedSocket-only amendment remains historical evidence and must not be activated by itself.

## Decision timing

1. **Must decide now? YES.** WP3 cannot truthfully claim owner-aware PostgreSQL/TLS socket accounting while allocations before `BufferedSocket::new` bypass the operation owner.
2. **Blocked work.** P1 `3993253930`, complete TLS accounting, PostgreSQL 17.6 qualification, WP4/#335, WP5 and Server Seam #247 remain downstream.
3. **Harder later.** Treating `BufferedSocket` as the first socket allocation would preserve unowned final socket boxes, Tokio reactor registration, DNS resolution and phase-split execution-owner identities in the accepted driver seam.
4. **Superseding evidence.** A pinned SQLx/Tokio/std API that exposes an earlier exact owner-aware socket/DNS/registration boundary, or measured proof that a listed allocation does not exist, may narrow this amendment.
5. **Not decided.** No production DNS policy, DNS result count limit, resolver replacement, protocol limit, allocator-overhead policy, generic Tokio DNS redesign, or generic Tokio reactor redesign is selected.

## One exact operation/runtime-owner identity

The owner-aware PostgreSQL establishment path must establish **one exact concrete runtime-owner Arc identity before DNS/TCP/UDS work** and reuse that same identity throughout the connection-establishment phases that use owner-aware execution or retained request custody.

Current `BlockingJobOwner` owns one `Arc<BudgetOwner>`. Tokio owner queues compare that Arc by pointer identity (`Arc::ptr_eq` through `OterynCharge::same_owner`). Two `BlockingJobOwner` values created from the same `ResourceBudget` are therefore **not** equivalent execution owners: they can create separate `OwnedQueue` nodes, worker lifecycles and independent queue-capacity opportunities even though byte debits share one ledger.

After `3993253945` final-owner correctness is protected and activated, the accepted composition must be:

```text
PgConnection/PgStream owner-aware establish
-> construct/fetch one BlockingJobOwner / runtime-owner Arc
-> E3 owner-aware DNS blocking work, when activated
-> E2 owner-aware TCP or UDS reactor-registration custody, when activated
-> E1 socket Box / BufferedSocket custody
-> TLS MaybeUpgrade / handshake
-> certificate/key/root blocking loads
```

The same `ResourceBudget` remains the single byte ledger. The runtime-owner Arc is an execution-owner capability for that exact operation, not a second budget. E1/E2/E3 and existing TLS loader plumbing must not mint a second `BudgetOwner` for the same operation. Any I/O-registration custody that needs an owner identity must retain/use the same concrete owner identity rather than creating a per-socket runtime owner.

This likely moves `BlockingJobOwner` creation earlier than the current `tls_rustls::handshake_with_resource_budget` construction point and passes/reuses it through existing admitted PostgreSQL/SQLx TLS seams. Ordinary owner-free behavior remains unchanged.

## Proven pre-BufferedSocket graph

Fresh exact-source review of #356 proves the TLS-positive TCP path is:

```text
PgStream::connect_with_resource_budget
-> sqlx_core::net::connect_tcp(host, port, MaybeUpgradeTlsOwned(...))
-> tokio::net::TcpStream::connect((host, port))
-> mio::net::TcpStream::connect
-> Tokio TcpStream::new
-> PollEvented::new
-> Registration::new_with_interest_and_handle
-> io::Handle::add_source
-> RegistrationSet::allocate
-> Arc::new(ScheduledIo)
-> MaybeUpgradeTlsOwned::with_socket
-> tls::handshake_with_resource_budget(..., SocketIntoBox, owner)
-> sqlx_core::net::socket::SocketIntoBox::with_socket<S>
-> Box::new(socket)
-> BufferedSocket::new(...)
```

So the old lease misses at least two retained allocations before `BufferedSocket`: a per-socket Tokio `Arc<ScheduledIo>` reactor registration and the final TLS socket Box.

`RegistrationSet` also retains an Arc clone after `TcpStream`/`Registration` begins teardown: deregistration moves a clone to the driver's pending-release list, and the I/O driver later removes/releases it. Releasing a request debit merely when the SQLx/Tokio socket wrapper drops is therefore too early for that control allocation.

The UDS path reaches the same reactor boundary. Pinned Tokio 1.53.1 `UnixStream::connect` / `connect_addr` constructs `UnixStream::new`, whose `PollEvented` registration reaches the same `RegistrationSet::allocate -> Arc::new(ScheduledIo)` lifecycle. Rust pathname-to-sockaddr construction is not treated here as a new heap-accounting problem; E2 is the common per-I/O registration problem for both accepted TCP and UDS connection entrypoints.

For hostname TCP connections, pinned Tokio 1.53.1 `ToSocketAddrs for (&str,u16)` parses IP literals without hostname allocation, but DNS hostnames allocate an owned host String and run `std::net::ToSocketAddrs` through ordinary `spawn_blocking`, returning resolver/address iterator backing before the socket is created. Requiring IP literals is forbidden because it changes accepted hostname semantics.

The existing owner-aware PostgreSQL `connection/tls.rs::maybe_upgrade_owned` has another final socket Box for `Disable`, `Allow`, and `Prefer` fallback (`Box::new(socket)`) before SQLx buffering. That path is already within existing #351 PostgreSQL authority; it must use the same truthful external Box-custody representation rather than remain an unaccounted mode-specific bypass.

## Prospective cells after protection

The correction is deliberately split so unresolved dependency internals cannot silently authorize or block path-disjoint accounting improvements. None is active merely because this document is protected.

### Cell E1 — SQLx/PostgreSQL final socket Box + BufferedSocket/shared backing

After protected readback, #162 may explicitly activate E1 for the SAME #351/#356 writer using only:

- `vendor/sqlx-core-0.9.0/src/net/socket/mod.rs` — owner-aware `WithSocket`/final post-handshake socket-Box representation;
- `vendor/sqlx-core-0.9.0/src/net/socket/buffered.rs` — owner-aware initial read/write backing, growth/replacement/shrink/drop and shared `Bytes` custody;
- `vendor/sqlx-core-0.9.0/src/net/mod.rs` — export/plumbing only if required; already within existing #351 SQLx-core authority;
- the already-admitted `vendor/sqlx-postgres-0.9.0/src/connection/tls.rs` only to route `Disable`/`Allow`/`Prefer` fallback boxes through the same owner-aware Box custody, with no TLS-mode semantic change;
- existing admitted `PgStream` / `tls_rustls` plumbing only as needed to receive/reuse the **pre-existing same-operation `BlockingJobOwner`**, never to construct a second runtime owner.

E1 preserves existing TCP/UDS/DNS/TLS-mode semantics. It cannot close `3993253930`, because reactor registration and DNS may remain unaccounted.

Each owner-aware final socket Box must reserve its exact Box allocation before `Box::new(socket)`. Its debit must live outside the Box it charges, deallocate the Box first, and release only afterward. Ordinary owner-free boxing remains unchanged.

The BufferedSocket part must:

- reserve actual initial read/write capacity before allocation;
- reserve growth/replacement prospectively, including simultaneous old/new capacity;
- release old backing only after destruction;
- carry one charge with shared `Bytes` backing across freeze/split/slice/clone descendants until the final backing owner dies;
- release exactly once across cancellation, partial IO, EOF/error, close and drop;
- preserve ordinary owner-free `BufferedSocket` behavior.

If truthful shared-`Bytes` finality requires another exact SQLx-core path/symbol, stop at one new `SHARED_LEASE_REQUIRED`; do not approximate custody.

### Cell E2 — Tokio per-socket reactor registration for TCP and UDS

E2 remains **NOT_ACTIVE**. Pinned source proves both accepted owner-aware connect families reach the same registration allocation:

```text
TCP: TcpStream::connect -> TcpStream::new -> PollEvented -> Registration -> RegistrationSet::allocate
UDS: UnixStream::connect/connect_addr -> UnixStream::new -> PollEvented -> Registration -> RegistrationSet::allocate
```

`RegistrationSet::allocate` performs `Arc::new(ScheduledIo)`, and the registration set/pending-release list can retain that backing after the socket wrapper starts teardown.

A truthful solution needs a separately reviewed owner-aware registration/finality seam that:

- covers **both TCP and UDS** owner-aware constructor/registration entrypoints;
- reserves exact `Arc<ScheduledIo>` control/value backing before allocation;
- receives and retains the **same pre-established same-operation runtime-owner identity** used by E3 and TLS blocking work; it must not mint another `BudgetOwner` Arc for the socket;
- survives every driver-held Arc clone and pending-release transition;
- releases only after actual final ScheduledIo Arc backing deallocation;
- handles register failure, deregister, cancellation, runtime shutdown and driver purge exactly once;
- introduces no raw/Weak finality bypass analogous to P1 `3993253945`;
- leaves ordinary unrelated Tokio TCP/UDS registrations unchanged.

No current #351 lease grants generic Tokio I/O driver paths. A future minimum lease must name the actual TCP + UDS constructor plumbing plus the common registration lifecycle, expected to include only the necessary symbols within:

- `vendor/tokio-1.53.1/src/net/tcp/stream.rs`;
- `vendor/tokio-1.53.1/src/net/unix/stream.rs` on Unix builds;
- `vendor/tokio-1.53.1/src/io/poll_evented.rs`;
- `vendor/tokio-1.53.1/src/runtime/io/registration.rs`;
- `vendor/tokio-1.53.1/src/runtime/io/driver.rs` and/or `registration_set.rs` for the common final custody.

Until that amendment is independently reviewed/protected, preserve:

```text
SHARED_LEASE_REQUIRED_TOKIO_IO_REGISTRATION
```

Do not charge after `TcpStream::connect`/`UnixStream::connect`, release on SQLx socket drop, treat reactor state as free runtime overhead, fail closed for UDS merely to avoid the shared registration problem, or infer authority from existing blocking-owner Tokio amendments.

### Cell E3 — DNS hostname resolution

E3 remains **NOT_ACTIVE**.

Any future owner-aware SQLx hostname path must reuse the **same pre-established `BlockingJobOwner` / runtime-owner Arc identity**; constructing a new owner from the same `ResourceBudget` for DNS is forbidden. Visible Rust DNS work may reuse the accepted owner-funded blocking primitive rather than changing generic Tokio DNS behavior, but acceptance requires proof that every controlled allocation is reserved before allocation:

- hostname owned String backing;
- blocking task/queue/worker custody under the same operation owner;
- resolver result/address-iterator backing and source/destination overlap;
- connect-attempt iteration and error cleanup.

The system resolver may allocate opaque platform/std-private backing for which SQLx has no prospective sizing seam. If exact proof cannot establish a finite preallocation/finality boundary without a new resolver architecture or semantic limit, preserve:

```text
ARCHITECTURE_BLOCKED_DNS_RESOLVER_PREALLOCATION
```

Do not truncate DNS results, invent a magic maximum, charge after resolution, require IP literals, silently route owner-aware work through ordinary Tokio `spawn_blocking`, or claim that accounting host/task/address backing proves opaque resolver internals.

E1 may be implemented later as partial accounting progress after its own explicit activation because it does not alter E2/E3 semantics and does not claim P1 closure. Terminal `3993253930`, `complete_tls_accounting`, and real hostname PostgreSQL qualification require E1 + truthful E2 + truthful E3. IP-literal TCP and UDS qualification require E1 + truthful E2.

## Required focused proof

Composition-wide:

- exactly one `BlockingJobOwner`/runtime-owner Arc identity is created for one owner-aware PostgreSQL establishment operation;
- DNS + TCP/UDS registration + TLS/certificate-loader phases reuse that same identity;
- one owned-queue family is observed and its configured queue capacity is not multiplied by phase count;
- all byte debits remain on the same `ResourceBudget`;
- SQLx/TLS wrapper teardown does not release execution-owner or I/O-registration custody early;
- `3993253945` final-owner proof is green before this earlier-created runtime owner is relied upon.

For E1:

- max-minus-one denies before every final socket `Box::new`; funded TLS-positive and non-TLS/fallback modes preserve behavior and release after actual Box deallocation;
- owner-free boxing remains unchanged;
- initial read/write funded and max-minus-one controls;
- read/write growth proves old+new overlap and rollback on denied growth;
- split/freeze/multiple concurrent final `Bytes` descendants keep one shared debit until final backing destruction;
- cancellation/EOF/error/close/drop leave no residual E1 debit.

For any future E2:

- run the registration proof for both TCP and UDS accepted connection paths;
- max-minus-one denies before `Arc<ScheduledIo>` allocation;
- driver registration list and pending-release clones keep the debit live after socket/Registration drop;
- final driver purge/deallocation releases exactly once, including concurrent shutdown/error paths;
- the exact same operation runtime owner is retained without creating another owner Arc;
- ordinary unrelated Tokio TCP/UDS registration behavior remains unchanged.

For any future E3:

- IP literal connect preserves its non-DNS semantics and creates no hostname String debit merely to mimic DNS;
- DNS blocking work uses the same operation runtime owner as later TLS loaders;
- DNS positive evidence proves prospective owner funding/finality of every visible controlled allocation;
- opaque resolver backing is either covered by a separately accepted finite mechanism or remains explicitly architecture-blocked.

Final qualification still requires actual SQLx AWS-LC TLS-positive + PostgreSQL 17.6 traffic through the protected complete owner-aware path; compile-only/plaintext evidence is insufficient.

## Explicit exclusions

No numeric resource maximum, DNS result cap, resolver replacement, generic Tokio I/O/DNS authority, registry, Cargo/lock, workflow, TLS policy, PostgreSQL protocol redesign, decoder/cache/SASL expansion, Foundation, B/#335, WP5, Server Seam #247, production, deployment or external-repository authority follows.

This amendment does not resolve `3993253957`, which remains `ARCHITECTURE_BLOCKED_THREAD_HANDLE_FINALITY`. It does not reopen focused repairs `3993253977` or `3993253965`; the separate OwnedQueue final-destruction regression remains governed by its existing Tokio blocking-owner authority.

## Lifecycle

```text
candidate
-> independent exact-head HIGH review
-> exact-head repository checks
-> native FULL Merge Queue + merge_group game-gate
-> protected-main readback
-> fresh #162 overlap/custody readback
-> explicit SAME #351/#356 activation of E1 only if still needed/non-overlapping and 3993253945 finality is already active/green
-> E2/E3 remain blocked until separately resolved/protected/activated
-> focused RED/GREEN implementation
-> final whole-diff qualification later
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
