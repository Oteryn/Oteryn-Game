# Loyalty skills v1

Capability 274 exposes command 488 for the admitted Character's own skills window.
It is offered on bundle Worlds with an active native training formula. The Game
reads Platform's account loyalty projection and the existing Character build owner;
it does not award points or change trained progress.

## Request

The request is empty. It cannot name another Character or account.
An unselected capability, stale session/connection/lease, unavailable content formula
or invalid request is rejected with an empty result.

## Reply

All integers are little-endian. The result is at most 58 bytes:

- `points:u32`: the current account projection, zero when no projection exists.
- `bonus_percent:u8`: 0, 5, 10, ..., 50.
- `title_length:u8`, then that many ASCII title bytes, at most 20. Zero means no title.
- Eight pairs `base:u16,with_loyalty:u16`, in magic, fist, club, sword, axe,
  distance, shielding, fishing order.

`with_loyalty` is the adjusted absolute level, not a percentage or level delta.
The values apply the existing loyalty projection to accumulated training points.
They do not include unrelated temporary skill modifiers. The client labels both
columns explicitly and shows the account percentage and title separately.

The server derives titles and bonuses from Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`,
`data/libs/functions/player.lua:819-844,857-876`. Separate base and loyalty levels
follow `src/server/network/protocol/protocolgame.cpp:9391-9402`.

This read has no durable mutation. The session ingress owner retains the command
result for same-command replay; a new command reads the current projection.
Loyalty point earning belongs to Platform. P12 character-info title presentation
is a separate integration while PR #2793 is open.
