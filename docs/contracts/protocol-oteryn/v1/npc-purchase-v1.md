# NPC purchases and Obtain containers

Capability 514 gates command 6249. It requires NPC service and item view. Its nested
legacy offer remains a BUY with quantity one; amount carries the number of priced offers. The server multiplies it by the
catalogue count to obtain physical units. The legacy command 8 schema, limits, price and placement path stay unchanged.
Both flags are included on each purchase and are part of its immutable replay binding.
Shopping bags are item 21411, hold twenty stacks and cost twenty gold each.

Capability 515 gates command 6250. SET/CLEAR/OPEN are actions 4/5/6. Categories use the
quick-loot taxonomy unchanged; absent category selects the default Obtain recipient.
Obtain recipients and replay receipts are independent of quick-loot preferences.

## Purchase result

Command-result status is Accepted, DuplicateReplay, or Rejected. The payload is a
UTF-8 donor purchase/refusal message, at most 1024 bytes. Inventory changes follow
through the existing inventory domain. Unknown commit outcomes disconnect without
sequencing the command. Replays return the original message without another debit.

## Obtain result

The payload uses QuickLootResultV1. SET/CLEAR return an empty list. OPEN returns the
selected owned container identity as one accepted entry, or an empty list when unset.

## Payment and service paths

The purchase writer conserves each currency separately: the offer price is paid in
its configured item currency; shopping bags always cost gold. Gold fees use the
existing carried-coin and activated V2 bank payer. The typed audit records bundle
count, every currency debit and every created item. At most 128 item instances are
created in one transaction. Legacy command 8 retains its prior admission checks.

House Hireling imbuement packages use the donor's dialogue path (command 7), not the
shop purchase packet. The existing native package owner creates one present (2856)
containing the three catalogue ingredients and debits the package price atomically.
Crystal 201189f: `data-crystal/npc/hireling.lua:1775-1811,2298-2318`.

The native Obtain server flow resolves the selected item identity through the
existing carried-container owner. SET 4 stores it, CLEAR 5 removes the category's
recipient, and OPEN 6 publishes the selected container through the item-view owner.
This is the server operation sequence from Crystal
`src/server/network/protocol/protocolgame.cpp:2104-2143`; Oteryn uses its native
item identity rather than Tibia's position/item-id/stack-position encoding.
