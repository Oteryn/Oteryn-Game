# Inventory capacity V1

Optional capability 508 requires item view 4 and visibility 6. It carries the
own character's carrying capacity and free capacity, which Tibia Global shows
as `Cap` in the inventory panel.

The server sends it as a header-only domain-1 type-2 visibility delta whose
only other field is optional field 7 (length-delimited, at most 12 bytes):
field 1 `capacity` and field 2 `free`, both canonical varint `uint32` in
centi-ounces (1/100 oz), in that order, each exactly once. `free` is the
capacity minus the carried weight, saturating at zero, and never exceeds
`capacity`. A delta with field 7 has no enter, update or leave entries and
nothing after field 7; it advances the domain-1 revision like any delta.

Capacity is the value item pickups and moves into the character check: it is
derived from the current level, vocation and capacity allocations, and the
carried weight covers every equipped item and the main backpack tree. The
server reads both through the session's current character item fence and
sends a new value only when either changed since the last one it sent on this
connection; a new connection sends the current value again. An unavailable
read (busy holder, unknown weight, stale fence) sends nothing. Observations
use a single read-only repeatable-read snapshot, including the recovery,
session, scope and node checks, without mutation locks. Each connection
observes at most once per five seconds, including unavailable reads; visibility
ticks do not issue additional capacity reads during that interval.

The client shows `Cap: <free / 100>` (whole ounces). Without the capability
the field is refused and nothing is sent; unselected payloads are unchanged.
