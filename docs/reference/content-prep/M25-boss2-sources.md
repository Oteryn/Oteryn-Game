# M25: Ahau and Mitmah Vanguard donor evidence

Evidence only; this document does not declare the encounters implemented or played. Sources are Crystal `201189f871d23597b82caf88fd611454a2ddefb8` and Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`. Every fetched file below returned HTTP 200 and its Git blob matched the pinned repository tree. The Lua-cache audit verified all 8,185 Crystal and 5,381 Canary Lua files, with no missing or changed files. Local detailed receipts: `/tmp/m25-source-evidence/receipt.json` and `corpse-facts.json`; these temporary paths are inspection receipts, not production dependencies.

## Entry and exit behavior

The two donors' three action scripts are byte-identical. Coordinates below are literal donor coordinates, before any Oteryn frame conversion.

| Owner | Exact source behavior |
| --- | --- |
| Ahau lever | `actions_ahau.lua:1–24`: boss (34008,31696,10); ordered pads x34037,34036,34035,34034,34033 at y31714,z10; all arrivals (34008,31703,10); arena (33999,31692,10)..(34018,31705,10); lever (34038,31714,10); configured zone exit (34036,31717,10); after-kill removal 60 seconds. |
| Ahau exit | `actions_ahau.lua:18–19` and `boss_lever.lua:337–338`: SimpleTeleport StepIn at (34002,31706,10), outside the arena's max y31705, to (34036,31717,10). No custom condition. `teleport.lua:5–27` accepts Players only, attempts teleport, ignores its boolean result, then emits teleport effect at the player's post-call position and returns true. No USE requirement or invented teleport-item creation. |
| Mitmah lever | `actions_mitmah_vanguard.lua`: boss (34067,31409,11); ordered pads x34048..34052 at y31431,z11; all arrivals (34067,31415,11); arena (34054,31400,11)..(34081,31419,11); lever (34047,31431,11); configured zone exit (34060,31432,11); after-kill removal 60 seconds. No exitTeleporter configuration. |
| Mitmah separate exit | `actions_mitmah_vanguard_exit.lua:8–17`: USE at (34068,31418,11); Player guard; effect11 at origin, then teleport attempt to (34053,31431,11), ignoring its boolean result. No destination effect. The function falls through with nil. This destination differs from the lever's zone-removal destination. |

Neither lever action declares a custom onUseExtra/createFunction, quest-storage gate, or required level. The neighboring Iksupan entrance has its own level150 gate; it is not a gate on either boss lever.

Shared BossLever behavior (Crystal `boss_lever.lua:52–81,149–282,315–338`): constructor defaults requiredLevel0, minPlayers1, cooldown20 hours and fight timeout20 minutes (the latter two from `config.lua.dist`). Activator must stand on a configured pad; disabled/occupied handling precedes per-pad checks. Each Player pad checks level and cooldown under the source's account/group/GameTester exemptions, then onUseExtra. Missing/non-Player pad occupants pass that condition function; actual Players are separately collected by the lever helper. After conditions/minPlayers pass, remove old monsters, spawn boss with source true/true placement, attach BossLeverOnDeath, teleport pad occupants, write cooldown and schedule timeout. `lever.lua:112–198` supplies bottom-creature selection and helper semantics; do not replace this with an invented roster rule.

Cooldown is Player KV `boss.cooldown.<raceId>` with an absolute os.time deadline (`player.lua:664–679` Crystal, 607–622 Canary). Ahau raceId2346 and Mitmah2464 are actually assigned: `register_monster_type.lua:89–90` calls bossRaceId setter, whose C++ implementation writes `info.raceid`; the raceId getter reads that field. Do not infer raceId0 from an unrelated getter.

Generic `boss_lever_death.lua:1–39` cancels the fight timeout, schedules the configured 60-second post-kill zone removal, and shows BOSSDEFEATED to valid Players found through the damage map (`functions.lua:1075–1082` Crystal). It contains no quest-storage or party credit write.

Both separate Mitmah exit and the shared registered BossLever Action wrapper can return nil: `boss_lever.lua:321–324` discards `self:onUse`'s return. C++ `actions.cpp:703–729`, `luascript.cpp:308–324`, and `lua_functions_loader.hpp:118–119` treat nil as false. Ordinary action fallback continues (`actions.cpp:429–440`); this alone does not establish a particular final client reply or undo already-performed effects. `creature_functions.cpp:877–911` returns false on failed teleport, but these Lua callers ignore it.

## Attached monster callbacks and attack closure

Crystal's data-global and data-crystal Ahau/Mitmah bodies have no custom monster.events or mType callbacks. The two Crystal Mitmah variants differ in loot (the global variant also has iks footwraps); do not merge variant loot into a new donor.

Canary Ahau has no custom events/callbacks. Canary Mitmah `mitmah_vanguard.lua:21–23` attaches `iksupanBossesDeath`; lines145–157 contain empty Think/Disappear/Move/Say callbacks and an Appear callback which calls setReward(true) when its current type isRewardBoss. There is no creature==monster guard in that callback.

A complete pinned Lua search finds **no definition or registration** of iksupanBossesDeath in either donor; Canary's sole occurrence is the attachment itself. There are no additional creaturescripts XML registrations in these pinned trees. This has a concrete stock-engine outcome: `register_monster_type.lua:332–336` forwards the attachment; Canary `monster.cpp:107–114` logs a failed unknown-event registration and continues construction; `creature.cpp:1674–1678` returns false for absent event; `creatureevent.cpp:49–56` requires an existing loaded exact-name event. Thus this exact dangling stock attachment is an inert registration failure/warning, not evidence for an invented missing quest callback. This conclusion is bounded to the audited pins, not arbitrary extensions.

Mitmah's hugeblackring attack has interval3500/chance20 and a scripted exact 17×17 ring mask (`mitmah_vanguard_hugeblackring.lua`, identical between donors), Death damage/MORTAREA, execute(creature,var), selfTarget=true. No delayed callback, storage mutation or phase transition appears in that spell. Ahau's declared attacks are ordinary native attack definitions. The shared generic BossLeverOnDeath registration remains separate from the dangling Canary attachment.

## Corpse material and temporal chains

Native policy absence is not evidence of a guessed capacity. XML and DAT were decoded separately for each pinned donor using the existing source-item decoder. The confirmed native Mitmah body is Canary, so its chain must use Canary XML and DAT together.

| Donor / item | XML line | Container capacity | Duration seconds → decay ID | DAT flags present |
| --- | ---: | ---: | --- | --- |
| crystal / 42069 | 74406 | 30 | 10 → 42072 | container, usable, forceuse, unmove, lying_object, corpse |
| crystal / 42072 | 74423 | 30 | 300 → 42071 | container, usable, corpse |
| crystal / 42071 | 74417 | 30 | 300 → 42070 | container, usable, corpse |
| crystal / 42070 | 74412 | — | 60 → 0 | corpse |
| crystal / 44687 | absent | — | not declared → not declared | container, usable, forceuse, unmove, lying_object, corpse |
| crystal / 44686 | absent | — | not declared → not declared | container, usable, corpse |
| crystal / 44685 | absent | — | not declared → not declared | container, usable, corpse |
| crystal / 44684 | absent | — | not declared → not declared | corpse |
| canary / 42069 | 74205 | 30 | 10 → 42072 | container, usable, forceuse, unmove, lying_object, corpse |
| canary / 42072 | 74222 | 30 | 300 → 42071 | container, usable, corpse |
| canary / 42071 | 74216 | 30 | 300 → 42070 | container, usable, corpse |
| canary / 42070 | 74211 | — | 60 → 0 | corpse |
| canary / 44687 | 77074 | 20 | 10 → 44686 | container, usable, forceuse, unmove, lying_object, corpse |
| canary / 44686 | 77068 | 20 | 300 → 44685 | container, usable, corpse |
| canary / 44685 | 77062 | 10 | 300 → 44684 | container, usable, corpse |
| canary / 44684 | 77058 | — | 60 → 0 | corpse |

Ahau's explicit chain is 42069→42072→42071→42070→0, with capacities30/30/30/plain and times10/300/300/60 in both donors. Blood fluid is explicit on all four.

Canary Mitmah's explicit chain is 44687→44686→44685→44684→0, capacities20/20/10/plain and times10/300/300/60. Blood fluid is explicit on the first three; no fluidSource is declared on the final item.

Crystal XML has no declarations for those four Mitmah IDs, including ranged declarations. Its DAT contains them, but that does not establish the Canary decay chain in Crystal. Crystal `items.hpp:323` and Canary `items.hpp:307` default maxItems to8. DAT container flags assign container type/group (`items.cpp:172–174` Crystal, 175–177 Canary); Container(type) delegates to items[type].maxItems (`container.cpp:25–26` Crystal), then maxSize(initSize) at43. Thus an unoverridden Crystal DAT container defaults8; it must not be silently promoted to Canary20. Missing XML decay is likewise not a license to invent decay. Neighboring Crystal44688..44690 are unrelated scenery, not this chain.

The DAT's missing boolean flags remain false under its decoder; first corpses are unmove/forceuse, later container stages lack those flags. Preserve per-stage physical facts instead of cloning one template across the chain.

## Pinned source inventory

URLs below are immutable raw donor files. SHA256 covers the complete raw file, including XML/DAT rather than extracted rows.

| Donor | File / pinned URL | SHA256 | Git blob |
| --- | --- | --- | --- |
| crystal | [data-global/scripts/quests/adventures_of_galthen/actions_ahau.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_ahau.lua) | `637e8b3bac91884d5fc64be387847de05c5ad5fdaa2283134fd6f5777777911d` | `a5300abb8f32c7c302736e74f7bd74c8335a7271` |
| crystal | [data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard.lua) | `47896b61cb0b2d1a4f04c72c9330cce5a4e3a885a1069117eeab5cf5150c170a` | `abc8f387714fc375177196b455da94eb330806de` |
| crystal | [data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard_exit.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard_exit.lua) | `c3f77d9b03aa49462940df41bdc709b1525be39a5d6f18b8ab0cc3cde1034198` | `93e3a7d43d98599a02bd6eafd69e9c6caa15c5cd` |
| crystal | [data-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua) | `944dfe3c80292a4bfe6bae209634f8a94c885cdef5db2285954e2ff1442b52ac` | `e48a5a477d43ce6a2c38e2f23fc130004662205a` |
| crystal | [data-global/scripts/quests/adventures_of_galthen/actions_iksupan_entrance.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_iksupan_entrance.lua) | `e66c9767a592a34776c33e3a03ec31fc2effe80bb1b13eb2ad5c1d10c0e7d293` | `8ec086eeb8b7cf7dd546d41e52747429fb79b7d5` |
| crystal | [data-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua) | `eb641fb768f3e684b07e658fe41c08db1c5b9dfd43695182f7436a0072366a38` | `09f53f88c79781a620f749c49df4762731cd5805` |
| crystal | [data/libs/functions/boss_lever.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/libs/functions/boss_lever.lua) | `0a91d34c17c8fb2a20cdfddd3bb57df23d52ddafbc2053b03c938ee9162edd03` | `1a2412551d425e671a19af2ea5ea676e957bbe35` |
| crystal | [data/libs/functions/teleport.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/libs/functions/teleport.lua) | `adcfed0cf0c1bdc792ed7e3c6cc01544463c16c5ab172c49123997d26a9eb9a4` | `266e29d4fd887b76aa32676f947518b49b2ff787` |
| crystal | [data/scripts/creaturescripts/monster/boss_lever_death.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/scripts/creaturescripts/monster/boss_lever_death.lua) | `25155bf01a3b717408842c593af196bd19011ba5012d6e23a3380c3534ffbcf7` | `6fbd9a1ac7f9ec17391ea4d0d7532e2c6ec18c16` |
| crystal | [config.lua.dist](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/config.lua.dist) | `93f50fc744c9e5f1bc39165ea2ea037f5f56131a3768f1b127ebb23be4b6a544` | `f215fa6e4612bd8f500e859c504935ef649e3e00` |
| crystal | [data-global/monster/bosses/mitmah_vanguard.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/monster/bosses/mitmah_vanguard.lua) | `bb631589f2d316ad907ed3a2fa0bdfacbae29d0fe9ab1909805e39e6c0e8855e` | `48ea84f34cc9449f75a96ced23aa6170a22e0af5` |
| crystal | [data-global/monster/bosses/ahau.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/monster/bosses/ahau.lua) | `7cadc3ac4adc02a216df390e566704317470fc359c895de735041c296368e8c6` | `6cbc0ea2b26dc412164a9ced9962b69b2e57503c` |
| crystal | [data-crystal/monster/bosses/ahau.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/bosses/ahau.lua) | `9211071c0d06f8a446e2204ed32ce5cee86d04aeadb6474977535ca90a412a08` | `2a092363682ddcddf042ca8fe757863c859b063f` |
| crystal | [data-crystal/monster/bosses/mitmah_vanguard.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/bosses/mitmah_vanguard.lua) | `68972c6f565ca5855f2b1bf487f989dcabc13d772c519b63f821ac554d501615` | `bc57ccd5179443780b6543513ba854d223622509` |
| canary | [data-otservbr-global/scripts/quests/adventures_of_galthen/actions_ahau.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/adventures_of_galthen/actions_ahau.lua) | `637e8b3bac91884d5fc64be387847de05c5ad5fdaa2283134fd6f5777777911d` | `a5300abb8f32c7c302736e74f7bd74c8335a7271` |
| canary | [data-otservbr-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard.lua) | `47896b61cb0b2d1a4f04c72c9330cce5a4e3a885a1069117eeab5cf5150c170a` | `abc8f387714fc375177196b455da94eb330806de` |
| canary | [data-otservbr-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard_exit.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard_exit.lua) | `c3f77d9b03aa49462940df41bdc709b1525be39a5d6f18b8ab0cc3cde1034198` | `93e3a7d43d98599a02bd6eafd69e9c6caa15c5cd` |
| canary | [data-otservbr-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/adventures_of_galthen/actions_idol_of_tukh.lua) | `944dfe3c80292a4bfe6bae209634f8a94c885cdef5db2285954e2ff1442b52ac` | `e48a5a477d43ce6a2c38e2f23fc130004662205a` |
| canary | [data-otservbr-global/scripts/quests/adventures_of_galthen/actions_iksupan_entrance.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/adventures_of_galthen/actions_iksupan_entrance.lua) | `e66c9767a592a34776c33e3a03ec31fc2effe80bb1b13eb2ad5c1d10c0e7d293` | `8ec086eeb8b7cf7dd546d41e52747429fb79b7d5` |
| canary | [data-otservbr-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/adventures_of_galthen/actions_iksupan_exit.lua) | `eb641fb768f3e684b07e658fe41c08db1c5b9dfd43695182f7436a0072366a38` | `09f53f88c79781a620f749c49df4762731cd5805` |
| canary | [data/libs/functions/boss_lever.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/libs/functions/boss_lever.lua) | `f7d54aa7a0eea9d0342396615d0811db22c80e61fee30311d42f94415e1a9bc2` | `74013db9b062e73166ebc67476d22ac2dcfbe881` |
| canary | [data/libs/functions/teleport.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/libs/functions/teleport.lua) | `adcfed0cf0c1bdc792ed7e3c6cc01544463c16c5ab172c49123997d26a9eb9a4` | `266e29d4fd887b76aa32676f947518b49b2ff787` |
| canary | [data/scripts/creaturescripts/monster/boss_lever_death.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/scripts/creaturescripts/monster/boss_lever_death.lua) | `e057e5135960be1b063352d81649c928eabbca8921c2e1cc3f90a2d6d15246b3` | `35a86a97e92ab85aa9adbf49255f445507b09b95` |
| canary | [config.lua.dist](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/config.lua.dist) | `5ad741b00ff26c0200a9d0c3a4f056801647be5a59a45b0ef687c87887a762d3` | `55ccf0802351aa84a98bfe59e176a5f98f94a56b` |
| canary | [data-otservbr-global/monster/bosses/mitmah_vanguard.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/bosses/mitmah_vanguard.lua) | `a5ad42956dadd61b9e4fef0b9d7c5afea1a41049bfad23f83eb21be8a6a7d7f4` | `de8e7beca626079ad2137f7f1ab4c24930e4a03b` |
| canary | [data-otservbr-global/monster/undeads/ahau.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/undeads/ahau.lua) | `9236c2dd65d960e81efc5ed8e9fe251967147856d3fe2e2322013d83ced0d3d8` | `ce5704769a944d3677e5663efd9f67f64e49f610` |
| crystal | [data/libs/functions/lever.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/libs/functions/lever.lua) | `a90667dd7efb3f3663c8969573c75ff0e7153b0cb1f0e186f295bf9f72032bbd` | `651b1e9e908cbd9875fbd33d339166fd8495e034` |
| crystal | [data/libs/functions/player.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/libs/functions/player.lua) | `897edd1b4509622480dfc713d226ae54dbef1ce06c7a4a7af556fbc6744f6a0e` | `6edaa6b2ac2814634bde69782353b0a3c3396bc3` |
| crystal | [src/lua/creature/actions.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/creature/actions.cpp) | `4f25975684796718094463fd36a5624c59465f06beda6dfa14093718332626fe` | `00fa23de85930466a4b4b3e162b2a1654da2052c` |
| crystal | [src/lua/scripts/luascript.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/scripts/luascript.cpp) | `7402636bd627043a3b0d8695ed7a025136c99388459811cea7ada1cb6cd3bb3d` | `a5ea30095b5970fd270208b2304cfe1a8acd0286` |
| crystal | [src/lua/functions/lua_functions_loader.hpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/lua_functions_loader.hpp) | `53daa6dd4c3659f33d857f73f1ff18ab59612e2a3f409ec94eed102d970c0aa9` | `709a38539cd431b3641817a13313f8769e388185` |
| crystal | [src/lua/functions/creatures/creature_functions.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/creatures/creature_functions.cpp) | `fb8d8a65fa24edb11d4c068047839be29429981da3803f7a197b8ed97d2623ac` | `03242ac84346da6525c6c7ad79fb98726478e804` |
| crystal | [src/utils/utils_definitions.hpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/utils/utils_definitions.hpp) | `03ebfc0da66b01cdef46dca4195c4f1a0132a0fce058d6c670cb401bcc35ae7d` | `1b1edb7e6af67c03a0e2a5ef8b7831f0e91bd66e` |
| crystal | [data-crystal/scripts/spells/monster/mitmah_vanguard_hugeblackring.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/scripts/spells/monster/mitmah_vanguard_hugeblackring.lua) | `67b2740bbb93c1578376d9bee00f58aa7c465aacd03f8e108248ffe753a90d12` | `e698797c506c03307e90d1f342e56b7c5a0a4168` |
| crystal | [data-global/scripts/spells/monster/mitmah_vanguard_hugeblackring.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/spells/monster/mitmah_vanguard_hugeblackring.lua) | `67b2740bbb93c1578376d9bee00f58aa7c465aacd03f8e108248ffe753a90d12` | `e698797c506c03307e90d1f342e56b7c5a0a4168` |
| canary | [data/libs/functions/lever.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/libs/functions/lever.lua) | `a90667dd7efb3f3663c8969573c75ff0e7153b0cb1f0e186f295bf9f72032bbd` | `651b1e9e908cbd9875fbd33d339166fd8495e034` |
| canary | [data/libs/functions/player.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/libs/functions/player.lua) | `26e85b0c577b76ef42c429a459428758a6a8dfc3e019bb771403bc33fbbf5c0c` | `0ca29c387806995913db9d955e59f627b50341bd` |
| canary | [src/lua/creature/actions.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/lua/creature/actions.cpp) | `0da71894afa171b0e2c013353d9cfbddea630b937fede43fbc07462966651f24` | `94d51f51c860d791a53f5080762d4e6180302f0f` |
| canary | [src/lua/scripts/luascript.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/lua/scripts/luascript.cpp) | `0667b471318fde5b6a740921cd51d4294d56b74b912fc47f05c9af208495dbf7` | `2bbfed787aaa39f63f11a69165e9d47fca8aa067` |
| canary | [src/lua/functions/lua_functions_loader.hpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/lua/functions/lua_functions_loader.hpp) | `6fadaba7fe81ac542e3f8801fbc29b09c3018125131489b659d1a17613eb657c` | `a81f4458f451a906407bb550e4fe4894503558e3` |
| canary | [src/lua/functions/creatures/creature_functions.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/lua/functions/creatures/creature_functions.cpp) | `399757016f7fd226fdff1e9bbf406a5a65771b0869e1f4daf1644d2eeda47156` | `66a8843aa510dc8f93b2cc1b4a6fbe131c12e7ba` |
| canary | [src/utils/utils_definitions.hpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/utils/utils_definitions.hpp) | `bdd23ca8e86ab433bef04766f0c183d46fc2ca1796a81bab1d44db8d3fd685d1` | `d645b99d0c2c7ebe101b66cc6c6297b7f0cb8935` |
| canary | [data-otservbr-global/scripts/spells/monster/mitmah_vanguard_hugeblackring.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/spells/monster/mitmah_vanguard_hugeblackring.lua) | `67b2740bbb93c1578376d9bee00f58aa7c465aacd03f8e108248ffe753a90d12` | `e698797c506c03307e90d1f342e56b7c5a0a4168` |
| crystal | [src/creatures/creature.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/creature.cpp) | `c38b9b326b8b6943e33c5b06ea53a8d3d081784453b4a600acfec9185e75e39a` | `d1e52e5881ea75cdc5835e0563dc842b38372f85` |
| crystal | [src/creatures/monsters/monster.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/monsters/monster.cpp) | `6474c1dd275ee8321e6bd73fb36c995b888f333d5a21fe6fa487cd342de2301c` | `3f7b35f57414c2cb37c490662cacb01e11e1c9ed` |
| crystal | [data/libs/functions/functions.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/libs/functions/functions.lua) | `6a6f06d48a5b541c52ea3db6bcc68c934c1771e63a3db612e0e93df229ff110a` | `6aeb1fdb7891a45b3bb344b3728ebb0268c91e52` |
| canary | [src/creatures/creature.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/creature.cpp) | `f77f2c811f0d05a315ddd316a5d61f309814bc477ca290281a2b09e80c403683` | `89a4af117d4de4e9373ddcb96c2513006c22c6ab` |
| canary | [src/creatures/monsters/monster.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/monsters/monster.cpp) | `e0bf6b09c8e9732180ad3a5cf6f2d883b30406983ece7457d0ebb244632158d0` | `9ab1a92610989b234542c3155481035253504d14` |
| canary | [data/libs/functions/functions.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/libs/functions/functions.lua) | `0aa8c6427b7c35678af3ba8be3d64b6cd3b6cd020bb0fc11538cb7f1bcdd6bbe` | `9eaaf99bc3ad6479711b30a00a3dbe40c341148b` |
| crystal | [data/scripts/lib/register_monster_type.lua](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/scripts/lib/register_monster_type.lua) | `0c3957278446eab9f7d5f6c921bffa13d877fae09f409f7ea3c21aabd0729b38` | `d8a43ca3d14661bb6527591f94c585a64a925203` |
| crystal | [src/lua/creature/creatureevent.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/creature/creatureevent.cpp) | `5969dd4887b716be6dda0ddc711cc95ccaf962cf946245922a73b35126800fff` | `7589856539986493e7d5be2358bbd18e84d74463` |
| canary | [data/scripts/lib/register_monster_type.lua](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/scripts/lib/register_monster_type.lua) | `42165d6a26189c0c8b8631011f061f8a90ec2534a33404af4cc93e1ce011a187` | `6bb42593a2e529e25bb1e5b7df21a288a64ffa33` |
| canary | [src/lua/creature/creatureevent.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/lua/creature/creatureevent.cpp) | `038c38b68aa3d7853aeeb7414837cf138768956b8025ea62274dfd239e49ddaf` | `741c23744d43aa1f41b46a3b719a759e77645017` |
| crystal | [src/lua/functions/creatures/monster/monster_type_functions.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/lua/functions/creatures/monster/monster_type_functions.cpp) | `c4d84b86ccda05d2bc96a1145255d4a2edbd4da340801ba9e9b2c7a56e0aaa0a` | `53441a53ac2067ac3f02dc94f0d452f3894a74d9` |
| crystal | [src/creatures/monsters/monsters.hpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/monsters/monsters.hpp) | `24c5b9830d89c4150ccdaecf12f6a5d892e926b5e81c1f812d0081df32220426` | `890e4cd8250e5663d980c79e8d73d13219cd9803` |
| canary | [src/lua/functions/creatures/monster/monster_type_functions.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/lua/functions/creatures/monster/monster_type_functions.cpp) | `acd1374b0d4049a2a810d626b461304bd883daf60ef1d3710d9aa1a67c88bc15` | `54e698113faa3ca67bc9c025ee7d51a13f9e4808` |
| canary | [src/creatures/monsters/monsters.hpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/monsters/monsters.hpp) | `ba4b06827d1e88dd40eeb44a832e90b4ae18ad9108879a8aa997cb7702d7e022` | `74cf29a519015932002d149c4bb568bd131f1c71` |
| crystal | [data/items/items.xml](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml) | `8a4a526c6e45b5a282679334eb1e1e25a5b72af13aa92c8a0d409755ebdbc509` | `ea4196beae8f77be898f05de89db81de134886b6` |
| crystal | [data/items/appearances.dat](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data/items/appearances.dat) | `17a72b30b5c3c9ca8c1283cfb2febd2a93a145ff8ab66916f7a412d0f1dee5a1` | `37ce9c8ee497b7edffe809f40d02c5d12ec4ccf0` |
| crystal | [src/items/items.hpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/items/items.hpp) | `06d52ee83250ba852fe804c6b006b0aac150ceb26893e9079fa8c93502d209ca` | `b502ec621701f1cebfe7e509207649081ee3e470` |
| crystal | [src/items/items.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/items/items.cpp) | `f39d8fc6c1c81b8ec6f749e1f152dd945472ffd5a36b9cf3cd9a3a64b2f19b1d` | `bdb7b4b48aadfeb9e031311ca107aadb4ee6e30b` |
| crystal | [src/items/containers/container.cpp](https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/src/items/containers/container.cpp) | `a96aa0355e347d18c61b846a0aa74f826db0a37856d2a9ed4b33af7a7290065b` | `77828d0225515dd553b24551a2e1af9587757f68` |
| canary | [data/items/items.xml](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml) | `1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2` | `a3f0a44db14bf020029e4e09a319cbda320bd799` |
| canary | [data/items/appearances.dat](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/appearances.dat) | `aa44a154f30c7ed59acc25f246286396e4043851ef0b54ef3cf3951e46d1ce50` | `2cc2f4910af4f002f99f39e486d1a91b1b56a728` |
| canary | [src/items/items.hpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/items.hpp) | `b8c12f45122eeb575ad548719c5fb178a96460d8a41ad882d0d88b2e66d797e6` | `5181a8894449fd552626bd932ce0aaaeff749919` |
| canary | [src/items/items.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/items.cpp) | `f695ed647e111923cf8e4d487c4a0706f19e7edbfb7896655c20494d6d927fd7` | `eecbf060ef9e85b452bb897277ff3d3d013a0cf9` |
| canary | [src/items/containers/container.cpp](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/items/containers/container.cpp) | `b988ef26ec847e6849240344d45c75daa8842826702a4c4a975fe07e2083883a` | `2688a2d59bebac33b801cfdd11d0aa5c26a07016` |

## Shared complete-policy prerequisite observed in the M25 play cases

The first real M25 dispatch PG run rejected the same complete Item policy in all
three gameplay cases. The canonical House projection had already supplied stack
order while historical reward/movement/terminal captures omitted those fields.
Crystal `src/items/items.hpp:343` initializes order to zero, and
[`src/items/items.cpp:203-208`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/items/items.cpp#L203-L208)
selects clip=1, otherwise top=3, otherwise bottom=2 from the pinned DAT.
The dedicated producers now capture those same source facts. The regression
compares all 23 closed reward/movement/terminal policies against the canonical
native catalogue, retaining full-record equality and the existing identity and
custody guards. It failed for all 23 records before the fix and passed after it.
The whole canonical Item-policy qualification then passed in 15.752 seconds.

## Existing throne on Ahau's boss cell

The pinned OTBM closure puts Item 31483 on `[34008,31696,10]`, the exact
BossLever boss position. The captured tile and DAT policy both state
`block_solid=false`; the production native catalogue previously had no policy
for this existing Item and therefore conservatively blocked the boss cell.
Crystal [`data/items/items.xml:58927-58929`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data/items/items.xml#L58927-L58929)
names the throne; the pinned DAT and loader supply its immutable, non-pickupable,
passable physics. `galthen_boss_furniture.py` reuses the source Item decoder,
preserves its existing exact Crystal binding and grants only Ground custody.
The native decoder compares the complete record against its generated source pin.
No canonical Item identity or static WorldObject alias is added.

The same map-to-native check finds existing immutable Item 21287 on Mitmah's
waiting cell `[34049,31431,11]` and return cell `[34060,31432,11]`. The pinned
`galthen-mitmah-vanguard/source-room.json` and DAT policy both prove that it is
nonblocking, non-pickupable and immovable. It joins the same closed source
packet, retaining its existing Crystal identity and Ground-only custody.

## Complete carried loot closes native death settlement

The real Galthen PG case reached native lethal combat but settlement refused
`loot_item_inadmissible`: five Ahau yields and fourteen Mitmah yields still had
reference-only admission. `galthen_boss_loot.py` qualifies those nineteen existing
identities from the pinned Canary XML, DAT and loader defaults, freezes both
unchanged canonical loot-table hashes, and checks the complete registered bodies.
The source unit regression failed for all nineteen entries before registration.
Canonical identities, quantities and probabilities remain with their existing owners.

Ahau's canonical source is
[`data-otservbr-global/monster/undeads/ahau.lua`](https://raw.githubusercontent.com/opentibiabr/canary/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/monster/undeads/ahau.lua)
(SHA-256 `9236c2dd65d960e81efc5ed8e9fe251967147856d3fe2e2322013d83ced0d3d8`).
Mitmah's already listed Canary source supplies its loot. All nineteen records
retain their pre-existing exact identity bindings; no new Item identity is granted.

Eight yields have imbuement sockets. Slot counts agree between the two pinned
donors. Allowed families follow the existing Imbuement owner's Canary value-source
rule and the owner's no-OT-features decision. The capture records three differences:
Crystal XML at 77456/77476 gives the boots/sandals tier10 speed, while
[Canary XML at 76897/76918](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml#L76897)
gives tier3 speed and paralysis protection; Crystal's robe at 85407 omits energy
protection present in [Canary at 84001](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data/items/items.xml#L84001).
The Crystal spelling `paralysis removal` maps to the existing canonical
`PARALYSIS_REMOVAL` family, like Canary's `paralysis deflection`.

## Physical pushes preserve disconnected-player control loss

Canary's [`Game::playerMoveCreature`, lines 1796–1901](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/game/game.cpp#L1796)
checks the pusher's action availability, adjacency, the pushed creature's
pushability, destination distance, occupancy and protection-zone boundaries,
then invokes `internalMoveCreature` on the pushed creature. The pushed creature
does not need its own input command. Oteryn's reconnect grace remains a distinct
control-loss state: a scoped physical-step witness retains both exact actors,
sessions, leases and predecessor positions, plus one cardinal destination. It
does not clear control loss or authorize casts, damage or client commands. Drop
and cancellation revoke the witness; a changed pusher, target or write fence
invalidates it. Ordinary STEP, ropes and callback teleports retain their input
control checks. Native PUSH coordinates are unprojected through the same pinned
wire frame as ordinary movement before checking destination distance.

The destination field read uses `read_player_step_tile_in_transaction`, which
checks the exact live push witness, actor/session, Character lease, runtime scope,
ownership generation and Content pin before and after SQL. The existing standing
and command readers retain their active-session requirement. This closes the
observed durable-reader rejection without granting a disconnected player input.
The actual two-player Ahau PG case was red with `NotPossible`/`Rejected` and now
passes in **234.26252329 s**, including preserved grace, one participant returned,
one destination effect and the real durable return receipt. Its filter is
`m25_ahau_player_push_exit_returns_only_pushed_participant_postgres`.
