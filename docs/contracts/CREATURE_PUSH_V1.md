# Creature push v1 — PARITY-PUSH-ROUTE-1

Owner-requested server slice, 2026-10-25 (parity gap 23). Capability 82 requires
spatial entities (6) and paced movement (13); command 98 is strictly rejected before
owner dispatch unless selected. No new state domain or migration is needed: a pushed
player's position is the existing durable position, saved by the existing writers.

The command carries exactly one visible entity identity and generation (tag 1, the
attack-target encoding) and one destination cell in the pinned native frame (tag 2,
the spatial position encoding). Unknown, repeated, missing, trailing and oversized
inputs fail closed (`PUSH-RL-01`, 64 bytes). The result is `CreaturePushResult`:

| Disposition | Meaning | Canary `Game::playerMoveCreature` |
| --- | --- | --- |
| 1 MOVED | the target was moved to the destination | `internalMoveCreature` (game.cpp:1896) |
| 2 NOT_MOVABLE | non-pushable creature, summon, familiar, or a player mid-step | `RETURNVALUE_NOTMOVABLE` (game.cpp:1844) |
| 3 REJECTED | malformed, over the intent window, or the pusher cannot act | input gate |
| 4 NOT_POSSIBLE | unknown/self target, target not adjacent, zone rule, diagonal player push | `RETURNVALUE_NOTPOSSIBLE` (game.cpp:1856, 1865) |
| 5 NOT_ENOUGH_ROOM | destination blocked, occupied or not walkable; NPC target | `RETURNVALUE_NOTENOUGHROOM` (game.cpp:1862, 1869, 1878) |
| 6 DESTINATION_OUT_OF_REACH | destination not a same-floor neighbour of the target | `RETURNVALUE_DESTINATIONOUTOFREACH` (game.cpp:1851) |
| 7 EXHAUSTED | inside the push delay after a committed push | `PUSH_DELAY` (configmanager.cpp:337) |

Every disposition other than MOVED commits nothing: all checks run before the single
position commit, under the same Channel owner lock as that commit.

A creature is pushable only when it is a wild live creature (no master, not a familiar)
whose bound behavior profile is `pushable` and `can_walk` (Canary monster.cpp:275).
Movement owns the creature relocation (`relocate_creature`): walkable, no door, no
teleport, no protection zone, no floor change, no occupant. A player target moves by
one ordinary paced step of its own session: collision, PvP barriers, field contact,
tutorial interception and its equipment speed apply, and the step restarts the
target's shared actor step clock (Canary creature.cpp:520). The step fences the
target's and the pusher's exact snapshots, so a relocation between the check and the
commit refuses the push.

Zones: a target on a protection-zone or no-PvP tile is not pushed onto a tile without
that flag (game.cpp:1865). An unknown origin or destination tile refuses.

Limits: 25 push commands per second per connection before decoding (`PUSH-RL-02`);
1000 ms after each committed push before the next is attempted (`PUSH-RL-03`). Canary
defers the push itself by the delay; Oteryn commits at once and refuses pushes inside
the delay with EXHAUSTED. Owner work runs within the existing ten-second connection
deadline.

Excluded: walking to a distant target before pushing (Canary game.cpp:1813), diagonal
player pushes, ghost and `CanPushAllCreatures` flags, `pushWhenAttacking`, NPC,
summon and familiar pushes, `CannotBePushed` player flags, creature walk-delay
immunity (creature.hpp:207), bed sleep (no production bed owner is composed yet), and
client presentation. Numeric registrations are capability 82, command 98 and limits
1120-1122; reserved state domain 47 and migration 0212 are unused.
