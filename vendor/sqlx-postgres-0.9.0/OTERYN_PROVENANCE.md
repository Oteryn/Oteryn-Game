# Oteryn sqlx-postgres0.9.0 provenance

## Exact upstream import

- Source: crates.io `sqlx-postgres` version `0.9.0`, original `.crate` archive.
- Archive SHA-256: `87a2bdd6e83f6b3ea525ca9fee568030508b58355a43d0b2c1674d5f79dcd65e` (verified before extraction).
- Upstream VCS: `003b698e99e024f3621b8043a2426fde5b741171`; subdirectory `sqlx-postgres`.
- Licenses: upstream `LICENSE-APACHE` and `LICENSE-MIT`, preserved verbatim.
- Game authority: issue351, Work admission comment5560220858; immutable main `53c6bdf06a2282d893035a995c46052c88f935b4`.
- Import method: Python tarfile safe data extraction of the verified archive; all original regular files preserved. No cache-generated `.cargo-ok` or replacement manifest imported.

## Complete patch manifest

All upstream files remain byte-identical. The only addition is this provenance record. The root path patch preserves the existing SQLx type universe; there is no PostgreSQL decoder, options or TLS-plumbing change in this checkpoint.

## Qualification boundary

Refer to the core provenance record for the open TLS proof gate. No driver resource or actual PostgreSQL qualification follows from an unchanged import. Root Cargo.lock changes only remove source/checksum for the two path-patched packages; no dependency versions/features were changed.

## Original file digest manifest

These SHA-256 values identify the original archive bytes, including the files explicitly changed above. Every other original file must continue matching its digest; new files are identified in the patch manifest. Final candidate identity belongs in PR/check evidence, not a self-referential digest here.

