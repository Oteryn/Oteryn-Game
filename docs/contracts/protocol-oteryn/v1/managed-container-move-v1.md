# Managed container moves v1

Capability 602 requires item views (4) and quick-loot assignments (98).
Bundle Worlds offer command 6365. Item move, container sort and quick-loot
commands keep their existing formats. The server reads the currently
admitted Character's existing managed-container category and unassigned
targets. The request never carries a destination or a category override.

## Request

- Preview is ten bytes `00 <recursive 0 or 1> <source u64 big endian>`
  for a positive session item handle resolving to a currently carried bag.
- Confirm is nine bytes `01 <preview CommandId u64 big endian>`. Send it
  only after approval. For recursive previews show that nested contents
  are included. Discard the preview to cancel.

The preview selects direct entries, or eligible entries throughout the
carried subtree when recursive is set. It skips containers themselves,
unassigned items with no managed target, and stacks already in their
assigned bag. The existing carried-tree bounds apply (500 entries and 8
descendant levels). Both steps require current Premium and item-service
eligibility. Confirm consumes the last preview from this connection and
revalidates the application plan under the same exact item fence. A stale
preview, placement or assignment, unavailable capacity, or an ineligible
source refuses the whole batch.

## Reply

Exactly five bytes on success, a tag followed by a positive unsigned
32-bit big-endian stack count. Tag 0 is preview and tag 1 is committed.
Preview makes no custody changes. A refusal is REJECTED with an empty
payload and the client text “Sorry, not possible.” A committed command
refreshes the existing item views. The batch and all child move receipts
commit together. Retries of the original confirm command reconcile the
retained result using the Character, World, CommandRef and preview ID.

The Tibia controls manual (§4.1.10) describes routing nested items to
configured Obtain containers. Crystal 201189f8 `protocolgame.cpp:2125`
and `game.cpp:3361` establish server-owned managed targets. Oteryn uses
its existing managed-container assignment owner for this operation.
