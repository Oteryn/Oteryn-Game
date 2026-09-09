# Oteryn sqlx-core0.9.0 provenance

## Exact upstream import

- Source: crates.io `sqlx-core` version `0.9.0`, original `.crate` archive.
- Archive SHA-256: `05b44e85bf579a8eeb4ceaa77a3a523baf2bf0e9bac7e40f405d537b5d2d5ccb` (verified before extraction).
- Upstream VCS: `003b698e99e024f3621b8043a2426fde5b741171`; subdirectory `sqlx-core`.
- Licenses: upstream `LICENSE-APACHE` and `LICENSE-MIT`, preserved verbatim.
- Game authority: issue351, Work admission comment5560220858; immutable main `53c6bdf06a2282d893035a995c46052c88f935b4`.
- Import method: Python tarfile safe data extraction of the verified archive; all original regular files preserved. No cache-generated `.cargo-ok` or replacement manifest imported.

## Complete patch manifest

- `src/net/mod.rs`: additive export of owner-supplied accounting primitive.
- `src/net/tls/mod.rs`: adds the dedicated test module and bounded synchronous Reader/File data-owner primitives; the original async loader/dispatch remains unmodified.
- `src/net/tls/tls_rustls.rs`: additive checked decoder-phase and private pre-verification chain bounds; no active handshake/socket accounting hook yet.
- New `src/net/resource_budget.rs`: reservation/transfer/backing-drop custody, no TLS bound or new budget constructor.
- New `src/net/tls/resource_budget_tests.rs`: independent denial, overflow, lifetime, cancellation, concurrency, bounded-debug, decoder-bound and actual File/growth tests.
- New `OTERYN_PROVENANCE.md`: this import, delta and proof-obligation record.

Existing TLS dispatch/socket protocol behavior remains upstream in this checkpoint. `ResourceBudget` must be supplied by B's existing owner ledger; the crate creates no independent limit. Neither the primitive nor its tests establish TLS/SQLx preallocation coverage. There is no budgeted TLS constructor or PostgreSQL activation yet.

## First TLS proof gate — OPEN

The admitted first checkpoint gate is complete TLS capacity/lifetime pre-call accounting. It is not closed by this custody primitive. Do not begin substantive PostgreSQL decoder work or claim B/Server Seam readiness from this checkpoint.

Pinned Game runtime graph: rustls0.23.43, ring0.17.14, Rust1.94 x86_64. The untouched upstream crate Cargo.lock resolves rustls0.23.40 for its separate crate tests; those tests qualify the component primitives on that lock, not the complete Game TLS path. The root locked build/Clippy use0.23.43. Both upstream Cargo manifests/locks remain intact; no test/dependency suppression occurred.

### Proven source and capacity terms

- `rustls/src/msgs/handshake.rs` and `msgs/codec.rs`: TLS1.3 CertificateEntry is48bytes and CertificateDer24 on the pinned target. Scratch source probe with13,105 entries observed entry capacity16,384 and owned-chain capacity32,768: both retain786,432bytes. In-place collection changes element capacity, not backing bytes. The cloned chain has exact13,105 capacity,314,520metadata bytes plus DER bytes. At8,192-to16,384 entry growth, old393,216 plus new786,432 may overlap:1,179,648 bytes.
- `client/tls13.rs` and `handshake.rs`: `cert_chain.end_entity_ocsp().to_vec()` overlaps original owned OCSP, the helper's returned clone and the caller's destination: three payload copies. Borrowed input backing is separately retained.
- `client/tls13.rs`, `msgs/persist.rs`, `client/handy.rs`: incoming ticket construction clones the peer chain before cache eviction. Original retained chain, eight cached clones and the incoming clone may coexist. Each distinct ticket has its own payload; `nst.ticket.clone()` is an Arc clone, not an additional byte-buffer clone. Secrets and owned metadata remain charged. SQLx creates fresh configuration per connection; idle connection/config/cache lifetime is not handshake lifetime.
- `msgs/handshake.rs`: ECH retry-config parsing occurs even when ECH is disabled, before connection-state rejection (`conn.rs` constructs Message before `process_main_protocol`). Unknown empty ECH configs cost4wire bytes but112bytes per slot.8,193 entries cause capacity16,384 (1,835,008bytes); possible moving growth overlaps old917,504 and new1,835,008:2,752,512. This counterexample is decoder amplification, not a TLS verification bypass or proof that all valid traffic exceeds the slot.
- `msgs/deframer/handshake.rs`: span-vector capacity remains after processed entries drain. Its historical high-water cannot be inferred from the current message's remaining wire bytes. Account resident old capacity. Span growth and subsequent AST growth are distinct phases. Source closure: `conn.rs:953` drains ready messages before reading another record, and `conn.rs:1098–1099` immediately coalesces each input. Before input, spans are empty or one incomplete span; dissection emits complete messages using at least four bytes and at most one incomplete tail. Ring TLS1.2/1.3 plaintext is at most16384; pre-handshake raw record payload is at most18431 (record-header rejection at18432). Therefore live spans are at most `1 + ceil(18431/4) = 4609`, and the pinned initial16/doubling Vec has capacity at most8192:327680 resident bytes,491520 during moving growth. Retain that source-derived high-water across the connection lifetime, independent of accepted NewSessionTicket/KeyUpdate counts; no monotonic lifetime-record cap or public reset callback is needed. Decoder phase must include resident span capacity plus AST growth; it does not coincide with span growth.
- `rustls-pki-types/src/pem.rs` slice parser borrows lines, starts base64 scratch at1024 and requests DER capacity `3 * ceil(base64_length / 4)`. Padding truncation retains that capacity. Account immutable input and SQLx's copy, scratch growth, accumulated DER/vector capacity, unknown labels and malformed-line errors. Preflight source bytes once; do not check a file and then reread it unrestricted.
- Correction to earlier pre-admission shorthand: ring's generic bigint parser may allocate before private4096-bit semantic validation. Generic capG=1024bytes, valid private modulusN=512bytes, valid primeH=256bytes. Source-derived constructor payload sum over all allocation sites is `4G + 3N + 6H + 528 = 7696`; retained successful RSA payload `2N + 7H + 528 = 3344`. Signature extra payload is `7N + 3H` on x86_64, `7N + 5H` on generic paths, including output signature. Add actual key/signer Arc/Box layouts and DER input. EC SEC1 conversion overlaps originalK, DER-wrappedW(K), andW(24+W(K)); EC/Ed ring key material/scratch is inline, not a guessed heap table.

These terms are independently useful but are NOT a complete capacity proof or an authorized fixed driver/TLS allowance. Source byte limits are not Vec capacities. No new numeric TLS, certificate, input or wire cap is chosen here.

### Remaining bounded work before TLS activation

1. Close all client decoder branches, including unexpected handshake types parsed before state rejection, nested minimum Vec capacities, ECH suites/extensions, OCSP responder IDs and extension BTreeSet allocation. NonEmpty ALPN and PSK binder entries require at least2wire bytes, DistinguishedName at least3; do not use an invalid zero-name assumption.
2. Complete configuration/root/provider/cache/key object layouts, selected-feature algorithm scratch and send/error overlap. Initial memory-cache capacity32 includes hash buckets/control bytes and ServerName deque; one host's eight-ticket deque is additional. Preserve original verification, TLS modes/versions/features and cache semantics.
3. Construct a source-derived phase reservation against the same remaining B slot balance *before* entering rustls. A late verifier or session-store callback is insufficient: input decoding and incoming chain construction already happened. Retain cache/backing high-water until actual drop or a proved charged transfer.
4. Qualify hostile inputs and valid ledger-feasible accepted operation maxima on the exact Game dependency graph. Exhaustion is unavailable before the call, not a new smaller semantic cap. After possible DB submission, B ambiguity/custody remains untouched.

Charged resident-work accounting covers requested capacities and owned metadata. It is not an RSS guarantee or an arbitrary allocator-overhead percentage. Inline reservation/enclosing Arc/Box metadata needs its containing owner's charge; an inner destructor cannot certify the enclosing allocation has already been freed.

### Window3 exact scope insufficiency — BLOCKED (P1 accepted/fixed)

The blocking-loader/Cell acceptance property cannot be implemented in the admitted
two-crate paths without guessing or releasing custody early. This is now a concrete
excluded-dependency boundary, not a general claim that all TLS accounting is
impossible:

- `CertificateInput::data` delegates file reads to `crate::fs::read`, which calls
  `crate::rt::spawn_blocking`. The latter selects Tokio when a current Tokio handle
  exists and otherwise selects async-global-executor, smol, or async-std according
  to enabled runtime features. Both `src/fs.rs` and `src/rt/mod.rs` are imported
  upstream files outside the exact core accounting amendment.
- In the configured root graph, Tokio1.53.1 `spawn_blocking_inner` first constructs
  `BlockingTask<F>` and then calls private `task::unowned`. That call allocates a
  private `Cell<T, S>` containing `Header`, `Core<T, S>` (scheduler, ID and the
  future-or-output stage), and `Trailer`. The exact allocation layout is computed
  from private generic types and private offsets in Tokio `runtime/task/raw.rs`.
  SQLx receives only a `JoinHandle<R>`; there is no public preallocation callback,
  allocation-layout query, or owner-custody attachment point for the Cell.
- Capturing a `ResourceReservation` in the blocking closure is insufficient. The
  reservation then lives *inside* the Cell's future/output stage and is destroyed
  while Tokio tears down that stage, before the enclosing Cell allocation is
  deallocated. Dropping the SQLx JoinHandle is also insufficient because Tokio
  blocking work may continue after cancellation and the runtime/scheduler retains
  the Cell. Thus either placement releases before backing dies or cannot survive
  cancellation/idle runtime custody.
- Cell custody alone is also insufficient. An input/job can cause the shared
  blocking pool to grow its job-queue backing or worker-map backing, or to create
  or grow a worker/thread packet. That operation-attributable backing can outlive
  its Cell through pool idle retention or runtime shutdown. Releasing all charge
  when the Cell is deallocated would therefore release custody while actual
  backing remains.