| Original path | SHA-256 |
|---|---|
| `.cargo_vcs_info.json` | `3f5f23361f406ff0359c17a3a67210ac0345f7a387c9cc8348e4e8b47f894fd4` |
| `Cargo.lock` | `56cd1cd5af9d722f305696b56a5dc2c58c3875902e5a19cba78aeaefe49b6ee7` |
| `Cargo.toml` | `6dc974c55ac97ddf1005f549fbdc4d46c641974191499357d2001ee9af17c125` |
| `Cargo.toml.orig` | `c509150be3f6718d208bc4bdec97fc815943bb1e9bb4fb9ff56eda4de18d8f1c` |
| `LICENSE-APACHE` | `154397a08d0cb342110281129971868cd562542d0780b0d10722788cd11a1399` |
| `LICENSE-MIT` | `72af89a828df52a9dcc42512ed64663b3b8d7961dd7a5586d44478eede82bc7a` |
| `src/advisory_lock.rs` | `979f34dc7ca90b5674406ad3f93a39313fb0f9e2515b435c54a60ae2e4ca53a0` |
| `src/any.rs` | `99cf43503a74181bce0dc18dee554de113e5c48a49ccad81b3c90d9f7ef0e345` |
| `src/arguments.rs` | `9cf557fe3d45139768e1149f3610d393875de91e646daed5384be511e086f556` |
| `src/bind_iter.rs` | `2f1856eb989ea621ce0d15c8af6d99518cbac9cd19e32f94266ee4e3f8f3ea5e` |
| `src/column.rs` | `029bd155ffc40b4d38f0aaf58ed3792d2d7022f358eaac66f59fef0c320c8774` |
| `src/connection/describe.rs` | `5f6be121170f1d0c4f3deff8db992ac84f56b1679b1b0430125b4b25d14d0541` |
| `src/connection/establish.rs` | `bd68ddaf75db5239b4701f03ff90efddf4259835ccb10c58e12454ecaadeb3a1` |
| `src/connection/executor.rs` | `721ead17743d912c625a56b5728669d8111c750797b6fee24e234226a5b6fbe6` |
| `src/connection/mod.rs` | `efeafd9bc07c0bd20e0d2c2312bd2db45039c9d139aa754b1ff6574dd280de72` |
| `src/connection/resolve.rs` | `8db3f9ac44236d3c86b336b6046e0fe91cd01e44b7f583ef7ab4b784ba9e50cf` |
| `src/connection/sasl.rs` | `3ce9b4eb68ea03eb1f9b9aa2673d777183203daabab01ebfd72479136cf65899` |
| `src/connection/stream.rs` | `185f39afdd96284a7dd1f40fff1a9292fd289c85c2106c65a121045b551c00c5` |
| `src/connection/tls.rs` | `e93bd2605e010a71b9a0200b27dfdd101a8427385dc39468e522fb6547958e07` |
| `src/copy.rs` | `06a2bd576ae6659023380d973c4bb7c60b659436a7441f4725218a6beb76c689` |
| `src/database.rs` | `8abf34d10d91d9025609699db5ec1061785a26ea72ccdadaa777e0af0bbbb6b3` |
| `src/error.rs` | `1a4c4f60a9bddd3965f781b92ac66d9a95e520a2e88ddbf19972a644bf17cbf0` |
| `src/io/buf_mut.rs` | `bfeb8991e7bbbf6134ff83a67f6c65820c693b0d7e8b2ef8a9aa4444a696cecd` |
| `src/io/mod.rs` | `5ffb01f5b17c83c11d10de097142c514b17e1bb8544d767d238539bcaeda9ab9` |
| `src/lib.rs` | `cb50e3b0a6b038c81719c4972acc6d28eac003156a4e7e255316d84c76380514` |
| `src/listener.rs` | `bd8e909f848db7271b7024983c18547cfe18223bea52c8e6ecacaa0b5f410233` |
| `src/message/authentication.rs` | `d72a8a381d686fcb719f5582db48bd4e5bcb64931baa30f736179fdc2fd3341d` |
| `src/message/backend_key_data.rs` | `add437e94e69b7152695094517885c3f4dde48bb53aeb10201c2e67446034d1a` |
| `src/message/bind.rs` | `f48f6c588d710bcf0807f5c7f630ab85ad733026ea43c143a14f316aa091c3ed` |
| `src/message/close.rs` | `1cedb64d6208ee051d1cb3e5734116489a3bde4ca62ad159ad04533b460b507d` |
| `src/message/command_complete.rs` | `c2950bddc035e86dcc5d97c3d0d6aaea821f957d5ecccdb5fcd99e44d2611930` |
| `src/message/copy.rs` | `d42d3097114a1085c9e643b2378705a8661c40075d5d11e302d9801ea63bf290` |
| `src/message/data_row.rs` | `50a13a82c07ff5cd3734cb5bbbc69eb51e85a4960a105ae357f16308a388be47` |
| `src/message/describe.rs` | `c9b13e5fb99e48bcec2af8215f07b84d8de610a500175ccbb504beeed4ac7f34` |
| `src/message/execute.rs` | `779ea6f980d56cde11b50ff37a72a96eab3837c1c1a2fe28caa73a0fac7180c2` |
| `src/message/flush.rs` | `b20b5850e008c9a14f2dd942c5da0dd08b8d4ebde74bde96ddcc2d4a9ecbccb5` |
| `src/message/mod.rs` | `8be84536b93e50775654c8e7207ca144e1b78d2c12953bb43eec72690da3f0d9` |
| `src/message/notification.rs` | `b15c7260232e29ebf3dc2712af0ed12426fb1777f039110364aaf609cb062d6e` |
| `src/message/parameter_description.rs` | `cefef4381ce4e9fde9961d849d30c1c36b48198b1b26744c6d0e175b8bdd933d` |
| `src/message/parameter_status.rs` | `9d95d05077699a856e63fb3f88ecfca14de6805fe9595132744fa24d20da14b5` |
| `src/message/parse.rs` | `9ec502f111e5bbcb2f8173c78b121a1caa0c19246181946612fd8ad2d071cf78` |
| `src/message/parse_complete.rs` | `02b06b13961142e8052c4543357dac18c217587891e66d38489a470448514bf3` |
| `src/message/password.rs` | `68bf988d1300d8498faad72fcb53fbbcac843fc8db304131f073511bf2c25f85` |
| `src/message/query.rs` | `4d256ea1648ee8a42ee126007982986ab6b55afd98e6412999272600f8387381` |
| `src/message/ready_for_query.rs` | `fe79869e33d0da9f218350817915a0f88cd536ae7ccd3604e376f96e5918fe9b` |
| `src/message/response.rs` | `364300af0da9e85d435daab4dd0fa887355cf859b6aed7f89a2e453101126072` |
| `src/message/row_description.rs` | `af5e046634af3fd61b7f2771b206c60976b6985fb552bf34f43baa5966f82d6d` |
| `src/message/sasl.rs` | `3bdda8fd09c45623c6544166959d3e2b3f20d80548f3cfe5330a3d4f868d5804` |
| `src/message/ssl_request.rs` | `53b87eb7bb3156e1fbe1f27a1aa757ce1583183d9b8b8abaa73edfa6d98f766d` |
| `src/message/startup.rs` | `9e1781291a648be9058b0b9f0014355a6510725ef88e04f19a545a5600518b00` |
| `src/message/sync.rs` | `5a2c8fe69cb07a856092dbae6703a0c946812cf10db90115c6a02253caf99647` |
| `src/message/terminate.rs` | `bf70e28b7f2bc61d6686c48ef221f3c1a3a9f222ebb7453cd3aeb0f6ca7fd716` |
| `src/migrate.rs` | `de1fb1f13fa2d4afb887a9037cb2d517ae0f8e08ba5c0cc76c71a6ff18a44866` |
| `src/options/connect.rs` | `48889caf4c11fc11952b7c0dd038e07108b4011344c697d51f232f56d588c4b9` |
| `src/options/doc.md` | `33882b389d8c5733da137d0fdda411188c22e922f79adac9dfdde8b959506f4c` |
| `src/options/mod.rs` | `1886a4cf46dfdf57d33c78403f9588c3e2f429641dcbd5837f4c8b29273fbc36` |
| `src/options/parse.rs` | `b4ce65626f5cac13e048a6dbae6a063857c8245a655178352a5a61bc659f7666` |
| `src/options/pgpass.rs` | `96f1bcf5c3573a5af90b27241a887ad5fc53547895a9803091532a7be9574080` |
| `src/options/ssl_mode.rs` | `24f9be300645a3e23c61530671c1eb7a491086cda7e4e9a47ec4968a5297136c` |
| `src/query_result.rs` | `8858d975a4e2fc0e80c5880be9c3add174ef056ade336a6287ee648d8d0f9b62` |
| `src/row.rs` | `a65585b8c6e585f9429e3af7867bdeca3d1a2363760b452df785eb12a60edadf` |
| `src/statement.rs` | `d9f6a36611672f5cbd755a1fa389c4e63fa3ae17984362c5b5f7e0c94bbd754c` |
| `src/testing/mod.rs` | `fb94867585c3a7a9779f7c3a57460d0c4db7f733a83a7434bbcf93dfa09db4b4` |
| `src/transaction.rs` | `3f326f8d6bf840a112cb98e465390d9add493b88f87e8a56006d91396bc0ce00` |
| `src/type_checking.rs` | `074d3bdade4108f7051acb285ca4dbb42a1647ff641b55afd44e3e6c3a0a44b3` |
| `src/type_info.rs` | `c5a1576dac0cbb9be48d186052a631faa78200e2e69c4a9b78596ccf878afebe` |
| `src/types/array.rs` | `e4c34e06ba827fb3924257df58be63ec735ea8440287d52a7ed056ccbf99f821` |
| `src/types/bigdecimal-range.md` | `968dac5676a333f1953df12b9ed1beb930537e6fae190140ac729e8dc16b2749` |
| `src/types/bigdecimal.rs` | `8b818a2fb6fb2917fec92401eed0556f6f19b188e1bb6bef56928ce4d36bb5a9` |
| `src/types/bit_vec.rs` | `b3f120d07c9ba2e16f60c1ca498ef6b347cd522de32d1dce01a723b50cab5d04` |
| `src/types/bool.rs` | `4ade8342b8f63bd7b33bf6e878591796420491079c072e09e683c0279e7a102d` |
| `src/types/bytes.rs` | `d4850ba4e8d9bf662074471879036763da6d57a2d86db2b56801b82a1b780719` |
| `src/types/chrono/date.rs` | `96d053917becadd885f2417521d690f140b1ad49fb12acda9502ffc14bc04060` |
| `src/types/chrono/datetime.rs` | `cad2020b1ae7dd3b7b734b2a762f847210c87eda0af6222e8b87eb2409e11bbd` |
| `src/types/chrono/mod.rs` | `32f64ebfce9248c267bc6734716c39157fa00d5504a9578b85790c10bb672bf6` |
| `src/types/chrono/time.rs` | `5e0b99da298402a0bdd9116575e285991f9ceef9d62a947f6ff1cc31370e6035` |
| `src/types/citext.rs` | `3995b5807aeda68262de20009233f2ee82b2579a510f26316134730e71dd33fb` |
| `src/types/cube.rs` | `d9bce5160101bc29d8239b5308567d02beb2c509dd1a8bf057cd7e400b36c8a5` |
| `src/types/float.rs` | `f450fc0113ff18dbd1868775153a4cd989dfdd6650c12caa555518288cbbafb5` |
| `src/types/geometry/box.rs` | `c45c766ebd954904876df09be65854df1fa95ecfc0d4720cca41ac6194c7d98e` |
| `src/types/geometry/circle.rs` | `e255595815272d3c7cd9be04f800fd629f1b113092a71f5a930d7fa36ac80f88` |
| `src/types/geometry/line.rs` | `b160a29fbd80ddebbf60bb877b08391651a42c478729035af955d47aa656e52b` |
| `src/types/geometry/line_segment.rs` | `4fece26104f95418a760c6b0e2cdea2e7bf41a5af05cde4e03758a1a4232818d` |
| `src/types/geometry/mod.rs` | `e620ccb5e2ee7b385090ed19244f628aaf77f0b96d1b12129c2789b9afe9ac9e` |
| `src/types/geometry/path.rs` | `e35a9b4ab12d26b2317a09c18db16c8b0fc59f94c04c8048ba94dfa79ee24859` |
| `src/types/geometry/point.rs` | `82df096fd5c62780858ca1efd7cb9e5a53f958c1748191923fafec8f4a5f5b10` |
| `src/types/geometry/polygon.rs` | `37add7a2c134cd435ae940999218cb149569067bcc8ac60b52bc91e9439bd010` |
| `src/types/hstore.rs` | `6ae215a7c70b577038816031c4874970b4727d4ed11665794a403a1a3b3f9b73` |
| `src/types/int.rs` | `747a52950d1f07a5e124ff375a8cb18f4d82d90172fe775e6b4e239af89bcdff` |
| `src/types/interval.rs` | `499670796f10b13a41703e6a2fae287da5ce4fe3c310aa9885a436d05b1c4215` |
| `src/types/ipnet/ipaddr.rs` | `714ab6e26c2ed3557588c91cd8e73cdcb349646581e3456f3cc5ff1c26dc7182` |
| `src/types/ipnet/ipnet.rs` | `5d222c3ed39ac8763e8ecb9ccfe03069f6b0e41b48920ad15b998dee09ef9e3c` |
| `src/types/ipnet/mod.rs` | `0ed539539b878b2cf6559e9c8a76324a129199daa75a2ae2ee39d251d2fecaf0` |
| `src/types/ipnetwork/ipaddr.rs` | `1e7b4fff8a36c5be98c9187c7e72ee7c9cbe47afaf5396a416d8888e471995cc` |
| `src/types/ipnetwork/ipnetwork.rs` | `e0279872b2c28518c4e0a0fa6a5f2546c0c51bb81630305ee5cc342b9d51197a` |
| `src/types/ipnetwork/mod.rs` | `a67917a49378a579709b91330b8a8c4b79c65d8f817cda9230f34f2e577bdd4a` |
| `src/types/json.rs` | `a7343984bf6b2d1aa031ad60890a9bf1e39e4667df94381b53dbf02951ce1dae` |
| `src/types/lquery.rs` | `4636b7e54ded97181ad19063d795cd5c3064a2dd043dee9eeefca659fb00082f` |
| `src/types/ltree.rs` | `7ff83c99cd2130d221ed7ddc527f52085ac8f9e6aee41e079d48c5dc51ce3295` |
| `src/types/mac_address.rs` | `fd330557132ab6df24a9a4e2407d2885347eb26ef2f79d24f612f49ca0d5ab82` |
| `src/types/mod.rs` | `ec2b7ff40e74aab418d459c0483fc28ae8f5becabf0e09d0d3b8c301e5c8160e` |
| `src/types/money.rs` | `9c94722979e67671c62e881773a4aa478ce5a69ae983dc3c6ed0213c6f10e64b` |
| `src/types/numeric.rs` | `08522610ab1862fa47746ef1105a5b86959d19198f68b4e337792189afd8628e` |
| `src/types/oid.rs` | `a268d6519ce4656af68a012f54649dffecbd5166fc93cb68cf5f1ec19bf3023a` |
| `src/types/range.rs` | `442dcc1d36b427e48aef24b046b5a88c1ec51008427f5d06354228d945bb4773` |
| `src/types/record.rs` | `6f63808e1974ac6a06026571bf6cdabbbd161c93c617d14dc4ca3615241df1f8` |
| `src/types/rust_decimal-range.md` | `50ea63448f02e7f4acb78d9f78f06d435d50460adb9a79223c8626f1e7598e8a` |
| `src/types/rust_decimal.rs` | `4e5f3b08f1d40add3c8797f41ba8f28af114824b5a02cb0d35d8742c99fb442e` |
| `src/types/str.rs` | `4ea69e4640bf53fec800742a05930121e70afbaa4ffce19c0c9d182a283aeca8` |
| `src/types/text.rs` | `2d2eed73fc9350a38b892dd553189346685835a6d4ca18139cce6c8f40640e0f` |
| `src/types/time/date.rs` | `ce1196c33e187df1d05e1145bd684f44837c701b81a751ee648bfda0494e1fab` |
| `src/types/time/datetime.rs` | `7f3bb4a37a992cb88a9e28aee98c4fee529350527da1eebd2504b125421992b8` |
| `src/types/time/mod.rs` | `2a9b0bd99f3bc6056cba009ed22e2dda958472f3396be1046fd58b3eaca4c77f` |
| `src/types/time/time.rs` | `4fdf167559e4a8cc0aa6de0918d6e7120ae06776799fe2dfd3a475139fb1bd3a` |
| `src/types/time_tz.rs` | `f92531ab0ccb3700694570c4a95a95a9cfb9f7e110df9da2b2b3f5f1079c64d1` |
| `src/types/tuple.rs` | `f34df3f1e8440f485c3970dd7270411ade105a48b750cbcdd8b96456583bb9b0` |
| `src/types/uuid.rs` | `47649777028fa971d331006af64b7685664d4a2a09fa8f2455d3f81560c5710b` |
| `src/types/void.rs` | `79fa3e4e4bd70216e9f3dd6cdd0a84995322b21aabbb1fba005c7b4c108048e3` |
| `src/value.rs` | `80f9303a892fb97ac0a4084532bc03e0a344d881495cad1af2d2357150cde0c6` |

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

