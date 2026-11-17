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
| 5 | event | boss, raid or event participation (an instance scope) |
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

The fresh admission revalidates the switch inside the admission receipt insert (migration 2858):
the first admission in a World sets the baseline, the same Channel is not a switch, a different
Channel inside the cooldown or with a lock the database can prove (logout block, a skull into a
Channel that is not Open PvP) is refused, and a refused or failed admission uses no cooldown. A
retry of the same admission cannot advance the guard twice.

## Party across the switch

The terminal release of an accepted `CHANNEL_SWITCH_REQUEST` keeps the Character in its Party and
holds the membership for two minutes (migration 2859). The committed admission into the World
takes the hold back; a refused admission leaves it. An expired hold without a current session
leaves the Party through the World Party expiry drain, as an ordinary logout would. Party chat,
Guild chat and private messages use the World relay and reach every Channel of the World.
