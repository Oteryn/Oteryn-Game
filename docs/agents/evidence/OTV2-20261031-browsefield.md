# Browse field (I18a, I18b, I18c)

## Scope and ownership

- Task: `OTV2-20261031-browsefield`.
- Canonical branch: `chatgpt/browsefield`; one publishing lead, three path-disjoint workers.
- Base inspected: `80dca5fd1578d029f953f05690c6246a0236fd16`.
- Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Deliverable: one draft pull request; no review request, heavy-tests label or merge.
- The user requests the first draft immediately and decisions through QUESTION/ANSWER comments.

## Verified starting point

The current container-view owner is command 21/domain 14. Command 9 reaches the existing
session-fenced durable item loop and its receipts. Browse fields are ephemeral views of tiles;
they must not create persisted virtual containers or a second item mover.

Crystal uses a 30-entry page (`container.cpp:55-70`), same-floor adjacency and walking followed
by retry (`game.cpp:5021-5100`), and validates aligned page seeks (`game.cpp:5175-5191`).
Ground-container Up reaches the tile (`game.cpp:4473-4503`).

## Implemented ownership and wire contract

Capability 230 (`BROWSE_FIELD_V1`) requires the existing container and map capabilities and
is offered only in bundle worlds. Command 440 carries Open and Seek; mechanic 0580 routes
it into the existing connection container-view owner. Command 21 retains Up and Close;
command 9 retains every durable transfer. No migration or new durable browse object exists.

The synthetic `ItemKey::BrowseField` identifies a tile, never item custody. Its visible
entries keep their original MapBase, MapAdded or durable Instance keys. Every observation proves the
current session fence, actor control, same-floor adjacency and qualified native item facts.
Transfers independently repeat their existing custody and session-generation authorization.

Ordinary views keep their twenty-entry and 646-byte per-view limits. The selected browse
extension has thirty entries per page and a maximum domain payload of 15,488 bytes. The
base handle allowance is 811; adding the existing map allowance gives 1,835. All bounds are
derived and measured in protocol tests. No existing resource-limit numeric value changes.

## Donor rules and implementation

| Rule | Crystal source at the pinned revision | Oteryn owner |
|---|---|---|
| Same-floor Chebyshev reach of one; wrong floor never starts a successful walk | `src/game/game.cpp:5027-5047` | `gameplay_transport/browse_field.rs`, `container_view/browse_rules.rs` |
| Walk to an adjacent cell, then schedule the retry after 400 ms | `src/game/game.cpp:5036-5044`, `src/creatures/players/player.cpp:6190-6192` | Existing paced movement and `connection/serve/autowalk.rs` |
| Dummy view ID is `15 - ((x % 3) * 3 + y % 3)`; a reached occupied slot sends Close for every alias of its occupant | `src/game/game.cpp:5074-5082`, `src/creatures/players/player.cpp:11773-11782` | `container_view.rs`, `browse_rules.rs`; alias removal follows the approved snapshot adaptation below |
| Exclude ground and UNIQUEID; reverse the eligible full tile stack | `src/items/containers/container.cpp:61-78` | `container_view/browse_projection.rs` |
| Share a weak tile cache; append additions and retain surviving order | `src/game/game.cpp:5064-5072`, `src/items/tile.cpp:379-393,504-518` | `container_view/browse_cache.rs`, existing move notifications |
| Capacity 30; aligned u16 seek below total; empty opening is allowed | `src/items/containers/container.cpp:55-56`, `src/game/game.cpp:5175-5198` | `protocol-oteryn/src/browse_field.rs`, existing container-view codec/state |
| Multiple IDs may show the same field, with independent pages; Up preserves the child view ID | `src/creatures/players/player.cpp:1208-1226,9018-9083`, `src/game/game.cpp:4481-4503` | `container_view.rs`, `container_view/refresh.rs`, selected container codec |
| Use the existing mover for browse entries; the virtual destination is locked | `src/game/game.cpp:2241-2258`, `src/items/containers/container.cpp:56,486-488,647-650`, `src/server/network/protocol/protocolgame.cpp:5777` | Existing `item_move.rs` and durable item loop |
| Rewind an emptied page; close beyond same-floor adjacency | `src/creatures/players/player.cpp:8674-8692,9063-9084`, `src/items/containers/container.cpp:832-838` | Existing refresh owner and movement effects |

Paths beginning with `gameplay_transport` are below `crates/gameplay/dispatch/src/`.
The donor's shipped browse callback accepts the operation; there is no blanket protection-zone
or house-guest prohibition on inspecting a tile. Existing movement authority remains unchanged.

