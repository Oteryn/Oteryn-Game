# Raid Schedule V1

Capability 149 reserves commands 171, 172 and 173. It is not offered: raid
firing, waves and cleanup are server driven by the game node's raid scheduler,
and no client command can start, advance or cancel a raid. Until the capability
is offered every command below is refused with no result payload.

## Query

Empty payload. Asks for the current channel's raid schedule as seen by the
admitted session.

## Query result

Reserved, at most 8192 bytes. The encoding is fixed when the capability is
offered.

## Subscribe

One byte: 0 unsubscribes, 1 subscribes the admitted session to raid
announcements on its channel.

## Subscribe result

One byte: the resulting subscription state.

## Notice ack

At most 16 bytes identifying a delivered raid notice. Acknowledgement never
changes raid state.

## Notice ack result

One byte: 1 when the notice was known, 0 otherwise.
