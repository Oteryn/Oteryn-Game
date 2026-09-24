# WP3 rustls deframer owner amendment

Coordinator: #162. Programme: #364. Existing sole worker: #351 / #356.

## State

```yaml
allocation_id: OTV2-WP3-RUSTLS-DEFRAMER-OWNER-20260907
repository: Oteryn/Oteryn-Game
allocation_base_main_sha: 3825e9c82ff388923f73548a807718807012f53f
allocation_state: NOT_ACTIVE
preparation_branch: coord/wp3-rustls-deframer-351
worker_branch: agent/sqlx-driver-budget-351
risk: HIGH
```

This prospective amendment extends the existing WP3 admission only after
independent review, canonical checks, normal FULL Merge Queue, protected
readback and explicit Work application after fresh custody verification.
It grants no present implementation authority and creates no replacement worker.

## Verified necessity

At WP3 checkpoint `e1cbb08e7ac58d104d0c0aa6dacdd0591190e327`,
return comment 5576331328 records SHARED_LEASE_REQUIRED. Successor
`283bb9b77000fa0dab9d3b7b74a008e57459bb76` preserves that blocker.
SQLx's reader receives a slice only after rustls privately prepares its backing.
In rustls 0.23.43, DeframerVecBuffer::read calls prepare_read before rd.read;
prepare_read resizes or shrinks the Vec. SQLx therefore cannot enforce the
accepted reservation-before-allocation invariant through its existing reader.

Independent source inspection confirms the boundary at official rustls tag
`v/0.23.43`, commit `fcf61cdbba30913cfd5b40aefa83989c6233812d`:
buffers.rs blob `bf0987407009993afa8e80073e31f4c511ec5610`;
conn.rs blob `4ea90e7e051935f913c1dbb4b2a40f998c4e7303`.
ConnectionCommon owns the private buffer. ClientConnection already exposes
DerefMut to ConnectionCommon, so no client/client_conn.rs change is required.

## Exact package and custody

The Cargo.lock-pinned package is rustls 0.23.43 with published archive checksum
`0283386ce02abc0151e1761d08802dfe86c173b0b494af5cbc086574e453da06`.

Permit the complete published package at `vendor/rustls-0.23.43/**` as
COPY-ONLY upstream bytes, except these exact authored paths:

- `vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs`: the incoming deframer
  allocation owner, lifetime accounting and focused inline tests only.
- `vendor/rustls-0.23.43/src/conn.rs`: one narrow ConnectionCommon owner
  installation method and wiring into its private incoming deframer only.
- `vendor/rustls-0.23.43/src/lib.rs`: the minimal std-gated public name/export
  for that owner interface only.
- `vendor/rustls-0.23.43/OTERYN_PROVENANCE.md`: exact package identity,
  upstream commit/original edited-file blobs, authored delta manifest,
  Game authority and validation evidence.

This is not blanket authored custody of rustls. Preserve package manifests,
features, dependencies and all license files byte-for-byte. Record the upstream
license expression Apache-2.0 OR ISC OR MIT. Import from the checksum-verified
published archive; a reconstructed Git tree is not archive-integrity evidence.

Existing WP3 root Cargo custody permits only adding this directory to
workspace.exclude and the exact rustls path patch under patch.crates-io.
Keep version 0.23.43 and dependencies unchanged. Expected Cargo.lock consequence
is removal of the registry source/checksum for that same package, not dependency
upgrades. Existing SQLx-owned tls_rustls.rs, resource_budget.rs and
resource_budget_tests.rs suffice for hook installation, ledger adaptation and
cross-crate tests. Existing task/plan/provenance custody continues.

No lease for rustls configuration/session/crypto/verifier/error/client modules,
unbuffered adapter redesign, a new dependency family, registry, Foundation,
WP4 SQL/migrations, WP5, workflow, production or external repository follows.
The existing include-only shared PostgreSQL target lease is unchanged.
Any materially necessary unlisted authored path requires a separate exact
SHARED_LEASE_REQUIRED before mutation.

## Required semantics

Use the same accepted owner ledger for reservation and release through one
owner interface. Install it before the first relevant deframer allocation.
Do not use unrelated reserve/release ledgers, copied private growth constants,
opaque whole-handshake charges or a requested-length-only charge.

Reservation must precede allocation and provably cover actual Vec capacity.
Growth and shrink must retain old and prospective-new backing charges through
their overlap until old backing is destroyed or a proved charged transfer.
A request length or capacity delta alone does not prove the new capacity.
If necessary, investigate pinned Rust 1.94 allocator behavior within this
boundary; do not silently broaden architecture or paths.

Denial must precede allocation and reader invocation and leave existing backing
and charge unchanged. Read errors and WouldBlock retain the resized backing
charge. Cancellation, connection/socket drop and ordinary destruction must
release exactly once, only after the relevant backing is gone. Preserve
unhooked rustls behavior, no_std compatibility, feature composition, wire limits,
TLS versions, verification and I/O semantics. No plaintext/unowned fallback or
new resource maximum is authorized.

## Qualification and release

Demonstrate focused RED to GREEN for preallocation denial, actual-capacity
coverage, growth/shrink overlap, denial preserving old state, read-error and
WouldBlock retention, cancellation/drop release, and ordinary unhooked behavior.
Use inline rustls tests and already-owned SQLx tests; no generalized framework.
Compare every imported archive byte outside the authored allowlist. Verify
locked metadata resolves one path-backed rustls 0.23.43 with unchanged package
dependencies. Run affected rustls/SQLx tests, applicable workspace build/tests,
strict Clippy, formatting and governance, proportionate to the change.

This hook proves only its boundary. Complete the already-accepted TLS
configuration, decoder, session/cache and handshake-overlap composition before
claiming complete TLS. Real TLS-positive and configured PostgreSQL 17.6,
independent exact-head high-risk review, canonical CI, normal protected MQ and
readback remain mandatory. Skipped/plaintext/unconfigured tests prove neither.

Before Work application refresh protected main, #351/#356 exact head, active
writers, root Cargo and rustls ownership, and shared PostgreSQL custody.
Keep WP4 frozen and return its shared target only after protected WP3 delivery
under the existing release rules. Separate CONTROL #420/#422 continues to own
its workflow repair. Preserve all prior admissions, counters and valid work.
Allocation protection alone is not implementation, TLS qualification, WP3
completion, WP4 activation or Server Seam release.
