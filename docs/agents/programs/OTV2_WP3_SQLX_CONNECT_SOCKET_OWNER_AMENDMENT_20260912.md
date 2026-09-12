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

## Prospective exact lease after protection

This correction is deliberately split into two activation cells so an unresolved DNS architecture question cannot silently authorize or prevent path-disjoint downstream custody work.

### Cell E1 — final socket Box + BufferedSocket/shared backing

After protected readback, #162 may explicitly activate E1 for the SAME #351/#356 writer using only:

- `vendor/sqlx-core-0.9.0/src/net/socket/mod.rs` — an owner-aware `WithSocket`/final post-handshake socket-Box representation; not a generic DNS redesign;
- `vendor/sqlx-core-0.9.0/src/net/socket/buffered.rs` — owner-aware initial read/write backing, growth/replacement/shrink/drop and shared `Bytes` custody;
- `vendor/sqlx-core-0.9.0/src/net/mod.rs` — export/plumbing only if required; this path is already within existing #351 SQLx-core authority.

Existing admitted PostgreSQL/TLS callers may select the owner-aware boxing/buffering representation. Existing `connect_tcp` hostname/IP semantics remain unchanged by E1. E1 does **not** make the preceding DNS resolver path accounted and cannot close `3993253930` or qualify a hostname PostgreSQL connection by itself.

The owner-aware final socket Box must reserve its exact Box allocation before `Box::new(socket)`. Its debit must live outside the Box it charges, destroy/deallocate the Box first, and release only afterward. Ordinary `SocketIntoBox` remains unchanged.

The BufferedSocket part must:

- reserve actual initial read/write capacity before allocation;
- reserve growth/replacement prospectively, including simultaneous old/new capacity;
- release old backing only after destruction;
- carry one charge with shared `Bytes` backing across freeze/split/slice/clone descendants until the final backing owner dies;
- release exactly once across cancellation, partial IO, EOF/error, close and drop;
- preserve ordinary owner-free `BufferedSocket` behavior.

If truthful shared-`Bytes` finality requires another exact SQLx-core path/symbol, stop at one new `SHARED_LEASE_REQUIRED`; do not approximate custody.

### Cell E2 — DNS hostname resolution

E2 remains **NOT_ACTIVE** after this document is protected unless a later explicit #162 activation says otherwise.

A future owner-aware SQLx connect path may reuse the already-protected owned blocking primitive for visible Rust DNS work rather than changing generic Tokio DNS behavior, but acceptance requires proof that every controlled allocation is reserved before allocation:

- hostname owned String backing;
- owned blocking task/queue/worker custody;
- resolver result/address-iterator backing and source/destination overlap;
- connect-attempt iteration and error cleanup.

The system resolver may allocate opaque platform/std-private backing for which SQLx has no prospective sizing seam. If exact proof cannot establish a finite preallocation/finality boundary without a new resolver architecture or semantic limit, preserve:

```text
ARCHITECTURE_BLOCKED_DNS_RESOLVER_PREALLOCATION
```

Do not truncate DNS results, invent a magic maximum, charge after resolution, require IP literals, silently route owner-aware work through ordinary Tokio `spawn_blocking`, or claim that accounting the host String/task/address Vec proves opaque resolver internals.

E1 may be implemented as partial accounting progress after its own explicit activation because it does not alter DNS semantics and it does not claim P1 closure. Terminal `3993253930`, `complete_tls_accounting`, and real hostname PostgreSQL qualification remain blocked until E2 is truthfully resolved.

## Required focused proof

For E1:

- max-minus-one denies before final TLS `Box::new(socket)`; funded path preserves TLS behavior and releases after actual Box deallocation;
- owner-free `SocketIntoBox` is unchanged;
- initial read/write funded and max-minus-one controls;
- read/write growth proves old+new overlap and rollback on denied growth;
- split/freeze/multiple concurrent final `Bytes` descendants keep one shared debit until final backing destruction;
- cancellation/EOF/error/close/drop leave no residual debit.

For any later E2:

- IP literal connect preserves its non-DNS semantics and creates no hostname String debit merely to mimic DNS;
- DNS positive evidence proves prospective owner funding/finality of every visible controlled allocation;
- opaque resolver backing is either covered by a separately accepted finite mechanism or remains explicitly architecture-blocked.

Final qualification still requires actual SQLx AWS-LC TLS-positive + PostgreSQL 17.6 traffic through the protected complete owner-aware path; compile-only/plaintext evidence is insufficient.

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
-> explicit SAME #351/#356 activation of E1 only if still needed/non-overlapping
-> E2 remains blocked until separately resolved/activated
-> focused RED/GREEN implementation
-> final whole-diff qualification later
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