- A numeric duplicate of Tokio's current private layout would be the forbidden
  hidden magic reservation: it cannot be checked through SQLx's public API, differs
  for the alternate supported runtime branches, and does not transfer/release with
  the actual allocation. Replacing `spawn_blocking` with a synchronous read would
  change scheduling/cancellation behavior and is not a semantics-preserving fix.

Reproduction evidence on the configured root lock: Tokio1.53.1 checksum
`202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`;
source SHA-256 `core.rs` `7c896dec6ef10054ed2b8049a2b35667eae348615c162a46d1ceb9181fc71b5c`,
`raw.rs` `0e20384326d63f003a0340a80ea2e404d0af3eb3ff0f03a9e87a363531077efc`,
and `blocking/pool.rs` `6e1f4f3c1e6f7974a8ccab4dab8b8784f447097990f763fc5703202e4d278a65`.
The admitted SQLx runtime dispatcher SHA-256 is
`436902d5a1d1a1b320fbc0b0a2f371db0b85b50bfce066a55d42db7febc1c449`.

Smallest required amendment: accounting at the blocking runtime's actual owners,
not a Cell-only hook. The protected amendment must (1) atomically reserve the
actual generic `Cell<T, S>` allocation before allocation and retain custody through
its real deallocation, (2) account for input/job-induced blocking-pool backing
attributable to the operation, including job-queue growth/backing, worker-map
growth/backing, and worker/thread packet creation or growth, and (3) preserve that
same ledger/custody across success, failure, cancellation, runtime shutdown and
idle retention. A charge may release only when its actual backing owner releases
the backing or through a proved charged transfer. Where shared-pool growth cannot
be exactly attributed as a per-operation reservation, the evidence-backed
alternative is a registered loading/runtime owner with explicit capacity custody;
it is not a fixed per-job share of global pool capacity or a copied private byte
constant. Merely leasing `sqlx-core/src/fs.rs` or `src/rt/mod.rs` without those
dependency/runtime owner hooks is not sufficient. Rustls and other dependencies
remain untouched. Per the task boundary, source mutation stops here; TLS gate,
PostgreSQL expansion, and the include-only driver test activation remain OPEN.

## Original file digest manifest

These SHA-256 values identify the original archive bytes, including the files explicitly changed above. Every other original file must continue matching its digest; new files are identified in the patch manifest. Final candidate identity belongs in PR/check evidence, not a self-referential digest here.

