# Public World totals V1 (PLAT-WIRE-1)

Status: proposed Game → Platform wire; Game producer implemented, Platform acceptance
and listener rollout pending. Family `world_totals`, contract version 1.
This supplements, without changing, `world_online` in
[public projections V1](../../OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md).
Admin-control V1 / ADR-0023 remains private and is not a public statistics source.

## Delivery and identity

`PublishWorldTotalsV1` pushes to `POST /internal/v1/game-public/world-totals`,
using the existing public-projection purpose, descriptor, TLS 1.3 mTLS,
3-second exchange bound, 256-byte response and response vocabulary from public
projections §3/§6. No inbound Game service or page-render query is added.
Request bound: 1024 bytes. The existing public-projection enable flag gates it.
Platform must accept the operation and the `world_totals` watermark family before
enabling that flag with this producer. A definite refusal raises an alarm and holds
this lane for 5 minutes; transient failures back off from 1 to 60 seconds.

The exact closed object is in `publication.schema.json`. Wide values are canonical
decimal strings (0..9223372036854775807), positive epoch/revision; World is the
existing public UUIDv7. All times are Unix seconds. There are no names, Character
IDs, account IDs, Channel IDs, session identifiers or player-supplied text.

## Meaning

Each row is one **sample**, even when its counts equal the previous sample.
`source_observed_at` is the database snapshot time, not delivery time.
`online_players` counts active Characters with an eligible admission guard on their
own World held by a session in state 2, with current transport and no control loss.
It counts each Character once across Channels. It does not apply the public name,
vocation or 4096-entry listing filters: the count has no list truncation limit.
`player_deaths` is the cumulative count of committed DEATH-0 receipts whose
`death_world_id` is this World, including PvE and PvP, independent of later
Character moves or retirement. It is not a kill count or a count of unique victims.
A rolled-back death adds nothing; occurrence replay adds nothing twice.

Migration 0977 backfills death totals from existing receipts and maintains them in
the receipt transaction. Sampling copies the count and online predicate at one
statement snapshot. Known Worlds are the union of Character Worlds, recorded death
Worlds and prior sample heads; this includes known empty Worlds with zero online.
A never-populated World needs a Game root/death or a prior head to be known here;
Platform must not turn absence into zero. The worker scans Worlds in UUID pages of
32, waiting 10 seconds between completed sweeps. Database contention or a large
fleet can delay samples: gaps are missing observations, never zero or interpolation.

## Durability, replay and restore

Per-World head locking assigns increasing revisions and admits at most one sample
per 10 seconds on the database clock. A shared database therefore deduplicates
multiple Channel publishers. Samples persist **before** sending; their complete
payload is immutable. Each acknowledgement updates only the exact
(World, epoch, revision). Pending rows are drained oldest first, 32 per pass.
A crash before acknowledgement replays the same bytes; Platform atomically inserts
by `(source_authority, world_id, projection_epoch, projection_revision)` and
acknowledges duplicates. Historical samples must be inserted even if newer samples
already arrived; `superseded` is valid only when that exact sample is already stored
or the epoch is obsolete. Never apply latest-only replacement semantics to a series.

History is retained without automatic pruning in V1. Requeue retained current-epoch
samples through the existing operator command, while the producer is stopped:

```sh
oteryn-game-ops public-projection resync --family world_totals --raise-epoch false
```

`--family all` also requeues totals in its existing resync transaction. Stage the rebuilt
series and switch it into view only when its watermark is caught up. The operator
needs the database owner role; no public replay/administration endpoint is added.

Epoch and the external high-water fence are the existing public-projection epoch
and fence. Restore must use public-projection epoch raise/resync **before** enabling
any publisher. Old-epoch pending samples are not delivered; a new-epoch sample is
forced on the next sweep. Platform invalidates the previous epoch at the first
higher-epoch publication or watermark. Counts after restore describe the restored
Game; cross-epoch subtraction is invalid. Revisions do not reset.

## Liveness

Use `PublishPublicProjectionWatermarkV1` on the existing watermark route with
`family: "world_totals"`. Its `complete_through` is strictly before the earliest
unacknowledged sample, and no later than the oldest latest sample of any known
World. A known World without a current-epoch head withholds the watermark. Thus a
stopped sampler cannot be hidden by an empty queue. The watermark is sampled-data
coverage, not a claim that every intervening presence transition was observed.
Reuse the 3-second watermark period, 30-second stale bound and clock checks of
public projections. Missing/stale data is unavailable; zero is valid only in a
received sample. Platform may chart online samples and derive death deltas only
within one World and epoch; a negative delta is a rebuild/error signal.
