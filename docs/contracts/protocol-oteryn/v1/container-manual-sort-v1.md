# Container manual sort v1

Capability 510 `CONTAINER_MANUAL_SORT_V1` is Tibia Global's Manual Sort Mode: a moved item is
placed in the container slot the player chose instead of the first slot. It requires container
tree capability 14 and extends only its command type 9 destination and the deposit of depot
command type 36 (capability 37); it grants no command or domain authority of its own.

With 510 selected, `ContainerDestinationV1` (`item_view_v1.proto`, `ItemMoveIntentV1` field 5)
may carry field 2 `slot` (uint32, explicit presence). The slot is the index the moved item takes
in the destination container's view order (domain 11 entries, newest first). Absent, the item
takes the first slot as before. Without 510 the client never sends field 2 and the server
rejects it as malformed, so a session without 510 keeps the earlier bytes and behaviour. A
repeated slot field fails closed. The command-9 payload bound under 510 is 32 bytes
(`MAX_ITEM_MOVE_INTENT_MANUAL_SORT_BYTES`): the source handle, a slotted container destination
and the count of capability 61.

With 510 selected, a deposit `DepotIntentV1` (`depot_v1.proto`) may carry field 6 `slot`
(uint32, explicit presence): the index the deposited entry takes in the box's view order (newest
first; page `p` shows slots `32p` to `32p + 31`). Only a deposit carries it; without 510 it is
malformed. The slot is part of the depot request digest, so a replay with another slot conflicts.

The server applies the slot to a carried container (the main backpack included) of the moving
Character, to a container of the current Named House reached through the fenced House loop
owner, and to a depot box. The existing item loop owner commits the move first, with its unchanged
stacking, merge, capacity and session-generation fence rules. When the move placed a new entry
in the destination (a move, split or pickup, not a merge into an existing stack), the same
transaction rotates the destination's existing placement ordinals so the entry lands at the
slot. A slot at or beyond the container's last entry places it last. The rotation is recorded as
a container sort receipt under the move's CommandRef, so the existing ordinal triggers admit
exactly that permutation; no other entry changes its container and no item state changes.
The move receipt's intent includes the slot, so a replay with a different slot is rejected and
a replay with the same slot returns the original result. Migration 2856 extends the receipt's
custody arm: a House entry is admitted only under the House loop proof that the same command wrote
in the same transaction with the receipt's exact session, connection, lease and scope generations,
and a depot entry only in the town and box of the same command's depot move receipt of that
transaction; the depot box's ordinals are rotated the same way, under a deferred uniqueness
constraint and an ordinal-only update trigger. One receipt holds at most 500 entries, so a depot
slot beyond 499 places the entry at slot 499.

The client toggles the mode with the `ToggleManualSortMode` hotkey action (Tibia Global default
Shift+S) for the current session; while it is on, container titles render orange and a drop on a
container slot sends that slot.