| Original path | SHA-256 |
|---|---|
| `.cargo_vcs_info.json` | `a3851597386fb151b4b1ba12343e6945134fe378f77ca6dfdb6f34b3a7b9d96c` |
| `Cargo.lock` | `1525cca78a944110b24a65a7c861a8e50730565d11ed05b3164039fe0078f4c4` |
| `Cargo.toml` | `d62bb81e87b97948b8a24c75d5c17519fb183f1a837085d76ddf30f0ab9b65be` |
| `Cargo.toml.orig` | `4830c40ed9af5bc41b3a316eb4347d15941213966a16f4e5e938a5b6e5d74ee3` |
| `LICENSE-APACHE` | `154397a08d0cb342110281129971868cd562542d0780b0d10722788cd11a1399` |
| `LICENSE-MIT` | `72af89a828df52a9dcc42512ed64663b3b8d7961dd7a5586d44478eede82bc7a` |
| `src/acquire.rs` | `0bf5340d56ca77941023f154e33cf008779ed49e95381d6ed9bc882732c48e72` |
| `src/any/arguments.rs` | `1e53007f0ecfddabcda11fcd0cb31a48bc14efdea2223b5e8c0e31592433cfa0` |
| `src/any/column.rs` | `7a4effdf7730d293ddc14e2f11ec55f2f9be153b70113b8f6282e7c243f6cf24` |
| `src/any/connection/backend.rs` | `6a5eba6c5c913abab6dc5681c2651553e5af048945a88ced2acac8b9c615aa1a` |
| `src/any/connection/executor.rs` | `ff3176167afae95b1ebed9b2fc756fd3fac7d3c79c06f45e5ef568d3aa4d4347` |
| `src/any/connection/mod.rs` | `43a82396a29e3cb2ab8d5b2d3870347e966324a6975dc03d0bf13faa6fc540e7` |
| `src/any/database.rs` | `bf52d5f08b53711a3279c8b95000ff3b6a13206e366135d77fc9c3250ac30afa` |
| `src/any/driver.rs` | `71d60a71419e05dd9a636884ae5e221f59c37132849fd0871e1ef9e3fa39babb` |
| `src/any/error.rs` | `98edd701b67da8aaebdec3381ccb8b873aab9461588a1e95d25f0ba32b848ac2` |
| `src/any/kind.rs` | `0e62c7fd74a377495c5cc17b154449072885f58b81d1fcd105b3f227c1418e31` |
| `src/any/migrate.rs` | `13d1beb7d1f6f821109bd7c9405a459786054670716a5a23b5752e047291a43b` |
| `src/any/mod.rs` | `f664321cdf211694358e6b4a6c310743c9bf4afe40797cf68a6543eea2ebb882` |
| `src/any/options.rs` | `e0d225c178c3a16bed9b323de2d424b04cf4d47e95f3b7e4a535260180bd45e4` |
| `src/any/query_result.rs` | `a646521abf8051b39c827cd8609a01f773939b023891708ee5a8d99751c7c902` |
| `src/any/row.rs` | `c0beb33071b862c65fe4dbdf0cb5c4144a52217d13a4354602d8ad604bb0b4ba` |
| `src/any/statement.rs` | `e917f3cbdba8e51cc135cf63088df02f40362b5a49e97811eaceac97efa31607` |
| `src/any/transaction.rs` | `d8a3c016ed354344a706745e03e3af8db4241930ea82bb8c83340d3d67d48ffd` |
| `src/any/type_info.rs` | `1021053c91aaec8538151d2751bc477f9070515debd0348ff24b34bb863d7eeb` |
| `src/any/types/blob.rs` | `1af4bac0dfaa46be02cf40a359e4d943edfa85100ad6f6c9573ad34638dbfaaf` |
| `src/any/types/bool.rs` | `edb4c52a42cc0b93cd228928d138219264b028f75db46c3d6a9e7b2b5670890b` |
| `src/any/types/float.rs` | `64cb0d269191f3d8cafc918801f44416ba240d8ed209e0be64ea95d6cde37ad0` |
| `src/any/types/int.rs` | `9b0c7d6a223f7f74770265c2ebcea8a682ad6a5efdad2dc2fcd041a10751cce7` |
| `src/any/types/mod.rs` | `6fac79cfc0a913a273aae9332f3841ee3d810330d234e75b9beb82b7f815d559` |
| `src/any/types/str.rs` | `44f4f0424abe0064628920597895415429e04ef1f4c03aa9275be6cc3e597952` |
| `src/any/value.rs` | `ebbbfa222f922c16079c547db9537755d814734ec7072ad37a25dc329e2f2fcd` |
| `src/arguments.rs` | `988065552995906e6cd4b6d5dbf7bb35c47a69a173336460b39975bdb9dc7fa4` |
| `src/column.rs` | `c2a3bd28ed0586d8c1777372bfb16b87842bf3526a8c7fa6633ee510bb3961f2` |
| `src/common/mod.rs` | `13ebe463198f937d331f1fc1c0eddfd8f45ac2599925d829b7c9a8fb5f97f080` |
| `src/common/statement_cache.rs` | `14a82d9719afac1abc088809c23c87f07069fafecaf0ba12cf64d02c802c3e62` |
| `src/config/common.rs` | `6c80e3a9e9e566ef0e2011f452bc9272280b905fb611be516725d6fb4337016b` |
| `src/config/drivers.rs` | `631d9e947bccdf741d35087b5cabbca25d7e3cef2853b7b99e45376255814179` |
| `src/config/macros.rs` | `dbb3e7ebe9af11e7dbe2702498c45464259b2ad2540294e6aa261e5e88b16b02` |
| `src/config/migrate.rs` | `a9930f3735cbdf0f258f0eaea780c9086811e08e709180f46a15c93509b25f5a` |
| `src/config/mod.rs` | `8973cb33874fad55c5932063df6a4507b3278d140cc24ffa1535ead0cc50b045` |
| `src/config/reference.toml` | `86f12cee6a2ddd12119a5a9577e0226f37681afca3f129be785ae847b7dc77ee` |
| `src/config/tests.rs` | `667fb1a9df343e093ba0da93bbfc7d387100a04e4efee5fefc04a7141f9ee34b` |
| `src/connection.rs` | `a0ea974fb1bfe6ec7891e594333098b3d2fac3b3e27737e733dea461e8dee737` |
| `src/database.rs` | `e242d2c0d1329755871849cd09f676513ca65cd3a5332d9b1996db78b7ba3864` |
| `src/decode.rs` | `d6d0b300079a0c63855355b6338fc17fba490c26b73653cee2daebf0e77a6a2d` |
| `src/describe.rs` | `82b8836416bf85ed6846654783cc5eceaf9f2691ccb68184edfc5275d4ca2a36` |
| `src/encode.rs` | `68afa36b1394c5d55598fa328d019ee88b08516770567def90e5f52af697a6d0` |
| `src/error.rs` | `c3953bfa7ea84b879a544e71069ef6db6a9da756a2c1f13e8362485a15556ec1` |
| `src/executor.rs` | `bd4427048da65cb279cfa900173a1d40b9d9154c049467f9f511cac63a05a651` |
| `src/ext/async_stream.rs` | `a7420868e090025b6c2ce288e77ffc41aac72862376085f39d4b445a6699a8e2` |
| `src/ext/mod.rs` | `252205889bf4b6947f2378ab834c09048a891ecd64e050bbd43f6bdaa586571c` |
| `src/ext/ustr.rs` | `d4e3c326b2327be2cd55bc5559facf6e32a6ab85dc8b7bc3c285954663fd7575` |
| `src/from_row.rs` | `a54f236b0e187fe9a9acce4231d4d2aaa518e84f930809e66847502d2967b1ad` |
| `src/fs.rs` | `7eb4bcf7c4ac1bc509b35b737ce2dc1350c8601e98035e89047a061481a7de56` |
| `src/io/buf.rs` | `f71a80c83a6ba4a9fd26f3fc4852eb9ef9875b3edd524f20b6708bc88ec6567f` |
| `src/io/buf_mut.rs` | `b99937ad2160b6d6f3d70a202bdc3a0e2e19d4739d6b556ca7eebbb7e965acbf` |
| `src/io/buf_stream.rs` | `c95c6a823b43d6469119c66afd9619d7858b6c03197d9eb748583a319b7029f5` |
| `src/io/decode.rs` | `7a4e6edd4436a8ff7933023ea1de12bdde6cb2e5d2f677ebbdf2605ddec25a9f` |
| `src/io/encode.rs` | `1ebc1083ab5c43309709279b8e19af131ee3a0cd148d24eb90039963df36b83c` |
| `src/io/mod.rs` | `6b97888c6064c0978974a5ac1af33a02ba4842db867e00cf4a147dc9b164260e` |
| `src/io/read_buf.rs` | `440f40dd9ef3a0592b7d2cbdbe974dc2047678effd19e6b4acad0cd67c04e752` |
| `src/io/write_and_flush.rs` | `bc389233b794489d50dbd3aae8ab2c9fc7f8ad27a748538c8fdbd36179548d86` |
| `src/lib.rs` | `902fd5e1c537df608de9921190e8a3291992b850fe5230be6db028f49f4f20d0` |
| `src/logger.rs` | `16b3cfb958f5ae19e14a84638fbff53858c43994dae8c78ef79ae5e4b513e38f` |
| `src/migrate/error.rs` | `0aa6497cc3dd152ef41c704eb10b8ad3570129a9b51e6c4d36d58ee20f5ad21e` |
| `src/migrate/migrate.rs` | `89ac45b5566b200e527f95dcc8791fd4c21e2b2e9a90e4e11cda5a815189e5ac` |
| `src/migrate/migration.rs` | `9c63cdca1281f7a39ae0b1582b77e06f5a0a84a4e490230492b679e42582e1c4` |
| `src/migrate/migration_type.rs` | `53d7520ed216c002d9e5c7c039d477873d99fbbd805143f4910461e64bb2967a` |
| `src/migrate/migrator.rs` | `fcd6701394aa58b895237fa2b8cae89d754bdde10b5a6037a8ae796f5bf377c2` |
| `src/migrate/mod.rs` | `2e6f6270c79d1d929d5e446e813877b2cd64f2a82de5664ef93654a6d6558094` |
| `src/migrate/source.rs` | `38908ab0f901c079034049ea440a1877203e7ebf0ef65961d6b735133c4d86a0` |
| `src/net/mod.rs` | `fc9ce11d0a2542a9f3a9e50d210b678670001edf8fe1d3e6c1cb37ccbb7fdb0c` |
| `src/net/socket/buffered.rs` | `4ed9cbb4dd4fa68c6104b1e626178994ddd5e2708c7423cc137c01ea51e2829e` |
| `src/net/socket/mod.rs` | `54756a5db25c045e7b2b4c92b5be7d19ff030ed33517d3097fe01f5930e0ac9c` |
| `src/net/tls/mod.rs` | `01696d72da790695731b565f9473e3047ff5651b4d71781ef38e9e53fe104c81` |
| `src/net/tls/tls_native_tls.rs` | `8ac868fb9c5407a07034be42f45d158e2d6665aceb03236651658a69e91c218e` |
| `src/net/tls/tls_rustls.rs` | `498c9e5862118c79e773c7e20f1b7a5193d0e182c2294be89e3ec5f4b93c4cf0` |
| `src/net/tls/util.rs` | `0165e75a20f0e2f8052244bbf97fe21bd4fb288ce1fe5564fc116238a8369001` |
| `src/pool/connection.rs` | `94269a532ef60aaa31321e43af17cba2424f919a36501d862328ed05958a08de` |
| `src/pool/executor.rs` | `dbd98cd32d60361c515ae1311d461f5e09fd41d0033f37a65ae1417484c1d4be` |
| `src/pool/inner.rs` | `cba56628f259270dccb2f4a0d7a351dcff34d9239f6e6e1aee187ebe0e39c539` |
| `src/pool/maybe.rs` | `add426222f6f6ba9f8b2027c0b5ec4bf6fd32de5043488358da45fedd0244dc2` |
| `src/pool/mod.rs` | `ff4b2a9c91c80e7bcf5228883c7d4aec5f423520a790cefe1332a6b35d1ebcdb` |
| `src/pool/options.rs` | `924cd57a062328b2e7c6ac55242eabb1706893d80ce1e8cb02fe643a592f4f83` |
| `src/query.rs` | `481d05b7e92026e0bc8c0cd416d0c6165a82d94df10e0e3ac8e0fa1140386664` |
| `src/query_as.rs` | `466c71ff0aa85acc0d04fe49fbf15f418b6b7ae42d390c1d08d2ded580423b38` |
| `src/query_builder.rs` | `da851c2fda19650d7529601edf14d67f4c53823722dbe0dd04d9468a0e3f2f95` |
| `src/query_scalar.rs` | `5ac6dc8a75fbc2bbb41dedff5b16b6f765c5a6a2b593641c9e2c7157b3899b29` |
| `src/raw_sql.rs` | `27e6e2b9caaa75760cb0eabd9956a9ba08c264a7670c147c3f323e6e59f15d4e` |
| `src/row.rs` | `0529a1a6fac5a2c2e610c3ea6ebb265a307ec28d59d2bd29991d721fccfcdede` |
| `src/rt/mod.rs` | `436902d5a1d1a1b320fbc0b0a2f371db0b85b50bfce066a55d42db7febc1c449` |
| `src/rt/rt_async_io/mod.rs` | `3c6f2487f82803e2c849760284d5c9b0269959bc3ea76002543e766f8b946de5` |
| `src/rt/rt_async_io/socket.rs` | `70bab2dd599b18ff99d364c52722e90b3bcc6024bd967f733ff42c1433f5c1fb` |
| `src/rt/rt_async_io/timeout.rs` | `e7e44b83ad25dcba18fa4aa696d9875ba2e55ac5aed482d819b2ef402066c0ac` |
| `src/rt/rt_tokio/mod.rs` | `f8f5c7831b5335094fbf7313331b2e3d7eab6b72991dae20929abdf8577a091f` |
| `src/rt/rt_tokio/socket.rs` | `1095f99bd714d36715fc3285c91e9e12ccb72e93d90a601f4991c57f966c06f5` |
| `src/sql_str.rs` | `57550e4bc95deb464b520dfbf0287fd6214c4f8167383c9c8b512793c75f7832` |
| `src/statement.rs` | `26463cdbf8de36428d2d8b4feb458f9819d10bc23029ca4f3cee8a84609aac26` |
| `src/sync.rs` | `40ce401bca26cc5dc71462e45d3258c69591be00a45ddd93529e2b1c2e72e597` |
| `src/testing/fixtures.rs` | `044c09839cc9a0aa77a8e84dd2381835d832eb6244dc545ead0029665e424f17` |
| `src/testing/mod.rs` | `fc814c671ea70bf8ffbe20d7fb4dbe9614acbf2c77219838455167c7966ad82a` |
| `src/transaction.rs` | `7c0e1b28f1f0b56920d71b3340407abc2eeef9fe442321d25f43d1bf1721dbf7` |
| `src/type_checking.rs` | `255e38b99503974949873613d7d7cf3bb64372b7237b36d7f0180d9da11c9c89` |
| `src/type_info.rs` | `d75095c7f1b1ea24b0ab32e03640f01671cef74b1a56b1f2e2f9ec0ad4534d3b` |
| `src/types/bstr.rs` | `919fb5c0abdefaf3f5baede9bae0a9b2e69478186e90d32a4f1b7e72d2185be6` |
| `src/types/json.rs` | `377015c5f695b1e780ea6dc2b7b7584b4dd2899e03db56200015c99655525d67` |
| `src/types/mod.rs` | `9465752e9726049b87388ec7d10735a22e5650372da2555257c53c87bf25d801` |
| `src/types/non_zero.rs` | `1d164a0e5df01e947304a630115075c394576643c72c583659df9992a18bf9af` |
| `src/types/text.rs` | `735bf3a3c694fcbb089c0cf2d421ada91b6b3044e84689b362c8ff7b4fc75b37` |
| `src/value.rs` | `546f85b9e7eefea448f9908fba309e29f451e616c54a8bd64d493851b0b146d4` |

## Complete upstream license notices

Independent review P2 repair: archive license files are preserved pointer placeholders. The complete notices below were read from upstream VCS `003b698e99e024f3621b8043a2426fde5b741171`; this additive section supplies their contents without changing imported files. Window1 remains continuous from15:25:35Z; no counter reset or pause deduction.

### LICENSE-APACHE

Source: https://github.com/transact-rs/sqlx/blob/003b698e99e024f3621b8043a2426fde5b741171/LICENSE-APACHE

SHA-256 of the UTF-8 notice text inside the fence (including its final newline): `c8f5453612253e8ea8bad241617fe74c4a6d5d592ba64ff881668c211b50ac99`.

