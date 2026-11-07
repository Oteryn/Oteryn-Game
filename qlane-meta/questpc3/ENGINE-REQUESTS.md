# QUESTPC3 engine requests

The green owned-absence batch is pushed at6a2333cb1d6378217e132e0b8cd8ca51c9e86a89.
Whole quests remain IN_PROGRESS until their complete journeys pass; no new whole-quest qualification is claimed.
New shared engine work belongs to qengine2 under the owner's latest binding rule.
The four active owned quests have been inspected; each needs one of these requested capabilities.
Do not start the sixteen cross-lane ownership exclusions.

| Quest | Requested engine capability | Existing API gap |
| --- | --- | --- |
| The Beginning Quest | Ground Item MINT and ground-item USE_WITH/cart callback | World rewards mint to owned backpack; ItemOnTarget binds a carried source, not a movable ground source. |
| Threatened Dreams Quest | Existing transaction-clock state operations on world reward callbacks | Reward callbacks expose SetNow/ElapsedAtLeast; world Storage exposes static sets/increments/thresholds, not those operations. |
| The First Dragon Quest | Conditional achievement participant in world Item MINT | CallbackEffects has item/outfit rewards but no achievement participant. |
| The First Dragon Quest | Annual calendar gate for NPC quest dialogue | Existing carried-use TimeWindow is minute-of-day, not an annual date range. |
| The Tainted Souls Quest | Live-creature selected item-use callback with atomic transformation and per-character credit, including summons | Quest carried-use ingress selects WorldObjectTarget; no live-creature target callback exists. Do not replace soul release with kills. |

## Source evidence

- Beginning: data-global/npc/zirella.lua:109-143, corpus crystalserver9f5a72c6, git-blob1b9058cff0c8b29783daaae30038c26b7d731072, SHA256d38ed72ad6088b832d09380ca10e79d0e3f8abd9c06bef34c9fa6308d1046380. Explicit ground branch creation, dragging, and USE_WITH/cart; existing canonical selected count remains five.
- First Dragon treasure: data-global/scripts/quests/the_first_dragon/actions_treasure_chest.lua:94-109, same corpus, git-blobedbcba3184fc3adfe3db6ef580a38eae9c84f9e1, SHA2568481fd65e997b7422192f4a85b6ae0a7c012afebe5eb561cde80bfe891ec761f. Twenty distinct UIDs14001-14020; Treasure Hunter at prior counter>=19 in the same item grant.
- First Dragon season: data-global/npc/tamoril.lua:45-63, git-blob2434aac117de37173927fad61dcf9b686cf97e02, SHA256d5d55a74c2bb9ade413c23442924b46ee6e5b05c9daf73ed8259e67f04576969. January14 through February12 gates quest conversation.
- Threatened harvest: data-global/scripts/quests/threatened_dreams_quest/event_raven_herb_bush.lua:46-53, git-blob84bb7ea71ddbd0457f714c31952a6822724276a1, SHA256ac2c7fd19872d03ed0dd365963715ed2343ea4e27327f3ed73867215fdc2ea01. Native per-character clock guard and timestamp write. Literal source duration is60*30*1000 seconds; do not silently substitute30minutes. actions_swan_spots.lua:8-19 (blob23078b85a58f75752f8d4945b439f6ff053da433) maps twelve positions to eight cooldown tracks, with five positions sharing track05.
- Tainted: packet LANE-L4.json entry oteryn:quest.the_tainted_soul, blocker and qualification_scope require live target use,100 souls and Ekatrix20h reset. Canonical content/quests/definitions/quests-00200-00299.json:1 records43 preserves explicit use/soul release and states no kill substitution. Captured Crystal scripts cover star-herb assembly and holy-water blessing; the live-creature mechanism remains unbound.

## Reuse inspected before requests

Canonical QENGINE2/QBASE implementation and migrations0999-1008 were reused from main1ae39c7fb, not independently recreated.
Fetched QENGINE2 5c01deabb33b448fe7de795a3f9271477af05ea6, QUESTPC4 8bf3338eca7e86708c7ae4a87493043b0976e831, QWORLD943fddf340608e4f14a99854cf7b4cdcbdc4f58a.
Their inspected callback surfaces do not expose the requested world achievement/live-creature capabilities.
Do not merge unrelated quest ownership or author these shared additions locally; wait for the coordinator's main merge announcement and reuse that implementation.

Packet evidence git-blob: fd2bf2793d54d77706f323aedb4f224d5def82a1.
