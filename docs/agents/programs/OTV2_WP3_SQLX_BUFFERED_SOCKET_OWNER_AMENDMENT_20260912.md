# WP3 SQLx BufferedSocket owner amendment

Coordinator: #162. Programme: #364. Existing sole material worker: #351 / Draft
PR #356.

## State

```yaml
allocation_id: OTV2-WP3-SQLX-BUFFERED-SOCKET-OWNER-20260912
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 1d0916c7476f37589524c7181b5857bcc6c141e1
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-socket-owner-finality-351
worker_branch: agent/sqlx-driver-budget-351
source_wp3_head: 2e37c9025d19e9a9f7c1b4dbc13e36f70dd58fdb
source_review_finding: 3993253930
source_authority_request: 5640554651
risk: HIGH
```

This is an allocation-only amendment for the **same** canonical #351/#356
writer. It creates no replacement worker or current source authority. It becomes
active only after independent exact-head review, canonical checks, protected
Merge Queue integration, protected-main readback, and an explicit fresh #162
same-writer activation.

## Proven gap

The owner-aware PostgreSQL path reaches `PgStream::connect_with_resource_budget`
but still constructs an ordinary `BufferedSocket::new`. The constructor allocates
the initial read/write backing before the caller-supplied `ResourceBudget` can
reserve it. Later peer-controlled reads can grow `BytesMut`, writes can grow or
shrink their buffer, and split/freeze operations can leave shared backing alive
after the mutable read view advances. Charging in `PgStream` after these events
cannot prove reservation-before-allocation, actual capacity, replacement overlap,
or final shared-backing custody.

Finding `3993253930` is therefore accepted as a real P1 scope boundary. A
PostgreSQL-side estimate, opaque whole-socket debit, arbitrary smaller buffer, or
post-allocation reconciliation is not an acceptable repair.

## Exact protected lease after activation

### Writable path and purpose

`vendor/sqlx-core-0.9.0/src/net/socket/buffered.rs` is added to the existing
#351/#356 material lease only for:

- a distinct owner-aware `BufferedSocket` construction path using the already
  propagated caller-supplied `ResourceBudget` identity;
- prospective reservation of the actual initial read and write backing before
  either allocation;
- prospective reservation for owner-aware read/write capacity growth, including
  checked old-plus-new overlap until the replaced backing is destroyed;
- truthful shrink/replacement transfer, with the old debit released only after
  old backing destruction;
- custody for read backing shared through `Bytes` split/freeze/slices/clones,
  charged once and retained until the last shared backing owner is destroyed;
- final read/write backing release only after the corresponding allocation is
  destroyed; and
- focused tests in the same file or the already-owned
  `vendor/sqlx-postgres-0.9.0/tests/oteryn_resource_budget.rs` surface.

No other SQLx-core socket file or symbol is added by this amendment. If the
minimum implementation proves that shared `Bytes` finality cannot be represented
inside this file without another exact path/symbol, the worker must stop at a new
`SHARED_LEASE_REQUIRED` boundary rather than approximating custody.

### Required invariants

1. The owner-aware path reuses the exact caller-supplied budget; it creates no
   ledger, default allowance, or per-socket reserve pool.
2. Every fallible reservation occurs before the controlled allocation/growth.
   Checked arithmetic covers the allocator-visible capacity used by the
   implementation and simultaneous old/new backing during replacement.
3. A failed reservation leaves the old socket state usable and unchanged and
   performs no controlled allocation.
4. A successful transfer cannot release the debit while any read slice/clone or
   write backing still retains the charged allocation.
5. Release follows actual destruction. A guard stored inside the allocation it
   charges is forbidden when its destructor necessarily runs before enclosing
   allocation deallocation.
6. Peer-advertised lengths and send growth preserve existing SQLx validity and
   transport semantics. There is no truncation, guessed capacity, new protocol
   ceiling, TLS downgrade, or retry through the ordinary path.
7. Ordinary `BufferedSocket::new` and ordinary owner-free SQLx behavior remain
   source-compatible and behaviorally unchanged.

## Required focused proof

- exact initial read/write funded positive and max-minus-one denial before first
  owner-aware allocation;
- read and write no-growth controls;
- read growth and write growth with the old and replacement capacities
  simultaneously debited until old backing destruction;
- failed growth leaves length, capacity, bytes, and ledger unchanged;
- shrink/replacement releases only after destruction;
- split/freeze plus multiple slices/clones retains one shared charge until the
  last backing owner drops, including concurrent final drops;
- partial read, cancellation, EOF/error, connection close, and drop release
  exactly once with no residual debit;
- the ordinary owner-free constructor/growth path remains unchanged; and
- actual owner-aware SQLx AWS-LC TLS-positive/PostgreSQL 17.6 qualification
  consumes this path later on the final #356 head. Compile-only and plaintext
  PostgreSQL checks do not satisfy that acceptance.

## Explicit exclusions

No numeric limit, registry, Cargo/lock, workflow, TLS policy, PostgreSQL protocol,
Foundation, Durability B/#335, WP5, Server Seam #247, production, deployment, or
external-repository authority is added. This is not the broader prospective
PostgreSQL accounting package and does not grant `socket/mod.rs`, decoder,
message, cache, SASL, or retained connection surfaces.

The allocation does not resolve the independent owner-finality/thread-spawn
findings `3993253945` and `3993253957`; those are governed by the companion
architecture candidate in this same control-plane change. It also does not
reopen already repaired findings `3993253981`, `3993253977`, or `3993253965`.

## Lifecycle

```text
this allocation candidate
-> independent exact-head HIGH review
-> exact-head repository checks
-> protected Merge Queue + real merge_group game-gate
-> protected-main readback
-> fresh #162 overlap/custody check
-> explicit SAME #351/#356 activation
-> focused RED/GREEN and final whole-diff qualification
```

`MERGE_AUTHORITY: REPOSITORY_CONTROL_PLANE_ONLY`
