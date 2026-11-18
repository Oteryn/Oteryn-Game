# QCX87 Tinder Box Quest

Base `2c1e9158c30b411de28ec064bc7198ea4f87dd44`.

The quest requires Premium, has no minimum level, and recommends level 25. The imported definition agrees on the requirements. The spoiler describes a seasonal island visit, a summit climb that may require Levitate, and using the summit bones for one Tinder Box every 20 hours. The imported callback inserts a summit step credit and a once-only completion. Crystal's pinned reward uses UID 3263 and grants item 20357 × 1, matching the identity and quantity of the runtime grant. [Quest](https://tibia.fandom.com/?curid=76271&oldid=791138) · [Spoiler](https://tibia.fandom.com/?curid=76272&oldid=800663)

The pinned Crystal 201189f8 [import evidence](../evidence/OTV2-20261102-quests-5-20261102/tinder_box_quest_chyllfroest.json) records seasonal entry at AID 50000 and seasonal reward at UID 3263. The playable summit callback selects the placed bones by position, appearance and ordinal. No matching active or held source callback, separate RewardClaim or NPC offer was found by quest key. The wiki lists only the item reward. Island access is a prerequisite, and the donor's subsequent torch and Ursagrodon interactions are outside the wiki quest method.

- BLOCKER Chyllfroest entry annual calendar guard on StepIn `data-global/scripts/quests/tinder_box_quest_chyllfroest/movements_prohibit_entry.lua:3`
- BLOCKER summit reward annual calendar guard on Use `data-global/scripts/quests/tinder_box_quest_chyllfroest/actions_reward.lua:7`
- OPEN-FACTS Calendar timezone and May 1 boundary time, exact coordinates and map IDs, when Levitate is needed, quest log visibility, and additional rewards or prerequisites are unspecified by the supplied wiki pages. Crystal uses an inclusive May 1 and a strict cooldown comparison at equality.
