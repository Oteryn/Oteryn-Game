# OTV2-20261003-main-red-item-packet-repin

```yaml
task_id: OTV2-20261003-main-red-item-packet-repin
title: Re-pin every Item-authoring packet that #1642 left stale (D304/D305 pattern)
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/main-red-sem2b1-stats-packet
pr: 1701
base_sha: ea045059e28cc10152ec08688489fd0abb82ff51
owner: implementation worker for the CP (#1622)
created_at: 2026-10-03
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - docs/agents/evidence/OTV2-2026100*-item-*.json
  - docs/agents/evidence/OTV2-20261002-stack-default-historical-native-context-v1.json
  - docs/agents/evidence/OTV2-20261002-tibiawiki165-historical-import-context-v1.json
  - apps/game-server/src/content/item_*_promotion.rs
  - tools/content-schema/item-authoring/lower_*.py
  - tools/content-schema/item-authoring/check_*historical_context.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - docs/agents/tasks/archive/OTV2-20261003-main-red-item-packet-repin.md
public_contracts: []
depends_on: []
blocks: []
```

## Outcome

The item-authoring workflow passes again. #1642 changed `content/world/objects` and `content/world/terrain` (index digests plus two new shards: `objects-12917-12931.json`, `terrain-08578-08611.json`). Every Item packet that records those inputs was stale. Each was regenerated with its own generator, and every pin of a regenerated file was moved to the new digest. No row, value, count or semantic field changed.

## Method

1. Ran each failing workflow step. The two steps that exited 2 (closed external family source guards, TibiaWiki Item stat snapshot self-test) only lacked the pinned requirements; both pass once `requirements.txt` and `requirements-dev.txt` are installed. They had no drift.
2. Regenerated with the generators' write mode. The packets with a historical context (stack default, stack historical, Forge 3332, weapon metadata) were rebuilt through the same checker contexts the CI check uses.
3. Compared every changed JSON with `HEAD` structurally. Allowed differences: 64-hex digests and the `*owner_inputs` entries (changed index digests plus the two added shard keys). Result: 0 other differences in all 34 JSON files.
4. Compared every changed `.rs`/`.py` with `HEAD` after masking 64-hex strings: identical, so only pin constants moved (30 files).
5. Cascaded the pins (Rust `*_SHA256` constants, `PROOF_SHA`/`RECEIPT_SHA`/`CONTEXT_SHA`, proof and receipt pin lists, the validator in `validate_world_project_v2_to_tree.py`) until every `--check` passed.
6. Left untouched on purpose: `OTV2-20261003-item-engine-items-repin-receipt-v1.json` (historical receipt, read by no code) and the frozen historical baselines inside the successor-eight receipt.

## Re-pin receipt

64 files changed. Packets are digest-only, rows unchanged; proofs, receipts and contexts changed digests only; code files changed pin constants only.

### Evidence packets, proofs, receipts and contexts

