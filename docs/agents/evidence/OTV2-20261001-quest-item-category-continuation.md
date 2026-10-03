# Quest sources and Item category continuation

Classification: **PROVEN** retained source facts and existing schema vocabulary;
**DERIVED** category crosswalk; **PROVEN** fresh public browser reads;
**UNKNOWN** identity correspondence for the nine additional candidates.

The owner's three distinct URLs are [Quests](https://tibia.fandom.com/wiki/Quests),
[Quest Items](https://tibia.fandom.com/wiki/Quest_Items), and
[Quest Log](https://tibia.fandom.com/wiki/Quest_Log). Duplicate URLs were removed.
Tavily extraction returned status 432 (plan usage limit). Direct reads of all three
URLs and one bounded MediaWiki revision query returned HTTP 402 in this environment.
After those failures, the owner-authorized internet-only Remote Desktop fallback read
all three pages through real Chrome/CDP on 2026-10-01. No remote project edits,
publication, builds, installation or administrative actions were performed.
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

## Fresh browser evidence and identity holds

These public DOM captures were obtained through **Remote Desktop + Chrome/CDP**,
not ordinary HTTP or Tavily. Full extracted main text was captured without truncation.
The SHA-256 values identify the decoded JSON captures retained in this session's
`/workspace/audit-continuation/remote-browser-evidence/` directory; they are not
raw HTML hashes or additions to the older digest-bound Item snapshot.

| Source | Page / revision | Retrieved UTC | Capture SHA-256 |
|---|---|---|---|
| Quests | 2352 / 995013 | 2026-10-01 12:59:40.618 | f66bda91a42ac540cb56fee1542b88560e1a1cf75ba4317684faab1cad27ae4d |
| Quest Items | 6294 / 895990 | 2026-10-01 12:56:10.687 | 65cb928870c89ec6e7bb11095a634f766d40caa22487c4a777d3a86ac2450703 |
| Quest Log | 12151 / 1114335 | 2026-10-01 12:56:10.759 | faecd022d46e70cb5bf4711e1824a5845d04fc047f4aad8caf4202175be4b872 |

Quest Items displays a dynamically transcluded list of **1,169** positions. The index
article revision alone does not pin the child articles or identify numeric Item IDs.
The fresh Quest Log explicitly says that it does not display all quests: multipart
missions appear after starting, repeatable quests need not be marked complete,
and reward chests generally are not listed. These pages corroborate the coverage
boundary; index membership does not establish a canonical Item category or identity.

Nine name matches among the 3,265 uncategorized candidates were then checked against
raw public article revisions via the same browser's anonymous MediaWiki API (HTTP 200).
Capture `nine-item-revisions.json`, retrieved 2026-10-01 13:07:05.969 UTC, SHA-256
`3a6a26969f19e39a8d092f59877d7ca22faa2e9870eb68bdbe3626a16f824408`.

| Candidate | Article | Page / revision | Raw wiki itemid | Existing raw-ID category |
|---|---|---|---|---|
| i908 | [Shapeshifter Ring](https://tibia.fandom.com/wiki/Shapeshifter_Ring) | 51266 / 1194074 | 907 | equipment_offhand |
| i5952 | [Poem Scroll](https://tibia.fandom.com/wiki/Poem_Scroll) | 8758 / 1114234 | 6119 | document |
| i22739 | [Mysterious Metal Egg](https://tibia.fandom.com/wiki/Mysterious_Metal_Egg) | 76606 / 1115536 | 19065 | quest_item |
| i28865 | [The Spatial Warp Almanac](https://tibia.fandom.com/wiki/The_Spatial_Warp_Almanac) | 85266 / 1115854 | 28853 | document |
| i30082 | [Blue Ectoplasm](https://tibia.fandom.com/wiki/Blue_Ectoplasm) | 86827 / 1115911 | 30203 | quest_item |
| i30083 | [Green Ectoplasm](https://tibia.fandom.com/wiki/Green_Ectoplasm) | 86825 / 1115909 | 30204 | quest_item |
| i30084 | [Red Ectoplasm](https://tibia.fandom.com/wiki/Red_Ectoplasm) | 86826 / 1115910 | 30205 | quest_item |
| i32757 | [Luminescent Crystal](https://tibia.fandom.com/wiki/Luminescent_Crystal) | 89736 / 1116054 | 32567 | quest_item |
| i36586 | [Old Parchment](https://tibia.fandom.com/wiki/Old_Parchment) | 6007 / 1114145 | 4831 | document |

All nine remain **identity holds**. Exact Crystal bindings and approved aliases point
each raw wiki ID to the canonical Item with that same ID, never to these candidates.
No retained wiki page binding or raw client/legacy ID establishes the proposed translation.
`item_wiki_stats_capture.record_key/rekey` adds a namespace prefix without translating IDs.
Official appearances show an active variant relationship for i908 to i907 and expiry
flags for the candidate ectoplasms; those facts do not establish an identity alias.
No additional taxonomy rows or charge/duration facts are copied between these variants.
The existing 9,436 taxonomy rows, 881 quest_item rows and 3,265 coverage gap remain unchanged.