## Consistency and lifetime

Each connection retains its open view IDs and page indexes. A Channel cache is keyed by the
bundle digest, reset epoch and tile coordinates; it retains only weak leases. Closing the last
view releases the shared ordering, and reopening reconstructs the donor's initial reverse
order. Reconnect does not restore virtual browse windows.

Aliases of one field share current contents and total but retain independent page indexes.
Open, Up, Seek and refresh update aliases coherently and clamp pages after removals. A refresh
observes each unique tile once. Only the union of currently displayed page entries receives
tree handles; other existing domains retain their own handle ownership.

An explicit Browse Open that toggles an occupied dummy slot closes and removes every alias
of that slot's current occupant. No hidden aliases remain in the connection state. Ordinary
command-21 Close removes only the requested view ID and preserves other aliases and pages.

Committed ordinary Ground moves notify the cache before releasing the existing movement lane.
This preserves append order even when pickup and redrop occur between two observations.
Idle observers use the existing visibility cadence. A slow owner refresh cannot indefinitely
postpone a due walk retry, and ready client input retains cancellation handling.

Failed tree encoding or revision exhaustion rolls back the existing handle-table undo record,
preserving the exact old handles and counter. This covers conflicting multi-tile observations
without sending a partial view or invalidating the client's unchanged handles.

## Accepted boundaries

Coordinator [ANSWER 6542717982](https://github.com/Oteryn/Oteryn-Game/pull/2765#issuecomment-6542717982)
selects A for all three domain questions:

- The browse-window destination stays locked. Ordinary Ground drops onto the tile update it.
- Existing general-mover restrictions remain separate: Ground-to-Ground, partial Ground
  pickup, non-carried container destinations and nonempty-container Ground drops. The durable
  `item_loop_move.rs` implementation is unchanged.
- Include qualified movable items and containers, excluding UNIQUEID. Unqualified fixed
  furniture remains omitted because native admission lacks the required block-path and
  wrapable appearance facts. Pickupable never substitutes for movable.

Coordinator [ANSWER 6543254285](https://github.com/Oteryn/Oteryn-Game/pull/2765#issuecomment-6543254285)
selects snapshot adaptation A for occupied dummy-slot toggles. Crystal sends Close for every
alias but removes only the dummy ID from its internal map (`src/game/game.cpp:5074-5078`,
`src/creatures/players/player.cpp:1230-1243,11773-11782`); retained aliases can still receive
Seek and item updates (`src/game/game.cpp:5175-5198`,
`src/creatures/players/player.cpp:9018-9083`). Oteryn removes all aliases of the occupant from
both the full domain-14 view snapshot and connection state. This is an approved adaptation
of the visible close behavior; the donor's hidden alias state is intentionally omitted.
Command-21 Close retains its single-view semantics.

The native client registration remains explicitly unsupported. The complete command path is
available to a client selecting the bundle-only capability; this change does not add a native
client panel or promote the capability into production offers.

## Publication decision

Coordinator [ANSWER 6542800961](https://github.com/Oteryn/Oteryn-Game/pull/2765#issuecomment-6542800961)
selects publication B: publish the task delta through the GitHub API with the initial draft
commit `8367dfb43505ced1f37b91a120370ba42c297342` as its sole parent. The published delta is
limited to this task's files and approved corrections. This decision replaces the usual
latest-main merge requirement for this publication; unrelated main-branch changes stay
outside the task delta. The draft remains under the single publishing lead.

## Verification coverage

Protocol tests cover malformed input, native coordinate bounds, selected versus legacy
decoding, measured frame/handle limits, ordinary-view limits and shared browse ownership.
State tests cover reach, toggle, order, paging, aliases, handle retirement/rollback, weak-cache
lifetime, range closure and the cancellable 400 ms retry deadline.

Five mounted PostgreSQL cases drive the real admitted connection and read committed rows:

1. Ground drop, pickup to the backpack and receipt replay after recreating the owner with
   stale handles.
2. Pickup to a carried container and atomic refusals, checking both source-bag and coin custody.
3. Ground-container Up while another field view remains open, then range closure of both IDs.
4. Thirty-one authored items, beyond the ordinary map cut, across two browse pages.
5. Distant opening through walking/retry, wrong-floor refusal and an unreachable path.

Real database successes print `BROWSE_FIELD POSTGRES_CASE_COMPLETED`; absence of the admin
URL prints a distinct skip marker. Exact execution commands, results, environment limitations
and the published head are recorded in the draft pull request's Tests run section. Test
presence alone is not a claim that a case executed.
