# Value-free world puzzles and ladder ascent

The owner assigns shared mechanisms to this lane. Existing qengine2 and current
PC3/PC6/PC8/world-change branches were checked before implementation; none had
readonly world requirements, owned ephemeral ladder presence or monster plates.

Readonly world requirements are bounded to eight exact appearances on unhidden,
non-pickupable native entries outside houses. They are checked under the held
Channel/SpellLane owner before projection. They gate opening only, so changing
a pattern cannot strand an already-open lever. These callbacks cannot include
Character state, input consumption, XP, rewards or other value participants.
Durable value admission remains with the existing fenced transaction owners.

Item presence is a Channel map projection keyed by its exact source placement.
Opening twice retains the same added identity. Closing removes only that source's
entry; reopening allocates a new identity. No Item instance, MINT, custody record
or portable reward is created. Initial admission is limited to known immovable,
walkable ladder appearances. Other projection owners at a position are preserved.
Channel teardown discards the projections with the existing map owner.

Monster callbacks are excluded from player USE/STEP dispatch. Their registry
rejects Character/value participants and requires an exact position. The existing
monster pass reconciles plates from a complete bounded census of live creatures,
rather than treating player steps as monster steps. This also handles death and
despawn without waiting for another movement event. Transformations must remain
non-pickupable. Incomplete census or missing map facts fail closed.

Ladder USE resolves a base revision or an exact live added identity and rechecks
it in the sealed movement proof. It shares rope's native upstairs landing order,
PZ admission, collision/door and other-actor checks, pinned actor control, facing,
chat range settlement and stair-hop cooldown. No client destination is accepted.
The owning generator derives ladder IDs from KNOWN immovable WorldObject records;
it does not hand-author a duplicate source-ID table.

Sources: pinned Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
`data/scripts/actions/items/ladder_up.lua:1-25` SHA256
730bc7664484e953b6efc3c8faeba9936d355c5cdde91d1f3e0686f714f4df13;
`data/libs/functions/position.lua:22-56` SHA256
c18d16465394a4064e82d35463105511c1f273e7c6de588f29b9da8b1eadaf9b.
Paradox puzzle script citations and recipe choices live in
`content/quests/authoring/paradox-tower.md`. This mechanism contract alone does
not qualify the whole Paradox route or claim historical full-world source closure.

The loop Ground snapshot bridge reuses PC6's existing delegation pattern at
390e024503a7c02b9e15d67e680cec0ac711fd5b,
`crates/gameplay/dispatch/src/gameplay_transport/loop_move.rs:268-275`.
`movement_item` and `movement_items` delegate to the wrapped map facts. The House
map adapter keeps its intentional exclusion of Channel projections. The actual
wire PG test exposed the missing bridge here before the methods were added.
