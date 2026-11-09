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

## Transfer selection and confirmations

Canary `99902524`, `src/server/network/protocol/protocolgame.cpp:12197-12249`
selects a house and exposes schedule move-out, cancel move-out, offer transfer,
cancel transfer, accept transfer and reject transfer. A transfer offer carries
recipient name, effective date and gold price. `src/game/game.cpp:13124-13339`
requires the current owner for scheduling/cancellation and the nominated
recipient for acceptance/rejection; accepting reserves the price plus rent from
the recipient's bank. Cancelling or rejecting an accepted transfer refunds both.
These are Game gold, not purchased coins. The existing Game bank contract owns
these writes; no commercial balance or StoreInbox is involved.

Oteryn retains its accepted bounded move-out scheduling window. Every mutation
requires an explicit confirmation displaying the exact house and action; offers
also display recipient, date and price, and acceptance displays price plus rent.
Refreshing/reselecting a different revision invalidates an old confirmation.
The house list includes current owned houses and incoming transfer offers, with
server-authoritative names, terms and allowed actions. No free-form house key or
client-supplied ownership fact is an authority grant.

For disposition, the existing `game_item_bindings.character_id` is the explicit
item owner. Placement/reclaim provenance alone is not an owner binding. Personal
house disposition uses that binding when present and otherwise the seller;
Guild disposition retains its separately accepted reclaim-subject rules.
`CharacterInbox` / `game_item_character_inbox_locations` is the accepted depot
mail inbox (DEPOT-0/INBOX-1); it is distinct from the Store inbox.

`src/creatures/players/player.cpp:12947-13017` rejects transfer to oneself,
a missing recipient, a recipient without a vocation, a non-premium recipient,
and an account already bidding on a house. Acceptance rechecks the nominated
recipient, premium and vocation, available price-plus-rent balance and whether
the offer was already accepted. Existing Oteryn Account/World housing-slot,
ban and bank limits continue to apply; Platform remains premium authority.
