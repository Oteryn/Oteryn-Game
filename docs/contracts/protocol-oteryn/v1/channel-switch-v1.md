# CHANNEL_SWITCH_V1 (capability 512)

Owner decisions of 2026-11-17 (task `chsw`) under the accepted
`docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md` §7-§11.

A voluntary Channel switch is a safe exit from the current Channel followed by a fresh admission
into the target Channel of the same World: the same login and the same position, never a teleport
or an in-place transport rebind. The Channel picker on the character-selection screen stays; a
relog into another Channel is the same switch.

Wire code: `crates/protocol-oteryn/src/channel_switch.rs`.

## Status request

Command 6247 `CHANNEL_SWITCH_STATUS`: the payload is empty. Any byte is `REJECTED` with an empty
result.

## Switch request

Command 6248 `CHANNEL_SWITCH_REQUEST`: exactly 16 bytes, the target ChannelId of the current
World. Any other length is `REJECTED` with an empty result.

## Result

Both commands answer five bytes: a lock code and the cooldown left in big-endian milliseconds
(`0..=300000`). The cooldown is non-zero only with code 0 or 9, and code 9 always carries it.

| code | lock | meaning |
|---|---|---|
| 0 | none | no lock; with a non-zero cooldown a switch waits for it |
| 1 | combat | in fight, a PvP logout block or a protection-zone block |
| 2 | skull | a red or black skull cannot leave for a Channel that is not Open PvP |
| 3 | trade | an open direct trade |
| 4 | value transaction | an unresolved item, currency or value commit |
| 5 | event | boss, raid or event participation in an instance or a live Channel encounter room |
| 6 | transition | a house, instance or travel transition in progress |
| 7 | checkpoint | a pending checkpoint, handoff or authority transition; retry shortly |
| 8 | destination | the target is not a current Channel of this World |
| 9 | cooldown | five minutes have not passed since the last switch (server clock) |
| 10 | same channel | the target is the current Channel |
| 11 | no logout | the current tile does not allow logging out |

`CHANNEL_SWITCH_STATUS` is `ACCEPTED` and reports the locks that do not depend on the target
(codes 0, 1, 3-7, 9, 11).

`CHANNEL_SWITCH_REQUEST` is `ACCEPTED` with code 0 only after the terminal release of the session
has committed: the server writes the result and closes the transport, as for an accepted logout.
The client then enters the target Channel by a fresh Platform authorization and a fresh admission.
Every refusal is `REJECTED` with its code and leaves the session, the actor and the guard
unchanged.

## Final admission

The Game control operator sets the directory PvP type with `assignment assign|replace --pvp-type OPEN_PVP|OPTIONAL_PVP`. The assignment receipt binds that choice and commits it with the scope assignment. Without the option a new Channel defaults to Optional PvP and replacement retains the stored type.

The fresh admission revalidates the switch inside the admission receipt insert (migration 2858):
the first admission in a World sets the baseline, the same Channel is not a switch, a different
Channel inside the cooldown or with a lock the database can prove (combat, kill or protection-zone deadline, a saved House on another Channel,
or a skull entering a Channel outside Open PvP) is refused, and a refused or failed admission uses no cooldown. A
retry of the same admission cannot advance the guard twice.

## Party across the switch

The terminal release of an accepted `CHANNEL_SWITCH_REQUEST` keeps the Character in its Party and
holds the membership for two minutes (migration 2859). The committed admission into the World
retains membership if the hold is still current. A late admission and the periodic expiry drain
both apply ordinary Party departure and leader succession. A refused admission leaves the hold intact. Party chat,
Guild chat and private messages use the World relay and reach every Channel of the World.

## Load

Command 6251 `CHANNEL_LOAD`: the payload is empty; any byte is `REJECTED` with an empty result.
The `ACCEPTED` result lists every current Channel of the caller's World with its online count:
one count byte (`0..=64`), then per Channel its 16-byte ChannelId and a big-endian `u32`,
strictly ascending by ChannelId (at most 1,281 bytes). The count is read from the durable current
sessions (`session_state` 1 or 2) of each Channel (a house session counts on its origin Channel),
so it is never stale and never invented; a World with more than 64 Channels lists the first 64. No
capacity, "full" state or full-Channel refusal exists yet; the window shows the count only.

## Channel on social rows (capability 513)

`SOCIAL_CHANNEL_V1` requires capabilities 40, 42 and 258. A session that selects it receives the
member's current ChannelId (16 bytes) on Party member entries (field 7; never on an invitation),
on Online VIP presence entries (field 6; never on Offline or Unconfirmed) and on Guild roster
members (field 5, UUID). The server omits the field for a session without 513, so existing peers
keep their exact wire shape. The VIP presence payload limit grows to 16,200 bytes.
