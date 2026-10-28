# Hireling v1

Capability 165 exposes command 220 to a session currently admitted to a House runtime.

## Request

Canonical bytes are `00` (query), `02` (dismiss), or `01 || x:u16be || y:u16be || floor:i8` (summon). The server derives the house, character and owner authority from the fenced session; coordinates must be adjacent to the player.

## Reply

One byte: `01` when the durable banker hireling is summoned and `00` when dismissed or absent.
