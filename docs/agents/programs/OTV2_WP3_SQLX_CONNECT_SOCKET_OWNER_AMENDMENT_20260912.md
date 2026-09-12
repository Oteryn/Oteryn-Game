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
control_plane_evidence: issue_162_comment_5645053755
risk: HIGH
```

This is a prospective correction for the SAME #351/#356 writer. It grants no current source mutation. The older BufferedSocket-only amendment remains historical evidence and must not be activated by itself.

## Decision timing

1. **Must decide now? YES.** WP3 cannot truthfully claim owner-aware PostgreSQL/TLS socket accounting while allocations before `BufferedSocket::new` bypass the operation owner.
2. **Blocked work.** P1 `3993253930`, complete TLS accounting, PostgreSQL 17.6 qualification, WP4/#335, WP5 and Server Seam #247 remain downstream.
3. **Harder later.** Treating `BufferedSocket` as the first socket allocation would preserve an unowned TLS socket Box and DNS resolution path in the accepted driver seam.
4. **Superseding evidence.** A pinned SQLx/Tokio/std API that exposes an earlier exact owner-aware socket/DNS allocation boundary, or measured proof that a listed allocation does not exist, may narrow this amendment.
5. **Not decided.** No production DNS policy, DNS result count limit, resolver replacement, protocol limit, allocator-overhead policy, or generic Tokio DNS redesign is selected.

## Proven pre-BufferedSocket gap

Fresh exact-source review of #356 proves the owner-aware path is:

```text
PgStream::connect_with_resource_budget
-> sqlx_core::net::connect_tcp(host, port, MaybeUpgradeTlsOwned(...))
-> tokio::net::TcpStream::connect((host, port))
-> MaybeUpgradeTlsOwned::with_socket
-> tls::handshake_with_resource_budget(..., SocketIntoBox, owner)
-> sqlx_core::net::socket::SocketIntoBox::with_socket<S>
-> Box::new(socket)
-> BufferedSocket::new(...)
```

The final TLS socket Box in `sqlx-core/src/net/socket/mod.rs` therefore allocates before the old BufferedSocket-only lease can reserve it.

For hostname connections, pinned Tokio 1.53.1 also reaches `ToSocketAddrs for (&str,u16)`: IP literals parse without hostname allocation, while DNS hostnames allocate an owned host String and run the system resolver through ordinary `spawn_blocking`, returning address-iterator backing before `WithSocket` or `BufferedSocket` exists. Requiring IP literals is forbidden because it would change accepted hostname semantics.

## Prospective exact lease after protection and explicit activation

The SAME #351/#356 writer may receive only these additional SQLx-core surfaces:

- `vendor/sqlx-core-0.9.0/src/net/socket/mod.rs` — owner-aware TCP-connect entry point and final post-handshake socket Box custody only;
- `vendor/sqlx-core-0.9.0/src/net/socket/buffered.rs` — owner-aware initial read/write backing, growth/replacement/shrink/drop and shared `Bytes` custody;
- `vendor/sqlx-core-0.9.0/src/net/mod.rs` — export/plumbing only if required for that exact owner-aware entry point; this path is already within existing #351 SQLx-core authority.

Existing admitted PostgreSQL paths may call the new owner-aware entry point. Existing `src/rt/resource_owner.rs` primitives may be reused but this amendment does not widen their semantics beyond separately protected authority.

### Final socket Box

The owner-aware `WithSocket`/boxing seam must reserve the exact Box allocation before `Box::new(socket)`. The reservation must live outside the Box it charges, destroy/deallocate the Box first, and release only afterward. Ordinary `SocketIntoBox` remains unchanged.

### Buffered socket and shared backing

The corrected BufferedSocket part retains the old invariants:

- reserve actual initial read/write capacity before allocation;
- reserve growth/replacement prospectively, including simultaneous old/new capacity;
- release old backing only after destruction;
- carry one charge with shared `Bytes` backing across freeze/split/slice/clone descendants until the final backing owner dies;
- cancellation, partial IO, EOF/error, close and drop release exactly once;
- ordinary owner-free `BufferedSocket` behavior remains unchanged.

If truthful shared-`Bytes` finality requires another exact SQLx-core path/symbol, stop at one new `SHARED_LEASE_REQUIRED`; do not approximate custody.

## DNS hostname boundary — explicit unresolved subcell

This amendment does **not** claim that simply moving hostname resolution into SQLx makes it accounted. A candidate owner-aware SQLx connect path may reuse the already-protected owned blocking primitive for DNS rather than changing generic Tokio DNS behavior, but acceptance requires proof that every controlled Rust allocation is reserved before allocation:

- hostname owned String backing;
- owned blocking task/queue/worker custody;
- resolver result/address-iterator backing and source/destination overlap;
- connect-attempt iteration and error cleanup.

The system resolver may allocate opaque platform/std-private backing for which SQLx has no prospective sizing seam. If exact implementation proof cannot establish a finite preallocation boundary without a new resolver architecture or semantic limit, return:

```text
ARCHITECTURE_BLOCKED_DNS_RESOLVER_PREALLOCATION
```

Do not truncate DNS results, invent a magic maximum, charge after resolution, require IP literals, silently use ordinary Tokio `spawn_blocking`, or claim that accounting the host String/task/address Vec proves opaque resolver internals.

The socket Box + BufferedSocket work is path-disjoint from that unresolved resolver subcell and may be activated separately after protected readback if #162 explicitly says so; terminal P1 closure and real hostname PostgreSQL qualification still require the DNS subcell to be resolved.

## Required focused proof

- max-minus-one denies before final TLS `Box::new(socket)`; funded path preserves TLS behavior and releases after actual Box deallocation;
- owner-free `SocketIntoBox` is unchanged;
- initial read/write funded and max-minus-one controls;
- read/write growth proves old+new overlap and rollback on denied growth;
- split/freeze/multiple concurrent final `Bytes` descendants keep one shared debit until final backing destruction;
- cancellation/EOF/error/close/drop leave no residual debit;
- IP literal owner-aware connect preserves allocation-free address parsing and does not create a hostname String debit;
- DNS positive evidence, when architecture permits it, proves prospective owner funding of every visible Rust allocation and does not infer opaque resolver backing;
- actual SQLx AWS-LC TLS-positive + PostgreSQL 17.6 qualification later consumes the final protected path; compile-only/plaintext evidence is insufficient.

## Explicit exclusions

No numeric resource maximum, DNS result cap, resolver replacement, registry, Cargo/lock, workflow, TLS policy, PostgreSQL protocol redesign, decoder/cache/SASL expansion, Foundation, B/#335, WP5, Server Seam #247, production, deployment or external-repository authority follows.

This amendment does not resolve `3993253957`, which remains `ARCHITECTURE_BLOCKED_THREAD_HANDLE_FINALITY`. It does not reopen focused repairs `3993253977` or `3993253965`; the separate OwnedQueue final-destruction regression remains governed by its existing Tokio owner authority.

## Lifecycle

```text
candidate
-> independent exact-head HIGH review
-> exact-head repository checks
-> native FULL Merge Queue + merge_group game-gate
-> protected-main readback
-> fresh #162 overlap/custody readback
-> explicit SAME #351/#356 activation of only proven path-disjoint cells
-> focused RED/GREEN implementation
-> final whole-diff qualification later
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
