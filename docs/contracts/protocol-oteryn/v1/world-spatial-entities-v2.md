# Spatial actor display names (S-ENTNAME)

Capability 286 `WORLD_SPATIAL_ENTITIES_V2` requires capability 6
`WORLD_SPATIAL_ENTITIES`. It extends the existing domain 1 snapshot/delta type 2
entry with field 13 `display_name`; it adds no command or state domain.

When selected, every player, creature/monster and NPC entry (including the own
player and House hirelings) must carry a nonblank UTF-8 name of at most 64 bytes,
without control characters. Objects must omit the field. Missing, repeated,
invalid UTF-8, misplaced and overbound names fail closed on decoding; encoding
validates the same bound. Names are never truncated. The entry bound becomes
194 bytes and the payload bound 50,688 bytes (256 entries plus 1,024 header bytes).
Without capability 286, entries retain the existing bytes, 128-byte entry bound
and 33,792-byte payload bound; field 13 is rejected by that decoder.

The server reads player Character names through the existing admitted chat-name
cache or durable Character name reader, monster names from the runtime creature
policy's display name, static NPC names from the installed NPC catalogue, and
hireling names from their current House projection. Only the observer's existing
interest set is enriched. No database schema or persistence format changes.
Names travel with enter/update entries and every snapshot/resync; a stationary
name change is an update. Capability negotiation, entity identity/generation,
visibility, ordering and leave entries retain their existing meanings.

SRV6 owner decision: a player with active invisibility is omitted from other
players' entity and appearance observations, with or without capability 286 or
empty-outfit support. Existing visibility deltas emit a leave for a previously
visible player; expiry permits a new enter with the same current identity and
generation. The invisible player's own observation remains available. A retained
entity reference, position or player name cannot bypass direct-target visibility;
held attack targets are cleared before observation, chase or damage. Untargeted
area effects retain their source rules. This deliberately differs from Crystal
`201189f8 src/creatures/players/player.cpp:1383-1396`, which keeps invisible players
observable to other players, and is the explicit C17b requirement of SRV6.

The native client's world-view rendering is owned by C04. The client registration
for this capability declares that rendering unsupported until C04 implements it.

Capability372 `NATIVE_CREATURE_ICONS_V1` extends the existing entity entry with
repeated field14 native icons. Each nested entry requires category(field1),
icon ID(field2) and uint16 count(field3), including an explicit zero. The complete
visible list has at most three entries; order and duplicate icon IDs are
preserved. A new full entity observation without icons removes the old list.
There is no new command or domain. Without372, field14 is rejected and encoders
omit it, preserving legacy bytes.

Source-qualified values are Quests category0 on players: RedCross2(count0..111),
ArrowUp17(count1..100), Hazard22(nonzero uint16 count); Modifications category1 on creatures:
ReducedHealth6(count1/5/15), ReducedHealthExclamation7(count0). These are pinned
Crystal `201189f` native definitions, with Soulpit selection from
`src/creatures/monsters/monster.cpp:2341–2348` and
`src/creatures/creatures_definitions.hpp:1470–1483`. The server retains source
keys for independently updated Rotten Blood modifiers, sorts those keys and
publishes the first three current icons. Client duplicate IDs remain distinct;
a count of zero is visible and never means removal.

General `hazard` points use nonzero uint16 counts in pinned Crystal
`src/creatures/players/player.cpp:5573–5583` (Canary same path:5628–5638);
`data/libs/systems/hazard.lua:163` publishes the selected level and Crystal
`data-global/scripts/systems/hazard_primal.lua:1–5` (Canary
`data-otservbr-global/scripts/systems/hazard_primal.lua:1–5`) permits level12.
The native count ABI is uint16 in Crystal `src/creatures/creatures_definitions.hpp:1549–1557`.
Rotten Blood's distinct `rotten-hazard` source key remains limited to
1..4 by its gameplay owner. That regional limit does not narrow the general
wire icon. Both Hazard22 entries may coexist under separate source keys.

Native presentation uses the existing glyph atlas to display ordered labels and
counts beside the actor name (or alone when the name is absent). This is a
presentation difference: it displays authoritative native icon IDs/counts
without inventing an official icon sprite identity. Both RedCross entries are
shown, including count0; a full update without icons clears their labels.
