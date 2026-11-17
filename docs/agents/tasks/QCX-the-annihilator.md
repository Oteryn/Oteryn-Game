# QCX39 · The Annihilator Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`.

## Sources

- The workspace MediaWiki API request returned HTTP 402. The user supplied both API wikitext captures from 2026-11-17 in `qcx-facts/the-annihilator.md` at `460e1d8c2c0e88c438ae5e54910db00822dc0129`. These are the [base revision](https://tibia.fandom.com/index.php?curid=2520&oldid=1090717) and [spoiler revision](https://tibia.fandom.com/index.php?curid=2527&oldid=1090718).
- The reachable TibiaWiki BR article supplies the walkthrough, item choices, achievement and Avar Tar outfit route. [TibiaWiki BR revision](https://www.tibiawiki.com.br/index.php?curid=4522&oldid=401062)
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies IDs, positions and NPC prompt details. The retained artifacts cite earlier donor snapshots, so the corresponding files were read at the requested revision. [Crystal tree](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global)

## Comparison

| Part | Source and current base |
| --- | --- |
| Requirements | The definition records Premium and level 100. The native outer expertise door and the lever enforce level 100, and the lever requires four participating characters. The sealed approach door denies characters with the shared claim, which enforces the Global unfinished-player rule along the mapped route. The lever has neither a separate claim predicate for the four station occupants nor an independent Premium predicate. |
| Lever and arena | The captured placement uses UID 30025 and lever 2772 → 2773. The active shared-access callback uses the four donor tile pairs, rejects an occupied room and spawns six Angry Demons. Two block the passage. The native Angry Demon profile already registers Fire Elemental summons at 10% every 2,000 ms, with a maximum of one, matching Crystal Angry Demon lines 57–62. Existing continuous-route tests cover the combat and reward-room path. The native route lacks the Global ten-minute transfer of remaining arena occupants to the reward room. Crystal global lever lines 3–62 and 83–84. |
| Chests | UIDs 6085–6088 on appearance 2472 at (33227/33229/33231/33233, 31656, 13) offer one Demon Armor (3388), one Magic Sword (3288), one Stonecutter Axe (3319), or one Present Box (2856) containing one Annihilation Bear (3213). All four share the once-per-character claim. Crystal chest table lines 1233–1263. The retained definition recipe describes only the Magic Sword choice and retains its historical holds. The active claim and captured placement records hold all four choices. |
| Achievement and XP | The active chest grants the Annihilator achievement. Crystal's global chest table has no achievement or XP field for these rewards. Fandom and TibiaWiki BR confirm the achievement and list no quest XP grant. Creature combat XP is separate. |
| Exit and access | Captured route exits to (33213, 31671, 13). The shared access callback binds AID 10102 and appearances 5113/5114 to the shared claim. Crystal door lines 3–24 checks numeric reward storage equal to 1, while the native gate uses claim existence and stays closed after the outfit callback changes its track to 2. |
| Outfit | Avar Tar's existing base callback requires reward track 1, advances it to 2 and grants the Demon base. Crystal lines 98–108 use look types 541/542. Separate Demon Helmet and Demon Oak quests own the addons. The generated `outfit` offer matches the complete command, so it misses `demon outfit`. Crystal matches the word `outfit` within that phrase (Avar Tar line 57 and NPC helper lines 13–20). The generated topic prompts and base reward reply differ from the donor. The base callback also omits the blue magic cue and unmet-prerequisite reply from Crystal lines 98–108. |
| Identity and source status | The shared-access records use `oteryn:quest.the_annihilator`, while the definition and outfit callback use `oteryn:quest.the_annihilator_quest`. The encounter generator currently verifies the former literal. The held or unlowered status in the import records does not describe the native route already exercised by the existing route tests. Fandom places the quest in The Ultimate Challenges log while the imported definition sets `shown_in_quest_log` to false. |

## Blockers and remaining work

- BLOCKER Annihilation room forced exit timed group transfer to reward room data-global/scripts/quests/the_annihilator/lever.lua:47-80. Fandom specifies 600 seconds and the donor omits the transfer. Main's timed encounter path rejects party stations and only schedules spawn and cleanup.
- OPEN-FACTS Exact Global coordinates for the ten-minute fallback landing and the reset schedule. Global confirms one-time treasure access and requires an unfinished party.
- OPEN-FACTS Whether the live Global admission check enforces Premium at the lever itself in addition to Premium access to Edron.
- PARTIAL Audit direct station defense in depth, the quest-log projection, historical import status and recipe, and the remaining NPC presentation and refusal differences.
- PARTIAL Run the Avar Tar phrase regression red and green and record the check results.