```text
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1. Definitions.

"License" shall mean the terms and conditions for use, reproduction,
and distribution as defined by Sections 1 through 9 of this document.

"Licensor" shall mean the copyright owner or entity authorized by
the copyright owner that is granting the License.

"Legal Entity" shall mean the union of the acting entity and all
other entities that control, are controlled by, or are under common
control with that entity. For the purposes of this definition,
"control" means (i) the power, direct or indirect, to cause the
direction or management of such entity, whether by contract or
otherwise, or (ii) ownership of fifty percent (50%) or more of the
outstanding shares, or (iii) beneficial ownership of such entity.

"You" (or "Your") shall mean an individual or Legal Entity
exercising permissions granted by this License.

"Source" form shall mean the preferred form for making modifications,
including but not limited to software source code, documentation
source, and configuration files.

"Object" form shall mean any form resulting from mechanical
transformation or translation of a Source form, including but
not limited to compiled object code, generated documentation,
and conversions to other media types.

"Work" shall mean the work of authorship, whether in Source or
Object form, made available under the License, as indicated by a
copyright notice that is included in or attached to the work
(an example is provided in the Appendix below).

"Derivative Works" shall mean any work, whether in Source or Object
form, that is based on (or derived from) the Work and for which the
editorial revisions, annotations, elaborations, or other modifications
represent, as a whole, an original work of authorship. For the purposes
of this License, Derivative Works shall not include works that remain
separable from, or merely link (or bind by name) to the interfaces of,
the Work and Derivative Works thereof.

"Contribution" shall mean any work of authorship, including
the original version of the Work and any modifications or additions
to that Work or Derivative Works thereof, that is intentionally
submitted to Licensor for inclusion in the Work by the copyright owner
or by an individual or Legal Entity authorized to submit on behalf of
the copyright owner. For the purposes of this definition, "submitted"
means any form of electronic, verbal, or written communication sent
to the Licensor or its representatives, including but not limited to
communication on electronic mailing lists, source code control systems,
and issue tracking systems that are managed by, or on behalf of, the
Licensor for the purpose of discussing and improving the Work, but
excluding communication that is conspicuously marked or otherwise
designated in writing by the copyright owner as "Not a Contribution."

"Contributor" shall mean Licensor and any individual or Legal Entity
on behalf of whom a Contribution has been received by Licensor and
subsequently incorporated within the Work.

2. Grant of Copyright License. Subject to the terms and conditions of
this License, each Contributor hereby grants to You a perpetual,
worldwide, non-exclusive, no-charge, royalty-free, irrevocable
copyright license to reproduce, prepare Derivative Works of,
publicly display, publicly perform, sublicense, and distribute the
Work and such Derivative Works in Source or Object form.

3. Grant of Patent License. Subject to the terms and conditions of
this License, each Contributor hereby grants to You a perpetual,
worldwide, non-exclusive, no-charge, royalty-free, irrevocable
(except as stated in this section) patent license to make, have made,
use, offer to sell, sell, import, and otherwise transfer the Work,
where such license applies only to those patent claims licensable
by such Contributor that are necessarily infringed by their
Contribution(s) alone or by combination of their Contribution(s)
with the Work to which such Contribution(s) was submitted. If You
institute patent litigation against any entity (including a
cross-claim or counterclaim in a lawsuit) alleging that the Work
or a Contribution incorporated within the Work constitutes direct
or contributory patent infringement, then any patent licenses
granted to You under this License for that Work shall terminate
as of the date such litigation is filed.

4. Redistribution. You may reproduce and distribute copies of the
Work or Derivative Works thereof in any medium, with or without
modifications, and in Source or Object form, provided that You
meet the following conditions:

(a) You must give any other recipients of the Work or
Derivative Works a copy of this License; and

(b) You must cause any modified files to carry prominent notices
stating that You changed the files; and

(c) You must retain, in the Source form of any Derivative Works
that You distribute, all copyright, patent, trademark, and
attribution notices from the Source form of the Work,
excluding those notices that do not pertain to any part of
the Derivative Works; and

(d) If the Work includes a "NOTICE" text file as part of its
distribution, then any Derivative Works that You distribute must
include a readable copy of the attribution notices contained
within such NOTICE file, excluding those notices that do not
pertain to any part of the Derivative Works, in at least one
of the following places: within a NOTICE text file distributed
as part of the Derivative Works; within the Source form or
documentation, if provided along with the Derivative Works; or,
within a display generated by the Derivative Works, if and
wherever such third-party notices normally appear. The contents
of the NOTICE file are for informational purposes only and
do not modify the License. You may add Your own attribution
notices within Derivative Works that You distribute, alongside
or as an addendum to the NOTICE text from the Work, provided
that such additional attribution notices cannot be construed
as modifying the License.

You may add Your own copyright statement to Your modifications and
may provide additional or different license terms and conditions
for use, reproduction, or distribution of Your modifications, or
for any such Derivative Works as a whole, provided Your use,
reproduction, and distribution of the Work otherwise complies with
the conditions stated in this License.

5. Submission of Contributions. Unless You explicitly state otherwise,
any Contribution intentionally submitted for inclusion in the Work
by You to the Licensor shall be under the terms and conditions of
this License, without any additional terms or conditions.
Notwithstanding the above, nothing herein shall supersede or modify
the terms of any separate license agreement you may have executed
with Licensor regarding such Contributions.

6. Trademarks. This License does not grant permission to use the trade
names, trademarks, service marks, or product names of the Licensor,
except as required for reasonable and customary use in describing the
origin of the Work and reproducing the content of the NOTICE file.

7. Disclaimer of Warranty. Unless required by applicable law or
agreed to in writing, Licensor provides the Work (and each
Contributor provides its Contributions) on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
implied, including, without limitation, any warranties or conditions
of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
PARTICULAR PURPOSE. You are solely responsible for determining the
appropriateness of using or redistributing the Work and assume any
risks associated with Your exercise of permissions under this License.

8. Limitation of Liability. In no event and under no legal theory,
whether in tort (including negligence), contract, or otherwise,
unless required by applicable law (such as deliberate and grossly
negligent acts) or agreed to in writing, shall any Contributor be
liable to You for damages, including any direct, indirect, special,
incidental, or consequential damages of any character arising as a
result of this License or out of the use or inability to use the
Work (including but not limited to damages for loss of goodwill,
work stoppage, computer failure or malfunction, or any and all
other commercial damages or losses), even if such Contributor
has been advised of the possibility of such damages.

9. Accepting Warranty or Additional Liability. While redistributing
the Work or Derivative Works thereof, You may choose to offer,
and charge a fee for, acceptance of support, warranty, indemnity,
or other liability obligations and/or rights consistent with this
License. However, in accepting such obligations, You may act only
on Your own behalf and on Your sole responsibility, not on behalf
of any other Contributor, and only if You agree to indemnify,
defend, and hold each Contributor harmless for any liability
incurred by, or claims asserted against, such Contributor by reason
of your accepting any such warranty or additional liability.

END OF TERMS AND CONDITIONS

APPENDIX: How to apply the Apache License to your work.

To apply the Apache License to your work, attach the following
boilerplate notice, with the fields enclosed by brackets "[]"
replaced with your own identifying information. (Don't include
the brackets!)  The text should be enclosed in the appropriate
comment syntax for the file format. We also recommend that a
file or class name and description of purpose be included on the
same "printed page" as the copyright notice for easier
identification within third-party archives.

Copyright (C) SQLx Contributors
Portions of this work Copyright (C) LaunchBadge, LLC

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

### LICENSE-MIT

Source: https://github.com/transact-rs/sqlx/blob/003b698e99e024f3621b8043a2426fde5b741171/LICENSE-MIT

SHA-256 of the UTF-8 notice text inside the fence (including its final newline): `5abbdd842310c8f1650fcde3a5a51a3e09412b9e5a6eea87519e0db9e32b339d`.

```text
Copyright (C) SQLx Contributors
Portions of this work Copyright (C) LaunchBadge, LLC

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```

## Window1 proof checkpoint after native publication

Native WIP head `703da76a870356f2ab94d6e7af11675dd24f6c04`, tree `07591d3c8668dec06c4190eb4784be04b3351469`, is draft PR356. Full TLS accounting remains OPEN; no runtime TLS adapter or PostgreSQL decoder accounting is activated. The original cache stays frozen; this documentation continuation uses a fresh cache normally fast-forwarded to that published head.

### Reachability and correlated allocation results

- Independent review confirms a private per-connection write-discard `ClientSessionStore` is behavior-equivalent for this SQLx adapter if installed through `Resumption::store`, preserving `SessionIdOrTickets`. SQLx creates one fresh private config for one connection. All session lookups occur in client/hs.rs1111/1119 and initial key-share lookup in client/tls13.rs317 before writes. HRR reuses the existing input and selects its requested group directly (hs.rs1012–1047); TLS1.2 renegotiation is rejected (common_state.rs204–213). Reconnect creates another fresh config. This is an accounting implementation candidate, not a new cache count policy; do not use `Resumption::disabled`. Initial default-cache construction still allocates before replacement and must be charged. Incoming validated tickets still deep-clone the peer chain before the callback and require pre-call reservation. This candidate is not implemented.
- Stock ticket-chain ownership remains independently verified: `CertificateChain(Vec<CertificateDer>)` derives deep Clone; the owned DER variant is a Vec. `tls13.rs1493` clones that chain before `persist.rs248` wraps it in a new Arc. The stock eight cached plus incoming ninth chains are separate allocations, despite sharing the same certificate contents. The incoming ticket payload Arc clone itself does not duplicate its bytes.
- Correlated ECH AST upper bound:3276720 requested heap bytes, excluding input, span storage and enclosing inline metadata. Let W<=65535 be list payload, n started four-byte configuration headers, and B<=W-4n remaining child bytes. Pinned sizes are outer112, extension32, suite8. Stable nested children are bounded by16B; child-construction moving overlap by24B. These factors derive from element sizes, wire prefixes and pinned Vec minimum4/doubling, not an allowance. Outer growth at n=C+1 is bounded by `3*C*112 + 16*(W-4*(C+1))`, maximized at C8192. Child construction with retained outer capacity is bounded by2621224 and is smaller. Source: codec.rs219–237, handshake.rs2879–3050, base.rs118–155 and pki-types server_name.rs198–204. This bounds the ECH branch, not every handshake grammar branch.
- A reproducible public-decoder witness with8193 configurations, one known config containing8180 empty extensions, uses65511 list-wire bytes,2097186 retained bytes and3014690 conservative moving-allocation peak. The scratch probe counts both requested old/new allocations during realloc. It establishes a witness, not the complete upper bound.
- Exact remaining feasibility gap: the conservative ECH upper3276720 plus persistent spans327680 plus accepted active request524288 plus deframer65536 equals4194224, leaving only80 bytes in a4194304 slot before configuration, crypto and result owners. Therefore these currently established conservative bounds do not certify the accepted maxima. This is not proof that actual simultaneous allocations exceed the slot. Tighten correlated span/AST reachability, or identify a necessary pre-decode accounting hook; do not invent a smaller input limit or reserve the whole slot.

### Verifier and lifetime closure still required

Pinned webpki0.103.15 `verify_cert.rs565–575` builds a Vec of decoded nonmatching EKU OIDs before deciding success; a matching final OID can still incur all prior temporary allocations. For OID byte lengths l_j and count n, a source-derived term is `24*C24(n) + sum(8*C8(l_j+1))`, where C is a checked pinned Vec capacity upper bound. Verification path search retains ranked errors while exploring another path; the six-intermediate limit bounds concurrent error contexts, not total lifetime attempts. Signature-algorithm errors also own input-derived vectors. SAN mismatch owns formatted names and lossy-UTF8 temporaries. The specific GeneralName formatter uses Display for lossy names: for payload b, DNS length is at most11+3b, URI29+3b, malformed IP20+4b, and fixed alternatives fit32+4b (subject_name/mod.rs322–406). With m names, S=`24*C24(m)+sum(C1(32+4*b_j))+C1(expected_DNS_bytes)` and T=`max(C1(3*b_j))`, construction/moving overlap is bounded by2S+T. Later error Display/Debug/log serialization may escape strings and requires a separate bound; it is not covered by this context-construction formula. Rustls stores an error clone before returning the original (conn.rs920), so returned error custody and the connection's saved error must remain separately charged. Repeated returned clones require per-call custody, not early release. The trace-enabled verifier OCSP log at server_verifier.rs273 also copies the response in that phase.

An accounting verifier wrapper can reserve before delegating actual-DER verification while preserving the configured checks. A session-store callback is too late to guard its incoming chain clone. Core's pre-call TLS guard cannot inspect encrypted handshake type/body before private decoding; absent a complete conservative grammar/lifetime bound, the conditional smallest visibility candidates are rustls conn.rs after decryption/before `Message::try_from`, and msgs/codec.rs before element decoding and Vec growth. These paths remain excluded and unmodified; their necessity has not been proved. Full grammar, configuration descendants, crypto/error layouts and ownership-phase composition must close before adapter activation.

Window1 stopped at 2026-09-06T16:20:49Z: 3314 seconds conservatively charged productive time, 286 seconds unused from the3600-second window, zero deducted pauses, zero counter reset. Continuation requires Work's next explicit bounded window. No adapter activation is authorized by this checkpoint.

## Window2 controlled continuation

Work comment5560554622 grants window2 from2026-09-06T16:25:30Z after fresh native `1363c9b5b238f4922615eda9b502866c305e83bf` bind. Immutable admission53c6 and prior counters remain unchanged. Accepted337 section3 makes maxima independently conjunctive:524288 is a queued charged-footprint maximum, not a mandatory reservation for every active request. The historical80/254-byte arithmetic remainder does not establish a scope/architecture blocker or promise that all independent maxima coexist. Source-derived reservations debit actual remaining owner capacity; positive work must be feasible, and otherwise-valid values may receive resource failure before allocation. No new field/count/wire threshold follows.

### Implemented, tested components

- Complete pre-state decoder grammar inventory independently closed for pinned std/tls12/ring client input, including unexpected message types. Non-ECH bound is `39*65535 +744+560 =2557169`: disjoint wire partitions cover typed vectors, moving old buffers, owned payloads, SNI copies and unknown-tag trees;744 covers minimum-capacity overhead for14 ClientHello vector families;560 is the largest extension Box. The ECH joint bound is3276650 including its200-byte enclosing ServerExtensions Box and unknown-tag tree. Nonroot BTree nodes retain at least5 keys; each node requests at most136 bytes and insertion split allocations persist into the tree, yielding a16-heap-bytes-per-wire-byte envelope including small roots. Known ServerExtensions descendants each fit their disjoint wire payload; duplicate ECH rejects before second decoding. For known ECH body `b=11+k+d+4m+4e+q`, k,d,m>=1, stable child allocation has at least174 bytes of slack below16b. The joint maximum is `3*8192*112 +16*(65535-4*(8192+1)) -174 -16*6 +200 =3276650`; six wire bytes belong to the containing extension header/list prefix. The all-unknown and nested-child-moving cases are smaller. `handshake_decode_heap_bound` implements the checked maximum and fails closed for an unqualified public type layout. This excludes input/span buffers, configuration, crypto, retained state and error owners.
- `HandshakeInputBound` derives the private unverified peer-chain footprint from bytes actually read before verification. TLS1.2 uses at most floor(W/3)24-byte entries; TLS1.3 at most floor(W/5)48-byte entries, with pinned minimum4/doubling capacity. Into-owned TLS1.3 preserves original backing bytes. Retained DER/OCSP source bytes are disjoint. W saturates at rustls' existing65535-byte message maximum without rejecting reads; it is not a lifetime quota. Future budgeted I/O must call public read_tls, update the bound, reserve, then call process_new_packets on every iteration; stock complete_io can loop internally. After handshake, use actual public peer-chain sizes instead.
- Synchronous Reader/File data-owner primitives reserve a4096-byte scratch buffer and old+new data capacity before geometric growth, read through EOF, and return the complete Vec bound to its guard. Linux File open's CString temporary is exactly path bytes+1 under pinned std source. Tests prove a7000-byte file retains8192 capacity,16384 total growth/scratch capacity succeeds, and16383 denies before growth with no retained charge.4096 is read granularity, not a certificate limit. The primitives do not schedule jobs or replace the current async File path.

### Exact outstanding blocking-loader boundary

Selected SQLx `rt::spawn_blocking` uses Tokio1.53.1's shared blocking pool (root lock SHA-256 `202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`). Beyond data storage, one call can grow the private global task VecDeque, resize the worker-thread HashMap or create a worker/thread packet (blocking/pool.rs406–421). None is a fixed per-job allocation from the public API. For ordinary pinned x86_64 configuration and closure/result alignment<=8, task-local Cell layout is `round_up128(120 + max(sizeof Option<F>, sizeof Result<R,JoinError>))`; large closures are separately boxed above2048 debug/16384 release bytes. This local expression does not cover the shared scheduler.

Dropping JoinHandle does not cancel a running blocking closure. The data guard must move into the returned owned result, because the result may wait in Cell after the closure finishes. Tokio can drop detached output before final task references/deallocation, so that result guard cannot also prove the Cell allocation has ended. A bounded registered configuration-loading owner could provide scheduler/job/result custody; alternatively exact accounting hooks would be needed around blocking pool growth and final task allocation release. No Tokio fork necessity has been established, no excluded dependency is changed, and no guessed scheduler reservation is used. The existing unbounded loader is not invoked by a claimed budgeted path; no budgeted path is active yet.

### Additional source-proved phase terms

- Fixed client-state forwarding DAG shallow peak2088 bytes, covering consumed outer Boxes through nested handlers; into_owned occurs after handle returns and is a separate smaller phase. Hash context224, HMAC key176, active ring KX208, record-key objects560 each, HKDF expander184; dynamic descendants and caller-prepared record-key pairs remain separate. SharedSecret owns at most the selected curve's secret length. These layout terms do not alone establish total TLS closure.
- Ring RSA verification uses at most five1024-byte generic limb buffers (5120 requested bytes). The generic parser rejects above MAX_LIMBS before allocation. Selected native Montgomery/limb objects use caller/stack storage, with no allocator symbols or indirect allocator calls in the inspected build. Signing/key-construction and other algorithm terms remain as independently enumerated earlier.
- For actual maximum certificate DER bytes d and expected hostname bytes h, source correlation closes EKU construction at `64*d+96`; supported-signature contexts fit `d+224` or `2*d` (14 borrowed static AlgorithmIdentifier entries). Let Q be their maximum. Concurrent path/signature errors have a source-derived10-context upper bound. SAN construction fits `137*d+208+4*h`, including formatted names, moving capacities and lossy UTF8 temporary. These are checked input-derived functions, not a certificate-size limit. Pinned Error/CertificateError/webpki Error inline sizes are64/56/56; external Arc/Box/returned-error ownership still needs complete composition.
- Client-auth transcript growth is finite until transition to Traffic, with HRR replacement/old-new overlap and actual configured client certificate flight included. Ordinary no-client-auth drops prehash buffering after start_hash. Each byte-Vec growth uses its actual length/capacity recurrence, not encoded length alone. Outgoing handshake conversion can hold encoded/flight and copied PlainMessage buffers together; record queue entries own exact payload+TLS framing capacities, and queue metadata high-water survives pops. Handshake output is not capped by the default64KiB application buffer limit. ClientHello encoding additionally allocates96 bytes of extension order plus up to192 bytes of cached sort keys. Full send/config/receive phase composition remains required.

Full TLS gate remains OPEN. No PostgreSQL decoder work, TLS mode change, cache adapter activation or hosted PostgreSQL/TLS qualification is claimed. Prospective test lease PR357 remains NOT_ACTIVE until protected admission; its future vendor test inclusion cannot supply qualification before actual exercised execution.

### Window2 validation and review scope

Sixteen core library tests passed on the unchanged upstream crate lock (rustls0.23.40); five tests are new in this window. The8193-entry unknown ECH decoder is actually executed. The mixed nested3014690-byte witness is a separately recorded scratch probe; its number is only compared with the bound in the unit test, not re-executed there. Strict vendor Clippy passes. Strict root Game server all-target Clippy passes on the locked rustls0.23.43 graph. This does not mean excluded vendor tests ran through root workspace CI, and it is not a budgeted TLS/PostgreSQL positive qualification. Explicit `cargo tree -p oteryn-game-server -i rustls -e features` readback confirms only rustls ring/std/tls12 for the server graph; logging and certificate compression are absent, so earlier conditional trace OCSP allocations are unreachable in this selected profile.

TDD RED/GREEN was performed for each new component. Strict Clippy found one fixture cast that could truncate; it was repaired with checked u16 conversion, with no suppression. Prior license repair history is preserved. Independent/runtime root review covers the three unactivated runtime diffs only; full adapter gates remain OPEN.

Window2 stopped at 2026-09-06T17:16:20Z: 3050 productive seconds conservatively charged, 550 unused seconds, zero deducted pauses and zero counter reset. Completed windows:2; repair cycles:2; rotations:0; identical-failure retries:0. Further implementation requires explicit Work continuation. Full TLS and PostgreSQL adapter gates remain OPEN; prospective PR357 remains NOT_ACTIVE.

## Window3 amended blocking-owner checkpoint: `BLOCKED_RUNTIME_BACKEND_OWNER`

The protected amendment at Game `main@73c48ce84b98d84298a26c5038ecbf7677d2ac82`
was merged normally with current protected main before this checkpoint. Its fresh
TDD RED is commit `f0dddec27ed8151f73d3db80750a89ccf15f3e77`: the focused
`cargo +1.94.0 test --lib --features _rt-tokio,_tls-rustls-ring-webpki`
reached SQLx-core compilation and failed specifically because neither the sealed
`rt::resource_owner::BlockingJobOwner` capability nor
`rt::spawn_blocking_owned` pre-spawn entry point existed (`E0432`, `E0425`). The
RED fixture is removed on this stopped checkpoint; no knowingly uncompilable API
is retained.

`BLOCKED_RUNTIME_BACKEND_OWNER`: the configured root graph enables only SQLx
`runtime-tokio`, pinned to Tokio 1.53.1 (crate checksum
`202caea871b69668250d242070849eb495be178ed697a3e98aebce5bc81a0bed`, upstream
tag `tokio-1.53.1` commit `75fef53d0a8590c2d1dbb63672aa7b7d1ef51155`).
SQLx's permitted dispatcher can acquire a Game charge before calling
`Handle::spawn_blocking`, but the public call accepts only `F` and returns only a
`JoinHandle<R>`. Inside excluded Tokio
`src/runtime/blocking/pool.rs:367-390`, `spawn_blocking_inner` constructs
`BlockingTask<F>`, allocates the private generic task through `task::unowned`,
and only then admits it to the pool. `spawn_task` then mutates its private
`VecDeque` at lines 393-407, may create a thread and insert its handle into the
private worker `HashMap` at lines 409-438, and retains worker state through idle
and shutdown. The inspected pool source SHA-256 is
`6e1f4f3c1e6f7974a8ccab4dab8b8784f447097990f763fc5703202e4d278a65`.

No public Tokio hook exposes the generic task layout, a fallible preallocation
callback, queue-capacity growth custody, worker-map growth custody, thread-packet
custody, or a final-release callback. Metrics observe counts after admission;
they cannot deny before allocation or transfer custody through queued, running,
completed, cancelled, detached, idle and shutdown states. Moving a reservation
into `F` or `R` charges closure/result data but releases independently of the
enclosing task and pool backing. A sealed SQLx wrapper would therefore only
assert ownership it cannot prove, and tests using a fake backend would not prove
the enabled backend.

GREEN is consequently not implementable within the amended paths. The smallest
next amendment is either (a) an exact Tokio 1.53.1 runtime hook covering fallible
preallocation/admission and final release of task plus attributable queue/map/
thread backing, or (b) an already-funded registered Tokio blocking-pool owner
with typed admission/custody that covers those same real lifetimes. It requires
Tokio runtime paths under `src/runtime/blocking/pool.rs` and
`src/runtime/task/{mod.rs,raw.rs,core.rs}` (plus the owning integration/test
paths), which remain excluded. This proposal does not authorize a fork, copied
private layout, fixed per-job share, new budget, TLS change or semantic cap.

TLS modes, protocol versions, certificate/hostname verification, cache semantics
and all accepted maxima are untouched. Complete TLS phase composition, actual
TLS-positive evidence, PostgreSQL accounting/17.6 qualification, independent
full-diff review, exact-head CI/MQ and protected readback remain OPEN. PostgreSQL
decoder work and shared-target inclusion remain stopped.

## Window6 enabled-Tokio adapter checkpoint

The protected Tokio owner surface is now adapted directly to the existing
`ResourceBudget` ledger. The adapter has no unowned fallback: it requires a
current Tokio runtime, supplies the accepted queue count 8 and explicit 2 MiB
worker stack configuration, and forwards task/queue/worker reserve and release
operations to the same operation owner. Other compiled runtime branches are not
used by the configured server graph and are deliberately not selected by this
owned entry point.

The Linux certificate-file composition binds the already-existing path backing
to a reservation before task admission, runs the previously-accounted reader
only through owned Tokio, and returns the charged `Vec` without detaching its
backing. Focused funded and denied tests prove denial before Tokio Cell
allocation, no fallback, returned-byte custody, and pool custody through runtime
shutdown. This proves the blocking-loader component, not the complete TLS gate:
activation in `CertificateInput`, retained rustls configuration/session/cache
ownership, full handshake phase overlap, actual TLS-positive evidence and
PostgreSQL 17.6 qualification remain OPEN.

## Window7 complete-TLS stop: rustls deframer owns pre-observation growth

The protected Tokio/certificate-loader prerequisite remains **PROVEN**, but it is
not complete TLS proof.  Continuing the configured rustls 0.23.43 handshake
reaches a different allocation owner outside every admitted SQLx/Tokio path.
`ConnectionCommon::read_tls` calls the private
`DeframerVecBuffer::read`; `read` calls private `prepare_read` *before* invoking
the SQLx-supplied `Read`.  `prepare_read` grows or replaces its private `Vec<u8>`
with `resize` and `shrink_to`, including old/new growth overlap.  Only after that
allocation has succeeded can the SQLx `StdSocket` return the number of bytes read.
The public rustls surface exposes neither deframer capacity nor a fallible
pre-growth reservation/custody callback.  Calling `read_tls` in a more granular
SQLx loop therefore remains post-allocation observation, while reserving a copied
private capacity schedule or an opaque whole-handshake allowance would violate
the accepted actual-capacity/no-magic-reservation invariant.

`SHARED_LEASE_REQUIRED = rustls-0.23.43/src/msgs/deframer/buffers.rs ::
DeframerVecBuffer::{prepare_read,read}` (prospective vendored path
`vendor/rustls-0.23.43/src/msgs/deframer/buffers.rs`).  The required resource is
the incoming TLS deframer `Vec<u8>` actual capacity and every temporary old/new
backing during growth/shrink.  The accepted invariant is reservation from the
same operation ledger before each backing allocation, followed by custody until
that exact backing is freed or a proved charged transfer occurs across success,
TLS error, cancellation, connection/session lifetime and drop.  The smallest
next decision is a protected rustls owner hook (plus its necessary public wiring)
that reports/accepts the concrete requested allocation before mutation and keeps
the permit with the private backing.  It must preserve rustls' existing wire
limits, versions, verification and I/O behavior; it cannot be a fixed private
layout constant, plaintext substitution or unowned fallback.

No rustls file was modified.  Configuration/decoder/session-cache and handshake
overlap accounting, actual TLS-positive qualification and PostgreSQL17.6 remain
OPEN behind this first newly proven owner boundary.  The include-only shared
PostgreSQL target was not changed or executed, and no PostgreSQL credit follows.

Follow-up exact-owner repair: Tokio's charged owner queue compares Arc identity.
Constructing a fresh adapter for each root/client certificate or key load would
therefore reject the second funded load as a different owner while the first
operation's queue remains alive. `BlockingJobOwner` now retains one runtime-owner
Arc across all loads in that operation. Focused tests execute two sequential
owned jobs and two sequential certificate loads on one runtime/ledger, proving
owner continuity without weakening the identity check or falling back to
unowned work. The deframer stop boundary above is unchanged.

## Window9 complete-TLS decoded-structure boundary

The protected deframer amendment closes only the incoming byte-buffer owner.  A
fresh composition audit reaches the next allocation before SQLx regains control:
`rustls-0.23.43/src/conn.rs::ConnectionCore::deframe` maps the decrypted
`PlainMessage` through `Message::try_from(pm).map(|m| m.into_owned())`.  Private
handshake decoding performed by that call allocates the decoded AST, peer
certificate chain and related owned payloads before `process_new_packets`
returns.  SQLx can neither identify the decoded branch nor attach a reservation
to that backing before allocation.  The #424 authority for `conn.rs` is narrow
public wiring for the deframer-buffer owner; it does not authorize a second
decoded-message owner or changes to this call site.

The already-reviewed conservative pre-call bounds cannot substitute for custody:
the correlated ECH decoder bound (3,276,650), retained handshake-span backing
(327,680), accepted active request (524,288), and deframer backing (65,536)
total 4,194,154 bytes, leaving 150 bytes in the accepted 4 MiB owner slot before
configuration, provider/crypto, peer-chain, session/cache, send, transcript and
error backing.  Those remaining owners are nonzero.  This proves that the
current conservative composition cannot fund the accepted positive case; it
does not justify a smaller TLS limit, a whole-slot reservation, or releasing
custody early.

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/conn.rs ::
ConnectionCore::deframe (before Message::try_from / into_owned)`, with only the
necessary public owner wiring.  The required resource is the actual private
decoded-message/handshake backing, including allocation-growth overlap and
transfer into retained peer-chain/session state.  The accepted invariant is
same-ledger reservation before allocation and custody until actual destruction
or a proved charged transfer across success, TLS failure, cancellation, drop,
cache/session retention and handshake overlap.  The current #424 allowlist
cannot satisfy it because its `conn.rs` grant is solely the deframer owner
installation surface and its protected scope expressly excludes decoded-state,
client/config/session/crypto/error ownership.  No plaintext/unowned fallback,
private-layout constant, semantic cap or TLS-policy change is acceptable.