## Frozen M02/M04 PostgreSQL custody continuation — 2026-09-13

This bounded continuation consumes the preserved WIP at canonical f568963 under
Work162 inventory5655890678, grouping5656017872, activation5656038508 and owner
handoff5656281533. E1 and existing PostgreSQL authority apply. It does not claim
complete M04 escaping-error custody, selected TLS, registered production-root
qualification, or terminal WP3. Historical checkpoints above remain evidence.

### Material behavior and source bounds

- PgStream uses the same pre-established ConnectionOwner for transport and TLS,
  the externally funded final socket Box, and owned initial/growing buffers.
  It validates the five-byte header and signed protocol length range before
  reserving the frame body. Completed frames use private shared OwnedBytes;
  decoder descendants retain its single debit after stream/connection drop.
- DataRow validates count, every length (only -1 means NULL), all body ranges,
  and trailing bytes before allocating the exact range array. The executor
  checks the wire count against current metadata before this allocation.
  RowDescription preflights all names, UTF-8, fixed field tails and total body
  before reserving exact Field-array and name-string backing. ParameterDescription
  and CopyResponse validate count/body sufficiency before collection allocation.
  BackendKeyData, ReadyForQuery, Authentication, Notification and status bodies
  now reject malformed minimum/fixed/trailing forms without indexing panics.
