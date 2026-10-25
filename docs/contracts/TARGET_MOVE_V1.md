# Target movement v1 — PARITY-TARGET-MOVE-1

Owner-requested server slice, 2026-10-25. Capability 64 requires spatial entities (6)
and paced movement (13); command 81 is strictly rejected before owner dispatch
unless selected. No new state domain or migration is needed.

The command carries exactly one of an exact cell in the pinned native frame,
stop=true, a cardinal facing, or an existing entity identity and generation.
The embedded cell, direction and entity use the existing spatial/step/attack
protobuf encodings. Unknown, repeated, empty, inconsistent and oversized inputs
fail closed. Results are disposition 1 accepted, 2 no path, or 3 rejected.
Acceptance starts a runtime route; it never asserts the destination was reached.

The admitted connection owns at most one ephemeral queue. New target movement,
a manual step, attack, logout, or item autowalk replaces it. Connection loss or
replacement drops it. CommandId replay uses ordinary session ingress; replay
cannot start a second walk. A valid follow clears the held attack target, as in
Canary. A refused target does not move the player.

Movement owns each ordinary step, the actor clock, collision, equipment speed,
PvP barriers, field contact and tutorial interception. Each queued step compares
the actor's exact snapshot under the movement commit lock. Relocation, floor
change, control loss or death invalidates the queue. FOLLOW additionally compares
the target snapshot under that lock, and resolves the same Channel identity and
generation, same floor, current visibility and living status before re-pathing.
Player and creature targets are supported; NPCs, corpses, self and stale refs are
refused. FOLLOW stays adjacent and re-paths using current occupancy as the target
moves. Target loss/logout/death/floor change ends it. Rejected obsolete follow
steps recheck after 100 ms; adjacent followers also check every 100 ms.

Both use the shared cardinal A* with radius 12, at most 1,024 expansions and 128
steps. Target cells are exact goals; FOLLOW's goal is distance one. Teleports and
floor-change cells are excluded. The existing Channel path-work budget is charged;
capacity defers the route rather than claiming it is unreachable. Input is capped
at 25 target commands per second per connection before retention/search. The input
and owner work run within the existing ten-second connection deadline. Ready
client input wins before an autonomous step; liveness and refresh ticks stay live.

TURN commits the existing runtime facing in place with an exact revision compare.
It advances the position revision so visibility emits the actual facing and spells
read the same owner value. Facing follows the existing runtime lifecycle and is
not persisted. Existing position save/logout writers remain authoritative.

Excluded: diagonal movement, cross-floor path planning, source client direction
lists, durable facing, client controls, invisibility/LOS policies beyond accepted
reference visibility, and generic movement notifications beyond spatial updates.
Numeric registrations are 64 and 81; reserved domain 29 and migration 0192 are unused.