Complete TLS therefore remains **NOT_PROVEN**.  TLS-positive execution and the
configured PostgreSQL 17.6 qualification remain correctly stopped; plaintext
PostgreSQL and CONTROL classifier results supply neither credit.

## Window10 protected decoded-owner amendment preflight

Protected allocation `OTV2-WP3-RUSTLS-DECODED-OWNER-20260908` was applied to
this existing worker after normal merge-up to protected
`main@dfc0fd3a9148cb85b7c75e7cad3b15a1fe70d2eb`.  Before changing decoded-state
source, the required ClientHello qualification exposed one narrower missing
symbol boundary inside an otherwise listed path.  The amendment grants authored
`client/hs.rs` custody only for
`ExpectServerHelloOrHelloRetryRequest::{handle,handle_hello_retry_request,into_expect_server_hello}`
and successor-state construction.  Initial and retry ClientHello input-derived
allocations instead occur in `emit_client_hello_for_retry`: the function creates
`Box<ClientExtensions>`, collects the named-groups vector, clones protocol and
QUIC transport-parameter vectors, and deep-owns certificate-authority names.
Its unchanged source blob is `34fc1ae1687e5978b735f0237d1de5e66b2eb609`.

Those allocations are explicitly part of the amendment's minimum focused
ClientHello extension/payload proof, but the function is not one of its granted
symbol boundaries.  Neither `conn.rs` wiring nor an owner-aware decoded
`Reader` reaches outbound ClientHello construction, and post-construction
inspection cannot deny before allocation or retain source/destination overlap.

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/hs.rs ::
emit_client_hello_for_retry` for same-ledger reservation and custody of the
actual `Box<ClientExtensions>`, collected vector capacities, cloned protocol /
transport payload capacities and owned certificate-authority-name backing.
Authority must cover reservation before each allocation or growth, checked
capacity arithmetic, clone overlap, rollback on ClientHello construction error,
and transfer into the successor state until actual destruction.  It must not
alter TLS versions, extensions, ECH/QUIC, certificate/hostname semantics or
ordinary no-owner behavior.  No decoded-owner source was mutated after this
preflight, and complete TLS, TLS-positive and PostgreSQL 17.6 remain OPEN.

## Window 11 ClientHello symbol amendment preflight

Protected main `e3d8a46871a98a309c73b3febaa41a7e6d2ec408`, including the
ClientHello symbol amendment and the immutable-tree PostgreSQL classifier
control repair, was normally merged into the canonical branch. The newly
authorized `emit_client_hello_for_retry` boundary does not reach the first
configured ClientHello allocation that its mandatory proof must cover.

`ClientHelloInput::new` clones `extra_exts.protocols` into
`ClientHelloDetails` before `ClientHelloInput::start_handshake` calls
`emit_client_hello_for_retry`. The source clone can retain its backing while
the destination allocation is made. Consequently, plumbing an owner only
inside the newly authorized function cannot reserve before this allocation,
measure its actual vector capacity, retain source/destination overlap, or bind
the destination custody to the successor chain. Post-call inspection would
again be too late, and moving or duplicating the clone inside the authorized
function would change an unallocated symbol rather than solve the authority
boundary.

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/hs.rs ::
ClientHelloInput::new (the ClientHelloDetails protocol-vector construction)`.
The smallest follow-up must permit same-ledger owner propagation into this
constructor and reservation/custody around the configured protocol clone,
including actual capacity, checked arithmetic, source/destination overlap,
construction-error rollback, and transfer with `ClientHelloInput::hello` into
the already-authorized successor chain. It must preserve ordinary no-owner,
ALPN, ECH, QUIC, resumption, wire, and security semantics. No rustls semantic
source was modified in this checkpoint. The decoded-owner matrix, complete
TLS proof, real TLS-positive proof, and PostgreSQL 17.6 qualification remain
OPEN; #422's merge-queue harness supplies no #356 qualification credit.
## Window11 protected ClientHello symbol amendment preflight

