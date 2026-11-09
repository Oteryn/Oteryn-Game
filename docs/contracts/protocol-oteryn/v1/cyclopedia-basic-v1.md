# Basic Cyclopedia map v1

Owner option A, 2026-10-23; command ids 72–73, capability 57. WORLD-SRC,
2026-11-05, replaces placeholder membership with admitted native subarea masks
and adopts the sourced donation lifecycle. Existing bounded sampling and exact-tile
finding remain Oteryn defaults, not full Reference parity. The feature is offered
only when native Entry membership and a reachable candidate pool are admitted.
No new state domain. Generic CommandResult payloads carry the following bytes.

## Request

All multibyte scalars little endian. Command 72 has an empty payload. Command 73
has exactly 9 bytes: operation u8 (1 start, 2 shuffle, 3 find at server position,
4 donate), area u32 (1), amount u32 (1..10,000,000 for donate; zero otherwise).
The retained wire value 1 aliases the current canonical subarea resolved from the
server-owned position; it never names canonical main area 1 or grants remote access.
The server never accepts a position, world, epoch, Premium flag or respawn factor.

## Result

code u8, Character revision u64, discovered bool u8, found u8, area percent u8,
World pool u64,
reset epoch u64, respawn factor u8 (1 or 2), visible count u8, then count records
of x u16, y u16, native floor i8. At most eight records. Code 0 success, 1 invalid,
2 Premium required, 3 already discovered, 4 no active exploration, 5 insufficient
pool outside current window, 6 insufficient funds, 7 historical area-improved
refusal. Current donations remain allowed during an active boost and fund a future
save. Area percent counts completed canonical subareas within the current main area.
Only the requesting character's unfound same-floor POIs in its viewport are sent.
Committed intent receipts retain the exact result, including query-time facts, for retries.
New queries read current facts; progress survives session and process replacement.
An epoch is improved iff the pool is at least 10,000,000; the goal is subtracted
and the excess carries over.

## Durable fee source

CYCLOPEDIA0 §5.3 authorizes `FeeBurnCause::AreaDonation { world, area, occurrence }`
as an amendment to the closed gold-fee source set. Native SQL cause 4 and the
append-only `OneItemFeeBurnCauseV1.area_donation` tag 4 bind the same World,
server-resolved canonical main area and UUIDv7 occurrence to the Character receipt. The existing fee writer burns
direct backpack coins and debits the shortfall from the bank in that same
transaction. Its existing junior and activation gates remain binding.
Migration 0176 adds bidirectional fee/receipt and pool/state transaction guards.
The active bundle and catalogue digest pin durable interpretation. Migration 0963
preserves the original legacy progress as evidence, credits completed legacy state
to every canonical subarea, and enforces monotonic completed state. Geometry changes
invalidate unfinished canonical POIs without clearing completion. Legacy donation
pools and epochs remain source-only under area 0; the reviewed Entry parent's pool
imports their spendable balance once, guarded by a unique import constraint.

Each canonical main area owns its pool and epoch selection. A funded save consumes
exactly 10,000,000 gp, carries excess, and may improve consecutive days. There is no
random draw or prior-winner exclusion. Exact source locators and compatibility
decisions: `docs/reference/content-prep/world-src-1105/map.md`.

The basic server exposes position checking as intent 3, using the current committed
runtime position. Queries poll the requesting character's visible POIs. Automatic
effect/message events remain subsequent presentation work. The native client
selects existing capability 57, routes 72/73 through its session loop, and exposes
the controls and key binding already implemented by main's0057 client feature.


## T079 atlas query extension

Existing command 72 / capability 57 also accepts nine bytes: selector128, native
floor:i8 (127 selects the current floor), origin x:u16/y:u16 little-endian
(both65535 center on the current actor), zoom:u8 (0..8; cell step=2^zoom),
marker page:u16 little-endian. Empty query and command73 operations stay unchanged.
No new capability, command or movement authority is added.

The bounded reply (at most32768 bytes) starts with the resolved nine-byte query,
World digest32, available-floor bitmask:u16 (bit -floor), legacy-progress length:u8
and its unchanged payload (length0 means exploration unavailable at the current
position), then marker total:u32. It carries exactly64x40 u32
membership cells (0 unknown), area count:u8 followed by id:u32,parent:u32 and
UTF-8 name (length:u8,1..128), then marker count:u8 followed by kind:u8,x:u16,y:u16
and label in the same text layout. Areas and markers are each bounded64. Marker
kinds are NPC1, passage2 and visible exploration POI3; marker paging is64.
All scalars are little-endian. Unknown/trailing/truncated data is rejected.

Each overview cell selects the canonical first intersecting admitted subarea;
it is an overview raster, never walkability or a route. Labels come from pinned
World membership, NPCs from public actors in the current runtime context (private
hirelings excluded), passages from the admitted floor/teleport owners. POIs remain
restricted to the existing same-floor viewport visibility result. Query execution
reuses the current Character/session/node/recovery/equipment fences and exploration
read owner. The atlas is cleared at activity handoff. Client pan, zoom, floors and
layer toggles do not mutate discovery, position, Premium, or donation authority.
