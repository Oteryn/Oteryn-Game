# Death protection V1 (NX-8 C20c)

Capability 382, command 664. Read-only, available in bundle worlds. The current
admitted session fence is proved in the Character transaction; an old connection
cannot read another Character's receipt. The query changes no gameplay state.

## Request

Empty payload. The native client requests automatically at login and after
respawn, and on actual alive-vitals wire events so a coalesced death/respawn
still refreshes the receipt. Cached vitals republished after command replies do
not trigger queries; only one query is in flight. This response is death-correlated by the immutable occurrence, not by a
comparison of the current blessing inventory. The occurrence remains in the wire
and client model for correlation; the player notice displays consumed names. Reconnecting reads the same latest
receipt even after the pending respawn was consumed.

## Reply

`00` means no committed death. Otherwise: `01`, the 16-byte death occurrence,
a one-byte count (0–32), then for each consumed blessing a one-byte UTF-8 key length
(1–128) and key bytes. Keys are strictly increasing `oteryn:blessing.*` names.
The result is at most 4146 bytes. Consumed means exactly `blessings_before` minus
`blessings_after` from that death's immutable receipt. A death with an empty
consumed set is distinct from no death. Twist of Fate is a normal named consumed
key; the client never infers its consumption from current holdings.
