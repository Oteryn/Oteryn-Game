# QCX87 Tinder Box Quest

Base `2c1e9158c30b411de28ec064bc7198ea4f87dd44`.

The quest requires Premium, has no minimum level, and recommends level 25. The imported definition agrees on the requirements. The spoiler describes a seasonal island visit, a summit climb that may require Levitate, and using the summit bones for one Tinder Box every 20 hours. The imported callback inserts a summit step credit and a once-only completion. Crystal's pinned reward uses UID 3263 and grants item 20357 × 1, matching the identity and quantity of the runtime grant. [Quest](https://tibia.fandom.com/?curid=76271&oldid=791138) · [Spoiler](https://tibia.fandom.com/?curid=76272&oldid=800663)

The pinned Crystal 201189f8 [import evidence](../evidence/OTV2-20261102-quests-5-20261102/tinder_box_quest_chyllfroest.json) records seasonal entry at AID 50000 and seasonal reward at UID 3263. The playable summit callback selects native bones at (32085, 31025, z1), appearance 4285 and ordinal 1. The native corpus omits UID 3263 and the placement records have no quest-bound AID 50000. No matching active or held source callback, separate RewardClaim or NPC offer was found by quest key. The wiki lists only the item reward. Island access is a prerequisite, and the donor's subsequent torch and Ursagrodon interactions are outside the wiki quest method. The donor also emits entry and reward messages and an entry teleport effect. These belong with the source callback admission. [Native placement audit](../../reference/QUESTPC8-CONTRACT-NOTES.md)

- BLOCKER Chyllfroest entry annual calendar guard on StepIn `data-global/scripts/quests/tinder_box_quest_chyllfroest/movements_prohibit_entry.lua:3`
- BLOCKER summit reward annual calendar guard on Use `data-global/scripts/quests/tinder_box_quest_chyllfroest/actions_reward.lua:7`
- OPEN-FACTS Calendar timezone and May 1 boundary time, exact coordinates and map IDs, when Levitate is needed, quest log visibility, and additional rewards or prerequisites are unspecified by the supplied wiki pages. Crystal uses an inclusive May 1 and a strict cooldown comparison at equality.

The corrected bones callback checks Premium on use, grants one item 20357, and records the claim instant in the same transaction as the mint. The global 20-hour interval admits a fresh claim at 72,000 seconds. The summit approach no longer needs a recorded step. The loader registers the exact predecessor hash sampled from the baseline PostgreSQL case for characters who already recorded the old approach. Existing once-only grants predate the new cooldown track.

PG via shard (pgsplit build files applied on `027777d6d1eb257b8c870cec4edd4d8b5f4dcca3`, with the new test fixture). Baseline 0 passed, 2 failed at the intended dispatch assertions. Direct use 82.824 s, 20-hour repeat 45.318 s.

PARTIAL Seasonal entry and reward guards remain held. Main's chosen-recipe format represents once and daily repeats, so the imported approximation remains historical metadata. The new timer starts with the first claim under the corrected callback.

## Validation on the corrected revision

- PG via shard (pgsplit build files applied on `022b74f42662b31683befc60c1555d18c5dcb4e7`). 2 passed, 0 failed. Direct use 76.068 s, 20-hour repeat 46.206 s. The repeat case also checked the loader's predecessor registration.
- With `OTERYN_TEST_POSTGRES_ADMIN_URL` unset, the same filter on the same Cargo-built test binary returned 2 passed, 0 failed in 0.00 s. The second Cargo invocation began rebuilding dependencies after the World compiler invocation, so the route-less check reused the exact binary.
- Quest authoring passed all 44 check invocations, including 269/269 schema cases. The reward generator and formatting checks passed.
- World pin refresh and pin-check passed for one pin, digest `0676b1568f55a55fbe994f344a8e0c60debf766751d12849f56c19f60c93aae5`. The first refresh invocation inherited the wrapper's four-job default and was interrupted. Both completed World commands used a local Cargo shim to enforce one job.
- Dispatch library Clippy with all features and `-D warnings` passed.
