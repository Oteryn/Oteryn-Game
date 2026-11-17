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
- `src/game/game.cpp:5593-5621` converts furniture to decoration kit `23398`
  and records its original form for unwrapping. This automatic disposition path
  does not select a legacy manual-wrap kit. Cask amounts are retained in the
  wrapping metadata (`game.cpp:5593-5621`) and restored on unwrapping
  (`game.cpp:5623-5655`); wrapping never refills a partly used cask.

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
client fields are selection intents, never custody or ownership proof. Management
uses the current Channel or named-House assignment and its live actor, session
and lease generations. The origin Channel cannot authorize a command while the
actor is inside a House. Private activities do not inherit House authority:
owner decision B 2026-11-09, reference silent on Oteryn activity instances. WorldId
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

## Durable transfer terms

An offer retains its nominated Character, both Account/World parties, current
ownership revision, price, rent and date. Accepting debits price plus the pinned
rent into a retained hold. Cancelling or rejecting an accepted offer refunds the
whole hold; completion pays the seller the price and consumes the rent. Existing
bank headroom includes both possible returns and the eventual seller payment.
An accepted offer reserves the recipient's Account/World housing slot.

The offer's creation is the notice origin for the existing 24-hour to 30-day
window; accepting later does not move its agreed date. Accepted cancellation or
rejection is allowed before that date and before disposition freezes. An
unaccepted overdue offer remains cancellable/rejectable. Acceptance closes at
the effective date: owner decision B 2026-11-09, reference silent on the boundary
between a durable background job and a simultaneous acceptance.

The buyer's first paid period follows the existing HOUSE-1a acquisition policy
(`0143_house_ownership.sql:482-489`): thirty days from completion. An accepted
sale retains its Character/Account participants against ownership or lifecycle
transfer until refund or completion: owner decision B 2026-11-09, reference
silent on Oteryn's independent character-sale authority. This is a Game
participant restriction, not authority over a Platform coin balance.

Every sale transition retains Foundation event 8, kind 5, with the offer,
parties, terms and exact reserve/refund/payment/rent-consumption amounts in the
same transaction. World completion also retains the existing physical
contents-disposition event. No actor or session is fabricated for that job.

## Request

House capability 49 also owns command 6129. Version 1 is a bounded binary record
in this exact order: version byte `1`, action byte (0 catalogue, 1 move-out,
2 cancel move-out, 3 offer, 4 cancel transfer, 5 accept, 6 reject), house text,
expected property revision (u64), date (nonnegative i64 milliseconds), recipient
text, price (u64), catalogue cursor text. Integers are big endian; text is UTF-8
with a big-endian u16 byte length. No trailing bytes are accepted. House/cursor
are at most 512 bytes, recipient is a valid 2–29-byte character name. Maximum
request is 1088 bytes. Only an offer carries recipient and price; only move-out
or offer carries a date. Mutations require a selected house and nonzero revision;
a catalogue request carries only its cursor. Empty/unused fields must be zero.

## Result

Version byte `1`, server calendar milliseconds (i64), has-more byte (0/1), row
count byte (0–20), followed by ordered rows: house, display name, owner name,
recipient name (length-prefixed text), revision, date, price, rent (eight-byte
integers), transfer-state byte (0 none, 1 offered, 2 accepted), allowed-action
bitmask (bits 1–6), move-out-state byte (0–4). Display name is at most 128 bytes;
character names at most 64. Maximum result is 16384 bytes. The server qualifies
house display names from its actual catalogue. Rejection is the existing
CommandResult rejection; it never supplies invented success rows.