| File | Old digest | New digest | Result |
|---|---|---|---|
| `docs/agents/evidence/OTV2-20261001-item-capacity-promotion-v1.json` | `48a596c1771adf1a7c89a96081056c5fb3fe33d156411e720a24daa6dd6ed0d0` | `f1243e1c7bbca79aeb5201db302384ef14ff59917d811ab6bf4c073b0604eb3e` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-document-promotion-v1.json` | `026e3f263d64ae96570705cdc390acf8eb15f244ac7a0dfda3915ff855986015` | `745622f8e97290dc113ce764444eb5fce8d29610ade2624638dc31137c128020` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-forge3332-promotion-v1.json` | `aacc1065f6cda4118033b6f4ecfbd4056368347099419c1b392f29cfd0647cc4` | `393115e94a693941568313cf7ee294fdd9e8eb464cad41b08584e07024be7e9b` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-market-true-promotion-v1.json` | `be6ba4161460bcf8dda41db3fff1eaa80f4abfa40698e2e351b524f0cd364a07` | `77e7bc9c8780287a4d5b27cecf7b0b0ce26ba624ec1e2a1be2c3ab8caab9d8fe` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-movable-promotion-v1.json` | `3a89b69c36b7e7887d0223cd378c3958ffe336a0200b941a73a60944d59c08cc` | `d0a7d5fd90550fcab690e3a30412921a587b0cc7141277493fe8c5d0ef4686ec` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-physical-promotion-v1.json` | `35db476de57f9acf8d7349ded74298d0b5749acd231c094c2f8584d3df6c8a41` | `e09fd6094c566c72731731db84baa5dc6cb5ce59510736216bcb672d6d2987ba` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-stack-default-promotion-v1.json` | `ea50e9c57b9cb1b401c707e2c065f17b155772e00a8bba15833acbded78c0908` | `6c1206f266fc7ce82fe84f42cfac86f6484600796c3fa052e7f8262bb7eb1382` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-stack-false-promotion-v1.json` | `0b0a776ebffb877f0b635cea5a601ff696d98b7ee1c9bfd26d55a6b70dcaabb8` | `3e5837ff2e3c8300a0ec353d8d6fd6a1b09d12e5759d6d3cd9550cd08cd4c868` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261001-item-stack-historical-promotion-v1.json` | `214fa6ac8a5fb66916cbaad63277acdfb5c060fe99cd0dfdd8f639fa6a1d31e9` | `3798105b2ff54722bf38022a67a0e1fd7477fa1d4cbbc16570cc761f5d5d1ddf` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-description-promotion-v1.json` | `cbc653086a9c161b2ca332d34a595a62d9f75b705071e1841fc9e7d3f1052aab` | `38c725236fa80b5b0fffdc7b66ddb18b9753b03df468393dfa12f753cee028fd` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-description-wiki-promotion-v1.json` | `6d8f3c18f2f628cc463eb5442b77672492bf4cb12cd53b25b0678a9dc50349d7` | `7582d579083e767a8c731e68ff2fddd4a7c7f3cd4e87b9c25d3a7ff29c5ac100` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json` | `c42df6be0f3e0f1920e2ef8a5daf5990878f0be6c1b8e84807e446e71166fedf` | `535b64847d57e355d34483d1180e4d9337d3b996f609bb8f9e18e1aeea7264f4` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-source-qualification-v1.json` | `c1f91bcae413daa55c4ab04da644f645ee138737ae467500dbe7ccef29527db6` | `0d730e671d50c71b485e16d3a6d06e71eddf1c0fe0754f7a856e04b9db4f5a5d` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-hit-magic-promotion-v1.json` | `830f34dd0a66dabe92902c6144822caae8db2a1375f95d591ab6f20bff33492a` | `0fcfee82c2c6f1a2b2eceb77277b4ea400d8082d98a553171fdbd7d571bbe314` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json` | `c116893184a8d1516e124b193280ee214f5d17fd1a6538fd7cf345a32e9b5a03` | `67dd870a3aaf80dd4d9a88c12ddfe21ed5fbd7f2f3754b784b7b359a14c37ee2` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v1.json` | `49c373903389a37c5f97ba6dd34fb8dade27997dac685c91be49c7295faaf2a6` | `cd24e7aeed03024f5ed75e91e5c562389899c1922c04c075e69c1d41c85bd4ea` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-name-promotion-v1.json` | `df202253ef9b0a4817eba98dcacb5a65960828e022e34b9590efdb96952f4383` | `cf01b5ddc3955bb5a46194d7edbd3cd4744acba269233ef0f2c3ba64b465eed8` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-name15-historical-source-v1.json` | `294760f3f064da10cdb5b1c38398706b1a6db471af2027d317de840a939cfeb0` | `bdca769f08d5f449bca96c91c60225a2e40b52f64935ed11066cdd29c981092c` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-name15-promotion-v1.json` | `ebd4ba404de47fa33792d60007e3ae9e980efe01500a0faee070712cd7d42a01` | `e68470b2ec6bbe22600edb081f894215de694c1a852a4e60d2d74c729a0b09cc` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-name15-source-qualification-v1.json` | `4f9de51763e524148ab41e2f34a4ecfa51059a597c56fea6df0c3f07d6b7c8ed` | `c1e463f9e5843b4ffd62f16f266c056e9268d03e7b5263b1a4274969f68fb6d3` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v1.json` | `7e4cb77cbc3320eb1bfd656f6cfacb91db65f31c1a65188b7cdc7f673ee5b430` | `8e8733867b54a315b2646002369249011efc125f25e7e95a8c1d6079922ca939` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json` | `5c6145fea0899c43bea88cbb1b3094792c37e429f401d228084009ee068fad6f` | `61a6e5cb80e82ec35cb11a612e4a872f2ba77305f4f700f52de2996936b54d05` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json` | `ab2f69c2a8833f3f2ba2e08cd991d5ad2276da866ef02db3a9d367241dc4bf79` | `6b628627bc8f0e89dc882e52b56f449125766c5b12c50e9f8c46a9bf34e40309` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-source-qualification-v1.json` | `f54f066cc186cce0829346664485d3a993d80ed9fc291096c2641008755fa990` | `96ce87a055a3cf472198d2677e0275b24cc888d707247f4d834101b68c91a263` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-use-observation-promotion-v1.json` | `f4087b6eeb6f448db543ac129437e0796c80b222919affed7d02880a18653cf5` | `0092f54004ea0aef91fa971653eb4d5d6a1c4a4044c4ce2fd3c69e6abef06b56` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-use-observation-source-qualification-v1.json` | `263dddb8fcccbf7d5e494cfbcd53b9cd0303b21b774cb900d0ce534460952c61` | `d4a19507c3a3e92729455b182b2e5b401d972191e841635ab8c61d150cce6bc1` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-weapon-metadata-additional24-source-qualification-v1.json` | `3276ec12a3a10f1bef08dbd646a1296407112895f4b91d76de7e9bfc80579577` | `97f97a1665a86e007f4e9f11a6c866dc7631b8a30e86d6041aa4b57b52783dc5` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json` | `5814324f49b967e147b8d66b4d465161cf5b05bcec510670b999b563fd3e162c` | `eba82e8a623b1231fab365540f74d07897082823d366c462028fab1bebf2b1e7` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-item-weapon-metadata-source-qualification-v2.json` | `bc50d64e281534e9b8026029d3ee0b78e23114ddf042e01d01401c69c1dcd02a` | `a9fa62e51f822251d2cd7c2b465e8397d02113f2380def178da8341e1654c2ae` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-stack-default-historical-native-context-v1.json` | `d8a31202c7eec25b68b7c0fe95dad5e02c5e6298935b695497262d07a6f3fb20` | `d411d00902bc66daf1ca3522435f38d3a93510c77b69374f0b9fc487b6b8a716` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261002-tibiawiki165-historical-import-context-v1.json` | `0c1d86236adf953da981527ac45d5b4f90711eae09e1244d2b10f66fbc9e5164` | `31b1a66b9ffabde5a7605ce9190116a14dba959d4f17c981e1561cccd6ec8476` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261003-item-market-true-promotion-v2.json` | `dcc0eb7f693b2188804b4b5edebdd567ca039d46b3a0a0f7975e9345bbfe3d6a` | `baaf6416cd70248d59a4c9e870e3e5b09d73778601447af8a70265daefeeea7f` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261003-item-movable-promotion-v2.json` | `12fb60981ad76e9cd79a5845c5e4c976ce6c76f9d2fea9983c92ff7e5f5ec7b9` | `8f513eef6575614bc5e3898272e5271855127a1fb233bbfd602572907415949d` | digest-only, rows unchanged |
| `docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v2.json` | `05f5ef964ff51b067b3d420227995556c74331522076e40a43848e4dcb168149` | `f51b75ce19008d74def2818f38ad605e358d0c9c330022bc890a47dbc3c2d506` | digest-only, rows unchanged |

