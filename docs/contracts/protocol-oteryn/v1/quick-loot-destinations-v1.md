# Quick loot destinations

Capability 607 requires capabilities 4 and 98 and gates commands 6376 and 6377.
Command 114 retains actions 1–7 and the existing results for requests without a selected item. Each new action is
valid only on its registered command. Requests use the tagged field layout in
[quick_loot_v1.proto](quick_loot_v1.proto).

## Manage destinations (6376)

- Action 8 removes a Loot assignment. Field 3 selects a category. Omission
  resets the Unassigned Loot destination to the main backpack.
- Action 9 reviews saved destinations. Optional request field 10 opts in to the saved filter settings.
- Action 10 assigns an Obtain destination. Field 3 is optional for the default
  Obtain destination, field 4 selects the recipient kind, and field 5 carries
  the item instance UUID for a container or gold pouch.
- Action 11 removes an Obtain assignment with the same category convention
  as action 10.
- Actions 12 and 13 set one recipient for every category and the default
  destination, for Loot and Obtain respectively. Omit field 3. Action 12
  uses field 6 as the new Loot fallback preference.
- Action 15 assigns an individual Loot destination with the same fields as
  legacy action 3, including the fallback preference, and supports every
  category below.

The three recipient kinds and legacy categories retain their wire numbers.
Management uses 1 Gold, 2 Creature Product, 3 Equipment, 4 Consumable,
5 Other, 6 Armor, 7 Necklace, 8 Boots, 9 Containers, 10 Decoration, 11 Food,
12 Helmet, 13 Legs, 14 Potion, 15 Ring, 16 Rune, 17 Shield, 18 Tools,
19 Valuables, 20 Ammunition, 21 Axe, 22 Club, 23 Distance Weapon, 24 Sword,
25 Wand, 26 Quiver and 27 Fist Weapon. Category assignments derived from
specific item semantics take priority over a matching broad category.
The default applies when neither category is assigned. Command 114 accepts
only categories 1–5 on assignment. Commands 6376 and 6377 accept an item reference as a 16-byte instance UUID
or an eight-byte little-endian nonzero handle from the current session item
view. The server resolves a handle at ingress and proves current custody for
either reference. Command 114 uses the session-local handle in field 9 for a selected Click.
Other fields are forbidden. Premium is required for category-specific
Loot management, bulk assignment and Obtain changes. The existing carried
container and gold-pouch checks apply. Writes use the current Character item
fence. An accepted mutation has an empty result. Refusals are empty.

An accepted action 9 result starts with a one-byte Loot fallback flag (0 or 1).
It then has the Loot and Obtain sections in that order. Each section starts
with a count byte (0–28) followed by rows in ascending category order. Each row
has a category byte (0 means default), a recipient kind byte, and the 16-byte
item instance UUID for kinds 1 and 2. There is always a Loot default row. An
absent Obtain row means no saved assignment. The query reveals only the current
Character's preferences, including saved items that may no longer be carried.

When request field 10 is true, a tagged suffix follows the destination sections.
Tag 1 is the active filter mode (varint, 1 Skip or 2 Accept). Tags 2 and 3
are the Skip and Accept lists, respectively, as packed little-endian fixed32
Item definition references in the admitted index. References are nonzero,
ascending and unique, with at most 2048 per list. Empty lists omit their tags.
Entries absent from the admitted index stay persisted for future generations.
Omitting request field 10 preserves the original reply bytes.

For a Gold Pouch assignment on legacy command 114, field 5 also accepts the
Store Inbox fulfilment UUID. The server resolves it within the current Character, account and World
and applies the existing Gold Pouch and Inbox custody checks. A durable item
instance UUID retains precedence and saved destinations use that identity.

## Distribute contents (6377)

Action 14 supplies the carried source container reference in field 2.
Current Premium and custody are required. The server considers direct contents
in placement order. It resolves each item's category from current content and
looks up the specific or broad category's Obtain destination, then the default
Obtain destination if no category assignment exists. Entries with no valid destination or no room
remain in place. Nested containers participate as single items. Normal custody,
cycle, capacity and gold-pouch restrictions apply. Moves and their replay
receipt commit together. An accepted result uses `QuickLootResultV1` with a row
for each considered item. A refused command has an empty result.

[Crystal category numbering and classification](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/enums/object_category.hpp).
[Crystal managed-container operations](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/server/network/protocol/protocolgame.cpp#L2104-L2142).

[Tibia manual, Sorting Containers and Manage Containers](https://www.tibia.com/gameguides/?subtopic=manual&section=controls).
