# QCX39 · The Annihilator Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`.

## Sources

- The workspace MediaWiki API request returned HTTP 402. The user supplied both API wikitext captures from 2026-11-17 in `qcx-facts/the-annihilator.md` at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. These are the [base revision](https://tibia.fandom.com/index.php?curid=2520&oldid=1090717) and [spoiler revision](https://tibia.fandom.com/index.php?curid=2527&oldid=1090718).
- The reachable TibiaWiki BR article supplies the walkthrough, item choices, achievement and Avar Tar outfit route. [TibiaWiki BR revision](https://www.tibiawiki.com.br/index.php?curid=4522&oldid=401062)
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies IDs, positions and NPC prompt details. The retained artifacts cite earlier donor snapshots, so the corresponding files were read at the requested revision. [Crystal tree](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global)

## Comparison

| Part | Source and current base |
| --- | --- |
| Requirements | The definition records Premium and level 100. The native outer expertise door and the lever enforce level 100, and the lever requires four participating characters. The sealed approach door denies USE by characters with the shared claim and closes after passage. The lever has neither a separate claim predicate for the four station occupants nor an independent Premium predicate. The recommended level 130 is advisory in the Global page. |
| Lever and arena | The captured placement uses UID 30025 and lever 2772 → 2773. The active shared-access callback uses the four donor tile pairs, rejects an occupied room and spawns six Angry Demons. Two block the passage. The native Angry Demon profile already registers Fire Elemental summons at 10% every 2,000 ms, with a maximum of one, matching Crystal Angry Demon lines 57–62. Existing continuous-route tests cover the combat and reward-room path. The native route lacks the Global ten-minute transfer of remaining arena occupants to the reward room. Crystal global lever lines 3–62 and 83–84. |
| Chests | UIDs 6085–6088 on appearance 2472 at (33227/33229/33231/33233, 31656, 13) offer one Demon Armor (3388), one Magic Sword (3288), one Stonecutter Axe (3319), or one Present Box (2856) containing one Annihilation Bear (3213). All four share the once-per-character claim. Crystal chest table lines 1233–1263. The retained definition recipe describes only the Magic Sword choice and retains its historical holds. The active claim and captured placement records hold all four choices. The bear inscription is already present in the item definition. |
| Achievement and XP | The active chest grants the Annihilator achievement. Crystal awards the achievement for all four UIDs in `data-global/scripts/actions/system/quest_reward_common.lua:178-184`. Fandom and TibiaWiki BR confirm the achievement and list no quest XP grant. Creature combat XP is separate. |
| Exit and access | Captured portal 1949 at (33236, 31655, 13) exits to (33213, 31671, 13), before the level gate. The shared access callback binds AID 10102 and appearances 5113/5114 to the shared claim. The arena and exit-side doors use the captured map. Crystal door lines 3–24 checks numeric reward storage equal to 1, while the native gate uses claim existence and stays closed after the outfit callback changes its track to 2. |
| Outfit | Avar Tar's existing base callback requires reward track 1, advances it to 2 and grants the Demon base. Crystal lines 98–108 use look types 541/542. Separate Demon Helmet and Demon Oak quests own the addons. The generated `outfit` offer matches the complete command, so it misses `demon outfit`. Crystal matches the word `outfit` within that phrase (Avar Tar line 57 and NPC helper lines 13–20). The generated topic prompts and base reward reply differ from the donor. The base callback also omits the blue magic cue and unmet-prerequisite reply from Crystal lines 98–108. |
| Identity and source status | The shared-access records use `oteryn:quest.the_annihilator`, while the definition and outfit callback use `oteryn:quest.the_annihilator_quest`. The encounter generator currently verifies the former literal. The held or unlowered status in the import records does not describe the native route already exercised by the existing route tests. Fandom places the quest in The Ultimate Challenges log while the imported definition sets `shown_in_quest_log` to false. Crystal `data-global/lib/core/quests.lua:4345-4383` lists only Barbarian Arena missions in that log. |

## Blockers and remaining work

- BLOCKER Annihilation room forced exit timed group transfer to reward room data-global/scripts/quests/the_annihilator/lever.lua:47-80. Fandom specifies 600 seconds and the donor omits the transfer. Main's timed encounter path rejects party stations and only schedules spawn and cleanup.
- BLOCKER Lever eligibility per-participant durable completion predicate data-global/scripts/quests/the_annihilator/lever.lua:28-45. The shared encounter schema accepts station positions and a common minimum level. It cannot express a claim check for each occupant at activation. The sealed approach door provides a per-character check on USE.
- OPEN-FACTS The Global Annihilator quest-log mission ID, appearance condition and state text.
- OPEN-FACTS Exact Global coordinates for the ten-minute fallback landing and the reset schedule. Global confirms one-time treasure access and requires an unfinished party.
- OPEN-FACTS Whether the live Global admission check enforces Premium at the lever itself in addition to Premium access to Edron.
- PARTIAL Quest-log projection, historical import identities, status and recipe, and the remaining donor NPC presentation and refusal differences.

## Change and validation

The Avar Tar opening offer accepts `demon outfit` and advances to the existing confirmation and base choice. The authoring change is confined to that offer in `qengine_rewards.py`. Its output is generated into `content/quests/shared-reward-offers.json`.

- PG via shard (pgsplit build files applied on `922ef6745bc88625d4d016cc1ba6cb4bb8a7ce60`): 1 failed in 61.64 s. The real NPC dispatcher returned empty lines for both `demon outfit` and the following `yes`, exactly the pre-fix failure.
- PG via shard (pgsplit build files applied on `1b6b4cfb0635698d7a0f29199baf3b24f5e72d6a`): 1 passed in 54.11 s. The full phrase opened Avar Tar's offer and `yes` reached the reward choice.
- The same shard and filter without `OTERYN_TEST_POSTGRES_ADMIN_URL`: 1 passed in 0.00 s. This exercises the route-less guard.
- The NPC offer generator check and both formatting checks passed. World pin refresh reproduced digest `0676b1568f55a55fbe994f344a8e0c60debf766751d12849f56c19f60c93aae5`, and pin check verified the one world.
- Quest authoring checks completed all 44 commands. The schema verifier passed 269 of 269 cases.
- Dispatch library Clippy passed with all features and `-D warnings`.
