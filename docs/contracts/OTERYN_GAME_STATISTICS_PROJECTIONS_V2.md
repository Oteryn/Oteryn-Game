# Game statistics projections V2: owner-only online intervals

Status: **ACCEPTED**, owner 2026-11-07, PUBPROJ-D, reconciling Platform
`OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md` revision 2. Initial scope is
qualified testing/preproduction; producer completion and activation are separate.
The world/day kill family is specified by its companion Game contract. No
public online-time ranking, account linkage, journal or admin feed is accepted.

## Authority and source

Game owns committed online transitions and retained interval facts. Platform
owns read models and must independently prove current Character ownership and
its privacy floor for every interval read. Raw intervals and derived schedules
are owner-only and private/no-store, including statistics UUIDs. Browser ids,
names or a fresh projection never prove ownership. A transfer suppresses
pre-transfer history for both owners; no ownership history is inferred.

The source predicate is the committed Game ONLINE predicate: an eligible
Character admission guard held by the same World's session, session state 2,
current transport present and no control-loss epoch, under an eligible public
profile. Capture runs at the authoritative transaction boundary, independent
of the publication flag. It is not reconstructed from the ten-second presence
scan. Deferred source triggers observe the final committed predicate so an
intra-transaction Channel handoff creates no false split. A committed loss
closes at that loss transaction; a crash-left ACTIVE session stays open until
its independently fenced terminal/lease-expiry release commits. Silence or a
new assignment alone supplies no end. World and ownership changes split the
interval at their authoritative boundary. Intervals never overlap.

Every new interval mints and persists an independent UUIDv7 `statistics_id`;
it is neither a session id nor a derivation of any admission identity. Only
CharacterId and this statistics identity are subject keys. Private source rows
retain the ownership binding needed to recognize transfers; it is erased with
the payload and never transmitted. Future admin analytics may aggregate these
same durable intervals only under their own separately accepted authority.

## Exact wire

Operation `PublishCharacterOnlineSessionV2`, compiled POST
`/internal/v2/game-public/character-online-session`, ceiling 2048 bytes.
V2 envelope and transport are Game public projections V2. Ordered members:

`contract_version: 2, operation, source_authority, character_id, statistics_id,
projection_epoch, projection_revision, source_observed_at, sessions`.

All ids are canonical lowercase non-nil UUIDv7. Epoch/revision are positive
canonical decimal uint64 strings. Times are canonical nonnegative decimal
Unix-second strings (body times strictly positive), floored from persisted
milliseconds. Exact member sets, compact UTF-8, no escapes/BOM/null/unknown or
duplicate members, no trailing newline, at most five nested containers.
`sessions` is 0..1 records. A record's exact ordered members are:

`world_id, started_at, ended_at`.

`ended_at` is `[]` if no end is proven or `[exclusive_end]` otherwise. Require
`started_at <= exclusive_end <= source_observed_at`; zero-length closed
intervals are valid. Open intervals supply no completed duration and no proof
of current online status. Never extrapolate or clamp an unsafe end.
`sessions: []` is a tombstone/suppression, not an empty or zero-length interval.

Reuse the dedicated purpose-bound public-projection mTLS identity, configured
authority, V2 acknowledgements and `/internal/v2/game-public/watermark` with
family `character_online_sessions`. No session/account/transport/lease/Channel,
node, position, name or command binding is transmitted. Existing V1 routes and
its closed watermark enum remain unchanged. Connect 1s, handshake 2s, whole
exchange 3s, response 256 bytes; one in-flight snapshot per family. Refusals,
backoff, holds and fresh-only serving follow the accepted V2 common profile.

## Durability, completeness and recovery

An authoritative transition, source interval, revision and outbox commit
atomically. Rollback advances none. Exact retries keep their original content;
ACK deletes only through the acknowledged pair and preserves a concurrent
successor. Source capture remains active while publishing is disabled.
Publication is disabled by default through
`[platform.public_projections] character_online_sessions_enabled = false`,
subordinate to the disabled-by-default master switch.

Missing historical starts at installation are explicitly unknown: a private
coverage-pending record holds family freshness until an authoritative offline,
World or ownership boundary establishes new coverage. Existing sessions are
never assigned an invented migration-time start. Unsafe clocks retain a
private unavailable marker and stall completeness pending independent source
reconciliation, rather than replacing the interval by guessed duration.

The watermark cannot pass pending source coverage, unacknowledged intervals,
retention tombstones or corrections. An open but faithfully published interval
is not itself a delivery gap. Freshness is the family watermark, never the
observation time of an old closed interval.

The existing stopped-node operator resync accepts
`--family character_online_sessions`; `all --raise-epoch true` retains the
same external `F_pub` fence. Rebuild enumerates every retained statistics
subject, including payload-erased fences and tombstones. The existing 60s
transaction deadline remains the gate; exceeding it rolls back and requires
qualification, not an unversioned portal paging API.

## Retention and suppression

Retain closed interval payloads for 30 days after authoritative end. An open
interval has a 30-day ceiling after start; expiry publishes a tombstone and
erases payload, while a private pending coverage marker keeps its unknown end
unavailable. It is never auto-closed. New coverage starts only after a proven
boundary. The 400-day retention in the proposal belongs to World/day kill
aggregates; it does not expand raw interval retention.

A bounded production maintenance pass erases at most 32 expired payloads per
transaction and atomically queues their higher-revision tombstones. Minimal
opaque subject fences survive so replay/resync cannot resurrect erased data.
Restrictive profile/deletion/ownership transitions enumerate and suppress all
retained exported subjects; a source snapshot never overrides Platform's
independent immediate ownership/privacy fence. Suppressed historical payloads
are never automatically republished to a later owner. Platform independently
enforces serve-time expiry, purges payload/derived caches within 24h and
applies the accepted backup purge policy. No payload-bearing operational logs.

## Shared evidence and source adaptation

`tools/contracts/generate_online_session_fixtures.py` regenerates open, closed
and withdrawn byte fixtures. Typed and PG cases cover exact encoding, unsafe
ends, source rollback, ACK race, committed transition boundaries, retention,
unknown initial start and independently fenced crash closure.

Donor reference evidence, adapted rather than copied:

- Canary `99902524e052f37574194466c2949c576e4ab269`,
  `src/io/functions/iologindata_save_player.cpp:320,359-360`: persisted last
  logout and online seconds measured from a saved login boundary.
- Crystal `ff7ede593c69d4c658b382c97443e8155926924a`,
  `src/io/functions/iologindata_save_player.cpp:200-206,364-378`: retain a login
  checkpoint and advance it with a successful save. Its negative-time clamp
  is deliberately superseded by this owner's explicit unsafe-clock denial.

The accepted owner ONLINE predicate, independent statistics UUID, atomic
outbox, owner-only privacy and 30-day erasure are new Game obligations; legacy
cumulative donor counters are not imported as interval authority.
