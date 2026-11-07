# Raid Schedule V1

Capability 149 routes read-only command 171 on bundle-booted Worlds. Raid firing, waves and cleanup are
server driven by the game node's raid scheduler, and no client command can start,
advance or cancel a raid. Commands 172 and 173 remain reserved and unrouted.

## Query

Empty payload. Asks for the current channel's raid schedule as seen by the
admitted session.

## Query result

Protobuf-style fields: raid key (1, UTF-8), running (2, bool), committed action
count (3, uint32), and active durable creature count (4, uint32). All fields are
absent when no firing has run on the admitted channel.

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
