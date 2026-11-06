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
