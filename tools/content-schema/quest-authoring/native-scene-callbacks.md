# Native source scene projections

`native_scene_callbacks.py` produces `content/quests/native-scene-callbacks.json`.
The existing world callback owner admits the actor, exact world placement and
object revision. The scene extension prevalidates its entire finite edit list,
then stages a copy of the existing callback projection and publishes it once.
It adds no inventory or quest-state owner.

A scene has `open` and `close` lists, each limited to 32 distinct (position, removed appearance) pairs, and
optional `remove_self`. Each edit declares a position, an optional removed
appearance, an optional created/replacement appearance and the resulting
`passable` fact when creating/replacing. A missing removal refuses the complete
operation unless `allow_missing` explicitly requests a conditional no-op.
Every target must exist in the current bundle and be outside housing; required
source objects must match unambiguously. Created and replacement objects must
be non-pickupable. Invalid or stale source revisions publish nothing.

Scenes are lever operations. They may compose bounded quest storage through the
existing durable callback receipt and recovery path. They cannot mix carried
reward MINT, offerings, encounters, portals or item-cycle projections. The
previous passage primitive keeps its eight-effect limit and source truth tables.

Optional `evacuate_open` and `evacuate_close` lists declare source cells and
separate player/NPC and creature destinations. The world factory checks every
destination against the pinned geometry and pending spell reservations. One
complete current actor census is rechecked at commit; stale arrivals, lost player
control and ambiguous TOP cells refuse. `all: false` requires an unambiguous single
actor per source; `all: true` evacuates every actor. The existing group bound remains32actors. Phase lists admit eight source cells;
legacy `relocations` retains its32origin bound. Each move stays on its source
floor, with destinations outside edited cells. Explicit source stacking is allowed. Ordinary STEP, teleport and encounter
entry restrictions are unchanged. A failed map projection restores every actor.

`relocate_items_close` requires an all-actor close evacuation. Current movable
map origins and durable ground custody use the existing map MINT/item-loop
writers, after live callback gate preflight. Stable item consequences derive from
the parent USE occurrence. Item moves are sequential, matching the source Lua;
refusal or unknown outcomes prevent the lever publication, and retries use the
same existing receipts. Removed source borders are restored with their original
placement identities/revisions, avoiding duplicate objects on repeated toggles.

The initial consumer is the Mintwallin Cyclops source lever. It removes itself,
creates appearance1270 at32592,32105,14 and32592,32106,14, and removes appearance1271
at x32593..32601,y32104,z14. The committed native map contains its lever at
32600,32105,14 (appearance2772, ordinal1), without a legacy action tag. The binding
uses that exact native selector rather than inventing action30008 on the map.

Source: https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/data-otservbr-global/scripts/quests/mintwallin_quest/actions_devil_helmet_lever.lua

These mutations remain Channel-local, as in the existing callback projection.
Reconstructing the source floor through normal bundle boot restores the original
lever/walls and removes created objects. No reset schedule is inferred from the
source script. The PostgreSQL dispatch case proves stale refusal, all eleven
wall edits, collision/visibility, self-removal, command replay and reconstruction.
The source reward cache remains a separate existing claim; qualification of this
scene alone does not certify its native selector or the entire quest.

The committed `relocations` shape remains supported on both lever phases, with its
shared landing and actor bound unchanged. Phase-specific lists add separate
creature destinations. Both shapes use the same `commit_source_scene_relocations`
carrier owner, including native NPC slots; they cannot be mixed in one scene.
