# W15b quest chest coverage evidence

Task: `chatgpt/quests`, [draft PR #2770](https://github.com/Oteryn/Oteryn-Game/pull/2770).
Evidence date: 2026-10-31. This document records source derivation and bounded validation; live PR comments and checks remain the task-status authority.

## Evidence status

- **PROVEN locally:** the pinned source capture check, the 12 Python native recipe tests, canonical plain authoring validation, and the exact remaining-key set. No generated manifest or index was edited by hand.
- **DERIVED:** the expected census below comes from current authored definitions plus the current in-memory projection. It is not a claim that cloud generated files have already been updated.
- **UNKNOWN for this final implementation:** Rust compilation, current PG execution and final generated/pinned content checks remain pending the coordinator's remote run. The previous 54-recipe snapshot did not reach the new chest PG cases. Test names and expected counts below are planned qualification evidence, not recorded PASS results.
- **CONFLICT / UNKNOWN source admission:** the 28 retained claims below preserve their accepted source or Item holds. There is no partial grant from one of these recipes.

## Expected final census

| Measurement | Expected count | Basis |
|---|---:|---|
| Authored claim definitions |336|All plain and variant shard definitions.|
| Supported claims |308|220 ready plain claims,21 existing Canary key variants,67 native projections.|
| Held claims |28|6 source/charge conflicts,16 Item admission holds,6 repeat/random claims.|
| Native projections |67|54 preserved container/shared-choice recipes,12 written leaves,1 exact Crystal key leaf.|
| Additional supported plain full-charge leaf |1|UID 6082, already included in the 220 ready plain claims.|
| Unique Items in native chest recipes |129|Every root and direct child; no missing child is omitted.|
| Native chest closure plus Holy Icon |130|The129 chest identities plus Item 9388.|
| Expected grant-test coverage classifications |8|Existing steel helmet plus seven positive chest source-selector cases. Actual new PG results are still pending.|
| Expected supported-but-unqualified ledger denominator |300|308 supported minus the 8 bounded grant-test classifications; each still needs its own exact World binding and qualification.|

The extension adds 68 supported claims to the previous 240:54 container/shared-choice claims,12 text leaves, key3002, and one full-charge leaf. Five additional native profiles are expected because two of the seven added closure identities,2815 and5956, already have valid native profiles. Existing native weight, effects and known stack semantics retain authority.

## Source and accepted rulings

Primary donor: [Crystal 201189f](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua). The exact table Git blob is `7b9f823542f9b8d5f99fb6960abe070324c89b8e`; the handler blob is `709a1d76512b284933a04d707a634d8c538309ce`. The captured witness is [`container_native_source.json`](../../../tools/content-schema/reward-claim-authoring/container_native_source.json), SHA256 `b073d546930a60adf511677f932e6e0d37df3a230313dc85c4b9253742325757`.

| Element | Source / accepted rule | Native implementation |
|---|---|---|
| Complete container contents | [Handler232–265](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L232). The root and every direct child grant together. | Existing `bundle_chest`, `reward_claim_mint`, custody and one-item audit owners; migration 0650 freezes/proves the complete plan. |
| Once per character / shared storage | [Handler289–306](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L289); Annihilator entries [1235–1262](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1235). | One existing durable claim identity; selecting a second alternative cannot grant again. |
| Root authored text | [Handler204–210](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L204), exact attributes below. | Optional immutable `root_text` in the same MINT reservation/receipt; existing ItemText initializer in the same transaction. |
| Child authored text | [Handler245–246](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L245); UID 6195 [attribute57–63](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L57). | The exact selected child alone gets text; custody, body, revision and timestamp are part of the complete-plan proof. |
| Crystal key3002 | [UID 6093 table1294](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1294); [handler193–202](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L193). Item 2969 is a silver key at [XML6502], with `type=key` at [XML6504]. | Exact UID, Item, position and key binding; existing `game_item_keys` producer and guards. |
| Full-definition charge leaf | [UID 6082 table1207](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1207) gives Item 3081 argument 5, matching the accepted definition's full 5 charges. | Existing plain authoring permits only this pinned case; existing timed-state owner treats absent timed state as full definition values. It mints one instance. |
| Source LF normalization | [`ots_chests.py:278–279`](../../../tools/content-schema/quest-authoring/ots_chests.py) already uses `text.strip('\n')`; [`source_text_authoring.py:47`](../../../tools/content-schema/quest-authoring/source_text_authoring.py) retains that transform. | All12 written leaves preserve the existing normalized digest; five differ from the literal donor only by outer LF. |
| Accepted repository conflicts | [Q3–Q6 answer](https://github.com/Oteryn/Oteryn-Game/pull/2770#issuecomment-6542716086). | Preserve Falcon 24h, two100-stack recipes at 6218/6226, UID 6078 Item 12510, UID 6335 Item 9238/current position. |
| Text transport boundary | [Q13/A](https://github.com/Oteryn/Oteryn-Game/pull/2770#issuecomment-6543387341). | Grant and durable text only; ordinary carried-book READ/TextWindow remains out. LOOK is unchanged. |

[XML6502]: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml#L6502
[XML6504]: https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml#L6504

The XML type witness belongs to pinned blob `ea4196beae8f77be898f05de89db81de134886b6`; source lines6502–6506 were read from the coordinator's complete pinned checkout. The primary-source snapshot contains the exact key witness 6504.

### The12 added written leaves

All give exactly one Item. `T` is `data-global/startup/tables/chest.lua`; `H` is `data-global/scripts/actions/system/quest_reward_common.lua`, both at the pinned Crystal revision.

| UID | Claim suffix after `oteryn:reward-claim.` | T line | H attribute line | Item | Normalized UTF-8 bytes |
|---:|---|---:|---:|---:|---:|
|6112|`kv.powerbolt2`|[1440](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1440)|[22](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L22)|2821|1505|
|6013|`kv.shortswordbook`|[676](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L676)|[6](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L6)|2821|115|
|6179|`kv.steelhelmet4`|[1985](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1985)|[42](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L42)|2815|70|
|6187|`quest.u7_6.explorer_society.books.hengis`|[2049](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2049)|[51](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L51)|2821|430|
|6196|`quest.u7_9.dreamers_challenge.chest_book`|[2124](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2124)|[64](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L64)|2821|41|
|6202|`quest.u7_9.the_pits_of_inferno.chest_tible`|[2171](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2171)|[74](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L74)|2836|155|
|6203|`quest.u7_9.the_pits_of_inferno.corpse_book`|[2178](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2178)|[83](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L83)|2816|249|
|6214|`quest.u8_0.the_ice_islands.formorgar_glacier_chest`|[2259](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2259)|[100](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L100)|2820|255|
|6250|`quest.u8_1.secret_service.lottery_ticket`|[2530](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2530)|[107](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L107)|5956|127|
|6272|`quest.u8_2.the_thieves_guild_quest.reward_book`|[2630](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2630)|[113](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L113)|8117|1431|
|6296|`quest.u8_6.an_interest_in_botany.botany_almanach`|[2807](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2807)|[146](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L146)|11699|998|
|6500|`quest.u8_2.the_inquisition_quest.reward_room_text`|[3167](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L3167)|[160](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L160)|2821|2042|

## Remaining inventory

All28 exact keys below are retained. UID:line links point to the pinned Global chest table. Groups are mutually exclusive; secondary blockers are stated. Canonical Item admission comes from the exact `oteryn:item.tibia.i<ID>` definitions under [`content/items/definitions`](../../../content/items/definitions), inspected through the existing [`load_items`](../../../tools/content-schema/reward-claim-authoring/reward_claim_authoring.py) authoring input. Charge evidence is retained in [`source_charge_evidence.json`](../../../tools/content-schema/reward-claim-authoring/source_charge_evidence.json) and variant `charge_dispositions`; [D277 ruling](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5933264015) remains binding.

## Source or accepted charge conflicts — 6 claims

| Exact claim key | Crystal UID:line | Reason |
|---|---|---|
| `oteryn:reward-claim.kv.ornamentedshield` | [6063:1051](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1051) | Source charge mismatch: Item 3083 argument 1 versus canonical full 150; Item 3083 also remains nonmaterializable. |
| `oteryn:reward-claim.quest.pre_u6_0.ornamented_shield.rewards.ornamented_shield` | [6062:1042](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1042) | Source charge mismatches: Item 3085 argument 1 versus full 200 and Item 3048 argument 1 versus full 20; Item 3085 also remains nonmaterializable. |
| `oteryn:reward-claim.quest.u7_3.the_medusa_quest` | [6183:2017](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2017) | Authored SOURCE_WRITTEN_TEXT_CARRIER_UNRESOLVED. Canonical text has no carrier among 3436/3567/3324; pinned Global AttributeTable has no 6183 row. Preserve the accepted hold until source reconciliation is authorized. |
| `oteryn:reward-claim.kv.deeperfibula4` | [6011:660](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L660) | Source Item 3082 charge argument 1 versus canonical full 50; Item 3082 also remains nonmaterializable. |
| `oteryn:reward-claim.quest.u7_2.the_queen_of_the_banshees.reward.stone_skin_amulet` | [6056:1000](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1000) | Source Item 3081 charge argument 1 versus canonical full 5. Item is admitted, but granting a full 5-charge instance would overwrite the accepted partial-charge conflict. |
| `oteryn:reward-claim.quest.u15_24.targuna.silver_amulet_chest` | [14093:3503](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L3503) | Source Item 3054 charge argument 1 versus canonical full 200; Item 3054 also remains nonmaterializable. |

## Canonical Item admission — 16 claims

| Exact claim key | Crystal UID:line | Reason |
|---|---|---|
| `oteryn:reward-claim.kv.desert2` | [6158:1813](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1813) | Canonical Item 3084 materializable=false; complete container recipe remains held. |
| `oteryn:reward-claim.kv.draconia1` | [6023:748](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L748) | Canonical Item 3284 materializable=false; complete container recipe remains held. |
| `oteryn:reward-claim.kv.fireaxe1` | [6067:1079](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1079) | Canonical Item 3085 materializable=false; complete container recipe remains held. |
| `oteryn:reward-claim.kv.goblintemple2` | [6119:1499](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1499) | Canonical fluid-carrier Item 2874 materializable=false and fluid semantics UNKNOWN; do not reinterpret its subtype as a quantity. |
| `oteryn:reward-claim.kv.lifering` | [6131:1593](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1593) | Canonical Item 3085 materializable=false; complete container recipe remains held. |
| `oteryn:reward-claim.kv.present_box` | [6113:1448](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1448) | Canonical Items 2882 and 2881 materializable=false with fluid semantics UNKNOWN; both missing children keep the entire backpack held. |
| `oteryn:reward-claim.kv.triangletower` | [6160:1831](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1831) | Canonical Item 3083 materializable=false; complete container recipe remains held. |
| `oteryn:reward-claim.quest.u7_9.dreamers_challenge.chests_wine` | [6197:2131](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2131), [6198:2139](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2139), [6199:2147](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2147), [6200:2155](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2155) | Canonical fluid-carrier Item 2874 materializable=false and fluid semantics UNKNOWN; all four placements share one held claim. |
| `oteryn:reward-claim.kv.edrongoblin1` | [6034:835](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L835) | Canonical Item 3054 materializable=false; source/full charges both 200 do not supply missing Item admission. |
| `oteryn:reward-claim.kv.ghoulroom1` | [6136:1636](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1636) | Canonical Item 3083 materializable=false; source/full charges both 150 do not supply missing Item admission. |
| `oteryn:reward-claim.kv.powerring2` | [6152:1762](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1762) | Canonical Item 3056 materializable=false; source/full charges both 200 do not supply missing Item admission. |
| `oteryn:reward-claim.kv.silveramulet` | [6154:1780](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1780) | Canonical Item 3054 materializable=false; source/full charges both 200 do not supply missing Item admission. |
| `oteryn:reward-claim.kv.stealhring1` | [6174:1949](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1949) | Canonical Item 3084 materializable=false; source/full charges both 250 do not supply missing Item admission. |
| `oteryn:reward-claim.kv.timering3` | [6051:962](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L962) | Canonical Item 3082 materializable=false; source/full charges both 50 do not supply missing Item admission. |
| `oteryn:reward-claim.kv.trollcave1` | [6039:873](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L873), [6040:880](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L880) | Canonical Item 3083 materializable=false at 6039. The shared-claim alternative 6040 (Item 3372) stays held with the complete claim. |
| `oteryn:reward-claim.kv.weddingring1` | [6043:903](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L903) | Canonical Item 3085 materializable=false; source/full charges both 200 do not supply missing Item admission. |

## Separate repeat/random mechanic — 6 claims

| Exact claim key | Crystal UID:line | Reason |
|---|---|---|
| `oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.coast` | [6105:1383](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1383) | Accepted 24-hour cooldown plus one uniform random reward from seven entries requires durable repeat eligibility and one frozen random result. Explicit W15b scope is once per character; PR #2770 Q3/A preserves the accepted repeat interpretation despite donor timerStorage omission. |
| `oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.island` | [6106:1391](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1391) | Accepted 24-hour cooldown plus one uniform random reward from seven entries requires durable repeat eligibility and one frozen random result. Explicit W15b scope is once per character; PR #2770 Q3/A preserves the accepted repeat interpretation despite donor timerStorage omission. |
| `oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.lower_bastion` | [6109:1415](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1415) | Accepted 24-hour cooldown plus one uniform random reward from seven entries requires durable repeat eligibility and one frozen random result. Explicit W15b scope is once per character; PR #2770 Q3/A preserves the accepted repeat interpretation despite donor timerStorage omission. |
| `oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.shortcut` | [6108:1407](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1407) | Accepted 24-hour cooldown plus one uniform random reward from seven entries requires durable repeat eligibility and one frozen random result. Explicit W15b scope is once per character; PR #2770 Q3/A preserves the accepted repeat interpretation despite donor timerStorage omission. |
| `oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.throne_hall` | [6107:1399](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1399) | Accepted 24-hour cooldown plus one uniform random reward from seven entries requires durable repeat eligibility and one frozen random result. Explicit W15b scope is once per character; PR #2770 Q3/A preserves the accepted repeat interpretation despite donor timerStorage omission. |
| `oteryn:reward-claim.quest.u11_80.the_secret_library.falcon_bastion.chests_timer.underground_bastion` | [6110:1423](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L1423) | Accepted 24-hour cooldown plus one uniform random reward from seven entries requires durable repeat eligibility and one frozen random result. Explicit W15b scope is once per character; PR #2770 Q3/A preserves the accepted repeat interpretation despite donor timerStorage omission. |

## Transaction-proof review

The final bounded review checked the following existing-owner relationships:

1. [`reward_claim_mint.rs`](../../../crates/gameplay/durability/src/durability/reward_claim_mint.rs) validates root text before reservation, binds exact bytes into intent, saves and compares the body on reservation resume, and uses existing receipt reconciliation before applying anything again. A `None` body leaves the old intent encoding unchanged.
2. [`0650_reward_claim_container_contents.sql`](../../../apps/game-server/migrations/0650_reward_claim_container_contents.sql) compares reservation/receipt root text, requires exact fresh Item identity and singleton quantity, and checks body, revision 1, writer NULL and frozen timestamp in existing Text tables. The original root claim/audit/custody guard remains in place. Its trigger is `DEFERRABLE INITIALLY DEFERRED` under [`0012`](../../../apps/game-server/migrations/0012_reward_claim_backpack_mint.sql), so receipt insertion before text/children does not expose a partial committed plan.
3. The existing 0012 reservation guard compares the whole row except its monotonic work-unit field. New root-text/child-plan columns are therefore immutable automatically. The new code never updates or replaces a committed text grant during replay.
4. [`item_text.rs`](../../../crates/gameplay/durability/src/durability/item_text.rs) initializes only a fresh live quantity 1 Item with the exact mint transaction and no previous state transition. It inserts revision 1 without UPSERT and creates no player-write receipt.
5. One concrete mismatch was repaired during this review: [`0267_item_text_write_limits.sql`](../../../apps/game-server/migrations/0267_item_text_write_limits.sql) caps storage at 4000 Unicode scalars. The new claim preflight, native schema and0650 checks now enforce both that scalar limit and the conservative 7996-byte reward bound. Root/child tests reject4001 ASCII scalars and oversized UTF-8; the accepted byte-boundary test uses3998 two-byte scalars. All actual source texts fit these limits.
6. The UID 6082 test checks one minted Item, native full charges5, no overriding timed-state row, the same instance after cold replay and one durable claim/receipt. It does not admit the different partial-charge UID 6056 case.

No further concrete proof/replay defect was found in this bounded pass. This is static review evidence, not a substitute for PostgreSQL execution.

## Validation record and expected remote checks

**Locally executed:**

- `python3 -m unittest -v test_container_native` in `tools/content-schema/reward-claim-authoring`:12 passed. The final rerun includes the four-key witness set and unchanged67-recipe census.
- `python3 tools/content-schema/reward-claim-authoring/capture_container_native_source.py --crystal <pinned-checkout> --check`: passed exact source capture comparison.
- In-memory `build_records` plus `validate`:220 ready /11 held plain definitions, empty validation error list. Complete support census equals308/28, and the 28 exact keys equal this inventory.
- Python bytecode compilation and `git diff --check`: passed before handoff; final candidate validation remains coordinator-owned.

**Expected after regenerated content and a real database; not yet PASS:**

| Filter | Expected tests / marker evidence |
|---|---|
| `world_reward_claims::container_tests` |6 unit cases.|
| `reward_claim_mint::` |10 cases including child identity and root/child text intent bounds.|
| `quest_container_chests` |9 mounted cases; all require `OTERYN_TEST_POSTGRES_ADMIN_URL` for actual body execution.|
| `quest_callbacks` |3 existing cases.|
| `world_reward_claims_the_embedded_shards_decode` |1 case,308 candidate definitions and11 plain NotReady holds.|
| `test_quest_callback_profiles` |8 Python cases; complete native closure expectation130.|

The three new PG markers are `W15B_LEAF_TEXT POSTGRES_CASE_COMPLETED`, `W15B_CRYSTAL_KEY POSTGRES_CASE_COMPLETED` and `W15B_FULL_CHARGES POSTGRES_CASE_COMPLETED`. The existing six chest markers cover Bear bag, nested key, shared achievement choice, child text, full backpack and combined incoming weight. Merely returning `Ok(())` when the database URL is absent is not execution evidence.

## Explicit boundaries

- Ordinary carried-book READ/TextWindow is absent; Q13/A accepts durable grants while leaving that transport consumer out. LOOK is unchanged.
- The six Falcon claims need durable repeat eligibility and one frozen uniform random draw. The current `once` claim constraint keeps `next_allowed_at` NULL. Source selection is [handler309–313](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/system/quest_reward_common.lua#L309); Q3/A preserves the accepted 24h interpretation despite the donor timer omission.
- The16 Item-held definitions cover20 source placements because wine and Troll Cave share claim identities. No supported alternative is split off from a held full recipe.
- UID 6340's donor z110 coordinate is a separate binding/source-coordinate defect. It is not included among the 28 unsupported definitions.
- Supported is not a deployed-World certificate. The300 supported-but-unqualified claims retain exact-map/admission/dispatch requirements. The8 expected grant classifications do not certify later Item consumers or full quest completion.
- Migration 0650 is extended in this unmerged task;0651 remains unused. There is no new client command or capability.
