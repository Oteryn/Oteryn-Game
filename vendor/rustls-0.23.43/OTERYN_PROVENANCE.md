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
