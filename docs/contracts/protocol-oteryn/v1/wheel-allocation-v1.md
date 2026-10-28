# Wheel allocation v1

Command 161 requires offered and selected capability 148 (`WHEEL_ALLOCATION_V1`). This is a
server-first addition to protocol-oteryn; no Canary packet compatibility is implied.

The canonical codec is `crates/protocol-oteryn/src/wheel.rs`. Requests and results are bounded
to 256 bytes. Unknown fields, duplicate scalar fields and malformed shapes are rejected.

| Request field | Wire tag | Value |
| --- | --- | --- |
| action | 1, varint | 0 query, 1 replace/save, 2 reset |
| occurrence | 2, bytes | Exactly 16 bytes; mutations require a UUIDv7 receipt identity |
| expected_revision | 3, varint | Expected Wheel revision, including 0 for the initial allocation |
| points | 4, repeated varint | Exactly 36 unsigned 16-bit values in state-slot order |

A query contains only action 0. Both mutations carry all remaining fields. Reset requires
36 zero points. Replacement preserves the current vessels; Atelier custody is separately owned.

The result has `refused` (tag 1, varint boolean), `revision` (tag 2, varint), and 36 current
`points` (tag 3, repeated varint). A well-formed refused allocation returns `ACCEPTED` with
`refused=true` and the unchanged committed allocation. Malformed, unnegotiated and stale
admission requests return `REJECTED`; unavailable or indeterminate durable operations end the
connection, allowing the same occurrence to reconcile through its retained receipt.

The server derives level, vocation, promotion, Premium and temple eligibility from their owners.
Any decrease, including action 1 that redistributes points, requires a protection-zone tile
within 10 tiles on the temple's floor. Action 2 supplies no location authority. A map without
retained temple evidence refuses decreases. W-2 qualifies the native source town; bundle
decreases fail closed without town evidence.
This command persists allocations only. Runtime HP/mana, skill and leech contributions are
deferred with all of W-FX-1 until resistance is available, as required by WHEEL0-FX-1
in the accepted Wheel delivery decision §5.2. See `WHEEL3-FINAL.md` for split evidence.
