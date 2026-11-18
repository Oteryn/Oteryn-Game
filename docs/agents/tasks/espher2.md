# espher2 Elemental Spheres engine audit

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Crystal source pin
`201189f871d23597b82caf88fd611454a2ddefb8`. Quest-content audit
`2b93e3bd:docs/agents/tasks/QCX-elemental-spheres.md`.

## Chamber

Crystal `data-global/scripts/quests/elemental_spheres/actions_lever.lua:40-63`
checks the four station occupants individually. Sorcerer carries 946, Druid
947, Knight 948, and Paladin 942. Each must have reached quest stage 1.
Admission preserves the carried items. The chamber lever script at lines
89–129 sets the machine flags, transforms each pulled switch, and closes the
wall associated with the puller's station. Four active switches start the
boss and the warning/reset timers at five and ten minutes. The reset routine
at lines 63–69 restores switches and world flags. Its earlier lines remove
the walls and boss and eject the captured occupants still in the room bounds.

Main's shared station declaration requires a ground offering
(`crates/gameplay/quests/src/quest/world_callbacks/effects.rs:333`). The
QCX placement audit records the five untagged levers. Main needs carried
admission and switch coordination in the shared encounter path. Content needs
the corresponding placement bindings and wall item policies.

## Neutral Matter

Crystal `data/items/items.xml:22845-22857` gives fresh ashes 8181 a
ten-second loot window and ashes 8182 a 300-second duration.
`data-global/scripts/actions/tools/skinning.lua:259-268` accepts both
stages with a player's storage value at most one. It sets the value to one
for fresh ashes or two for the second stage. The source therefore allows
repeated fresh-ash grants and has no encounter membership lookup. The
requested participant entitlement needs an explicit association with the
encounter's captured players. Main already has corpse retirement dispatch
in `crates/gameplay/dispatch/src/gameplay_transport/corpse_decay.rs:34`;
the Lord ashes definitions are held from materialization.

## Arkulius

Crystal `data-global/npc/arkulius.lua:162-165` consumes 954 and selects the
reward by vocation. Main already covers the same four choices and stage
transition in
`crates/gameplay/quests/src/quest/npc_rules/tests.rs:1567`.
Neutral Matter and the four rewards remain nonmaterializable in the
canonical item definitions. Production payout still needs the admitted
native item profiles and a PostgreSQL dialogue-dispatch case.
