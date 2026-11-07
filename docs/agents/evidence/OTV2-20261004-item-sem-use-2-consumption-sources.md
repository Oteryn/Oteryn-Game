# ITEM-SEM-USE-2 consumption sources

Task `OTV2-20261004-item-sem-use-2-food-and-potion-content`, governing Issue #1622. Scope: packet
2026-10-04 section 1.5, **widened by the owner on 2026-10-13**: every edible food with a source
value, every health/mana/spirit restore potion and every empty flask (the 11 potions and 3 flasks of
the packet are a floor, not a ceiling).

## Sources

- **TibiaWiki** (Fandom API, revision-pinned in `OTV2-20261004-item-consumption-sources-v1.json`):
  `Category:Food` and `Category:Potions` pages. Used for `regenseconds`, `stackable`, the item id and
  name, and the three empty-flask pages (`Empty Potion Flask (Small|Medium|Large)` = i285 / i283 /
  i284), whose notes list the potions each size holds.
- **Canary 04b83b51** `data/scripts/actions/items/foods.lua` (food value x 12 s) and
  `data/scripts/actions/items/potions.lua` (ranges, level, vocations, flask), fallback only. A
  Canary-only value is `OTS_HYPOTHESIS_ONLY`.
- Canary `data/items/items.xml` (pinned copy `imports/canary/items-xml`, sha256 1cf2992c...) records
  the Canary disposition on stackability: it is silent, so TibiaWiki `stackable` decides the shape.

## Findings that differ from the packet text

- **Potion ranges are Canary-only.** TibiaWiki has no structured range per potion page, so every
  potion range, level and vocation set is `OTS_HYPOTHESIS_ONLY`. Levels are cross-checked against the
  TibiaWiki `Potions` page for the nine potions it names; a mismatch would be reported, none occurred.
  TibiaWiki prose omits Monk from some sentences that predate it; Canary's vocation sets win.
- **The health potion (i266) leaves the small flask i285, not i283.** TibiaWiki (flask pages) and
  Canary agree; the packet acceptance line naming i283 follows neither. The sources win.

## Counts

- Food lowered: 137 (by source: {'TIBIAWIKI': 137}).
- Potions lowered: 12 (2 with Health and Mana entries).
- Items admitted (v3 shape): 147.
- Reported and not lowered: 52.

| Reason | Rows |
| --- | --- |
| `FOOD:no_item_record` | 4 |
| `FOOD:no_wiki_page_for_stack_shape` | 1 |
| `FOOD:non_positive_value` | 28 |
| `FOOD:regeneration_1200_seconds_or_more` | 1 |
| `FOOD:stack_conflict` | 1 |
| `FOOD:wiki_name_mismatch` | 6 |
| `POTION:attribute_or_condition_potion` | 7 |
| `POTION:no_range_source` | 4 |

`non_positive_value` rows have `regenseconds = 0` on TibiaWiki. The attribute, transform and
condition potions and the antidote (`attribute_or_condition_potion`) need a condition grammar that
does not exist; they belong to the separate follow-up lane CONSUMABLE-CONDITIONS-1. Rows are listed
with item id and title in the facts packet (`reported`).

## Reproduce

`python3 tools/content-schema/item-authoring/lower_item_consumption_packet.py --check` (offline, from
the sources snapshot); `--capture` refreshes the snapshot from the network.
