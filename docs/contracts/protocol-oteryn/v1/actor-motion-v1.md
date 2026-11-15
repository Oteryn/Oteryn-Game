# Actor motion v1

Optional capability 502 requires visibility 6, paced movement 13 and appearance
25. Only selected sessions receive optional visibility-entry field 14 (length-delimited,
exactly 48 bytes). Objects never carry it. An absent field means the presentation source is unavailable; clients do not predict it. Unselected payloads are unchanged.

The payload is little endian: u64 last translation revision, u32 step duration
in milliseconds, u32 active mount look type (zero means dismounted), then eight
u32 adjacent-step forecasts in N,E,S,W,NE,SE,SW,NW order. Durations are bounded
by the diagonal pacing receipt limit. A changed translation revision with zero
duration is a teleport observation, including an adjacent teleport; it jumps.
Turns, health and riding changes retain the last translation revision.

Only the observer's own entry has forecasts. A nonzero forecast proves an
ordinary adjacent destination and its duration at the observed position. Zero
means blocked or not qualified; the client waits for the server. Forecasts
are advisory: all commands still traverse the real movement owner. The client
permits one pending prewalk and reconciles to every authoritative reply. A
snapshot or changed incarnation cancels animations and pending predictions.

## Intent

Command 6242 is exactly `[8,0]` (dismount) or `[8,1]` (mount), using the existing
ordered, replay-fenced command ingress. Mounting requires a currently validated
selected mount and the current controlled actor/session. This volatile state
does not change the durable cosmetic selection. Fresh actors start dismounted.

## Reply

The ordinary command status with an empty payload. A rejected command does not
change riding. The authoritative mounted look arrives through visibility for
the player and every observing session that selected this capability.