- Status strings borrow charged frame slices. The status Vec grows with exact
  old+new capacity overlap; duplicate-name replacement destroys old frame
  descendants under their original charge. Server-version parsing uses a fixed
  three-number array and checked arithmetic, with no peer-driven allocation.
- Metadata uses an exact array/name/Arc-layout reservation shared through private
  finalizing AllocationLease handles. PgColumn clones retain this lease;
  Metadata destroys its data Arc before its outer lease. Arrays include
  parameters, columns, and (name,index) entries; each shared name uses the padded
  Arc<str> layout. Reverse lookup preserves last-duplicate-column-name behavior.
  ResolvedTypes carries its OID-vector charge through use and destruction.
- The owned statement cache reserves its complete capacity times CacheEntry size
  before the entry Vec, charges each copied SQL key before creation, preserves
  LRU/replacement behavior, and releases keys after destruction. Entry count and
  every variable backing are constrained by the supplied ledger, not a new
  numeric driver allowance. The selected root retains configured capacity100.
  The selected root rejects non-built-in OIDs before generic type/table discovery;
  custom discovery remains ordinary/unqualified profile behavior. Any/offline
  compile compatibility is retained; conversion of owned rows/statements into
  Any's naked name maps is rejected because the selected root is typed Postgres.
- PgConnectionInner Box is reserved before Box::new, with its reservation outside
  the Box. Rows, values, metadata and columns may outlive this connection safely.
