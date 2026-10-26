# PODIUM-1 server display state

Task authority: PODIUM-1, 2026-10-26. Protocol-oteryn only. This increment does
not activate capability 133. House ownership/custody, WorldId, ChannelId and
session generation retain their existing owners.

The Item owns a mutable, revisioned presentation row in `game_item_podium_states`.
Migration 0263 adds this extension and a permanent before/after command audit in
`game_item_podium_records`. Neither configuration nor rotation changes custody,
quantity, definition or character revision. Moving the Item preserves its display.
A state revision begins at zero (default south, visible podium, empty display).
Each successful configure/rotate advances it; immutable command receipts prevent
a repeated rotation applying twice. This is ordinary mutable Item state; write-once
text rules do not apply.

The existing House placement writer owns placement from inventory/equipment.
Configuration runs with the carrier House runtime lock, then a native semantic
transaction/deadline and recovery fence. SQL re-reads the node incarnation with
possession, current named House session/connection/lease, assignment generation,
Character account/World/lifecycle/content, property digest, House access and
content/disposition fence, Item lifecycle/definition/custody and tile membership.
The House item lock serializes placement/removal against configuration. The actor
must be within one tile on the same floor. The display's outfit, addons and mount
must resolve to the current qualified cosmetics and owned starter/earned grants
under the locked Character root; hidden selections are checked too. Rotation
retains every other field and wraps north/east/south/west. A refusal writes nothing.

Renown definitions are `oteryn:item.tibia.i35973` and `i35974`.
Owner decision 22 resolves Vigour (`i38707`) as a boss display, following the
pinned references, not verified against Global. It remains refused: there is no
qualified Bosstiary expertise/read owner or boss-display catalogue in this
implementation. Ordinary Bestiary kill progress is not Bosstiary authority.
Vigour must not accept a Renown outfit/mount request as a substitute.

## Wire and activation boundary

Command 154, mechanic 0368 and capability 133 (`PODIUM_V1`) are registered.
Production, Item view and bundle offer sets all exclude 133. The real connection
command gate and the composed owner both require a selected capability. No
unselected client receives this result format.
The House connection loop routes command 154 through the registered podium
handler under its existing input permit and ordered command stream. It retains
the result for transport replay without changing House custody. Both accepted
window/mutation results and refused results keep the actor inside the House.

Request layout is big-endian: version u8=1, action u8 (0 window, 1 configure,
2 clockwise rotate), house UTF-8 length u16 and house bytes (at most 512), Item
UUIDv7 bytes[16], expected display revision u64, display bytes[15]. Window and
rotate require the default display; window requires revision zero. Maximum 555
bytes (the registry conservatively admits 556). No trailing bytes are accepted.

Display layout: outfit look type u16, addon mask u8 (0..3), head/body/legs/feet
palette bytes[4] (0..132), mount look u16, mount palette bytes[4], direction u8
(0..3), flags u8 (bit 0 show outfit, bit 1 show mount, bit 2 show podium).
A visible outfit/mount requires a nonzero corresponding look. With a zero outfit
look, addons must be zero. All other flag bits are refused.

Configure/rotate and every refusal return: refusal u16 (zero for success), display
revision u64, display bytes[15]. A successful Renown window appends version u8=1,
palette count u16=133 and RGB bytes[399], outfit count u16 and entries (look u16,
permitted addon mask u8), then mount count u16 and mount looks u16. Each choice
list has at most 4096 entries, strictly ascending by nonzero numeric look. The
maximum result is 20911 bytes; no names, private account data or arbitrary text
are carried. Labels/appearance assets must resolve from the session's qualified
content. The pending request action distinguishes a window from a mutation result.

Window reads the same locked House/Item/session/root and current account grants as
configure, inside the semantic transaction/deadline. It includes starter or
compatibly earned outfits, only authorized addon bits 1 and 2, and compatible
owned mounts. Hidden/unowned addons are not advertised. Ambiguous duplicate or
unrepresentable look identities fail closed with 1636. A window does not mutate
the Item, its revision, custody or audit. It preserves stored display even if the
querying guest does not own that display; catalogue choices grant no ownership.

This result does **not** substitute for the House map Item snapshot/appearance
delta or a native client window consumer. Those prerequisites remain unimplemented
here and capability 133 stays unoffered.

## Refusal allocation

| Code | Meaning |
|---|---|
| 1630 | Malformed request |
| 1631 | Current House/session/node/lease authority rejected |
| 1632 | No owner, subowner or invited guest rights |
| 1633 | House content/disposition fence |
| 1634 | Not a live supported podium on this House tile |
| 1635 | Outfit/addon/mount not owned or unknown |
| 1636 | Qualified cosmetics unavailable or mismatched |
| 1637 | Expected revision or command binding conflict |
| 1638 | Display revision exhausted |
| 1639 | Outside same-floor one-tile reach |

## Reference evidence

Canary [`game.cpp` at 47dfd51](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/game/game.cpp#L5227)
and Crystal [`game.cpp` at 201189f](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L4640)
store custom Item attributes and update spectators after configuration. Both
`protocolgame.cpp` revisions carry the attributes on map Items and their podium
window, and both `item.cpp` revisions serialize custom attributes. Renown rotation
reuses configuration. Reference behavior is not verified against Global. Oteryn
refuses unauthorized selections as the task explicitly requests; the pinned
implementations clear unauthorized cosmetics instead.
