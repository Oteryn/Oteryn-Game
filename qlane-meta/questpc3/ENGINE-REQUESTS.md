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

## Additional existing-owner integration request: First Dragon boss rooms

ENGINE-REQUEST questpc3: StepIn boss admission using existing occupancy, named-boss cooldown and timed room cleanup owners | needed by: The First Dragon Quest | where: gameplay/quests world_callbacks::Encounter and gameplay/dispatch quest_callbacks/encounter.rs

Retained Crystal data-global/scripts/quests/the_first_dragon/movements_entrance_teleport.lua:41-87, tree9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d, git-blob8543eff38c536b247ca605cdb1972fe603388863, SHA256d7fd9593fb416160ed4b1d63e53890ef3d3ae8db8f559a15750c9bd062c094a0. Source checks occupied room, canFightBoss, Questline1 and the respective kill/chest/life/quiz threshold, then spawns the named boss, teleports the player, records a two-hour named-boss cooldown and schedules room cleanup. Literal cleanup delay is30minutes despite the ten-minute speech. Reuse existing boss cooldown and room lifecycle owners; do not write a second boss authority. Current Encounter schema has only minimum_level, players, monsters and room bounds, with no named cooldown/cleanup policy. Dispatch's boss_encounter.rs already exposes the native durable cooldown query. Reuse inspected QENGINE2e5850715e and QUESTPC4 8bf3338ec; no matching First Dragon entry binding found. Continue the existing-engine life-site and quiz slices while this integration is routed to qengine2.

First Dragon room/exit lifecycle refinement (2026-11-08): the admitted placement index contains zero occurrences of source exit UID35005..35008 (read-only complete-region scan /tmp/questpc3b-first-dragon-exit-placements.json). Do not qualify the four callbacks using synthetic geometry alone. The owned room activation/birth mechanism must supply native exit selectors or equivalent source-destination portals, then the ordinary movement owner can apply the unconditional exits. Source data-global/scripts/quests/the_first_dragon/movements_exit_teleport.lua:1-40 blob666108d36b2e8e9afadc60e6e85f35c40e02efea SHA2562c9c103aaa3532cc45a5517bd183d9e1f1ba174f21679ff56c33f319986fc7f6; entrance room centres/prerequisites/cooldown/occupancy in movements_entrance_teleport.lua:1-87 blob8543eff38c536b247ca605cdb1972fe603388863 SHA256d7fd9593fb416160ed4b1d63e53890ef3d3ae8db8f559a15750c9bd062c094a0. Pending old exit draft is not source placement or playable proof. This refines the existing boss-room engine request rather than creating a new quest scope.

Tainted talisman recipe refinement (2026-11-08): source actions_star_herb.lua transforms antlers10297 to22008 with no quantity argument, consuming one herb3736. Lua Item::transform defaults subType=-1 (src/lua/functions/items/item_functions.cpp:802-841, blob86afbd32a4f0c12d5ae62b62e280d80c2d4b61c4, SHA25648e27deb9bbfd53b0c7917183e1bdbccf7d1e14242eace2f0d96ef1b4ac11732). Game::transformItem retains current subtype/count when newCount=-1 (src/game/game.cpp:3125-3148, blob0d92f0ad2c25ddde690488729e33ec036254f6b8, SHA2564b26d0c5168e0907ae9044b2587a37ba647f6b6651a94e02f1c12a2e3af58d45). Current carried_row only accepts count1 operands and emits fixed unit replacements (crates/gameplay/items/src/item_craft.rs:152-195), so it must not silently split a whole target stack into one talisman. QENGINE2 freshly fetched2ddd4f9cd has no applicable target-stack-preserving conversion; its event-container whole-source consumption is a different operation. Need generic atomic carried-target transformation with exact capture/quantity/attributes and stack-to-nonstack policy before both talisman recipes are marked qualified. Canonical22008 remains held Unknown stack,22009 held NonStackable facts; native133 holywater and10297 antlers already admitted. Do not redo their admissions. Existing live-creature talisman request remains separate.

2026-11-08 Tainted Item-preparation closure:22008/22009 canonical and native custody now pass unchanged PG18.6 test56207ms (stack17/nonstack1, exact identity/quantity and drop replay receipts). They are no longer held for stack/physical/lifetime/classification admission. This closes only the source Item-data prerequisite; whole-target-stack conversion and live-creature cleansing requests remain open. Original133/10297 admissions were retained, not redone.
