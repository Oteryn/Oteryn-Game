# Shared world-change integration

The production owner is `crates/gameplay/durability/src/durability/world_event.rs`.
It extends the existing reset-bound world-event store rather than introducing a
second event system. The PostgreSQL tables are introduced by migrations
1320–1322; keep those numbers in QWORLD's coordinator allocation.

## Author a world change

Add a source-qualified definition to
`content/encounters/world-changes/runtime.json`. Preserve the three required native
definitions: `fury_gates`, `grimvale`, and `their_masters_voice`. Their schedules
reuse the existing daily decision owner. A generic definition has this shape:

```json
{
  "key": "example_invasion",
  "schedule": {"kind": "rotation", "period_epochs": 3, "active_epoch": 1},
  "counter": {
    "creature": "oteryn:creature.rat",
    "kills": 100,
    "initial_active": true,
    "active_on_goal": false
  },
  "sources": ["replace-with-the-source-for-this-rule"]
}
```

This is a schema example, not a qualified title rule or an instruction to install
its invented threshold. Establish the actual title's schedule, creature and
threshold from source evidence before authoring it.

Other schedules are `at_reset` with `chance_permille` from 0 through 1000 and
`calendar` with `months` and `days` arrays. An empty calendar array matches every
value in that dimension; both arrays cannot be empty. Rotation uses the committed
reset epoch modulo `period_epochs`, rather than elapsed wall time. Calendar uses
the reset's persisted UTC activation date. All decisions are frozen for that
occurrence, including the catalogue used to produce them. Changing content does
not rewrite an active occurrence; the new catalogue applies at the next reset.

Counters belong to one world and reset occurrence. They start at zero for each
new occurrence and count matching native creature death settlements through the
existing receipt owner. Replaying a settled death does not increment again. At the
threshold, the counter saturates and the state becomes `active_on_goal`. The
initial state is the schedule result combined with `initial_active`; counters
are not a cross-reset cumulative quest tally.

For a rule discovered during bootstrap, use
`DurabilityRoot::bind_world_change_definition` with reconciled authority, world,
and the active epoch. Rebinding the same key with a different definition is an
error; binding must not reset existing progress. Prefer the authored catalogue
for rules that should recur automatically at every reset.

## Consume the state

- Quest transitions: set the optional `world_change` field to the definition key.
  The quest writer checks it inside the fenced transaction and refuses an
  inactive state before awarding progress.
- Item/step world callbacks: use the same optional `world_change` field. The
  callback consumption owner rechecks it with the current world context.
- Paid travel: set the optional `world_change` field on the route correction in
  `crates/gameplay/content/src/content/npc_catalogue/travel_routes.json`. The
  quote and payment paths check the condition; the binding is part of receipt
  identity.
- Dialogue and previews: call `DurabilityRoot::world_change_active(world, key)`.
  This is an advisory read. A later mutation must use its transactional gate,
  rather than trusting an earlier preview.
- Native map and spawn projections: use
  `DurabilityRoot::world_change_snapshot(world, epoch)`. Channel boot already
  supplies this persisted snapshot to the existing daily physical projection
  owner for Fury Gates, Grimvale and Their Master's Voice. A new change's map or
  spawn behavior must connect its existing physical owner to the durable state;
  registering a boolean alone does not create a new overlay or spawn policy.

A missing key is inactive. A stale epoch, corrupt payload, or missing decision
for an already-bound occurrence fails closed. Do not recover by rolling a new
choice from the boot clock. Legacy calendars may upgrade once from the original
reset activation date.

## Focused evidence

`qworld_postgres_occurrence_dispatch_and_restart` exercises real reset dispatch,
kill settlement/replay, both counter directions, generic quest/travel gates,
wire responses and recovery refusal. The latest recorded run took 57.98 seconds.
`qworld_postgres_fury_grimvale_voice_physical_restart` exercises persisted active
and inactive native overlays, spawns and restart; it took 122.43 seconds.
These qualify the three titles' world-state paths. Generic quest/travel fixtures
do not establish complete title-specific NPC stories.

See `QWORLD-CONTRACT-NOTES.md` for missing source facts and validation history.
Use the official bundle compiler pin-refresh/pin-check tooling after changing the
catalogue, focused crate formatting/check/Clippy, the duplicate-migration check,
and only the relevant PostgreSQL cases. Do not run the full PostgreSQL suite.
