# T003: optional admission capacity queue detail

`ProtocolError` fields 16 and 17 add advisory queue information to the existing
N8 `ADMISSION_CAPACITY_EXCEEDED` (1115) refusal. Reserved fields 6–15 stay reserved.
The envelope remains pre-admission, generation zero, transport-fatal, with all
command/sequence correlations zero. The connection ends after the refusal.

Both fields are absent for the legacy refusal. Otherwise both must be present:

- `admission_queue_position`: 1–128, assigned by the authenticated admission
  runtime's bounded waiting list. It is never a client estimate.
- `admission_retry_after_ms`: 1–30000. The client must also respect its existing
  grant lifetime and cancellation; this value never extends a grant.

Zero, duplicate, overflowing or partially present queue values fail closed.
Queue fields on another error class or correlated/non-transport-fatal errors are
invalid. Older readers skip these optional fields and retain the existing 1115
retry behavior. No route, world/channel assignment, resource budget, session
identity or admission permission is granted by the metadata.

The producer must authenticate the grant and preserve the existing N8 refusal
precedence before revealing a position. Positions refer only to that runtime's
waiting list; entries need bounded leases and removal on successful reservation.
The wire codec and real client-session reader are implemented first. The real
server waiting-list producer and the position display remain tracked as T003
PARTIAL until their callers and tests are wired.
