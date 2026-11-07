# Browse field V1

Capability `230 BROWSE_FIELD_V1` requires `14 CONTAINER_TREE_V1` and `18 WORLD_MAP_VIEW_V1` and is
bundle-world only. Command `440 BROWSE_FIELD_INTENT` opens a virtual tile
container or seeks an existing browse page. Close and Up reuse command 21;
item moves reuse command 9 and its existing session-fenced durable transfer.
No browse-field row, item identity or move receipt is persisted separately.

The source is Crystal `201189f871d23597b82caf88fd611454a2ddefb8`:
`src/server/network/protocol/protocolgame.cpp:3817-3826,5768-5793`,
`src/items/containers/container.cpp:55-84,152-161`, and
`src/game/game.cpp:5021-5097,5175-5198`.

## Request

Command 440 is a strict protobuf oneof, at most 12 bytes:

| Field | Type | Meaning |
| --- | --- | --- |
| 1 | length-delimited Open | Browse a native tile position |
| 2 | length-delimited Seek | Seek an existing view |

Open has `sint32 x = 1`, `sint32 y = 2`, and `sint32 floor = 3`. The bundle's
coordinates require x/y from 0 through 65535 and native floor from -15 through
0 (`world-bundle/src/bundle.rs`, `placement_key`; protocol `world_map` floor
constants). Coordinates use the same native frame as `WorldTilePositionV1`;
native floor is the negation of donor z. Absent coordinates default to zero.
Seek has `uint32 view_id = 1` (0 through 15) and `uint32 index = 2` (0 through
65535). Zero values may be omitted. The wire does not decide reach or whether
the current view has that index.

An absent action, multiple actions, duplicate fields, unknown fields,
invalid coordinates, invalid view/index bounds, truncated values and payloads
over the byte limit fail closed. The server additionally requires a live
selected session and applies the existing container-view command rate limit.

## Reply

The reply reuses `ContainerViewResultV1` (at most four bytes). Successful
open/seek sends `Opened` followed by the resulting domain 14 view delta;
toggling an already occupied donor browse slot sends `Closed` and removes
every alias of its current occupant from the resulting view delta. Existing
`Stale`, `TooFar`, `NotAContainer` and `TooManyViews` retain their meanings.
Crystal's silent missing-view/invalid-seek refusal is represented by `Stale`
without a view mutation. Command status continues to report capability,
input and session failures. The reply carries no durable state.

## Container view extension

Domain 14 keeps its existing ownership, type ids, session revisions and
sixteen view slots. Only when capability 230 is selected may a
`ContainerViewV1` contain field 6, `BrowseFieldPageV1`:

| Field | Type | Meaning |
| --- | --- | --- |
| 1 | sint32 | Native tile x, 0 through 65535 |
| 2 | sint32 | Native tile y, 0 through 65535 |
| 3 | sint32 | Native floor, -15 through 0 |
| 4 | uint32 | First item index, within u16 |
| 5 | uint32 | Total eligible items, within u16 |

A browse view has capacity 30, no parent, a session-local synthetic container
handle and the complete current page in donor display order. The page index
is divisible by 30 and below the total, except an empty tile has index 0,
total 0 and no entries. Its entry count is `min(30, total - index)`.
Crystal `src/items/tile.cpp:1056-1059,1692-1695` bounds the total at 65535;
`Game::playerSeekInContainer` requires aligned indexes below the live size.

The same browse container may occupy multiple view ids with independent
page indexes, as in Crystal `src/creatures/players/player.cpp:1208-1226,9018-9083`
and `src/game/game.cpp:4481-4503`. Views sharing its synthetic container
handle must describe the same position and total. Their pages may expose
the same entry handle, but an entry cannot belong to different browse
containers or to both a browse container and an ordinary container.
Within each individual view, all handles remain distinct. Ordinary
containers retain the existing one-view-per-container rule.

An explicit command-440 Open uses the donor dummy slot. When that reached
slot is occupied, the toggle closes and removes every view referring to the
occupant's container, including aliases in other view ids. This applies to
the slot's current occupant, even when the requested tile differs. No hidden
aliases survive in the connection state. An ordinary command-21 Close
removes only its requested view id; other aliases retain their pages.

This is the full-snapshot adaptation A approved in
[ANSWER 6543254285](https://github.com/Oteryn/Oteryn-Game/pull/2765#issuecomment-6543254285).
Crystal sends close packets for every alias of the occupant but erases only
the dummy id from its internal open-container map
(`src/game/game.cpp:5074-5078`,
`src/creatures/players/player.cpp:1230-1243,11773-11782`). Its retained aliases
remain addressable by Seek and item updates (`src/game/game.cpp:5175-5198`,
`src/creatures/players/player.cpp:9018-9083`). Oteryn deliberately removes
that hidden state as part of the approved snapshot adaptation; a removed
view id returns `Stale` on Seek until reopened.

Ordinary durable views have no browse metadata and keep capacity 1 through
20, even in a selected browse session. The existing legacy codec functions
continue to reject field 6 and retain the 10384-byte limit. The selected
codec functions accept up to 15488 bytes: sixteen views, each at most
965 bytes plus a three-byte repeated-message wrapper. This aggregate bound
is registered on domain 14; it does not permit an unselected session to
receive the extension.

Browse selection raises the base live-handle bound from 651 to 811: the
existing 315 handles plus sixteen times one synthetic container handle and
thirty page entries. Existing map-view handle allowance is added separately
when its capability is selected. Changing pages drops handles no longer
visible through any view. Synthetic handles never name durable items.

Opening an empty tile is legal. Explicitly seeking index zero on an empty
view is refused, as Crystal checks `index < size`. Browse fields are
non-durable and close with ordinary container-view lifecycle boundaries.

## Item transfers and admitted scope

The browse window is locked: command 9 cannot use its synthetic container
handle as a destination. Crystal creates it with `unlocked=false`
(`src/items/containers/container.cpp:56`), refuses additions to a locked
container (`:486-488,647-650`) and advertises drag-and-drop disabled
(`src/server/network/protocol/protocolgame.cpp:5777`). A drop onto the field
uses the existing command-9 Ground destination for the tile. An item taken
from the view retains its ordinary item handle and goes through the existing
durable transfer; refreshing the view observes the committed tile contents.

The existing mover keeps its supported transitions and atomic refusals. This
capability does not expand partial Ground pickups, Ground-to-Ground moves,
nonempty-container drops or destinations outside carried container custody.
The browse filter uses qualified native movable/container facts and excludes
unique-id items. Fixed furniture whose wrapability and block-path facts have
not been admitted stays outside this slice; its facts are not inferred from
pickupability or a transform target. These boundaries follow the
[coordinator's decision on PR 2765](https://github.com/Oteryn/Oteryn-Game/pull/2765#issuecomment-6542717982).

## Client registration

The shared protocol codecs and command registration are available. The
native client feature is recorded as unsupported until the existing native
container UI migration supplies its input and rendering; this capability
is not added to the production fallback offer.