- Startup's outer parameter list is a six-entry inline array. Its complete wire
  length is checked from all supplied key/value strings and terminators before
  encoding. Every FrontendMessage now exposes a source upper bound. Bind includes
  two bytes per parameter/result format and the actual result-format count;
  Execute includes four limit bytes; CopyFail includes its trailing NUL. The
  complete prefix/body reservation precedes the Vec encoder. Neither a guessed
  size hint nor a formatted replacement allocation error is a funding boundary.
- If Sync or Drop rollback cannot reserve output, the stream is poisoned and
  later send/receive/flush refuses it. The socket and its R/T charge remain held
  until connection destruction; no detached cleanup or early permit release is
  introduced. A failed cache insertion also poisons the connection, preventing
  reuse that would accumulate untracked prepared statements. The existing
  `src/transaction.rs::start_rollback` change is this same admitted cleanup seam.
- Owned peer notices are decoded under charged backing but never emitted to the
  notice logging sink. The root LogSettings disables query/slow-query logging.
  Owned unexpected-auth diagnostics do not format peer authentication contents.
  PgDatabaseError continues to preserve original SQLSTATE and borrowed fields.

### SCRAM temporary and decoded ownership

The ordinary authentication implementation remains the ordinary route. The owned
route retains SCRAM-SHA-256/non-channel-binding semantics and uses exact-capacity
AuthText allocations, each reserved before String::with_capacity and destroyed
before its external reservation. Nonce/proof/base64 buffers and HMAC/SHA state
are inline. An inline StdRng seeded from SysRng avoids rand0.10.2's thread-local
Rc; nonce length and printable-except-comma alphabet are preserved. The server
nonce must extend the sent nonce. Hi retains its existing periodic runtime yield;
this batch does not replace the authoritative root connect deadline.

Decoded server-first/final messages precharge copied message/nonce/salt/verifier
backing and private reservation Arc layout before construction. Missing,
duplicate, malformed or nonpositive iteration attributes are rejected; errors
and resource denial do not format a replacement peer string. Salt/verifier
base64 capacity follows base64::decoded_len_estimate, including spare capacity.

