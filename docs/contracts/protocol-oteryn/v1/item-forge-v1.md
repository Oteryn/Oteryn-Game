# Item Forge V1

Capability 0152 exposes command type 0181 in bundle Worlds with admitted native Item
content. It supports fusion, convergence fusion, transfer, convergence transfer,
DustToSlivers, SliversToCores, DustLimit and QueryHistory through the existing native
Forge owner. Availability can remove capability 0152 from the configured offer; it
cannot add it to the production fallback or Item-view offer.

The server resolves item custody, classification, equipment slot, tier, prices,
resources and random outcomes. Clients select existing session ItemHandles and choose an
operation; they never supply resource counts, price quotes, resulting tiers or rolls.
P19's existing Character dust ledger and native sliver/core items remain the resource
owners. No parallel Forge wallet is gameplay truth.

Eligible inputs are recursively carried, unequipped native items without imbuements.
Materials can be consumed from carried inventory and the existing native stash under
their current custody proofs. A successful fusion or transfer preserves the retained
ItemInstance; consumption burns the source through the native item lifecycle. No
Exaltation Chest is created, so these operations do not require a spare backpack slot.
Conversions check room before spending because they create a physical output item.

## Request

Exactly 51 bytes:

| Bytes | Field |
|---|---|
| 0 | operation (`u8`) |
| 1–16 | occurrence UUID (16 bytes) |
| 17 | item-presence and optional-core flags (`u8`) |
| 18–25 | first session ItemHandle (`u64`, little-endian, nonzero when present) |
| 26–33 | zero padding (8 bytes) |
| 34–41 | second session ItemHandle (`u64`, little-endian, nonzero when present) |
| 42–49 | zero padding (8 bytes) |
| 50 | history limit (`u8`, at most 50) |

Operations: 0 QueryHistory, 1 Fusion, 2 ConvergenceFusion, 3 Transfer,
4 ConvergenceTransfer, 5 DustToSlivers, 6 SliversToCores, 7 DustLimit.

Flag bit 0 declares the first item; bit 1 declares the second. Operations 1–4 require
both bits and both item fields. The first item is the receiver/retained item; the
second is the consumed or downgraded source. For transfer the second item donates its
tier. All other operations require both presence bits and both item fields to be zero.
Every padding byte must be zero. A present zero handle is malformed. Only handles
resolved by the current `SessionItemView` to a native `ItemKey::Instance` can start a
new item attempt; unknown, stale and other-kind handles receive a retained refusal.
Native ItemInstance UUIDs never appear in the request or result. This Q7 amendment
keeps the existing payload size and every non-item operation's bytes unchanged.

For normal Fusion only, bit 2 opts into the success-rate core and bit 3 opts into the
tier-loss protection core. Each costs one core before applicable donor bonuses.
Convergence and all other operations reject these bits. Bits 4–7 are reserved and
must be zero. Both core bits clear select an unboosted, unprotected normal fusion.

QueryHistory requires a zero occurrence; a zero history limit returns an empty array.
Mutations require a UUIDv7 occurrence bound to the authenticated Character and a zero
history limit. Reusing an occurrence with a different operation, item pair or optional
core selection is a conflicting request, never a new attempt.

## Reply

The command returns a bounded JSON result or history array (at most 50 entries and
32,768 bytes). The ordinary mutation result contains:

- `operation`: the operation's snake-case name;
- `succeeded`: whether the requested action succeeded; a failed fusion is false;
- `applied`: whether the Forge attempt changed durable state;
- `reason`: a refusal code or `null`;
- `bonus`: the donor bonus code;
- `result`: `null` on refusal, otherwise the original session item handles/tiers, current dust and
  dust limit, and the actual gold/core/sliver costs and conversion outputs.

`result` fields are `first_item`, `first_tier`, `second_item`, `second_tier`, `dust`,
`dust_limit`, `gold_spent`, `cores_spent`, `slivers_spent`, `slivers_created` and
`cores_created`. Item fields are the original unsigned64-bit handle values; nullable
item/tier fields represent an absent output. A retained reply or history entry does
not make an old handle live in a later connection: new selections use the current
inventory/container view. An ordinary
fusion can return `applied: true, succeeded: false`: its failure costs and source
downgrade/destruction still commit.

DustLimit keeps its existing result shape: `operation`, `succeeded`, and `result`
containing `dust` and `dust_limit`; a retained revision-change refusal keeps
`succeeded: false, result: null`. QueryHistory returns newest entries first as
`[{"ordinal": ..., "receipt": ...}]`, including the existing native DustLimit history.
History includes every applied action and applied fusion failure. Terminal refusals
retain replay receipts but do not add history, matching the donor's early-error behavior.

A decoded, terminally evaluated command has command status `Accepted`; the durable
result distinguishes a refused attempt from an applied fusion failure. Malformed
requests, conflicting occurrences and rejected authority have command status
`Rejected`. Unavailable or unknown commit state disconnects rather than inventing an
outcome.

## Authority, replay and state consumers

Each mutation uses the existing item lane and one session-generation-fenced native
transaction for its item changes, fees, Character dust, outcome receipt and history.
The durable owner re-proves current session, lease, node, scope and content under its
locks. The transport verifies the selected capability and current actor before a new
attempt. A service cannot spend items offered in a trade.

Receipt lookup precedes current handle resolution, inventory, content, trade and cooldown decisions.
An identical occurrence under a new CommandId returns the recorded result, including
its recorded random outcome, without applying or charging again. Refusal receipts
also replay their recorded result after resources change. A different intent bound
to the same occurrence is rejected. A superseded session cannot use receipt lookup
to bypass generation fencing. The reservation binds the original canonical 51-byte
wire intent separately from the resolved native operands. Reconnecting creates new
session-view handles; replay of the recorded original bytes remains valid even
when the original source was consumed or its old handle no longer resolves.

New Forge attempts and history reads share a 490 ms Character cooldown. An evaluated
attempt starts this delay even when its resources or items are refused; a retained
receipt replays before that gate. This follows Crystal's configured
`uiActionsDelay = 500` minus 10 (`config.lua.dist:469`, `game.cpp:11018–11063`).

After an accepted item mutation, the existing inventory and open-container projection
refreshes in the same command turn. Item inspection reads the native tier rows.
Equipping a forged item uses the existing equipment owner and combat modifier
projection, which already consume those tier rows; no second equipment tier cache is
introduced.

## Reference and scope

The implementation follows Crystal `201189f871d23597b82caf88fd611454a2ddefb8`:
`src/creatures/players/player.cpp:10545–11028,11259` for mutation/history behavior;
`src/game/game.cpp:10990–11063` for action routing and random draws;
`src/server/network/protocol/protocolgame.cpp:6773–7138` for eligible pair presentation,
core-option selection, results and history; and the pinned config/item-tier tables for
prices, chances and caps. Exact rule and test mappings are recorded in the Forge PR.

The existing Oteryn history route remains a bounded newest-first query instead of
Crystal's nine-entry page packet. The native-client Forge panel remains explicitly
unsupported in its client feature registration; this contract defines the server
mechanic and reusable request codec.

DustLimit preserves admitted D1230 progression: initial limit 100, +20 per upgrade,
cost 25 plus the previous upgrade count, at most 225 upgrades, final limit 4600.
This intentionally differs from the literal Crystal configuration, whose threshold
225 stops the 20-unit progression at 240. PR #2785 owner answers Q1–Q3 confirm this
progression, recursive custody/material scope and native ItemInstance preservation.
