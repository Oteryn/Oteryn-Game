# ITEM-CYCLO-CAPTURE-1: Cyclopedia item data with Tibia Global compatibility marker

Reference evidence only. `content/` and `rulesets/` are unchanged; admitting these values into
item definitions is a separate content task.

## What this is

`items.jsonl` holds one record per item id. Every record is read from the official Tibia client's
Cyclopedia item dialog (Global client build `7f6555b1…af34`, see `manifest.json`) through native
client functions; no crafted packets. A record is emitted only when both parts were captured:

- `basic`: the Basic panel as label → value (Weight, Armor, Attack, Defence, Vocations,
  Required Level, Imbuement Slots, Classification, Protection, Skill Boost, Tradeable In
  Market, …).
- Item dialog values: `npc_sell_to` (NPCs that buy the item from the player), `npc_buy_from` (NPCs
  that sell it to the player), each with NPC, location, price and currency when it is not gold;
  `average_market_price`, `loot_value` with `loot_value_source`, and `has_weapon_proficiency`.

To keep the packet small, empty NPC lists, a `false` proficiency flag and zero market or loot
values are omitted. The capture session's server reported 0 for every market and loot value, so
an absent value means "not observed", not a Global market value of 0.

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
re-collection is complete (`collection_state`): 6369 records over the 6228 catalog targets (alias
ids add records). 98 targets could not be resolved through the Cyclopedia item search and have no
record.