stringprep0.1.5 borrows printable ASCII without allocation. Its non-ASCII path
uses unicode-normalization0.1.25 NFKC, then validates the prepared output.
Pinned source-table census: 2081 canonical entries, max4 scalars; 3849 compatible
entries, max18 scalars at U+FDFA; Hangul decomposition has at most3. Let
`N=max(8,18*input_scalar_count)` and `P=size_of::<(u8,char)>()`. A checked finite
reservation `N*(3*(P+size_of::<char>()+4)+P)` covers old+new geometric backing
for decomposition pairs, recomposition chars and UTF-8 String, plus at most N
stable-sort scratch pairs. The reservation is obtained before saslprep and held
through its returned Cow<String> destruction. This is a conservative finite
source bound against the same remaining budget; no ASCII-only credential
restriction, new semantic cap, dependency source edit or hidden allocator
percentage is introduced. Tests include U+FDFA normalization and prior denial.

### Qualification evidence and remaining scope

Local Rust1.94 locked root graph: PostgreSQL lib tests161 PASS/0 FAIL/0 ignored,
including hostile body/count tests, asymmetric encoders, shared row/value
backing, metadata clone finality, cache denial/eviction/replacement, status
replacement/growth denial, SCRAM normalization/challenge denial, and Sync/Drop
rollback denial fencing. The actual TCP mock test executes startup and a simple
query, then checks row/value/column custody after connection and runtime drop.
This is a protocol-component test, not PostgreSQL17.6 qualification.

The malformed fixed-body test first failed at byteorder's BackendKeyData read
of an empty body; the repaired three-case group passed. A previous TLS-budget
fixture hung because denial now correctly precedes TCP connect. Its corrected
mock sets the denial gate immediately before sending SSL acceptance, so the
existing test still exercises TLS-phase denial rather than waiting for a
connection that should never occur. No test is ignored or weakened.

Strict Clippy passes for PostgreSQL lib/tests, core lib/tests, and game-server
all targets; unchanged rustls dependency warnings remain. Changed Rust files
are rustfmt checked. Any/offline compilation passes using an external temporary
manifest pointing to exact source, with ordinary APIs. Root Cargo/lock and both
imported package manifests/locks are unchanged. Original digest manifests remain
intact; only the explicitly listed authored paths change.

No local PostgreSQL executable/container is configured. The exact canonical
successor still requires actual PostgreSQL17.6/AWS-LC VerifyFull/TLS1.3 CI; no
absent-env skip is reported as qualification. The core provenance records the
baseline-reproduced historical HRR and parallel KX test gaps.

**Remaining precise error boundary:** core src/error.rs is excluded by protected
plan §Authority and E1. Its Error::Database outer Box allocation/deallocation and
consuming database-error/downcast APIs cannot be funded by an interior
PgDatabaseError lease. Existing Configuration/Tls escaping boxes share this
separate finality problem. No naked-box workaround, SQLSTATE loss or unallocated
core error edit is included. Work162/5656188739 retains that amendment lane.
Final selected TLS composition is M03; complete per-query byte census,
registered root/active-pass custody and completion/ambiguity capacity are E01/M05.
The nested/custom/Any/COPY/listener families are not promoted to production-root
reachability merely because their ordinary compatibility code compiles.

Exact authored paths in this continuation (relative to this package):

- `src/any.rs`
- `src/arguments.rs`
- `src/column.rs`
- `src/connection/establish.rs`
- `src/connection/executor.rs`
- `src/connection/mod.rs`
- `src/connection/resolve.rs`
- `src/connection/sasl.rs`
- `src/connection/stream.rs`
- `src/connection/tls.rs`
- `src/error.rs`
- `src/message/authentication.rs`
- `src/message/backend_key_data.rs`
- `src/message/bind.rs`
- `src/message/close.rs`
- `src/message/command_complete.rs`
- `src/message/copy.rs`
- `src/message/data_row.rs`
- `src/message/describe.rs`
- `src/message/execute.rs`
- `src/message/flush.rs`
- `src/message/mod.rs`
- `src/message/notification.rs`
- `src/message/parameter_description.rs`
- `src/message/parameter_status.rs`
- `src/message/parse.rs`
- `src/message/parse_complete.rs`
- `src/message/password.rs`
- `src/message/query.rs`
- `src/message/ready_for_query.rs`
- `src/message/response.rs`
- `src/message/row_description.rs`
- `src/message/sasl.rs`
- `src/message/startup.rs`
- `src/message/sync.rs`
- `src/message/terminate.rs`
- `src/options/oteryn.rs`
- `src/row.rs`
- `src/statement.rs`
- `src/transaction.rs`
- `src/value.rs`


## M02/M04 repair 1: independent audit findings (2026-09-13)

