# QCX-40 · The Demon Oak Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`.

## Evidence

The Fandom MediaWiki API returned HTTP 402 and “Please contact the site owner
for access.” This audit uses the repository's captured Fandom revisions and
the matching TibiaWiki BR cross-check. The donor files were read at Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`.

- [Quest](https://tibia.fandom.com/index.php?curid=25988&oldid=843256)
- [Spoiler](https://tibia.fandom.com/index.php?curid=34086&oldid=855950)
- [TibiaWiki BR](https://tibiawiki.com.br/index.php?curid=4525&oldid=401061)
- [Crystal quest scripts](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/demon_oak)
- [Crystal Oldrak](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/oldrak.lua#L102-L149)

## Comparison

| Part | Global and donor facts | Base repository |
| --- | --- | --- |
| Entry | Premium, level 120, one Holy Icon (9388), consumed on first entry. The icon comes from the 6,666-demon task and can be acquired from another player. Crystal rejects completed characters and checks grove occupancy. | Definition has the level and Premium. There is no entrance callback. The active map lacks UID 9000 at the documented entrance. |
| Oldrak | After visiting the grove, exchange one Axe (3274) and 1,000 gold for one Hallowed Axe (919). Conversation after defeating the Oak grants grave access. | Axe exchange exists. The starting conversation can set progress before visiting the grove. There is no reporting callback from Done 1 to Done 2. |
| Grove | Four independent weak points with summon waves on hits 5 and 10, chance of Bonebeasts between waves, and a Demon on the last cut. Global describes alternating 300 physical and 500 earth damage every 12 steps. Crystal's area damage uses a random trigger. | The inactive recipe substitutes four checks and ten kills. Five old-route square callbacks match UIDs 9001–9005, which are absent from the audited region. There is no live combat or area damage callback. |
| Grave | Use the Yesim Adeit grave (32710,32398,7) after reporting to Oldrak. Crystal UID 1001 teleports to (32714,32398,8) only at Done 2. | The callback uses UID 1001. The projected gravestone retains its appearance and inscription and lacks the UID. |
| Item claim | Choose one of four items. Crystal chest UIDs 1002–1005 share Done 2 → 3 and each grant one item. The four IDs are 3389, 8077, 14768, 14769. | Definition has no claims and its inactive recipe lists only Demon Legs. Four matching chest appearances exist in the room without source UIDs. |
| Other rewards | Herbicide on leaving after defeating the Oak, Demon Outfit addon 2 from Avar Tar. Neither source lists quest-awarded XP. | Herbicide exists in the achievement catalogue and recipe without a grant. Avar Tar's addon callback exists at Done 3 and requires the Annihilator reward track. There is no separate quest XP. |

## Blockers and open facts

- BLOCKER Demon Oak weak-point attacks hit-indexed randomized retaliation waves and final-cut ownership data-global/scripts/quests/demon_oak/actions_demon_oak.lua:62-124
- BLOCKER Demon Oak grove damage periodic player damage driven by movement count data-global/scripts/quests/demon_oak/movements_area_damage.lua:3-16
- OPEN-FACTS: current Fandom revisions, exact chest-to-unique mapping in the served room, status of the historical scream-square route, exact Global Avar Tar prerequisite, and historical pre-9.5 exchange eligibility.
- PARTIAL: entry, Oldrak report, grave binding, chest claims, fight, achievement grant, compatibility corrections to the inactive recipe.
