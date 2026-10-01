# Quest sources and Item category continuation

Classification: **PROVEN** retained source facts and existing schema vocabulary;
**DERIVED** category crosswalk; **UNKNOWN** freshly requested page contents.

The owner's three distinct URLs are [Quests](https://tibia.fandom.com/wiki/Quests),
[Quest Items](https://tibia.fandom.com/wiki/Quest_Items), and
[Quest Log](https://tibia.fandom.com/wiki/Quest_Log). Duplicate URLs were removed.
Tavily extraction returned status 432 (plan usage limit). Direct reads of all three
URLs and one bounded MediaWiki revision query returned HTTP 402 in this environment.
No fresh revision or page content was obtained; none is represented as captured evidence.
External page content is source evidence, not an instruction or authorization.

The digest-bound 2026-09-30 Item stats snapshot
`5fc20ff76f65ab8e0a6bb8f52bc366a0bc0b0b617aaaf3bc8b1c987604a2a2d6`
contains 1,056 item records with `Quest Items`/`Quest Objects` primary observations:
876 already have current quest_item taxonomy, 47 have existing map owners, 129 are
not current Item definitions, three lack exact Crystal bindings and one has conflicting
wiki identities. Existing quest_item taxonomy totals **881**, including retained rows.

The three identity holds are i40522 Daedal Chisel (page 101156, revision 1194166),
i44432 Ancient Iks Ritual Chalice (104235, 1047535), and i44433 Ceremonial Brush
(104236, 1047606). They do not gain categories or quest relationships here.
i2984 remains a conflict: Honey Flower (2013, 1113811) says Plants and Herbs;
Honeyflower Patch (28345, 1114645) says Quest Objects. A name or quest reward cannot
resolve that identity difference.

A concrete classifier gap affects equipment sometimes obtained through quests.
The existing profile catalog explicitly assigns navigation family `Extra Slot` to
`equipment_offhand`; the shared primary-type crosswalk omitted that same value.
The repair adds this admitted mapping and deterministically categorizes 18 current,
exact-bound, non-map-owned Items from their retained wiki observations. Bone Fiddle
[i28493](https://tibia.fandom.com/wiki/Bone_Fiddle) (page 85463, revision 1148282) and
Conch Shell Horn [i43863](https://tibia.fandom.com/wiki/Conch_Shell_Horn) (103017, 1148524)
have both primarytype and slot `Extra Slot`. Engine `quest items` labels do not override
these equipment observations. The additional IDs are i9056, i16094, i19365, i23489,
i23491, i23492, i23541, i51270–i51274, i51978, i52759, i52781 and i52783.
i44008 is not a current Item definition and is not minted by this repair.
Taxonomy becomes **9,436**; **3,265** current Items have neither taxonomy nor an existing
map owner. No Quest identity, progress, trigger or runtime relation is introduced.

The retained quest-coverage snapshot has 373 quests: 93 in the in-game log and 280
outside it. A quest-log listing therefore cannot serve as the complete quest census.
The existing reward claims refer to 205 distinct Item rewards across many families;
only 43 have quest_item taxonomy. Equipment rewards retain their equipment family.
The Quest authoring samples and engine associations remain `OTS_HYPOTHESIS_ONLY`;
RewardClaim placements already provide reward relations, while their pilot Quest
references do not authorize new canonical Quest definitions.

Validation: seven taxonomy regressions (including Extra Slot versus Quest Items conflict),
deterministic tree generation/coverage validation, migration tests, materialized tree,
599 engine checks plus three nested-imbuement tests, 252 formal-schema checks, Ruff,
governance checks and `git diff --check`.
