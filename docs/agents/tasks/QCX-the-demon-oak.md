# QCX-40 · The Demon Oak Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`.

## Sources

The direct Fandom MediaWiki API request returned HTTP 402. The supplied
`qcx-facts/the-demon-oak.md` contains the MediaWiki API capture from 2026-11-17.
The Global facts take precedence over Crystal. TibiaWiki BR provided the
additional cross-check indicated below.

- [Global quest, curid 25988 oldid 843256](https://tibia.fandom.com/index.php?curid=25988&oldid=843256)
- [Global spoiler, curid 34086 oldid 855950](https://tibia.fandom.com/index.php?curid=34086&oldid=855950)
- [TibiaWiki BR, curid 4525 oldid 401061](https://tibiawiki.com.br/index.php?curid=4525&oldid=401061)
- [Crystal 201189f8 quest scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/demon_oak)
- [Crystal 201189f8 Oldrak](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/oldrak.lua#L102-L149)
- [Crystal 201189f8 item startup bindings](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/item.lua#L3783-L3788)
- [Crystal 201189f8 chest startup bindings](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L201-L221)

## Comparison

| Part | Global and donor facts | Base Oteryn and disposition |
| --- | --- | --- |
| Entry | Global requires Premium, level 120, and a Holy Icon (9388). The BR cross-check and Crystal agree on first-entry consumption. Crystal also admits eligible returning players and checks grove occupancy. The demon task is a means of obtaining an icon. | The definition already records Premium and level 120. No entry callback exists in the live or held registry. The projected ground 918 at the donor STEP position (32716,32344,7) lacks startup UID 9000. Admission and re-entry are partial. |
| Oldrak | Visit the grove, leave, talk to Oldrak, then exchange an Axe (3274) and 1,000 gold for one Hallowed Axe (919). Report the victory to Oldrak to unlock the grave. | The paid exchange and Done 1 → 2 report exist. The NPC start can advance an unvisited character and adds a separate confirmation before the axe instruction. Accurate visited gating remains tied to entry. |
| Weak points | Global uses four separate hit counts, summons on hits 5 and 10, sometimes summons four Bonebeasts on the other hits, and replaces the last wave with one Demon. A hit deals 300 earth damage to the player. Crystal uses 170–210 earth damage and checks for live unsummoned monsters before key actions. | The inactive recipe substitutes four point checks and ten kills. There is no live attack callback or defender-wave owner. |
| Area damage | The BR cross-check describes 300 physical and 500 earth damage alternating every 12 steps. Crystal uses a random trigger for area damage. | No live area-damage callback. |
| Historical access | Global history describes special scream squares as the access method before the Holy Icon. | Five legacy progress callbacks remain recorded. The corresponding UIDs 9001–9005 are absent from the served region and cannot qualify the current route. |
| Grave | Use the inscribed Yesim Adeit gravestone at (32710,32398,7) after reporting. Crystal's UID 1001 at appearance 1979 lands at (32714,32398,8) and requires Done 2. | The map carries the appearance and inscription without the UID. The projected-map alias binds the exact tile and appearance, with the same Done 2 gate and landing as the legacy UID callback. |
| Reward choice | Global specifies one of four. Crystal binds one Demon Legs (3389) at (32710,32393,8), one Rainbow Shield (8077) at (32712,32393,8), one Thorn Spitter (14768) at (32714,32393,8), or one Spellbook of Ancient Arcana (14769) at (32716,32393,8). | The projected chests are appearance 2472 at ordinal 2 and lack UIDs 1002–1005. The direct world reward callbacks share Done 2 → 3 with item delivery in the same commit. The inactive recipe still illustrates only Demon Legs. |
| Achievement, outfit, XP | Global awards Herbicide on leaving after defeating the Oak and makes the Demon Outfit's second addon available through the outfit quest. Neither source lists quest-awarded XP. | Herbicide is catalogued and lacks an exit grant. Avar Tar grants the addon at Done 3 with the Annihilator reward track at 2, as in Crystal. No independent quest XP is configured. |

## Blockers and open facts

- BLOCKER Demon Oak entry shared-grove player occupancy for the entry and re-entry gate data-global/scripts/quests/demon_oak/movements_entrance.lua:21-42
- BLOCKER Demon Oak weak points hit-indexed randomized retaliation waves and final-cut encounter ownership data-global/scripts/quests/demon_oak/actions_demon_oak.lua:62-124
- BLOCKER Demon Oak damage self-damage on each hit and periodic movement-driven area damage data-global/scripts/quests/demon_oak/actions_demon_oak.lua:125 and data-global/scripts/quests/demon_oak/movements_area_damage.lua:3-16
- OPEN-FACTS: Global grove occupancy rule, exact Global Avar Tar eligibility before claiming a chest, historical pre-9.5 reward exchange eligibility, and random Bonebeast probability. The World coordinates and four chest-to-item assignments are established by the donor startup tables and corroborated by the projected placements.
- PARTIAL: entry and re-entry, Oldrak's visit gating and dialogue sequence, weak-point combat, area damage, Herbicide on successful exit, Avar Tar timing and inactive recipe parity.

## Verification

PG via shard (pgsplit build files applied on `32dfe58e60dd35ac42cbaf22f9d67ffbe209b9bc`).
The baseline run used the prior embedded callbacks and the same corrected PostgreSQL
fixture. Each case seeded defeat and Oldrak's report through the existing durable
quest writer, then called the production world-object USE dispatcher. All five
baseline cases returned `NothingToUse` at the expected acceptance assertion.
All five candidate cases passed. Each chest case checked the exact item and
quantity and verified that a different reachable chest refused after the claim.

| Case | Baseline | Candidate |
| --- | ---: | ---: |
| Gravestone | 22.35 s | 15.57 s |
| Demon Legs | 17.78 s | 17.64 s |
| Rainbow Shield | 16.58 s | 13.99 s |
| Thorn Spitter | 20.29 s | 15.19 s |
| Spellbook of Ancient Arcana | 25.92 s | 25.17 s |

The same filter without `OTERYN_TEST_POSTGRES_ADMIN_URL` returned five successful
route-less cases in 0.00 s. The PostgreSQL evidence is the timed run above.
The World pin generator reproduced the pinned bundle digest, and `pin-check`
verified one World. The installed CLI required an explicit output directory.
Quest authoring checks passed all 44 commands, including 27 Python tests.
`cargo fmt --all -- --check` passed.