Authority: SAME frozen M02/M04/E02 batch and owner custody handoff; the work
coordinator returned both findings in canonical PR #356 comment
[5656534849](https://github.com/Oteryn/Oteryn-Game/pull/356#issuecomment-5656534849)
for one coherent repair. Parent candidate is
`a83af56e6269e288aade597e093eee086cf3ddcb`; this section supersedes any broader
value-descendant or denial-composition claim contradicted by that audit.
No new material cell or imported path is introduced.

- **WORK-AUDIT-M02M04-1:** `PgValue::as_ref` now passes the existing `OwnedBytes`
  backing to its reference. `ValueRef::to_owned` consequently slices the same
  charged backing rather than copying into ordinary Bytes. Repeated public
  round trips retain the original pointer and its single 88-byte test debit,
  even with no spare budget and with each predecessor dropped. Final descendant
  drop releases the debit; null, empty and ordinary owner-free values retain
  their expected value behavior.
- **WORK-AUDIT-M02M04-2:** a stack-only `PendingRequest` borrows the connection
  across complete `run` preparation and flush, and across `get_or_prepare`
  (including Parse/Describe, metadata resolution and cache eviction Close).
  Every early error or cancelled preparation drops the guard and poisons an
  owned stream. It neither clears queued bytes/pending ReadyForQuery count nor
  closes the socket or releases its custody. Thus an abandoned request cannot
  be flushed on reuse, while already submitted bytes retain their existing
  ambiguous-operation semantics. Success disarms the guard and retains the
  existing response handling; ordinary owner-free connections are not poisoned
  by this new guard. Sync's existing poison behavior remains in force.

Regression evidence was generated test-first, with unchanged parent production
implementation: `cargo test --locked -p sqlx-postgres --lib repair1_tests`
produced five failures and one compatibility pass. The value test observed its
88-byte debit fall to zero despite a live descendant. Actual mock TCP peers
received 15 abandoned Query bytes after metadata denial, and 8192 bytes each
for Parse/Describe, Bind/Execute and Execute/Close denial. The identical six
cases pass after repair; the peers receive no request bytes, failed reuse keeps
the full connection allocation debit, and dropping the connection returns it
to zero. These deterministic peers are protocol fixtures, not PostgreSQL
qualification servers.

Validation (runner `/tmp/a4-m01-run`, Rust/Cargo/Clippy/rustfmt 1.94.0;
`CARGO_TARGET_DIR=/tmp/a4-provider-target`, repository root working directory):

- Focused RED/GREEN logs: `/tmp/m02-m04-repair1-red.log` and
  `/tmp/m02-m04-repair1-green.log`, command above (5 failures/1 pass then 6/0).
- `/tmp/m02-m04-repair1-postgres.log`:
  `cargo test --locked -p sqlx-postgres --lib` — **167 passed, 0 failed, 0 ignored**.
- `/tmp/m02-m04-repair1-postgres-clippy.log`:
  `cargo clippy --locked -p sqlx-postgres --lib --tests -- -D warnings` — PASS.
- `/tmp/m02-m04-repair1-root-clippy.log`:
  `cargo clippy --locked --manifest-path apps/game-server/Cargo.toml --all-targets -- -D warnings`
  — PASS. Existing dependency rustls warnings remain separately recorded.
- Changed Rust files pass rustfmt with `--edition 2021 --config skip_children=true`;
  `git diff --check` passes. Tracked manifests/locks and core `src/error.rs` remain
  byte-identical to the parent. The earlier core six-test/Clippy and Any/offline
  evidence is retained without rerunning or claiming new exact-head execution.

The legal subset still needs independent successor review and configured
PostgreSQL 17.6/AWS-LC VerifyFull/TLS1.3 qualification on the eventual canonical
successor. No local PostgreSQL executable was available. The separately held
core Error custody boundary, baseline HRR/parallel KX evidence and remaining
M03/M05/root qualification are unchanged and are not claimed complete here.


## M02/M04 configured-CI denial phase fixture repair (2026-09-13)

Canonical parent `f10cab5b7d1d388fb20acf0beec4a5a62aef49fc` has the independently
reviewed repair-1 tree `e5b129530ae7ca7f8a0136f346153e72ad7f93d9`.
[Linux job 103804595570](https://github.com/Oteryn/Oteryn-Game/actions/runs/34787142075/job/103804595570)
reported 414 passing durability tests and one failure in the configured
PostgreSQL 17.6 AWS-LC qualification: its denial helper returned
`error communicating with database: out of memory`. The retained failure
excerpt is `/tmp/m02-m04-ci-f10-failure.log`. The qualification function runs
and verifies the positive helper before running denial; reaching this denial
failure therefore establishes that the configured positive helper completed
and emitted its required marker on this parent. It does not establish that the
whole qualification passed.

The old zero-budget TLS fixture now denies `ConnectionOwner::try_new`'s first
reservation in core `net/mod.rs`, before TCP/TLS; that admitted M02 path maps
its denial to `Error::Io(OutOfMemory)`. This is a fixture phase mismatch, not
proof of a TLS BudgetError classification regression. The coordinator admitted
only the existing E02 resource-budget qualification/helper phase correction.
Production code, including excluded core `src/error.rs`, is unchanged.

- A distinct `deny-transport` subprocess retains the zero-budget assertion,
  requires exactly `Io(OutOfMemory)`, verifies zero peak/final debt and no
  provider witness, and emits `OTERYN_WP3_CONNECTION_OWNER_DENIAL_UNAVAILABLE`.
- The TLS-denial subprocess uses the existing positive profile's unchanged
  SLOT/ROOT limits to reach TLS. An observed successful provider-shared debit
  arms deterministic denial of the next ordinary reservation. No byte-size
  threshold, larger numeric allowance or generic IO acceptance selects TLS.
  It requires exactly `Error::Tls` containing `BudgetError::Unavailable`, a
  recorded TLS-phase denial, positive provider-shared custody, bounded peaks,
  zero final ordinary debt and final root debt equal to the process-retained
  provider-shared debit. Timeout is no longer accepted as denial proof.
- The configured harness requires both distinct denial markers and retains
  its real PostgreSQL 17.6 VerifyFull/TLS1.3 positive verification unchanged.

Test-first local evidence: the new phase regression with the old zero-budget
constructor failed at its transport-prefix reservation. The same test passes
with phase injection and checks shared-root finality. The actual helper binary
was also run in fresh subprocesses against a deterministic SSLRequest peer:
transport denial opened no connection; TLS denial sent one valid SSLRequest,
received `S`, then denied funding before ClientHello. Its strict TLS source
check passed, with 34 denied bytes and 150096 provider-shared/root bytes retained
(the observed values are evidence, not fixture thresholds). This mock peer is
not a PostgreSQL server and does not replace configured successor qualification.

Validation uses `/tmp/a4-m01-run` (pinned 1.94 tools) and temporary manifest/lock
`/tmp/m02-m04-ci-helper/Cargo.toml` / `Cargo.lock`, with the binary source pointing
to the tracked helper and all four imported dependency patches to this worktree:

- `cargo test --locked --offline --manifest-path /tmp/m02-m04-ci-helper/Cargo.toml phase_tests`
  — RED then GREEN logs `/tmp/m02-m04-ci-phase-{red,green}.log`.
- `cargo build --locked --offline --manifest-path /tmp/m02-m04-ci-helper/Cargo.toml`
  and `python3 /tmp/m02-m04-ci-mock.py` — build/mock logs with the same prefix.
- `cargo clippy --locked --offline --manifest-path /tmp/m02-m04-ci-helper/Cargo.toml --all-targets -- -D warnings`
  — helper Clippy PASS.
- `cargo clippy --locked --manifest-path apps/game-server/Cargo.toml --test durability_postgres -- -D warnings`
  — targeted root Clippy PASS; existing dependency warnings are retained.
- Changed-file rustfmt and `git diff --check` pass. Tracked manifests/locks,
  production source and numeric qualification limits are unchanged. Initial
  unrestricted-target offline metadata preparation reported uncached Windows
  `etcetera 0.11.0`; Linux locked offline build/tests/Clippy subsequently passed.
  Temporary lock package identities remain within the canonical root seed plus
  the isolated helper package.

Independent successor review and canonical configured PostgreSQL 17.6 CI remain
required. Local PostgreSQL is unavailable. Prior accepted M02/M04 evidence and
the separately held core Error, M03/M05 and quantitative root boundaries are
retained without re-audit or expanded completion claims.

The root-profile validator permits only the exact precharged production startup
string for `transaction_timeout`, `statement_timeout`, and `lock_timeout`, each
fixed at `2000ms`.  Missing, reordered, additional, or mutated startup options
remain configuration errors.  The selected PostgreSQL qualification helper now
installs that same fixed profile before constructing its lazy holder, so its
configured positive and denial runs exercise the production validator path.
Acceptance also requires a private precharge proof installed only after the
Oteryn wrapper has reserved and built that exact retained string.  The ordinary
`PgConnectOptions::options` builder clears the proof before mutation, and an
ordinary options clone cannot copy it, so matching public-builder bytes cannot
forge sanctioned root-profile provenance.

## Selected configured E02/M03 qualification correction (2026-09-14)

The earlier configured helper parsed the ordinary administration URL and then
selected `VerifyFull` plus a file CA.  It did **not** call
`PgConnectOptions::new_oteryn_root_profile`; consequently its successful run was
an ordinary owner-aware TLS control and was not evidence for the selected M03
configuration.  The qualification now labels that ordinary control separately
and constructs the selected profile explicitly from the configured literal IP,
DNS identity `localhost`, inline CA PEM and the same caller-supplied root owner.

The selected test connection-generation enclosure is source-derived as
`4_475_170` bytes for the configured built-in-root verifier plus `27_485` bytes
of fixed TLS state (`4_502_655` total).  All source-reserved transport/metadata,
ClientHello/transcript/record, verification and exercised PostgreSQL backing
must fit simultaneously under that connection-generation test allowance;
AWS-LC provider residency remains a distinct root-shared debit.  The helper
fails rather than increasing the allowance.  This is not the logical
`SLOT_LIMIT = 4_194_304`, does not alter production policy, and does not replace
M05/E01 final root composition.
