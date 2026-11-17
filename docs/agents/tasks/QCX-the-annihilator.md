# QCX39 · The Annihilator Quest

Base `98d09d5ce17df2a621df3fac9c2ce341dccc073b`.

## Sources

- The direct MediaWiki API request returned HTTP 402. The repository retains reduced API facts in `tools/content-schema/quest-authoring/wiki_quest_facts.json` and the quest definition records both source hashes. These are the captured [base revision](https://tibia.fandom.com/index.php?curid=2520&oldid=1090717) and [spoiler revision](https://tibia.fandom.com/index.php?curid=2527&oldid=1090718).
- The reachable TibiaWiki BR article supplies the walkthrough, item choices, achievement and Avar Tar outfit route. [TibiaWiki BR revision](https://www.tibiawiki.com.br/index.php?curid=4522&oldid=401062)
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8` supplies IDs, positions and NPC prompt details. The retained artifacts cite earlier donor snapshots, so the corresponding files were read at the requested revision. [Crystal tree](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8/data-global)

## Comparison

| Part | Source and current base |
| --- | --- |
| Requirements | The definition records Premium and level 100. The native outer expertise door and the lever enforce level 100, and the lever requires four participating characters. The shared encounter record has no independent Premium predicate for the four participants. |
| Lever and arena | The captured placement uses UID 30025 and lever 2772 → 2773. The active shared-access callback uses the four donor tile pairs, rejects an occupied room and spawns six Angry Demons. Two block the passage. Existing continuous-route tests cover the combat and reward-room path. Crystal global lever lines 3–62 and 83–84. |
| Chests | UIDs 6085–6088 on appearance 2472 at (33227/33229/33231/33233, 31656, 13) offer one Demon Armor (3388), one Magic Sword (3288), one Stonecutter Axe (3319), or one Present Box (2856) containing one Annihilation Bear (3213). All four share the once-per-character claim. Crystal chest table lines 1233–1263. The retained definition recipe describes only the Magic Sword choice and retains its historical holds. The active claim and captured placement records hold all four choices. |
| Achievement and XP | The active chest grants the Annihilator achievement. Crystal's global chest table has no achievement or XP field for these rewards. TibiaWiki BR confirms the achievement and lists no quest XP grant. Creature combat XP is separate. |
| Exit and access | Captured route exits to (33213, 31671, 13). The shared access callback binds AID 10102 and appearances 5113/5114 to the shared claim. Crystal door lines 3–24 checks numeric reward storage equal to 1, while the native gate uses claim existence and stays closed after the outfit callback changes its track to 2. |
| Outfit | Avar Tar's existing base callback requires reward track 1, advances it to 2 and grants the Demon base. Crystal lines 98–108 use look types 541/542. Separate Demon Helmet and Demon Oak quests own the addons. The generated `outfit` and `yes` offers both return the same generic choice prompt. Crystal lines 56–75 have distinct topic-specific replies. The base reward callback uses a generic acknowledgment and has no authored unmet-prerequisite reply. |
| Identity and source status | The shared-access records use `oteryn:quest.the_annihilator`, while the definition and outfit callback use `oteryn:quest.the_annihilator_quest`. The encounter generator currently verifies the former literal. The held or unlowered status in the import records does not describe the native route already exercised by the existing route tests. |

## Open facts and remaining work

- OPEN-FACTS Global reset time and repeat-entry behavior once the reward or outfit has been claimed. Crystal enables daily mode at lever lines 4–5 and refuses moving the lever back at lines 63–78. Global evidence did not establish a reset schedule or the post-outfit door behavior.
- OPEN-FACTS Whether the live Global admission check enforces Premium at the lever itself in addition to Premium access to Edron.
- PARTIAL Captured Fandom facts could be checked offline. Fresh Fandom bodies and full English spoiler text remain unavailable.
- PARTIAL Validate the Avar Tar prompt regression red and green, then record the exact-head check results.
