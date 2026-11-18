# QCX44 · Forgotten Knowledge Quest

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`.

Global evidence is TibiaWiki structured wikitext fetched 2026-11-17. The Crystal comparison uses `201189f871d23597b82caf88fd611454a2ddefb8` as donor evidence. The retained Albinius file at `9f5a72c6` is byte-identical to the pinned Crystal file. Its SHA-256 is `1a476ee079849acee80dd99fb107ae5fd674c2d3c3343beed8002840d09cea18`. [Quest page, curid 79436, oldid 1083592](https://tibia.fandom.com/?curid=79436&oldid=1083592) · [Spoiler, curid 79437, oldid 1200256](https://tibia.fandom.com/?curid=79437&oldid=1200256) · [Crystal Albinius](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/albinius.lua#L172-L192)

## Comparison

| Part | Global | Base and disposition |
| --- | --- | --- |
| Definition | Premium, no required level, recommended 200, shown in quest log | Premium, required level 0 and log visibility agree. The definition has no recommendation field populated. |
| Temple entry | Five Heavy Old Tomes when the Temple is already rebuilt. A new world accepts all carried tomes and rebuilds at a shared threshold. | Albinius has the five-tome exchange for item 23986. The selected QuestPC7 route also has an Ivalisse one-tome shortcut and a placed tome grant at 32334,32090,7. The new-world phase is absent. |
| Tome chests | Five Brass-Shod Chest sites and creature loot provide the tomes. | Claims 6001–6005 each give one 23986 and match the five wiki map sites after conversion from mapper sectors to tile coordinates. The five claim appearances are 23740 or 23741. The B3 map entries have the corresponding appearances but omit all five source UIDs. The previously qualified cohort injects selectors from the claim records, so it does not detect this production map gap. |
| Albinius elemental offerings | Energy 50 Marsh Stalker Feathers, Death 50 Pelvis Bones, Ice 50 Fish, Earth 50 Acorns and Holy 50 Incantation Notes. Ice also requires The Contactman. | Energy 17462, Death 11481, Earth 10296 and Holy 18929 already use 50 each. The generated programme explicitly excluded Ice. |
| Portal placements | Separate elemental entrances from the Halls of Hope. Fire requires Mortal Combat in The New Frontier. Final Energy Gate requires all six elemental boss kills. | Crystal maps the elemental entrances and returns through action IDs 24873–24886 with appearance dispatch. The B3 Halls of Hope entrances at y=32087, z=7 have the expected appearances 25047, 25051, 25049, 25053, 25057 and 25055 at x=32325, 32328, 32331, 32334, 32337 and 32340 respectively, with no action attributes. The final appearance 10840 at 32332,32094,7 also has no action attribute. The selected route uses position-and-appearance selectors and links rooms in a fixed order. Its Energy entry at 32327,32093,7 skips the Replica Dungeon, and Fire entry at 32328,32093,7 tests the local programme stage and sends the character directly to Silus. |
| Energy | Two levers, five Golden and five Diamond replica kills in a 10-minute world gate window, then a one-to-five-player lever, four prism cycles with Lloyd, and a 10-minute fight. Repeat kills require fresh personal replica credit. | The selected route goes directly to Lloyd and counts one death. Crystal's lever schedules 30-minute eviction. |
| Death | Lantern, Old Note, portrait, Old Silver Key, lit-lantern wall passage, one-to-five-person Lady Tenebris room. | Lantern claim 23734 and portrait claim 23732 exist. Both B3 selectors omit their source UIDs 6310–6311. The note/portrait/key sequence and forward wall admission are bypassed in the selected route. The standalone action 24877 is the source-backed wall exit. |
| Ice | Albinius offering, Dragon Mother assistance and stolen egg, up to five players, a 17-minute fight, hatching and a report back before shrine access. | The selected route uses an egg prop to enter Thorn Knight and has no Melting Frozen Horror objective. Crystal schedules 30-minute eviction. The fish hand-in still needs compatible admission for existing saved progress. |
| Earth | Forest Fury gives an Empty Birdcage. Free three parrots, grow five birches with seeds and pond water, heal the tree, then fight Thorn Knight. | The selected route enters Thorn Knight from the Ice egg. It omits the Forest Fury exchange, bird and plant counters, and live tree portal. |
| Fire | Ivalisse's temple/father dialogue and two confirmations, Mortal Combat prerequisite, Silus password `zzubaran`, then the Soul of Dragonking Zyrtarch fight. | The local programme uses its own father and Silus stages, advances via abbreviated replies and bypasses Global portal admission. |
| Holy | Time Machine, Old Machine Imbuing Crystal, single five-player room with Time Guardian forms. | One Imbuing Crystal 24964 is attached to an existing claim. The B3 machine lacks source UID 6312. The selected route skips the Time Machine and adds a single Time Guardian objective. |
| Final | The six elemental kills include Melting Frozen Horror. The final lever admits up to 15. Astral Powers, Source and Glyph unlock the Lore Keeper fight. | The selected route counts six encounters including Lore Keeper itself and omits Horror. It goes straight from Time Guardian to the final room and uses a single final objective. Crystal configures a 14-day cooldown while the Global overview describes 20-hour room reentry. The snapshot does not explicitly resolve the final room exception. |
| Powerful access | Energy grants Cloud Fabric, Electrify, Swiftness and Punch. Death grants Lich Shroud, Reap and Vampirism. Ice grants Quara Scale, Frost and Blockade. Earth grants Snake Skin, Venom, Slash, Chop and Bash. Fire grants Dragon Hide, Scorch and Void. Holy grants Demon Presence and Precision. Final grants Strike and Epiphany. Six branches describe post-kill shrine use. | Dispatch has named boss-track checks and an alternative quest-completion rule. The selected QuestPC7 branch counters are separate from those canonical boss tracks. Its completion is via an Ivalisse `knowledge` keyword and can satisfy quest-wide rules. The wiki ties each branch to its own access proof. The 22 family names and effect amounts agree with the retained Canary tier-three catalogue. The access proof is the discrepancy. |
| Other rewards | Bosses drop Astral Shaper Rune pieces for the Stone Rhino mount route. The pages give no explicit quest XP award, achievement or outfit. | The eight imported claims contain the five tomes, the lantern, portrait and crystal. The selected Ivalisse completion has no item, outfit or achievement effect. |

## Reward placement correction

Crystal's chest table confirms all eight missing UIDs and their associated appearances, positions and item quantities. The world authoring generator validated the donor table against the existing claim records. The existing native reward placement registry now binds each claim to the verified untagged B3 entry using its exact position, appearance and ordinal. This keeps the original claim keys and avoids changing the saved quest definition hash. Reward quantities remain one per claim. [Crystal tome chests](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L590-L620) · [Crystal lantern, portrait and crystal](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2915-L2939)

## Engine dependencies

The selected quest's death objectives and the engine's encounter components exist. The blockers below concern the additional source behavior and its composition with quest progress.

- BLOCKER new-world Temple shared rebuild total and consume-all donation data-global/npc/albinius.lua:112-128
- BLOCKER Energy Replica gate timed world counter and temporary spawn/portal lifecycle data-global/scripts/quests/forgotten_knowledge/actions_servants_mechanism.lua:31-56
- BLOCKER elemental shrine access encounter-completion credit distinct from lever cooldown data-global/scripts/quests/forgotten_knowledge/creaturescripts_bosses_kill.lua:16-42
- BLOCKER Earth portal timed growing-birch and world tree transformation data-global/scripts/quests/forgotten_knowledge/actions_plant.lua:419-481
- BLOCKER Lore Keeper final fight party cooldown and Astral Power/Glyph encounter lifecycle data-global/scripts/quests/forgotten_knowledge/actions_the_last_lore_keeper.lua:31-82

- BLOCKER Ice Portal offering compatible predecessor registration for previously started Albinius progress data-global/npc/albinius.lua:172-192

## OPEN-FACTS

The wiki does not state the new-world rebuild total, server-side item IDs, drop rates or rune-part quantities, the exact final-room cooldown exception, or quest XP, achievement and outfit grants. Item IDs in the comparison describe checked existing records and the donor. The snapshot maps selected sites but leaves most portal selector IDs, per-spawn timings and exact target tiles unspecified.

## PARTIAL

Full route parity remains open for physical portal bindings and Fire admission, alternate Ivalisse route removal or migration, Death desk and lantern admission, Dragon Mother and egg, Forest Fury exchanges, Holy Time Machine and crystal placement verification, the final six-kill gate, individual shrine unlocks and all encounter dependencies above.
