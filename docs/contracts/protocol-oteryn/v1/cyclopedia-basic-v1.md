# Basic Cyclopedia map v1

Owner option A, 2026-10-23; command ids 72–73, capability 57. Oteryn defaults,
not Reference-qualified. Offered only with admitted authored Entry environs.
No new state domain. Generic CommandResult payloads carry the following bytes.

## Request

All multibyte scalars little endian. Command 72 has an empty payload. Command 73
has exactly 9 bytes: operation u8 (1 start, 2 shuffle, 3 find at server position,
4 donate), area u32 (1), amount u32 (1..10,000,000 for donate; zero otherwise).
The server never accepts a position, world, epoch, Premium flag or respawn factor.

## Result

code u8, Character revision u64, discovered bool u8, found u8, World pool u64,
reset epoch u64, respawn factor u8 (1 or 2), visible count u8, then count records
of x u16, y u16, native floor i8. At most eight records. Code 0 success, 1 invalid,
2 Premium required, 3 already discovered, 4 no active exploration, 5 insufficient
pool outside current window, 6 insufficient funds, 7 area currently improved.
Only the requesting character's unfound same-floor POIs in its viewport are sent.
Intent receipts retain the exact result, including query-time facts, for retries.
New queries read current facts; progress survives session and process replacement.

## Durable fee source

CYCLOPEDIA0 §5.3 authorizes `FeeBurnCause::AreaDonation { world, area, occurrence }`
as an amendment to the closed gold-fee source set. Native SQL cause 4 and the
append-only `OneItemFeeBurnCauseV1.area_donation` tag 4 bind the same World, area 1
and UUIDv7 occurrence to the Character receipt. The existing fee writer burns
direct backpack coins and debits the shortfall from the bank in that same
transaction. Its existing junior and activation gates remain binding.
Migration 0176 adds bidirectional fee/receipt and pool/state transaction guards.
The active bundle and authored catalogue digest pin durable interpretation; any
change fails closed until an explicit migration/product decision is implemented.

The basic server exposes position checking as intent 3, using the current committed
runtime position. Queries poll the requesting character's visible POIs. Automatic
effect/message events and client UI belong to subsequent presentation work.
