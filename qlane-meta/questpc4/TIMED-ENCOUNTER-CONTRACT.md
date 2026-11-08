# Source timed creature switches

Owner scope assigns missing shared mechanisms to this lane. Extend the existing
native encounter/carrier and Channel source-owner deadline loop for explicit
spawn-only timed encounters. Existing party entry still requires its original
1..8 player proofs; no existing admission or test guard is weakened.

A new optional, default-omitted timing descriptor belongs to Encounter. It is
allowed only on a value-free, irreversible native USE lever with no player
teleports, personal storage, inventory inputs, grants, XP or native state rule.
Spawn-only declarations have no party destinations; ordinary declarations retain
their prior bounds. Admit 1..8 exact native Creature definitions, a delay bounded
to 60 seconds, a reset bounded to one hour, and an inclusive cleanup rectangle
bounded to 4096 cells. Delay precedes reset. Queue capacity is bounded per Channel,
with one activation per exact source placement/revision. Closing/replayed/stale
lever actions cannot queue another activation.

Use the existing held Spell lane, Runtime owner/content/map fence and semantic
clock. Preflight capacity and native policies before changing the lever. Admit
all spawned creatures through a staged carrier operation with rollback on any
failure. The production source-owner loop wakes at the earliest due spawn/reset;
there is no separate scheduler. Source cleanup removes current monsters inside
the declared room, leaves players and outside actors intact, and settles monster
plates in the same owner turn. Restore only the source lever revision/appearance
owned by that activation. A different map/content/owner cannot inherit the queue.
This is ephemeral Channel state like the source Lua timer; it creates no Quest
progress or reward receipt and no synthetic kill/loot event.

Initial source consumer: Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
`data-global/scripts/quests/the_paradox_tower/action_third_floor_lever.lua:11-22`,
SHA256 `ace0db48b50044dcff1abce8ff159f1e24bde452d0e60eff817bfbb009d66324`.
USE9125 schedules Ghoul32479,31900,z5 after2000ms and transforms to9126;
100000ms clears monsters from32476..32481,31900..31901,z5 and resets9125.
The repository corpus contains these exact source bytes. The absence of a static
spawn in world-monster.xml is therefore expected, not a source gap.

Cleanup is explicit in the same pinned repository's
`data/libs/functions/position.lua:234-270`, SHA256
`c18d16465394a4064e82d35463105511c1f273e7c6de588f29b9da8b1eadaf9b`:
`removeCreatures=true, removePlayer=false` removes monsters, leaving players.
Targeted checks of current qengine2/PC3/PC6/PC8 quest callback mechanisms found no
spawn-only timed encounter implementation. Reuse the existing source-owner loop
and native encounter policies, rather than creating another timer authority.

Qualification requires real PostgreSQL RED/GREEN with a complete physical switch,
no early spawn/reset, native ghoul admission, monster plate/ladder, precise cleanup,
player/outside-actor preservation, and no durable value or progress writes. The
three native puzzle probes must become those physical route assertions before
GREEN. No whole Paradox qualification follows from a component proof alone.
