# S17 character consent persistence and egress barrier

This implements the P1 durable preference boundary from the accepted
`docs/architecture/design/S17-CAST-DESIGN.md` on
`claude/happy-davinci-ijz3wy-mechstat` (PR #3000), revision
`956a1cdff6188381b25419251654f63a8b19536f`. It does not activate casting.

Migration 0966 owns one optional row per character. A missing row means
`allowed = false, revision = 0`; a stored row has a positive revision.
The SQL default is false, including for characters created after migration.
Preferences belong to characters, never implicitly to their account or alts.

The durable owner reads or compares and sets the preference under recovered
authority and current session, connection, lease, scope, node and character
revision fences. It locks the character root before the preference, including
when no preference row exists. A stale expected consent revision conflicts
even when the requested boolean equals the stored value. A current equal-value
retry does not advance the consent revision. Consent changes do not advance
gameplay CharacterRevision. Reads use PostgreSQL, with no process-local cache.
Each actual preference change now commits a type-10 P30D security audit event
in the same transaction, through the 0967 adapter. CAS conflicts and unchanged
retries emit no additional event.

This stored value is historical data, **not an egress authorization proof**.
The native source intersects current committed interest with player visibility;
hidden/off-screen characters do not trigger a slate. Every visible bystander
must have fresh explicit consent, otherwise the whole scene is a neutral slate.
Explicitly starting a cast supplies only the caster's own consent.

0966 also owns the global consent sequence, denial floors, invalidation journal
and short-lived worker registrations. Withdrawing consent first installs a
minimum-revision denial floor and publishes an invalidation under the same
fenced durable owner. It releases database locks while waiting for every live
worker to invalidate and acknowledge the sequence (or lose its media lease).
Only then can the preference transaction commit false and its audit event.
Workers purge delayed/shown scenes and serialize immediate Clear delivery with
final nonblocking writes. Expired workers cannot write. Their conservative
one-second local deadline precedes their two-second database lease. Pending
floors deny fresh reads even while the historical boolean still says true.
A failed operation can therefore leave a conservative denial floor; a subsequent
explicit opt-in at a newer revision restores eligibility.

An opaque one-second capture permit binds cast, target, epoch and worker privacy
sequence before asynchronous preference reads. Publication checks the permit
before applying any returned preferences or scene. An in-flight old capture
cannot restore consent after an invalidation was acknowledged, including for
characters absent from the worker's current cache. All queues, caches and
invalidation batches have explicit bounds; overflow closes casts.

Invalidation metadata and dead worker registrations have P30D expiry through a
Control-only, bounded 128-row security-definer routine. Runtime cannot invoke
expiry. Denial floors retain preference-lifetime semantics. No world lock or
character transaction is held across worker acknowledgement/network waits.

0967 owns immutable, idempotent, node-incarnation-bound P30D cast audit records.
0968 is the companion Oteryn-Platform migration: account controls, password
policy and globally serialized observer reservations. Cast-on state is an
in-memory leased session: restart yields stopped casts, while consent, policy
controls and audit survive. Production offers and configuration remain off;
client UI and measured deployment qualification are separate tasks.
