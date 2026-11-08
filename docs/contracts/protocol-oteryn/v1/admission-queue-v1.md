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
The authenticated production owner hashes the verified grant as the retry key and
keeps at most 128 wait leases across its runtime scopes. Positions are FIFO within
one actual runtime scope, retries renew a six-second lease without reordering,
and successful reservation removes the entry. The retry interval is two seconds.
Expired heads cannot block a live waiter, and another channel cannot consume its
position. A full wait list falls back to the legacy 1115 refusal.

The native TCP adapter preserves the validated detail through the retry owner to
the login wait text. Queued retries use the server interval, remain limited to 16
waits and the existing grant lifetime, and are cancellable per attempt. A legacy
1115 without queue metadata retains its single retry. No client position estimate
is substituted when metadata is absent. Runtime reservation, real connection
writer, real Session reader and the HTTP/Gateway/TLS login caller have tests.
