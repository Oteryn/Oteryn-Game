# Game World daily projection V2

Status: **ACCEPTED**, owner 2026-11-07 (PUBPROJ-C, G-12), reconciling Platform
`OTERYN_V2_WORLD_DAILY_FAMILY_CONTRACT_PROPOSAL.md` revision 2 on main. This
acceptance authorizes the Game producer; exact-revision cross-repository
qualification and environment activation remain separate. The family defaults
disabled under the master public-projection switch and its own family flag.

Contract profile: `oteryn-game-public-projections-v2`, family `world_daily`,
operation `PublishWorldDailyV2`, POST `/internal/v2/game-public/world-daily`.
Game owns all committed source facts; Platform owns its read model and public
serving. No page-render queries or Platform-to-Game command are introduced.
The V1 routes and closed watermark enum are unchanged.

## Calendar reconciliation

The authoritative current calendar is **06:00 Europe/Berlin**, including DST.
Migration `0562_server_save_berlin_06.sql` superseded 0148's initial 10:00 value;
it migrated persisted calendar fields and changed the accepted source helpers.
`durability/world_reset/active_epoch.rs` and `raid_schedule/calendar.rs` use the
same 06:00 calendar. This profile changes no gameplay calendar.

For each reset epoch, `effective_from` is that epoch's committed `activated_at`
in whole Unix seconds. `effective_until` is the next calendar slot strictly
after that activation, derived from **that row's calendar and calendar_revision**.
Use the native PostgreSQL `Europe/Berlin` calendar conversion. Do not add 86400,
substitute a later observed selection, or borrow another revision's calendar.
Unsupported calendar evidence withholds publication and freshness. Spring and
autumn transitions yield 23- and 25-hour intervals respectively.

## Exact wire

Reuse the accepted Game V2 UTF-8/canonical JSON envelope, dedicated
public-projection mTLS identity, configured source authority, F_pub restore
fence, ordering, immutable retries and acknowledgements. Request ≤131072 bytes;
watermark≤512; response≤256. Connect≤1s, handshake≤2s, whole exchange≤3s,
one in-flight publication per publisher/family. No null, unknown, duplicate or
missing members, string escapes, BOM, insignificant whitespace or trailing newline;
container depth≤5. Oversize or malformed input is400 with no side effects.

Ordered envelope: `contract_version`(integer 2), `operation`, `source_authority`,
`world_id`, `projection_epoch`, `projection_revision`, `source_observed_at`, `daily`.
WorldId is a registered canonical lowercase non-nil UUIDv7. Epoch/revision are
positive canonical decimal uint64 strings. Times are canonical nonnegative
whole Unix-second strings≤9223372036854775, safely convertible to signed Game
milliseconds. Source milliseconds are floored. Observation is revision assignment
time, stable on retry; it is never an ordering key or heartbeat freshness claim.

`daily` is 0..1 records. Exact ordered fields:
`reset_epoch, effective_from, effective_until, creature_key, boss_key,
history_complete_from, history, raids`. Reset epoch allows 0. Both boost keys
are required, exact native keys,1..255 ASCII `[A-Za-z0-9:._/-]` bytes; preserve case.
Unknown boosts cannot become a known empty list.

History entries are exact ordered
`reset_epoch, effective_from, effective_until, creature_key, boss_key`, at most 32,
strictly descending unique reset epochs, before the current epoch. Start<end,
end≤the next newer interval's start; source intervals cannot overlap on wire.
Coverage floor is≤current start and≥current start−2592000seconds (clamped0).
Every selection interval beginning within the declared coverage is included.
If source history is missing, unclassifiable or over-bound, raise the floor to
the earliest fully retained interval. Empty history with floor=current start
explicitly claims no older coverage; it is not an inferred complete 30-day ledger.
Epochs are not dates and missing history is not evidence of no selection.

Raid entries are exact ordered `raid_key, scheduled_at, state`, at most 256,
strictly ascending unique `(scheduled_at,raid_key)`. Key1..128 uses the same
ASCII alphabet. State is `running`, `done` or `cancelled`. Scheduled time lies
within the current interval and is no later than source observation.

Only committed native **fixed-slot** firings appear. `FiringState::Running`
becomes running; Done/Cancelled retain the firing's original slot. The native
transition writer captures fixed-slot evidence at the new firing's commit;
existing rows without that evidence remain unknown. `source_kind=3` identifies
a SourceScheduled calendar callback and does not by itself prove a fixed slot.
Random/source-scheduled triggers, future checks, consumed-but-blocked slots,
metadata Applied return values and time passing never fabricate a raid or a
cancellation. Subsecond slots and unknown current-interval classification
withhold the family instead of rounding, guessing or silently excluding records.
No participants, player/creature counts, rewards or prediction outcomes cross wire.

`daily: []` is an explicit committed no-public-view withdrawal, such as removal
of an active World pointer, and removes current/history facts. Unknown or restored
missing source is represented internally as unavailable, never as that withdrawal.
Static names/artwork resolve through separately accepted content; no catalogue
adapter or guessed labels are part of this producer.

## Durability, freshness and resync

The source change, per-World revision and outbox touch commit in one physical
transaction. Sources are the World reset calendar/active pointer, immutable daily
boost selections, and committed raid schedule firings. Producer reads remain under
reconciled authority; it never changes gameplay state, leases or admission.
Publication errors only leave work undelivered. Snapshot retries preserve content
and revision observation exactly; valid accepted/superseded ACK clears only through
its pair and retains concurrent successors. An unknown source or unacknowledged
change prevents watermark completeness advancing past it. Expired current intervals
also hold freshness until authoritative reset/selection evidence exists.

Use `PublishPublicProjectionWatermarkV2`, POST
`/internal/v2/game-public/watermark`, ordered members
`contract_version, operation, source_authority, family, projection_epoch,
complete_through, observed_at`, family exactly world_daily. Reuse the Game V2
closed family enum. Testing/preproduction freshness is 30 s with 1 s uncertainty,
no stale display. Existing bounded retry/hold and identity/route limiting apply.

`public-projection resync --family world_daily` re-derives all current source
Worlds and retained exported subjects, then re-enqueues complete snapshots.
`--family all --raise-epoch true` uses the shared public epoch and external F_pub.
Only the authorized operator invokes resync; it is not an HTTP command. A restore
must reapply the fence and current privacy/withdrawal evidence before serving.

Public boost history retains at most 30 days/32 completed intervals; public raids
cover only the current reset interval. Retain minimal subject/epoch/revision and
withdrawal inventory for rebuild; no unbounded raw raid archive is added here.
Platform keeps the latest snapshot and bounded rollback candidate, purges withdrawn
facts from serving/cache/search immediately, and serves only a fresh, qualified
generation. This aggregate family contains no account/character identities, exact
Channel, node, instance, placement or private diagnostics.
