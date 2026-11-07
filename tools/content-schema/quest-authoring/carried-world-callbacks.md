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
later than the end. Endpoints must be distinct; the start is below 1440 and
the end may equal 1440. `[0, 1440)` explicitly admits every minute of the day,
while a start of 1440 and observed minutes outside the day remain invalid. CLOCK-1 uses
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

## Applying a carried tool to a world object

`trigger: "use_carried_on_target"` uses the same transaction and predicates,
but requires the usual world selector. Native USE_WITH supports both an
instance handle and a pinned hotkey definition. The dispatcher resolves the
selected object against the current map/content/reset fence, checks its
revision and reachability, and retains its position/key/revision beside the
exact carried source. The original source choice, encoded target and expected
revision enter a domain-separated 32-byte intent digest.

Target selector terms enter the definition hash. Optional minimum level enters
the binding only when present; existing self-use bindings and context JSON stay
unchanged. A target-only context cannot authorize self-use, or conversely.
Levers, passages, offerings, teleportation, native effects, shrine rewards,
experience and target appearance replacement are excluded from these carried
triggers. `return_on_failure` must be false. The target is read-only: a personal
repair must not increment a shared map object's revision.

A target recipe may retain the charged definition by replacing it with itself.
This proves exact custody through the existing receipt owner without removing
the tool. A terminal recipe can restore its empty definition. For a bounded
five-charge source, author five exact counter states instead of adding a new
expression language: counter5->4 through1->0, with a distinct per-player flag
for each target. `questpc3_threatened_barriers.py` uses this shape for all fifteen
source positions. Repeated targets, changed-target replay and wrong periods
must grant no additional progress or value.

`questpc3_threatened_rakes.py` uses identity replacement with an all-day
window for five once-only scratching flags. The source instance UUID and
quantity stay unchanged; the existing transaction owner records the interaction.

A carried-target selector may instead name one positive appearance, with no
fixed position. Resolve it through the selected authoritative base stack,
including placements absent from the sparse action/unique attribute index.
An absent replacement burns exactly one unit of the selected carried source
through the existing locked input owner. Ordinary world USE and carried self-use
retain their earlier selector and replacement restrictions. The delivery producer
uses this for two pallet orientations and the weeping stone; their donor item
types have no configured decay, so no world mutation is authored.