Protected allocation
`OTV2-WP3-RUSTLS-CLIENTHELLO-OWNER-SYMBOL-20260908` was integrated through
protected `main@a2ba218f94e83b36443afcdbd6ec8b748a677efe` and explicitly applied to
this existing worker.  A normal merge-up preserved the prior lineage.  Before
mutating the newly authorized function, a construction-order preflight found
that the same accepted owner is not available at that function.

SQLx calls `ClientConnection::new` and can install the existing public deframer
owner only after that constructor returns.  In pinned rustls, `ClientConnection::new`
immediately delegates through `ClientConnection::new_with_alpn` to
`ConnectionCore::for_client`; `for_client` calls `ClientHelloInput::new` and
`start_handshake`, which reaches `emit_client_hello_for_retry` and performs the
newly authorized allocations before construction returns.  The unchanged
`client/client_conn.rs` source blob is
`77b2dc5ea149ab8c72f40fc31a425ca4b6cc720e`.  Neither the #424 post-construction
setter nor the #425 decoded-message installation symbols can pass an owner into
this earlier outbound construction boundary.

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/client_conn.rs ::
ClientConnection::{new,new_with_alpn} / ConnectionCore::for_client :: accept and
install the existing owner before ClientHelloInput::new/start_handshake`.
The smallest amendment must provide a fallible owner-aware constructor (while
preserving the ordinary constructors unchanged) or equivalent pre-construction
wiring that passes the same #424/#425 owner identity into the common/client
context before `emit_client_hello_for_retry`.  Without it, reservations inside
that function cannot debit the accepted B ledger; a global/thread-local owner,
post-construction charge, magic aggregate reservation, or semantic deferral of
the initial ClientHello would violate the accepted ownership or behavior
requirements.  No rustls semantic source was changed.  The ClientHello RED/GREEN,
decoded-owner matrix, complete TLS, TLS-positive case and PostgreSQL 17.6
qualification remain OPEN.

## Window12 protected ClientHello constructor-owner application preflight

Protected allocation `OTV2-WP3-RUSTLS-CLIENTHELLO-PRECONSTRUCTION-OWNER-20260908`
was integrated at `main@7508a72705ab6cba95a33dd59571eca3e94b91cb`,
explicitly applied to this worker, and normally merged into this lineage. Before
adding the owner-aware rustls constructor, fresh call-chain inspection found
that the production SQLx TLS path has no accepted owner value to pass to it.

`tls_rustls::handshake` receives only `TlsConfig`; that structure contains TLS
policy and certificate inputs but no `ResourceBudget`. Its sole PostgreSQL
constructor in `sqlx-postgres/src/connection/tls.rs::maybe_upgrade` likewise has
no operation-owner argument or field. The only current `ResourceBudget` values
are focused-test ledgers and the certificate-loader helper inputs. Constructing
a fresh ledger, using a global/thread-local registry, or invoking the new rustls
path without an owner would violate the same-ledger and no-fallback invariants.

`SHARED_LEASE_REQUIRED = vendor/sqlx-core-0.9.0/src/net/tls/mod.rs :: TlsConfig /
vendor/sqlx-postgres-0.9.0/src/connection/tls.rs :: maybe_upgrade :: carry the
already-accepted operation ResourceBudget identity into tls_rustls::handshake`.
This requires an exact upstream operation-owner source and lifecycle decision;
#429 authorizes only switching the existing `tls_rustls.rs` constructor call
once that identity is available and grants no new SQLx path or owner model. No
rustls or SQLx semantic source was mutated. The focused ALPN RED/GREEN, decoded
owner matrix, complete TLS, TLS-positive case, and PostgreSQL 17.6 qualification
remain OPEN.

## Window14 caller-supplied PostgreSQL TLS owner propagation

Protected #430 was applied to the canonical worker at `093f9c001828050898888f779e1a8a6c2ade9e66`.
The preceding checkpoint is the RED evidence: no owner-aware PostgreSQL establish,
stream, TLS selection, SQLx-core dispatch, or rustls client-construction call existed.

The GREEN propagation surface is separate throughout. A caller supplies one
`Arc<dyn ResourceBudget>` to `PgConnection::establish_with_resource_budget`; the
same Arc is cloned only for capability retention through `PgStream`, PostgreSQL's
unchanged SSLRequest selection, SQLx-core's fail-closed rustls-only dispatch, and
the rustls deframer-owner adapter. The ordinary `TlsConfig`, `ConnectOptions`,
`PgConnectOptions`, `PgPool`, PostgreSQL establish/stream/TLS functions, and rustls
constructors remain owner-free. An owner-aware TLS error is returned directly and
never retries through the ordinary handshake.

The owner Arc retained by `PgStream` survives destruction of the caller's clone
and is released with the connection. No reservation is invented merely for this
identity field. `Prefer` retains its upstream plaintext outcome only for an
unavailable TLS build or a legitimate server `N`; after server `S`, owner-aware
handshake denial/error is terminal.

`OPERATION_OWNER_PROPAGATION = PROVEN` for the exact explicit entry-to-rustls path.
The next protected #429 ALPN allocation itself is not yet GREEN: the current
owner-aware rustls constructor installs #424 deframer custody, but ALPN clone/
ClientHello custody remains the next phase. The implementation stops before the
explicitly unallocated session-cache, key-share, ECH/configuration, and crypto
symbols. Complete TLS, TLS-positive evidence, and PostgreSQL 17.6 qualification
remain NOT_PROVEN.

## Window15 exact-head propagation review repair

All four P1 findings against `a03d6a2bf28c59d3292ffe1df739b7aa7c35d690` are accepted and fixed. `3956941303` is closed by the protected #429 separate rustls preconstruction path, which supplies the same owner before ALPN and the initial ClientHello rather than installing it after `ClientConnection::new`. `3956941320` is closed by using the already-owned Tokio certificate loader for inline/file trust roots, client chains and private keys while preserving the ordinary verification/client-auth decision tree. `3956941331` removes the unused public `PgConnection` accessor. `3956941337` adds focused driver tests through PostgreSQL establish, retained stream identity, SSLRequest `S`/`N`, terminal owned TLS denial and the ordinary owner-free path.

`OPERATION_OWNER_PROPAGATION = PROVEN` and the protected #429 ALPN seam is `PROVEN` after these focused tests. Complete TLS is still `NOT_PROVEN` and work stops at the next expressly unallocated boundary:

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/client/hs.rs :: ClientSessionValue::retrieve :: retained session-cache custody executes before the protected ClientHello emit/decode continuation`.
## Client PEM clone review repair (P1 3957222192)

