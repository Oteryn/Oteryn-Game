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
- `src/net/tls/mod.rs`: adds the dedicated resource-custody test module only.
- New `src/net/resource_budget.rs`: reservation/transfer/backing-drop custody, no TLS bound or new budget constructor.
- New `src/net/tls/resource_budget_tests.rs`: independent denial, overflow, lifetime, cancellation, concurrency and bounded-debug tests.
- New `OTERYN_PROVENANCE.md`: this import, delta and proof-obligation record.

No existing TLS implementation or protocol/driver behavior changed in this checkpoint. `ResourceBudget` must be supplied by B's existing owner ledger; the crate creates no independent limit. Neither the primitive nor its tests establish TLS/SQLx preallocation coverage. There is no budgeted TLS constructor or PostgreSQL activation yet.

## First TLS proof gate — OPEN

The admitted first checkpoint gate is complete TLS capacity/lifetime pre-call accounting. It is not closed by this custody primitive. Do not begin substantive PostgreSQL decoder work or claim B/Server Seam readiness from this checkpoint.

Pinned Game runtime graph: rustls0.23.43, ring0.17.14, Rust1.94 x86_64. The untouched upstream crate Cargo.lock resolves rustls0.23.40 for its separate crate tests; those tests qualify the std-only reservation primitive, not Game's pinned TLS path. The root locked build/Clippy use0.23.43. Both upstream Cargo manifests/locks remain intact; no test/dependency suppression occurred.

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

No excluded dependency mutation has been shown necessary yet. If the complete proof needs another path, report that exact boundary for protected amendment rather than implement an unproved allowance.

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
