# Oteryn provenance: rustls 0.23.43

## Published package identity

- Source: `https://static.crates.io/crates/rustls/rustls-0.23.43.crate`
- crates.io archive SHA-256: `0283386ce02abc0151e1761d08802dfe86c173b0b494af5cbc086574e453da06`
- Upstream tag commit: `fcf61cdbba30913cfd5b40aefa83989c6233812d`
- License expression preserved from the package: `Apache-2.0 OR ISC OR MIT`
- Original authored-file Git blobs: `buffers.rs` `bf0987407009993afa8e80073e31f4c511ec5610`; `conn.rs` `4ea90e7e051935f913c1dbb4b2a40f998c4e7303`; `lib.rs` `8652415afe564cc4d50fdd0742e1ece154a78429`.

The complete published package is copied at this directory. The original #424 comparison verified 116 package files outside its three source files and this provenance file byte-identical to the archive; its canonical sorted `SHA256  path\n` manifest has SHA-256 `4636769c464bc5e875ece3e5aa49499826534593b8e6fb30062664d25ad2c747`. Protected #429 subsequently authorizes three additional source files, recorded below; that original digest remains pre-#429 evidence and is not misrepresented as the current six-file authored exclusion set.

## Oteryn authored delta

Authority is protected amendment `OTV2-WP3-RUSTLS-DEFRAMER-OWNER-20260907`, applied to the existing #351/#356 worker after protected #424. Authored custody is limited to:

- `src/msgs/deframer/buffers.rs`: public std-only owner interface, exact-capacity replacement, overlap custody, drop ordering, and inline focused tests;
- `src/conn.rs`: the narrow `ConnectionCommon::set_deframer_buffer_owner` installation method;
- `src/lib.rs`: std-gated owner interface exports;
- this provenance record.

For an installed owner, rustls reserves the complete prospective `vec![0; capacity]` backing before constructing it. This construction reports exactly the requested capacity for `u8`; it does not copy or predict `RawVec`'s private reserve growth schedule. The old capacity remains charged while the replacement is allocated and copied. Rustls swaps the buffers, destroys the old backing, and only then releases its old charge. A pending RAII reservation rolls back denial/unwind paths. Read errors and `WouldBlock` retain the prepared backing charge, and `DeframerVecBuffer::drop` destroys backing before releasing the final charge. Denial occurs before allocation and before the reader call. The unhooked branch retains the upstream resize/shrink implementation and no-std builds contain no owner fields or interface.

The SQLx adapter uses the existing operation `ResourceBudget`; focused SQLx tests install it on a real `ClientConnection` and prove funded `WouldBlock` retention, connection-drop release, and denial before reader invocation. This proves only the deframer owner hook and preserves the earlier narrow `TLS_BLOCKING_OWNER = PROVEN`. Complete configuration, decoded structures, session/cache and handshake-overlap composition, actual TLS-positive evidence, and configured PostgreSQL 17.6 remain separate acceptance cells.

## Validation notes

The published archive intentionally omits upstream repository-only `testdata/` and `test-ca/` fixtures referenced by the crate's full `cfg(test)` build. Consequently, a full rustls library-test command cannot compile from the published package alone. The authored inline tests remain in `buffers.rs`; the same production hook is executed by the SQLx cross-crate test. Production rustls checks, no-std checks, SQLx tests/Clippy, root graph checks, byte comparison, locked single-path verification, formatting, and governance are the applicable package evidence.

The four authored inline owner tests passed after overlaying the three authored source files onto an exact detached checkout of upstream commit `fcf61cdbba30913cfd5b40aefa83989c6233812d`, which supplies the repository-only fixtures absent from the published archive.

## Protected #429 ClientHello preconstruction delta

The later protected #429 amendment authorizes the narrow additional changes in `src/client/client_conn.rs`, `src/client/hs.rs`, and `src/msgs/handshake.rs`. The owner-aware constructor now reserves configured ALPN inner/outer backing before cloning, reserves the outer protocol-name collection before construction, and reserves the `ClientHelloDetails` clone while its source remains live. A single RAII custody record retains the same owner through connection drop; ordinary constructors and ordinary ALPN conversion remain owner-free. Denial unwinds custody only after constructed backing is destroyed. This closes the protected ALPN seam only; session-cache, key-share, ECH/configuration and crypto custody remain separate open cells.

## Protected #432 session-retrieval owner delta

Protected allocation `OTV2-WP3-RUSTLS-SESSION-RETRIEVE-OWNER-20260908` is
implemented only at the three repaired retrieval surfaces. The ordinary
`ClientSessionStore::tls12_session` contract is unchanged. Its additive
owner-aware sibling fails closed by default, while `ClientSessionMemoryCache`
invokes the #425 retained-value owner-aware clone before destination secret
backing is allocated. That clone uses an exact-capacity byte vector, keeps the
stored source alive and charged throughout construction, and embeds RAII custody
whose field ordering destroys secret backing before releasing its charge.

