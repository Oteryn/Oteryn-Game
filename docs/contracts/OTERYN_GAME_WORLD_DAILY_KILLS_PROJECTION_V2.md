# Game World daily physical kills V2

Status: **ACCEPTED**, owner2026-11-07 (PUBPROJ-D), reconciled with Platform's
`OTERYN_V2_STATISTICS_FAMILIES_CONTRACT_PROPOSAL.md` revision2. Qualification and
activation are separate; the per-family publishing flag defaults false under
the public-projections switch. Game owns source capture, classification,
retention, source completeness, revisions/outbox and operator replay.

## Count and source

Exactly one physical occurrence counts, independently of damage-credit,
party/Character multiplicity, XP, loot, Bestiary saturation and corpse admission.
The identity is the existing sealed `CreatureDeathOccurrenceKey`, including its
actual tagged World/Channel-or-Instance/ownership generation/actor identity.
No synthetic UUID or principal-derived key replaces it. Respawn then another
physical death is a distinct occurrence. Player deaths and despawns are absent.
Native master-owned summons, Soulpit summon tags and familiars lack the eligible
ordinary-physical classification and are excluded; otherwise native World and
Instance physical deaths, including bosses, share the World total.

Boss classification is frozen from the active native policy, qualified boss
catalogue or actual Soulpit boss tag at the lethal owner turn. It is disjoint
from the ordinary-monster class. The first native capture freezes the UTC wall
clock under that same owner turn; retained actor/corpse replay keeps that exact
observation instead of moving the occurrence to a later day.

The owner admits the physical death without waiting on statistics SQL. A
bounded4096-entry memory buffer retains pending occurrences. The autonomous
owner flushes at most256 per transaction at a1000ms cadence, after releasing
runtime, spell, attack and queue locks. Retries retain the original events.
Buffer/clock/identity failure marks a source gap and never blocks combat or
silently becomes known zero. The source key and time are not inferred from a
credited Character receipt.

## Durable data and honest completeness

The durable source retains one dedup row per physical occurrence and a private
aggregate keyed by `(world_id, day_start, creature_key, boss)`. The same batch
transaction performs occurrence insertion, aggregate increments, per-subject
revision and outbox touch. Epoch and sorted World/day locks precede counters,
SUM and derived snapshots, so two Channels cannot replace a newer World total
with a snapshot computed before the other's increment. A conflicting repeated
occurrence aborts; an exact duplicate adds no count. u64 overflow aborts the
whole batch, never wraps or saturates.

Assignment generations establish durable coverage before gameplay. Source cuts
advance only through the earliest unflushed occurrence, excluding same-clock
future captures. An unclean source ending persists the last proven cut as a gap;
an unflushed crash window is unavailable, not reconstructed from reward receipts.
A family watermark cannot pass a live source cut or unresolved retained gap.
Restoration/replacement cannot erase those fences. Operator repair requires
actual authoritative evidence; silence and a restart never prove a clean cut.

A complete day requires its UTC end to have passed, a known World coverage
floor, and every intersecting source-generation cohort to have a proven cut
through its part of that day without a gap. An active zero-count World can emit
an explicit complete zero only from those same source cuts. Existing Worlds
start coverage at installation; their unqualified first partial UTC day is
withdrawn/unavailable. A truly new World's first post-installation assignment
proves there were no earlier Game actors, so its first day is qualified from
that UTC day start. Closed/offline absence is not invented as a zero baseline.

## Wire

POST `/internal/v2/game-public/world-daily-kills`, operation
`PublishWorldDailyKillsV2`, maximum2048 bytes. Ordered exact envelope:
`contract_version:2, operation, source_authority, world_id, day_start,
projection_epoch, projection_revision, source_observed_at, days`.
`world_id` is lowercase non-nil UUIDv7. `day_start` is a nonzero canonical
unsigned decimal-string Unix UTC midnight divisible by86400. Every other time
is a canonical decimal-string Unix second; epoch/revision are positive u64
strings. No body field is null or implicitly optional.

`days` contains zero or one record, ordered members:
`day_end, monsters_killed, bosses_killed, complete`. End is exactly start+86400,
counts are u64 decimal strings with zero allowed, complete is boolean. A complete
end is no later than the observation. `days:[]` explicitly withdraws unavailable
or expired state; it never means a zero count. All records are complete
replacement snapshots; equal pair/equal content retries are idempotent.

No creature key, raw occurrence, CharacterId, AccountId, session, ChannelId,
position, damage, loot or private topology crosses this public wire. V2 mTLS,
acknowledgements, backoff, external epoch fence and V2 watermark are the shared
accepted public-projection profile. V1 routes and its closed family enum remain
unchanged. A malformed response, even HTTP200, never clears the outbox.

## Retention and replay

World/day payloads and private dimensions expire400 days after UTC day end.
Bounded maintenance writes higher-revision tombstones and erases eligible
payloads; minimal World/day erasure fences remain. Dedup records expire under
the same retained window, and age/erasure checks reject later resurrection.
Unacknowledged tombstones retain their outbox; maintenance never truncates it.
Operator `public-projection resync --family world_daily_kills` rebuilds and queues
the retained subject inventory, including withdrawals, under the shared public
epoch. Full retained-set capacity must be qualified before activation.

## Later administrative analytics

The public wire sums away creature keys; the durable World/key/day/class
aggregate remains available to the future ADMINAN producer. It does not expose
an administrative route. Player-death analytics are a separate dimension over
the existing death receipt's World/day and its private primary-cause
classification (migration1352): native creature content key, literal `player`
for any player killer, and literal `unknown` for no attributable killer. Damage
contributors do not substitute for a final killer. G4 internal CharacterId is
never an admin-feed killer key. Invalid/quarantined cause is unavailable and is
not relabelled `unknown`.

The native Creature loader reserves the exact keys `player` and `unknown`;
its guard test proves both are rejected. This prevents a future administrative
aggregate collision while preserving all valid namespaced content identities.
