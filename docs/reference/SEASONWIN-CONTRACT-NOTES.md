# SEASONWIN: calendar windows and world rewards

Base: `e2152d050038cecdb3728c222af2ac184e1cac5a`. Migration: 2872.
The existing world-event query/announcement protocol is reused; no new capability
or command is required. This is the owner's accepted generic-owner/API slice.

## Source and scope

Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
`data/XML/events.xml:3-21` defines dated rate events (the examples are not seasonal
reward rules). `src/game/scheduling/events_scheduler.cpp:85-93` publishes active
event names: `if (now >= event.startTime && now <= event.endTime)` and
`activeEvents.emplace_back(event.name)`. Its `:336-352` parses start/end times.
`data-global/lib/core/storages.lua:1784,1786` has only
`AnnualAutumnVintage = {}` and `DemonsLullaby = {}`. Neither is a participation
producer. The accepted Oteryn calendar uses half-open server-save windows at
06:00 Europe/Berlin, consistently with the existing seasonal/reset owner.

The owner supplied these facts from MediaWiki API captures, without a new fetch:

| Rule | Source |
|---|---|
| Autumn October 1–8 and 17–24; 40 loads = 4000 bunches; potion health/mana +33% for seven days from success | TibiaWiki Annual Autumn Vintage and Annual Autumn Vintage/Spoiler (fetched 2026-11-17) |
| Emilie/Gruffy status: “As of now, X loads have been processed. Keep it up!” or “As of now, no berries have been processed...” | TibiaWiki Annual Autumn Vintage/Spoiler (fetched 2026-11-17) |
| Lullaby May 7–14; 500 pleased infant returns; successful-world XP +50% until May 21 | TibiaWiki Demon's Lullaby and Demon's Lullaby/Spoiler (fetched 2026-11-17) |

The owner's scope decision makes success immediate on crossing the threshold
inside the window. Autumn's October ranges have separate counters; each range
and year starts fresh. A server producer supplies a stable source receipt ID,
world, active reset epoch, event and positive amount. Reusing a receipt with a
different event or amount is rejected. Replaying an accepted receipt returns its
original outcome without adding progress, including after closure. New progress
outside an admitted window is refused. Progress saturates at the success goal.
Autumn's duration is 604800 elapsed seconds from success; the second October
fixture crosses the daylight-saving transition. Lullaby's expiry is the fixed
May 21 server-save instant, independent of the success date.

Round creation shares the existing reset finalization transaction. Runtime can
read the three owned tables but can mutate only through the guarded function.
The active reset row is locked before progress; a receipt advisory lock and a
round row lock serialize duplicate sources and simultaneous goal crossings.
Success and expiry are persisted once; rate reads select the active percentage
instead of modifying a cached multiplier. XP scaling follows receipt replay and
excludes exact quest obligations. Potion scaling follows the existing committed
item-use effect path and scales the deterministic draw before the vitals clamp.
The existing client world-event query adds active seasonal/reward announcements.

## Accepted follow-ups

No winterberry/cellar-mould or Demon Mother/infant participation handler was found
in the base Rust production paths. The existing Emilie starter dialogue is not
a berry producer. The owner explicitly accepts `record_progress` plus its tests;
cellar delivery/treading, the ten-bunch/24-hour entitlement, infant requests and
returns, and individual achievements/rewards remain participation follow-ups.
Emilie/Gruffy `status` uses the durable counter wherever their existing admitted
NPC dialogue is available; this change adds no NPC placement or content input.

## Qualification

Mounted PostgreSQL case:
`seasonwin_calendar_progress_buffs_restart_and_exactly_once_postgres`.
It drives the real reset finalizer and producer API for both Autumn ranges and
Lullaby, before/inside/after windows, concurrent duplicate receipts, conflicting
receipts, one success transition, seven-day versus fixed-date expiry, and a new
DB-only owner's recovery. Clock fixtures affect business time only, with all
publication, reset, recovery and runtime guards retained.

Local PostgreSQL 18.6 execution: one case passed in 70.40 seconds (all three
event windows); `migration_numbers_are_unique_and_match_embedded_migrator`
also passed. The case is mounted by game-server and reused by the smaller
durability lib test target for focused qualification.
