# Carried world callbacks

`content/quests/world-callbacks.json` also accepts `trigger: "use_carried"`
for an ordinary native item USE. This uses the existing Character quest
transaction and item replacement receipt; it does not create a separate
inventory or quest writer.

The callback must have an empty selector, one `item_on_target` source with
count 1 and a different portable replacement, nonempty storage requirements,
and a `carried_use` object:

```json
{
  "actor_region": {
    "minimum": {"x": 100, "y": 200, "floor": -7},
    "maximum": {"x": 110, "y": 210, "floor": -7}
  },
  "time_window": {"from_minute": 1080, "to_minute": 360}
}
```

Regions include both edges and use native negative floors. Time windows
include the start and exclude the end, wrapping midnight when the start is
later than the end. Endpoints must be distinct and below 1440. CLOCK-1 uses
2500 real milliseconds per Tibian minute and the default UTC epoch. The
server records the World clock fact; the client supplies no time or position.

Levers, passages, teleportation, offerings, native rules and world-target
replacement are excluded from this trigger. `return_on_failure` remains an
explicit required field. Storage requirements apply to locked durable tracks;
an unset track is -1. A cosmetic transform can preserve the story stage without
writing charge counters.

The dispatcher selects an exact physical instance from the current pinned
backpack tree and binds the original handle or definition selection to the
command. The durable owner checks that instance, area, period, content/reset
fence and all storage guards in the same transaction as the replacement.
Area, period, replacement and storage terms enter the quest definition hash.
Presentation strings do not. Existing callbacks without `carried_use` retain
their prior hash encoding and receipt serialization.

Committed replay acknowledges the original selection before resolving the
transformed source or checking a later clock fact. A changed selection under
the same command conflicts. An ambiguous attempt retains its original context
and reserves its source; the existing session resume gate and native ingress
receipt reconcile that attempt before accepting more input.

`questpc3_threatened_light.py` is a source-backed producer using this shape.
Its PostgreSQL dispatch test also covers lost acknowledgement, committed
replay after a clock change, and conflicting source selection.