The owner-aware client-auth path previously kept both charged loader results
alive while `get().clone()` created full, unreserved PEM vectors. The repair
removes those duplicate allocations: certificate and key parsing now borrow the
charged `Vec<u8>` backings directly. The loader reservations therefore remain
the sole PEM-backing custody throughout parsing and are released only after the
original charged vectors are destroyed. The ordinary owner-free path uses the
same slice parser and preserves its configured client-certificate/key behavior.

Focused controls cover a funded matching certificate/key pair without a second
PEM backing or ledger debit, insufficient balance before a second charged input
can be constructed, parse-error retention and unwind, and exact final release.
This closes only P1 `3957222192`; it does not allocate or enter the still-open
rustls key-exchange boundary.
## Window18 post-KX configuration-owner boundary

After the protected #451 KX/provider-resident checkpoint, the resource-owned
handshake acquires the accepted AWS-LC process/thread residency and then calls
`rustls::crypto::aws_lc_rs::default_provider()`.  On the exact pinned
x86_64-unknown-linux-gnu, non-FIPS graph, that function allocates two
configuration-owned vectors with `DEFAULT_CIPHER_SUITES.to_vec()` and
`DEFAULT_KX_GROUPS.to_vec()` before returning the provider to SQLx.  Neither
allocation is reserved on the caller's `ResourceBudget`; the #451
provider-resident debit covers AWS-LC process/thread backing, not these Rust
`Vec` capacities.

