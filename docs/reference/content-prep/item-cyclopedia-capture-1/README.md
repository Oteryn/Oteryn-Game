# ITEM-CYCLO-CAPTURE-1: Cyclopedia item data with Tibia Global compatibility marker

Reference evidence only. `content/` and `rulesets/` are unchanged; admitting these values into
item definitions is a separate content task.

## What this is

`items.jsonl` holds one record per item id. Every record is read from the official Tibia client's
Cyclopedia item dialog (Global client build `7f6555b1…af34`, see `manifest.json`) through native
client functions; no crafted packets. A record is emitted only when both parts were captured:

- `basic`: the Basic panel as label → value (Weight, Armor, Attack, Defence, Vocations,
  Required Level, Imbuement Slots, Classification, Protection, Skill Boost, …), and `basic_text`,
  the same panel as the client renders it.
- Item dialog values: `npc_sell_to` (NPCs that buy the item from the player), `npc_buy_from` (NPCs
  that sell it to the player), each with NPC, location, price and currency,
  `average_market_price`, `loot_value`, `loot_value_source` and `has_weapon_proficiency`.

`average_market_price` and `loot_value` were captured on a test-server session that reports 0;
treat 0 there as "not observed", not as a Global market value.

## Compatibility marker

`compatibility.target` is `tibia-global` for every record. `compatibility.verdict`:

| Verdict | Meaning |
|---|---|
| `CLIENT_VERIFIED_WIKI_AGREES` | Every Basic field TibiaWiki also states matches the client. |
| `CLIENT_VERIFIED_WIKI_DIFFERS` | At least one field differs; `wiki_cross_check.differ` names it. The client value is the observed Global value; the wiki page is usually outdated. |
| `CLIENT_VERIFIED_NO_WIKI_PAGE` | No TibiaWiki infobox carries this item id. |

`wiki_cross_check` lists the compared fields as `match`, `differ`, `only_client` and `only_wiki`.
The cross-check covers Weight, Armor, Attack, Defence, Required Level, Imbuement Slots,
Classification, Vocations, Protection, Skill Boost, Total Charges, Capacity and Required Magic
Level. `details_from_alias_id` is set when the Basic panel was read through the item's Cyclopedia
alias id.

## State

`manifest.json` records the count, verdict totals and the SHA-256 of `items.jsonl`. The native
re-collection was still running when this packet was cut (`collection_state`); later pushes replace
`items.jsonl` and `manifest.json` with the larger set.
