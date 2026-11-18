# QCX-89 Unnatural Selection Quest

Base `2c1e9158c30b411de28ec064bc7198ea4f87dd44`.

Global evidence is the retained MediaWiki wikitext fetched 2026-11-17 from the
qfacts branch at `460e1d8c2c0e88c438ae5e54910db00822dc0129`,
`qcx-facts/unnatural-selection.md`. The NPC source packet and
startup map tables use Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
[TibiaWiki quest, curid 41846, oldid 684058](https://tibia.fandom.com/?curid=41846&oldid=684058)
[TibiaWiki spoiler, curid 41847, oldid 1103829](https://tibia.fandom.com/?curid=41847&oldid=1103829)

## Comparison

The canonical definition has six main journal missions and no quest-owned
claim. The base has ten live world callbacks, including nine dance tiles
and one Ray of Light crystal. The quest has no records in
`world-callbacks-held`. The Tome claim and Cael offer are independently owned
records used by this quest.

| Part | Global and Crystal | Baseline and disposition |
| --- | --- | --- |
| Entry | Premium, Zao access and no minimum level. Level 40 is recommended. Lazaran opens Skulled through help, yes, mission, yes. Crystal tests New Frontier mission 3 value 3 when offering the mission. | The definition already records Premium and level 0. The Lazaran source was reference-only. The entry route uses the existing NPC offer and locked quest writer, with the donor Zao predicate and live Premium required at confirmation. The New Frontier peace branch and production of its mission marker belong to that quest. |
| M1 Skulled | Pass the sealed door, use the bone pile, then return one Skull of a Caveman. Crystal uses skull 10159 and UID 9259. | Door reference and quest journal tracks exist. The bone pile UID and native M1 use/return are missing. A map marker identifies the cave, with no exact pile binding in the supplied wiki. |
| M2 All Around the World | Lazaran gives the skull back. Global requires explicit skull use at 11 stops in order: Edron, Ab'dendriel, Femor Hills, Darashia, Ankrahmun, Port Hope, Liberty Bay, Yalahar, Svargrond, Thais, Carlin. Crystal advances on step-in with the carried skull and has 30 candidate squares across those stops. | The journal tracks exist. The skull grant, all 11 use steps and the skull return lack a live route. Global's explicit use takes precedence over Crystal's automatic step-in. |
| M3 Dance Dance Evolution | Lazaran refers the player to Ulala. Ulala starts the dance. A Plant Effect gives the next step and completion grants Talented Dancer. Crystal has nine AID 12333 tiles and 16 steps. | Nine active position callbacks commit the sequence and one achievement. Main has diagonal STEP. The briefing and visible next-step plant effects are absent. The retained tiles have the donor plants alongside the local terrain and lack the donor AID. Production movement through that retained collision remains unverified. |
| M4 Bits and Pieces | Ulala takes five Orc Teeth (10196), five Minotaur Leather (5878) and five Lizard Leather (5876). | Source and progress tracks exist. The live offer and atomic 5/5/5 turn-in are missing. The material profiles exist on main. |
| M5 Ray of Light | Use the red Large Crystal and report to Ulala. Crystal sets Mission05 1 to 2 and Questline to 11, with a holy-area effect and spoken success text. | The existing callback targeted UID 1052, which is absent on the retained red crystal. Crystal's exact 7807 object exists at (32982, 31431, -1), top-level ordinal 2, adjacent to the spoiler marker. The generated native counterpart binds that object and retains the legacy UID callback under its original identity. The briefing, Ulala report and exact VFX/speech still need qualification. |
| M5 Tome | One Tome of Knowledge (10217) from the chest one floor below and north of the crystal. Crystal has chest appearance 2472, UID 6293, at (32978, 31427, -2). | The physical chest, one-shot reward claim and native callback already exist under **Tomes of Knowledge**. A second claim would duplicate the reward. |
| M6 Firewater Burn | Ulala sends the player back to Lazaran. He takes one Pot of Beer (3465, beer subtype 3) for one Tribal Crest (10198). | The Lazaran and Ulala sources and quest tracks exist. The live briefing and subtype-sensitive exchange are absent. The crest now has native item facts, while the pot lacks them. |
| Warmaster addon | Cael trades one Tribal Crest for Warmaster addon 2 in Crystal. Crystal checks the base outfit and carried crest at the offer and checks for an already-owned addon on confirmation. | The live Cael callback consumes 10198 ×1 and grants addon 2. The offer uses New Frontier questline 27 and its addon track where Crystal reads outfit ownership and carried items. The existing Warmaster wardrobe writes questline 27. The wiki excerpt does not specify the full independent Cael eligibility rules. |
| Daily XP | Colour Me Good pays 500 XP. Magic Mushrooms pays 50 XP per brown mushroom up to ten (500 XP) and takes only those mushrooms. Two Orcs with One Stone pays 10 XP per orc. | No Unnatural Selection daily offer, task state, harvest or XP exchange was found. The pinned Lazaran script has no daily branch. Other quests' XP owners and kill objectives do not establish this quest's repeat eligibility, reward formula or target list. |
| Access and placements | The quest requires Zao access and uses the M1 and M5 quest doors. The spoiler gives Lazaran and Ulala map markers. | The definition retains the two door references. Neither quest NPC is in the production NPC placement scope. Crystal places Lazaran at (32992, 31471, 3) and Ulala at (32995, 31469, 1). Both positions have retained native tiles. Cael is already in the placement scope. No quest-wide new area unlock is specified in the wiki excerpt. |

Crystal line witnesses for the table:

- Entry, skull turn-ins and Firewater Burn. [lazaran.lua:63–165](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/lazaran.lua#L63-L165)
- Bone pile and ordered skull tour. [quest_system2.lua:119–127](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/actions/other/others/quest_system2.lua#L119-L127) [movements_mission2_all_around_the_world.lua:1–96](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/unnatural_selection/movements_mission2_all_around_the_world.lua#L1-L96)
- Ulala, dance and crystal. [ulala.lua:60–130](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/ulala.lua#L60-L130) [movements_mission3_dance_dance_evolution.lua:1–67](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua#L1-L67) [actions_mission5_ray_of_light.lua:1–13](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/unnatural_selection/actions_mission5_ray_of_light.lua#L1-L13) [item.lua:3859–3863](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/item.lua#L3859-L3863)
- Tome chest, Warmaster and placements. [chest.lua:2785–2790](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/chest.lua#L2785-L2790) [cael.lua:341–374](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/cael.lua#L341-L374) [world-npc.xml:18–20, 102–104](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/world/world-npc.xml#L18-L104)

## Blockers and open facts

```text
BLOCKER M1 bone pile and skull custody missing UID9259 placement and native item 10159 data-global/scripts/actions/other/others/quest_system2.lua:119
BLOCKER M2 skull tour explicit carried-skull use requires admitted item 10159 and Global target geometry data-global/scripts/quests/unnatural_selection/movements_mission2_all_around_the_world.lua:73
BLOCKER M3 plant hints and M5 crystal VFX stage-dependent presentation and source talk channel data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:49
BLOCKER M6 Tribal Crest beer-subtype intake lacks a native pot profile data-global/npc/lazaran.lua:153
```

OPEN-FACTS: main-quest XP amount/source, exact Global Zao prerequisite storage,
bone pile UID/position and door thresholds, precise accepted Global skull-use
tiles/floors, full dance effect sequence, full Cael addon eligibility, daily
colour list/rotation, mushroom harvest rules, orc eligibility and all repeat
windows. The supplied revisions establish the daily XP rates above.

PARTIAL: production Lazaran and Ulala placement, sealed door and skull route,
Ulala briefings and materials exchange, dance presentation and live map
collision qualification, M5 report and presentation, M6 beer exchange and
daily tasks. The entry dispatch test uses a test NPC placement.