The allocation-owning file is explicitly read-only under #451.  SQLx cannot
reserve actual capacities before these private `to_vec()` calls, attach custody
to the returned provider, or prove denial before allocation from its existing
call site.  Charging after `default_provider()` returns would be prohibited
post-allocation catch-up, and copying the current slice lengths/layout into
SQLx would duplicate private configuration semantics rather than account actual
allocator capacity.

`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs :: default_provider / default_kx_groups :: reserve and retain the same-ledger actual cipher-suite and KX-group Vec capacities before allocation through final provider/config destruction`

No decoded, ClientHello, PostgreSQL, workflow, registry, or production source
was changed at this boundary.  Later coordinator evidence `5597261954`
invalidated the aggregate #451 KX `PROVEN` label: its extra wrapper allocation
was uncharged and its reservation ended before returned-secret consumption.
Those two defects are repaired by inline handshake custody and a non-allocating
reservation token retained across synchronous `into_handshake`; the broader
#451 matrix still requires requalification, so KX remains `NOT_PROVEN`.
Complete TLS configuration/composition, a funded TLS-positive handshake, and
PostgreSQL 17.6 qualification also remain not proven.

The earliest remaining unleased allocation is unchanged:
`SHARED_LEASE_REQUIRED = vendor/rustls-0.23.43/src/crypto/aws_lc_rs/mod.rs :: default_provider / default_kx_groups :: same-ledger actual Vec capacity preallocation/custody`.

## KX publication and protected-layout verification

Canonical Linux workspace run `34323039456`, job `102373930802`, disproved the
earlier claim that the shared classical source has one layout: the canonical
graph compiles that source for ring with `KeyExchange = 208` and for AWS-LC
with `KeyExchange = 200`.  ML-KEM `Active = 40` and `ActiveHybrid = 96` remain
unchanged.  The compile-time drift assertion now pins both exact classical
provider layouts, and the owner-aware entry point separately rejects any shape
other than the reviewed 200-byte AWS-LC allocation before provider start.  The
reservation remains in the caller's non-boxed `ResourceOwnedKx` control value,
whose 40-byte layout is checked by the focused bound/lifetime test.  Thus the
repair adds neither a second active-KX heap object nor reservation fields to any
of the three covered provider allocations.  The protected
`554/1625/1705/6264/7881` bounds continue to apply only to the exact AWS-LC
provider layouts; no ring KX is admitted through the owner-aware path.

The focused exact-bound test continues to prove max-minus-one denial before
provider start, exact admission for all five groups, full debit retention while
the returned whole/component secret remains live, release after secret drop,
failure cleanup, and simultaneous initial/replacement custody.  Aggregate KX
remains `NOT_PROVEN` pending the previously recorded actual HRR wire,
cancellation, provider-first-use/thread-churn matrix.  Work still stops at the
unleased `crypto/aws_lc_rs/mod.rs::{default_provider,default_kx_groups}` owner
boundary above.

## Window21 AWS-LC provider configuration ownership

Protected #453 closes the allocation-owning provider boundary. The owner-aware rustls constructor establishes #451 residency before configuration work, derives the exact Rust 1.94 `DEFAULT_CIPHER_SUITES` Vec, `DEFAULT_KX_GROUPS` Vec, and `ArcInner<CryptoProvider>` requested layouts with checked arithmetic, and debits the same shared root before either Vec or Arc allocation. It retains one canonical process provider and returns allocation-free Arc clones after per-thread residency. SQLx no longer constructs an uncharged provider Arc per connection.

The #451 process-registration mutex now retains only a bool because successful shared-root process debits have no release API; it no longer extends a caller wrapper Arc to process lifetime. SQLx precharges the remaining per-connection owner-wrapper Arc allocation and keeps its reservation after connection state so backing dies before release. Ordinary provider/TLS behavior remains unchanged. Aggregate WP3 and aggregate KX remain open pending the full final-shape matrices and real TLS/PostgreSQL qualification.

## Window22 provider proof repair and PQ feature closure

Exact-head review found two proof defects.  First, the owner-aware provider's
source/order validation collected both KX-name iterators into temporary vectors
after the protected shared debit.  It now validates order allocation-free with
the already-required length/capacity checks and iterator `zip().all()`, without
changing the #453 formula or ordinary provider behavior.  Second, rustls's
in-crate four-thread test had not executed because the published package lacks
repository-only fixtures.  The executable SQLx AWS-LC test now performs four
racing first calls in a fresh isolated test process before any provider is
cached, proves all returned Arcs pointer-identical, and observes exactly one
process debit, one provider-configuration debit, and one 1,360-byte current-
thread debit for each racer.  The main test thread's later 1,360-byte debit is
recorded separately before KX qualification.

The exact patched/locked SQLx feature tree resolves rustls 0.23.43, Tokio
1.53.1, aws-lc-rs 1.18.0, and aws-lc-sys 0.44.0, but enables only rustls
`aws-lc-rs`, `aws_lc_rs`, `std`, and `tls12`.  It does not enable
`prefer-post-quantum`: `sqlx-core` declares rustls with defaults disabled, and
`_tls-rustls-aws-lc-rs` does not add that feature.  Under this exact profile the
ordinary `DEFAULT_KX_GROUPS` order is hybrid-last, so a source-equivalence check
cannot prove the protected `X25519MLKEM768 -> X25519 -> P-256 -> P-384` order.
No Cargo feature or provider ordering was changed under #453.

`provider_configuration_owner = NOT_PROVEN`

`SHARED_LEASE_REQUIRED = vendor/sqlx-core-0.9.0/Cargo.toml :: _tls-rustls-aws-lc-rs / rustls prefer-post-quantum feature :: protected #451/#453 require the qualified ordinary AWS-LC default provider to be PQ-first, while the exact SQLx AWS profile disables rustls defaults and currently does not enable prefer-post-quantum`

## Window23 protected PQ-first feature closure

Protected #458 adds exactly the `rustls/prefer-post-quantum` dependency-feature
edge to SQLx-core's `_tls-rustls-aws-lc-rs` profile.  The focused exact-profile
test now asserts the explicit ordinary and owner-aware four-group sequence
`X25519MLKEM768 -> X25519 -> secp256r1 -> secp384r1`; it does not qualify
additional `ALL_KX_GROUPS` entries or rely on equality between the two providers.
No dependency version, source, default-feature, root manifest, lockfile, or
rustls source change is part of this feature closure.

`pq_first_profile = PROVEN_FOCUSED`

`provider_configuration_owner = PROVEN_FOCUSED`

Aggregate KX/provider residency and complete WP3 remain open pending the full
actual-HRR, thread-churn, cancellation/drop, TLS custody, TLS-positive, and
PostgreSQL 17.6 matrices on the final graph.

## Window24 final-graph HRR and thread churn

The executable AWS-LC SQLx harness now processes an actual TLS 1.3
ClientHello/server HelloRetryRequest exchange selecting P-256. Reservation
events show the 7,881-byte initial hybrid debit, then the 1,625-byte replacement
debit, then release of the initial debit. This supersedes synthetic-only overlap
evidence. A fresh-process exhaustion control also proves exact 1,360-byte
first-use thread debits, allocation-free same-thread repeat, permanent shared
custody, and denial of the next churned thread when only two thread debits are
funded. Aggregate KX/provider residency and complete TLS remain open.