### Pin constants in code

| File | Old digest | New digest | Result |
|---|---|---|---|
| `apps/game-server/src/content/item_capacity_promotion.rs` | `cfa1e521ae2d3afc8e4c8b7e6020697ab76d77a8977cbd790105680fbc4b7e75` | `bac57f60bd1dc6786b0e4f869006bfee7780e79c1b05283a44919e8d80e396fb` | pin constants only |
| `apps/game-server/src/content/item_description_promotion.rs` | `f246f9922dd7285c91f20d6ce46a76ef529967eba30c058ff54f64191f5b0861` | `b78c0679d9df543886c0058c192cf7671985ca14c4dc0b8d3abb58575bdbbc3e` | pin constants only |
| `apps/game-server/src/content/item_description_wiki_promotion.rs` | `929ec6a43b969e0f06f6d6321894cc07ef07ea9cf36d76e890e9d4c89d8abbf3` | `ea7fe57196835d969525e9eb97c5a0472b2ae20dedc2a19f5cbf07f92b33566e` | pin constants only |
| `apps/game-server/src/content/item_document_promotion.rs` | `972534766bc1191b2d392705e1b9bbc805c88a3c3ec799c61d38c46e43f3df61` | `8dd76da04eb44bea76d337e41c24cee10811ac4eb769a70322da9dec7e6ff608` | pin constants only |
| `apps/game-server/src/content/item_elemental_magic_modifier_promotion.rs` | `0ea5d8371189f73f8fb840857eea977561874dbe57de8d518cbcd8a10f4bf5c5` | `170b39607f3e7588e8a4c1c18f8deee088b0cb2be7bc59da17a4e10fa744bace` | pin constants only |
| `apps/game-server/src/content/item_forge3332_promotion.rs` | `ae575737dd3d7b4884b6da2c1d23f73803d589d7534a51133ff5c7a7e2b8d7fa` | `2ce4c90be92436b467de5d28ee0d670474d495269733b08725781f1cc71b15fe` | pin constants only |
| `apps/game-server/src/content/item_hit_magic_promotion.rs` | `a3120bd09e6f20738b6d696fa1bf472a17341855d72ef6aa6e875a0f13f01014` | `6e069a97cb74e88b95def47f11fd6a1df49ce6d041ca65b9535bfbaa94a454e6` | pin constants only |
| `apps/game-server/src/content/item_mantra_bond_modifier_promotion.rs` | `ce8c5b2184893478a626470d616edbdb57dc992febf02ea0d78817c63fa9b01a` | `aef20fd87cb759ef8ca2df90a14037a04fe03e3263855df47faa6407ed6054b1` | pin constants only |
| `apps/game-server/src/content/item_market_true_promotion.rs` | `3abb6505a2dbb2590c89511aa45452cad4c2347bf2f9ba318bcbcfc172fa6d50` | `c85a03c40dcd028cf138b1b4191ed40657d9eacfd34708eeb75aaed6fdf5a775` | pin constants only |
| `apps/game-server/src/content/item_movable_promotion.rs` | `f8103cb4eeae2bda3710cd8bda7eeb3f3372f3c19bd8ca9d78305ef1cad7ea6a` | `87e4ed2d1e1c276036f66fbed8269242aa30da2a7c60937608a3873c953f17a7` | pin constants only |
| `apps/game-server/src/content/item_name15_promotion.rs` | `75b59cb37518f871e68cf5fbc944a5c9879f41ac5b64c0fb2967ffd2f2fee901` | `8bec11d3b518cfb5a65ce018f052666b033c06b0193f78d7418eae1584d77c53` | pin constants only |
| `apps/game-server/src/content/item_name_promotion.rs` | `0e35a7d6f83f98aadbd1672775e558d254629b8919727f58a8ef6f12b21927fb` | `299788140ec74e3c6fbc9849d348ebbdecb9f27b2c9a5e75cffdf4f707b41e9f` | pin constants only |
| `apps/game-server/src/content/item_numeric_modifier_promotion.rs` | `413f906937a14fe6d24d1446e318f8bbdc987a74f3c5cdb6ab36f2b8e76b4aa4` | `76b87f725849f99cc9bec96478465df53b19d15750d50dd6462c59a58f549371` | pin constants only |
| `apps/game-server/src/content/item_physical_promotion.rs` | `5018c7d327a5570761b769ef1a5378b146741ce49246253ca15732ac94ecd768` | `5e0d729555439b2210bec8b2f606ed3f076456de87af86856573d01610eccc01` | pin constants only |
| `apps/game-server/src/content/item_stack_default_promotion.rs` | `56e5b404065529bc4aacbfa2c1666711d710f12b7762a7889c60e007e8aaff5c` | `ba55cb88bea89c8d8622f0fcd2b16997c4f3098726bcde602bb674a882fe4110` | pin constants only |
| `apps/game-server/src/content/item_stack_default_successor8_promotion.rs` | `80e0c97a8ac9a180f04a558047ac695e9264da2ae3bd1912a06a63f08c27a0b6` | `f861446cd9b73e36a41c7176630d91ddbeba6d0efc784cee4b69c680069247ff` | pin constants only |
| `apps/game-server/src/content/item_stack_false_promotion.rs` | `575d610bc5fe34201bed90e5a1add4760cb37572013f92aca2dd3c949b1d2e3f` | `99c1d86930d0d032b8e09bf9e5824fc5d0ce9372422aae80385a1f2a3a482068` | pin constants only |
| `apps/game-server/src/content/item_stack_historical_promotion.rs` | `2010cd555a1af0bfb6d340e70f066ea5502c6667d2375383730000c36f2d3bf4` | `20d615061da93aa5dcc143b17aaa90cd4300ea8f0941df8f668348909355e96a` | pin constants only |
| `apps/game-server/src/content/item_use_observation_promotion.rs` | `faaa7a5a4d5bbde4f35e9a792fa5170fa8c002d91a1d679bb29687b4c7e32a4c` | `6b4028c16d3a0852d1de82846155ef1fc5e808ea6dd31549ff187929b650209e` | pin constants only |
| `apps/game-server/src/content/item_weapon_metadata_promotion.rs` | `55a7ae41a75ed3b679ba9d4ae32bd0884580ff92ce6194bd04c93c4d03564f07` | `cecc28f501a0e7636fb63ccd7c6b481140e360e1077718eb59ac97299c94aadd` | pin constants only |
| `tools/content-migration/validate_world_project_v2_to_tree.py` | `3fdafb18dd149fa7b9afe034bff076b72b51173da5ff259414e704c77830bf53` | `2ec8fd044e82a73f6e4f35117bbdf9d3f9a2b9223fe8e1de0b18e189d1b6a910` | pin constants only |
| `tools/content-schema/item-authoring/check_stack_default_historical_context.py` | `ba02602e08ca6ea843fcdcc1a4279e20239f9757842df39fa4993b90202f4f23` | `e6a0be3d47dd59d2a2b31b147e6df57ee65c9de6e19796b4ad912ff890084f33` | pin constants only |
| `tools/content-schema/item-authoring/check_tibiawiki165_historical_context.py` | `543917e4c890a242359a6b310bb5f4bd3ef9bf94467a45226702da6cc78cc6d9` | `f8a1850fde971c7157e530589b69556e8d2fa93a24576d6a44ebe38d91b317d7` | pin constants only |
| `tools/content-schema/item-authoring/lower_elemental_magic_modifier_packet.py` | `780b235ce86c0754e8e1bd1bac8beb5bb825dbe7f413bc41478cbc52d589bcd7` | `dd390ad38035c8fbe44886291676289525f5a37dcab842a0a0a8005b3a2ea242` | pin constants only |
| `tools/content-schema/item-authoring/lower_item_name15_packet.py` | `21f963ca0b5c6dedc6cd6c76e52554c5260a48215e3abf77ed4c6576a3b0d82a` | `4e0fd5d477d5028c2b685b5c430c1ff383427eaa4a1a26752453a573427c5955` | pin constants only |
| `tools/content-schema/item-authoring/lower_item_use_observation_packet.py` | `a06ce2a81e899bae2e04b5312622a79967ade74cdcbb17cbb6ec39e73194e190` | `6fb8ddb4c80477505406dd5f21885c34116693bc74d2e897a907eed4020e1ef5` | pin constants only |
| `tools/content-schema/item-authoring/lower_item_weapon_metadata_packet.py` | `878e2828695d7b0d8f8033a61a47b4a87b77f14451d556a512154364031d0ea9` | `b77745c9de054f5fafc3900f2b82c5f916b3a612f963b1d3883aa5f54eb675e8` | pin constants only |
| `tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py` | `4f1cf63a980da1a4e85863d43b5c84f62fd3723429d431f60c8cd72baf570815` | `cd586605ebafb3247a96c156b59f0fe39b8d1d0b809c6761b3dfc0edafdf108f` | pin constants only |
| `tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py` | `a70e9c3a121b6348f569477ae3d1dd4449e9b3f0adf0cda7f0f54ae275992d86` | `bb8ae85e4619c8cc90b2fd6b1a6f3122eff53601606a89a992b6c52944f75601` | pin constants only |
| `tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py` | `f60962a800eadd11891a5c48f7c8d5e201ac804a64e5e8629e6501aa8de5adbc` | `5fa7c2eaa6ca362f3d1275176150a3e49f362621c4dd5903ad87bc7f1d01a10e` | pin constants only |

## Validation

- Item-authoring workflow steps run locally (all with real exit codes): every packet test and `--check`, the historical-context cohorts (default, historical, forge, weapon, weapon-tests, formal, engine), fixture tests, Item Master census, client appearance census fixtures, appearance membership test, `world-object-authoring` tests: 0 failures. Steps that download pinned upstream files (census drift, appearance membership manifests, catalogue drift) were not run offline; `test_qualified_world.py` needs the same network source and fails identically on `HEAD`.
- `ruff 0.16.1` check and format check on both authoring directories: clean.
- `cargo fmt --all --check`: clean. `cargo test --locked -p oteryn-game-server --lib`: 1555 passed, 0 failed, 2 ignored. `--test content_world_project_repository`: 3 passed.
- `validate_world_project_v2_to_tree.py`: PASS. `git diff --check`: clean. `validate_governance.py`: pass.

## PR and closeout

- PR: #1701 (also carries the ITEM-SEM-2b-1 stats packet re-pin from its first commit).
