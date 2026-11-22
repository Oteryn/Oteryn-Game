# Quick loot destinations

Capability 607 requires capabilities 4 and 98 and gates commands 6376 and 6377.
Command 114 retains actions 1–7 and their existing results. Each new action is
valid only on its registered command. Requests use the tagged field layout in
[quick_loot_v1.proto](quick_loot_v1.proto).

## Manage destinations (6376)

- Action 8 removes a Loot assignment. Field 3 selects a category. Omission
  resets the Unassigned Loot destination to the main backpack.
- Action 9 reviews saved destinations. It takes only field 1.
- Action 10 assigns an Obtain destination. Field 3 is optional for the default
  Obtain destination, field 4 selects the recipient kind, and field 5 carries
  the item instance UUID for a container or gold pouch.
- Action 11 removes an Obtain assignment with the same category convention
  as action 10.
- Actions 12 and 13 set one recipient for all five categories and the default
  destination, for Loot and Obtain respectively. Omit field 3. Action 12
  uses field 6 as the new Loot fallback preference.

The five categories and three recipient kinds retain their existing wire
numbers. Other fields are forbidden. Premium is required for category-specific
Loot management, bulk assignment and Obtain changes. The existing carried
container and gold-pouch checks apply. Writes use the current Character item
fence. An accepted mutation has an empty result. Refusals are empty.

An accepted action 9 result starts with a one-byte Loot fallback flag (0 or 1).
It then has the Loot and Obtain sections in that order. Each section starts
with a count byte (0–6) followed by rows in ascending category order. Each row
has a category byte (0 means default), a recipient kind byte, and the 16-byte
item instance UUID for kinds 1 and 2. There is always a Loot default row. An
absent Obtain row means no saved assignment. The query reveals only the current
Character's preferences, including saved items that may no longer be carried.

## Distribute contents (6377)

Action 14 supplies the carried source container instance UUID in field 2.
Current Premium and custody are required. The server considers direct contents
in placement order. It resolves each item's category from current content and
looks up the category's Obtain destination, then the default Obtain destination
if no category assignment exists. Entries with no valid destination or no room
remain in place. Nested containers participate as single items. Normal custody,
cycle, capacity and gold-pouch restrictions apply. Moves and their replay
receipt commit together. An accepted result uses `QuickLootResultV1` with a row
for each considered item. A refused command has an empty result.

[Crystal managed-container operations](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L2104-L2142).
