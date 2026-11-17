# NPC purchases and Obtain containers

Capability 514 gates command 6249. It requires NPC service and item view. Its nested
legacy offer remains a BUY with quantity one; amount carries the physical purchase
quantity. The legacy command 8 schema, limits, price and placement path stay unchanged.
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

## Current admission boundary

The new purchase writer admits ordinary gold-priced catalogue offers with count one,
with at most 128 created item instances per transaction. Non-gold currencies, bundled
offers, and House Hireling package purchases remain refused by this new command.
The existing command 8 owner retains its prior admission and behavior.

Obtain SET/CLEAR/OPEN have a typed client session adapter and real server routes.
A native Obtain recipient selector is not rendered yet.