The dedicated owner-aware `ClientHelloInput` route now reaches that dispatch
before TLS1.2 cloning. TLS1.3 still uses the existing destructive ticket take and
moves its opaque custody without another session allocation or charge. The
owner-aware route is explicitly TCP-only; QUIC parameter copying remains outside
this amendment. Rejected compatibility, clock and expiry paths drop the returned
opaque value normally, so its backing is destroyed before embedded custody
releases. Custom stores without pre-clone support cannot call through to the
ordinary allocating method.

Focused exact-upstream-overlay tests prove funded TLS1.2 source/destination
charge overlap, denial before destination allocation, exact release, retained
source validity, and unsupported-store fail-closed behavior. Ordinary retrieval
remains available and unchanged. The next executable allocation boundary is the
still-unallocated `client::tls13::initial_key_share` call to
`SupportedKxGroup::start`; no key-exchange or crypto source was changed.

## Protected #425 decoded-owner span checkpoint

The owner-aware client constructor now initializes the handshake deframer with
the same existing `DeframerBufferOwner` before its initial span-vector
allocation.  Checked prospective capacity is reserved before each replacement;
old and new capacity charges overlap until the old backing is destroyed, drained
high-water capacity stays charged, and final backing destruction precedes
release.  Focused inline tests cover initial max-minus-one denial, exact initial
capacity, growth overlap, retained high-water custody, and final release.  This
is one protected decoded-owner boundary only; complete decoded/TLS accounting
remains open.

## Protected #425 decoded reader/list checkpoint

The same #424 owner identity is retained in `ConnectionCore` and propagated
into both normal inbound decoding and the first-handshake shortcut. Root and
nested `Reader`s share one connection decode tracker without creating a second
ledger. Generic TLS lists use checked `capacity * size_of::<T>()` accounting,
reserve the complete prospective exact capacity before growth, keep the old
charge through allocator replacement, and release it only after the old
backing has been destroyed. Focused controls cover funded actual capacity and
max-minus-one denial. Ordinary owner-free and no-std parsing remain unchanged.

This checkpoint is not complete decoded/TLS accounting. Payload, message,
transcript, certificate/OCSP, successor-state, compressed-certificate,
peer-chain and retained-session boundaries remain open.

The final-graph span regression funds the live 16-element backing with one byte
less than the prospective 32-element replacement overlap. It proves denial
before replacement allocation or mutation: pointer, capacity, contents, and
old charge remain unchanged until deframer drop.

The decoded-owner bookkeeping control block is now independently charged: the
pinned Rust 1.94 `ArcInner<DecodedOwner>` requested layout is reserved before
`Arc::new`, with an external connection field ordered after the Arc so the
control block is deallocated before release. Generic decoded-list growth uses
the pinned geometric capacity target, preserving amortized behavior. Local
partial-parse and actual-capacity mismatch paths destroy prospective backing
before rolling back its debit. Successful decoded-list custody is still
connection-aggregate and therefore remains explicitly not proven; it must be
replaced by backing-coupled custody/transfer before complete TLS accounting.

## Protected #425 decoded byte-payload checkpoint

Owner-aware nested readers now reserve the full qualified byte-vector capacity
before `PayloadU8` and `PayloadU16` copy their input. The returned allocator
capacity is checked; on mismatch the vector is destroyed before the prospective
debit is released. Focused controls cover exact-capacity admission and
max-minus-one denial. Owner-aware message parsing checkpoints aggregate decoded
custody and rolls back partial nested allocations only after parser unwind.
Generic-list mismatch cleanup likewise destroys the replacement before
releasing old and prospective charges.

This is not complete decoded/TLS accounting. Borrowed payload `into_owned`,
parsed/encoded message overlap, handshake AST, transcript, certificate/OCSP,
successor, compressed-certificate, peer-chain and retained-session ownership
remain open.
old charge remain unchanged until deframer drop.

## Decoded-owner Arc and list-growth repair

The connection reserves the exact Rust 1.94 `ArcInner<DecodedOwner>` requested
layout before allocation and holds that debit in external custody until after
the final Arc is destroyed. Generic decoded lists use checked geometric growth,
reserve the prospective full capacity before allocation, retain old/new overlap,
verify actual capacity, and destroy a mismatched replacement before rollback.
Ordinary readers retain amortized growth. Per-list backing-bound final custody is
still open; the aggregate decoded owner is not claimed as complete TLS proof.
