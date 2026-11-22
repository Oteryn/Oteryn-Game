# Whole-container stash confirmation v1

Capability 601 requires item views (4) and stash (62). Bundle Worlds offer
command 6364 to negotiated clients. Command 79 and domain 27 keep their
existing formats and single-stack behavior.

## Request

- Preview is ten bytes `00 01 <source u64 big endian>`, where source is a
  positive session item handle for an owned, carried container.
- Confirm is nine bytes `01 <preview CommandId u64 big endian>`. Zero IDs,
  unknown tags or flags and extra bytes are rejected. Send confirmation only
  after the player approves the shown quantity. Discard the preview to cancel.

The last valid preview belongs to the current connection and exact item fence.
Confirmation consumes it and revalidates the same current eligible stack
identities, quantities and placements under the item lane and Character fence.
Both steps require ordinary locker reachability. The confirmed transaction
retires plain, marketable, stowable stacks recursively and leaves containers
and ineligible items in place. The existing carried-tree bounds apply (500
entries and 8 descendant levels). A refused conversion rolls back every stack.

## Reply

An accepted reply contains five bytes, a tag followed by a positive unsigned
32-bit big-endian unit count. Tag 0 means preview and tag 1 means committed.
Preview never writes item or stash value. A refusal is REJECTED with an empty
payload and the client text “Sorry, not possible.” A committed command causes
the existing item views to refresh. Re-browse command 79 to reload stash rows.

After uncertain delivery, the normal command replay uses the retained batch
receipt, bound to the Character, World, CommandRef and original preview ID.
It does not convert anything twice. Once a connection ends, an unused preview
cannot authorize a new command.

Crystal 201189f8 implements container stow in `protocolgame.cpp:10242`,
`player.cpp:9604` and `container.cpp:169`. It recursively collects eligible
stacks and preserves the bags. The Oteryn confirmation binds that operation to
the contents the player approved.
