# House contents disposition v1

Authority: owner decision B, 2026-11-09, T135. This contract defines required
behavior; its presence does not assert that T135 is implemented or deployable.

## Reference

Canary commit `99902524`, `src/map/house/house.cpp`:

- `tryTransferOwnership`, lines 72–77, calls `transferToDepot(player)`.
- `transferToDepot(player, tile)`, lines 308–360, collects pickupable items and
  container contents, wraps wrapable furniture first, and leaves non-movable
  fixtures in place.
- `handleWrapableItem`, line 440, calls `g_game().wrapItem`; contents of wrapped
  containers are collected rather than discarded.
- Lines 340–349 route an owner-bound item not owned by the seller to the item's
  owner.
- Line 356 moves the item to `targetPlayer->getInbox()`. This is the depot mail
  inbox. `getStoreInbox()` is not used by this reference file.
- Failed moves are logged and leave the item in its current location.

## Required Game behavior

Ownership transfer and move-out use the same contents-disposition rules.
Wrap furniture using its admitted wrap conversion before relocating it. Preserve
item identity, quantities and container contents. Collect pickupable items and
container contents; leave fixed fixtures in place. Route ordinary movable items
to the seller's depot inbox and owner-bound items to their owner's depot inbox.
Never route disposition items to StoreInbox. Never delete an item because its
wrap or move failed, and never report such an item as delivered.

The buyer receives the house without the seller's movable contents. A failed
relocation remains retryable with the source item intact. Do not release a house
with undisposed movable contents: owner decision B 2026-11-09, reference silent
on reconciling a durable asynchronous handoff with the empty-house requirement.

## Authority and confirmation

The server resolves the current owner, item owner, World and session generation;
client fields are selection intents, never custody or ownership proof. WorldId
and ChannelId remain distinct. Replays cannot move, wrap or hand over twice.
Persist item effects, custody evidence and progress consistently, so retries and
restarts cannot duplicate or lose contents.

The client must show the selected house, recipient and effective transfer or
move-out terms before confirmation. Confirmations bind to that exact selection;
changing it invalidates confirmation. Wrapping confirms the selected item and
its admitted wrapped form. Closing or refusing confirmation emits no mutation.
