# OTV2-20261004-main-red-sem2b3-packet-repin

```yaml
task_id: OTV2-20261004-main-red-sem2b3-packet-repin
title: "Main red: re-pin the stats-dependent item packet chains after #1710"
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/main-red-sem2b3-packet-repin
pr: 1712
base_sha: d028be62
owner: claude-code-session-01QFRdKvFbNsNiCzMyR75FrC (control-plane P0 allocation)
created_at: 2026-10-04
updated_at: 2026-10-04
owned_paths:
  - apps/game-server/src/content/item_elemental_magic_modifier_promotion.rs
  - apps/game-server/src/content/item_mantra_bond_modifier_promotion.rs
  - apps/game-server/src/content/item_numeric_modifier_promotion.rs
  - apps/game-server/src/content/item_stack_default_successor8_promotion.rs
  - docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-magic-capacity4-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier13-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier13-source-proof-v2.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json
  - docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-source-qualification-v1.json
  - docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json
  - docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v2.json
  - tools/content-schema/item-authoring/lower_elemental_magic_modifier_packet.py
  - tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py
  - tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py
  - tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py
  - docs/agents/tasks/archive/OTV2-20261004-main-red-sem2b3-packet-repin.md
public_contracts: []
```

## Outcome

#1710 (ITEM-SEM-2b-3) changed the stats promotion packet (`91d95020…` → `194e3dd3…`), its compiler
`lower_wiki_stats_packet.py` (`3bb80236…` → `d5b5bb64…`) and `apps/game-server/src/content/reference_playable.rs`
(`e50682b6…` → `5424c687…`). Four item-authoring chains still pinned the previous digests and failed with
`source digest drift` on `main`: successor-8 stack defaults, numeric-13/17 with Magic4, Mantra and Bond,
and elemental magic. This PR re-pins them to a fixed point (D304/D305 mechanical re-pin, as in #1701).

- Every changed line differs only in 64-hex digests (33 lines before merging `main`, checked mechanically).
- Each chain's `--check` regenerates its packet byte-identically, so no packet content changed.
- Pins that no check verifies are left unchanged: the Forge-3332/289 source qualifications, the TibiaWiki-165
  historical context, the engine-items re-pin receipt and the monster native-loader proof, which cite
  `reference_playable.rs` as historical evidence. Re-pinning the Forge chains would also change content
  provenance, which is not a mechanical change.
- `reward-claim-variants.json` was stale on `d028be62`; #1699 (merged into this branch) re-sealed it, so this
  PR carries no change there.

## Re-pin receipt (against `main` `b3dab4df`)

| File | Before | After | Change |
| --- | --- | --- | --- |
| `apps/game-server/src/content/item_elemental_magic_modifier_promotion.rs` | `4716c5e77974e86a7039c301fbe07f6f822de2e8815486bc9c80af99dc696c14` | `b1e786ac7c72e80612402b618774f21de581fe6737fffeca08b0977f54f2c801` | pin values only |
| `apps/game-server/src/content/item_mantra_bond_modifier_promotion.rs` | `fcd11957cd30c3e6fde0f5e6b2859cb64586fb09daf77f930498c93f6b72c893` | `ffbde846a435a2493900acb202b24976450422d965c89c8994566ee75a142d11` | pin values only |
| `apps/game-server/src/content/item_numeric_modifier_promotion.rs` | `de5c37eac206817d21e87555c42fcfe5493055619c77b37bee2412f276bfc4a3` | `5ec194692eca4c0a5d1037403276bdda75737a98f9507ffcdfcd33e17289f3ac` | pin values only |
| `apps/game-server/src/content/item_stack_default_successor8_promotion.rs` | `2e5ecc17b9ffccc0b65f0993812d0b91012efe9f849c6ba1745ebce716ebe63a` | `f6a81742b7ed3812d5a6d8fba6bf95551b3ad48e24e83ead41145b6bec5b544c` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-promotion-v1.json` | `60dd9e0471bae254ff62f5227f8ec088ee01351be7fda7b2e5e986c77d9f9f31` | `bcb8410d40202d7fa4676c8dfa04e3e6a5a514c7294cd34277d754d82b446c70` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-elemental-magic-modifier-source-qualification-v1.json` | `de83af6b209f188fa21af3e7cbd96af917536c9b1730d1720578753e61ec5d6f` | `c800852f41264107087e23935081932b3072265cf5d858e1adf2c99164c70608` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-magic-capacity4-source-qualification-v1.json` | `9aba058979f9376996377163964cbad2a9a20cd826c071ea40c9e1273213238f` | `9c681e13094c62906d243b937240a85b82190b1555094923ec0598994784cf9f` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-mantra-bond-promotion-v1.json` | `29c0db2eb3b06b25f20740855efd3ad1d09ff12c7a3f8b88a8facae527300a3d` | `64c051bbe5648a2a87e5de0462c626e58b6d421cf0c37ce992d5bf3e07514a37` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-mantra-bond-source-qualification-v1.json` | `4533d873245a262a3fe07b05f60560adcb5f0c66db5d71d614e71a97a0259dab` | `c7145af1f27e7d5db54fdc1659acfcbee347aeb2f0a8e1edfead669eb4f82a9a` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier13-promotion-v1.json` | `f5023bdfa3b68f04b920a82765d9085ec1603d220d1c28d1a736fe66b44fffd0` | `351dd3064fc1b3c743f02aaa394a3b13aed449f76f1a0868feabbd5b5e8d6bc6` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier13-source-proof-v2.json` | `11b8a6a7f1f85765f2b9c99dabc7815405c2bf3b0c10bb78b39deebb741937c9` | `7a77689599d7f09bf37f8ae120f90e097ce0442095b6e0e015430805d3206c50` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-current-receipt-v1.json` | `427e648a656f35c6855dfc1d1020d80ac6c2aae2fd9833c14457adc73846d4f2` | `be0b91420416b1e8f9f12bea285c0d42d6e7ca46060286b7737be0939a134957` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-promotion-v1.json` | `7c146c365b978adfd7db2f6409882f881217575a5b8afb8e98d88a0ea6912fe0` | `1a8ffbaa48be0c50e12b9ae21271bef0699e98410cb69435b2cd195976e46710` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-numeric-modifier17-source-qualification-v1.json` | `c950c0c89db442612b4822e857b86e70ac01056974f9752996aadda415f2d71f` | `41cb8be311edaf65a5fd632f477e6c85987bc0c28faf2aa61ab2d64a70bea241` | pin values only |
| `docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json` | `8c71a2db90d8fc62a507bb22eb66cb406e3fc9bebe6318216f1953b92523db90` | `aaad7f5cd2b96a60e42aec5dbce1e0f48e33f338009974ee2fbdca38436c970a` | pin values only |
| `docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v2.json` | `de33d10ffca3b19240b5bf7c745ccc15dc8f70bb9a73221dc16ef74e74b5e4f2` | `5d6f08ebaa9c6d7948efdcbcb9aca9e6862cbd8056a201c761b20af7d5e893b2` | pin values only |
| `tools/content-schema/item-authoring/lower_elemental_magic_modifier_packet.py` | `95cdea1fc3b8c57350daf4d381033928f09526530b96f622ad973674d350dfef` | `1cc2b5026edb2ec3a8928f1ccff6cdc2d90bc65b0b2494fd974037ea17570d3f` | pin values only |
| `tools/content-schema/item-authoring/lower_mantra_bond_modifier_packet.py` | `2eb16f1f1b4413e0da2333b4cb12b399a3656e9be5a3a0a83d3090f092c4d87b` | `1eb88af759b99cbf796f5d9e173b439260cc04507b7d895f906e2054f1fdee91` | pin values only |
| `tools/content-schema/item-authoring/lower_numeric_modifier17_packet.py` | `e393dabf4af8a470fcecdd550a162fa19829405bc51858c4a233449c7b8a8690` | `4ab1d4dc6eee5f7e0636f06b28830c4a40e3a20a6979d4ad155bdbcf4813d19f` | pin values only |
| `tools/content-schema/item-authoring/lower_wiki_stack_default_successor8_packet.py` | `82a850a12d2c073ff000936d2d7ce97d8bc278eeb6a7193287e629aafd756999` | `d8b7df69edeb02bdf030e45ca695276af044606057ec7af37d8dc1238f371d56` | pin values only |

## Validation

- Every `item-authoring` workflow step, run locally in its workflow working directory: pass.
- `cargo test -p oteryn-game-server --lib`: 1604 passed. `content_world_project_repository` and `content_reference_artifact`: pass.
- `validate_world_project_v2_to_tree.py` and the content-migration item tests: pass.
- `reward_claim_variant_migration.py --check` and `reward_claim_authoring.py content --check`: pass.
- `cargo fmt --all --check`: clean.
- `git diff --check`: clean.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass.
