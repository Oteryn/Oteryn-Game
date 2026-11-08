# Personal map marks V1 (M16)

Capability 449 requires NPC service 3 and World map 18. Command 733 is a read-only query; it creates no marker, quest progress, item, receipt or gameplay input. The server offers it only when native map-mark content is installed. The Character Quest writer is the sole durable producer.

## Request

Empty. Reject any byte. Current controller, connection, actor lease, Character revision and runtime scope are re-proved. Unavailable or stale authority yields REJECTED with no payload, never an invented empty set.

## Reply

ACCEPTED carries the complete current Character/current World set, ordered by unique source key: little-endian u16 count (0..256), then each key and label (u16 UTF-8 byte length followed by bytes), u16 x, u16 y, signed i8 floor (-15..0), u8 kind (1 = Cross). Key 1..128 bytes; label 1..256 bytes and at most 128 characters; neither may be blank or contain control characters. Maximum 100866 bytes. Strict decoding refuses duplicate/out-of-order keys, unknown kinds, invalid UTF-8, truncation and trailing bytes. A rejected reply is empty.

These are immutable display facts, not client write authority. The client refreshes after admission and at a bounded five-second cadence; queries do not reset gameplay idle time. Local editable map marks stay separate. Server marks are never written to the local World-only exploration file, preventing transfer between Characters. A scope switch clears the projection until fresh facts arrive.
