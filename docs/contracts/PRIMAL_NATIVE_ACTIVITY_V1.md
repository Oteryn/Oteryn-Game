# Primal native activity v1

W10 uses Crystal `201189f871d23597b82caf88fd611454a2ddefb8` and the active
Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` native Creature records.
Wiki checks remain `wiki-pending`.
The following references describe rules, rather than authority supplied by a
caller. The bounded witness checks actual native Creature/Behavior/Ability
bodies, full source-definition digest and exact external source bindings.

| Rule | Pinned Crystal evidence |
| --- | --- |
| Boss400000HP and native attacks | `data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:75-154` |
| Initial Beast/Pod timers10/20s, later HP/hazard scaling | same file`:156-193,269-347` |
| Independent signed additive variance, growth and capped counts | same file`:195-206` |
| Up to6 real-map spawn probes then initial boss position fallback | same file`:209-239` |
| Source presentation at1/2/3s, spawn at3s; Beast requires live creator, Pod does not | same file`:242-266,285-336` |
| Beast older than20s becomes Fungosaurus without a death reward | same file`:338-347` |
| Primal alias retains attacks,70%HP, no XP/loot/corpse/Bestiary | `data-global/lib/quests/the_primal_ordeal.lua:1-14` |
| Player-only Pod ingress,2..4s damage window, double ceiling percentage plus500 | `data-global/scripts/systems/hazard_primal.lua:25-54`; `config.lua.dist:255-259` |
| Expiry resolves current Item at original position, force-spawns Fungosaurus, removes that Item | `data-global/scripts/systems/hazard_primal.lua:60-70` |
| Item39176 source timer uses integer wall seconds; expiry4s after creation | same file`:75-80` |
| Five source waiting slots, boss/arrival/bounds, level500 and literal cooldown enablement | `data-global/scripts/actions/bosses_levers/the_primal_manace.lua:4-20` |
| First kill quest progress and minimum selected hazard/max unlock rule | `data-global/scripts/quests/the_primal_ordeal/creaturescripts_the_primal_menace_killed.lua:3-18` |
| Source effect IDs11/12/1 and monster say36 | `src/utils/utils_definitions.hpp:56,66-67,390` |
| Exact nonmovable/nonpickup Pod appearance; XML has no extra Pod override | `data/items/appearances.dat`, object39176; `data/items/items.xml:71991` |
| Exact corpse39530→39529→39528→39527 stages | `data/items/items.xml:73328-73347` |

The production entry uses the shared boss-party owner. A sealed literal Primal
catalogue recipe is activated only with the current Content pair, Node possession,
source assignment/readiness and source-qualified lobby cells. No caller can
supply a replacement room, participant position, level requirement or duration.
The shared transaction retains per-member fresh eligibility, roster, cooldown,
Instance assignment, durable handoff and domain99 scope transfer. The independent
source room is reconstructed per Instance so mutable map facts cannot cross fights.

Native onThink runs before the corresponding reserved native attack turn. It
reads the current transactional monster map, exact live actors and current-session
positive damage contributors. Source `os.time` comparisons use integer seconds;
deferred event deadlines retain the actual subsecond scheduling time. Beast
creation uses the existing native masterless/AI owners. Pod callbacks survive
their creator's death; deferred Beast callbacks check that creator again.

Pod creation and removal use the existing single source Item transaction writer.
The new sealed Encounter/v2 cause is limited to the real Primal profile, current
Instance scope and exact source Item39176. Channel Ability receipt bytes retain
their previous representation. Player Item writes do not gain generic Instance
admission. Migration0985 adds tagged scope to existing lines and extends the
existing custody/audit proof chain; it does not introduce another physical writer.

A committed paced Player step is the only native producer of Pod ingress. The
sealed step retains exact session, lease, actor incarnation, position revision,
Content pair and source timer. Unknown SQL commit retains the pending occurrence
and freezes successor mutation until reconciliation. The source removal receipt
is installed before native LifeDrain through the existing player damage owner.
Original expiry callbacks remain scheduled after a step removes an Item. Their
current-position lookup can therefore consume a genuine replacement Pod, as in
the donor, or finish empty without creating Fungosaurus.

Native deaths use the ordinary kill queue and composed XP/Quest/hazard/reward
settlement. Corpse Item39530 uses the same scoped CreatureDeath MINT and existing
temporal/corpse consumers. The source boss has no authored material loot; no
private reward Item is invented. Canary47dfd51f
`data-otservbr-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua:71-74,113`
independently confirms400000HP, corpse39530 and empty authored loot.
After-kill return retains the source60s duration
and crosses the existing domain99 barrier to the literal source return position.

The existing native map/domain17, actor/domain1, Item handles, presentation,
vitals and boss/hazard views carry the result. Source map/client floors are−15;
physical runtime positions retain legacy+15. No protocol number is allocated.

Focused production case:
`srx3_primal_real_source_entry_wave_pods_and_terminal_postgres` prepares an existing
classed Character through actual Character owners, invokes real command178,
drives native waves and source Item callbacks, performs a real paced Pod step,
and uses a normal physical final hit. Its initial nonlethal boss injury is a
bounded test precondition, preserving the actual400000 maximum and emitting no
death. It validates scoped corpse MINT and real10s first-stage consumption,
hazard unlock, source60s terminal return and actual command179/domain99 frames.
Optional `SRX3_PRIMAL_CAPTURE_PATH` writes actual production wire snapshots for
native Session/PlayView replay. Verification results belong in the execution
report; this contract alone does not claim a passing PostgreSQL or client case.

Stacked Pods preserve the donor's observable movement-event order. Crystal
`src/lua/creature/movement.cpp:385-398` and Canary's same file `:300-313` iterate
live tile indices. Crystal `src/items/tile.cpp:1101-1107` inserts newly created
same-order top Items before older ones; `:1297-1313` removes them immediately.
Appearance 39176 has source top order 3. Removing the first contiguous Pod shifts
its neighbour behind the next visited index, so three Pods trigger indices 0 and
2; two trigger index 0. The SQL owner captures the actual newest-first Ground
stack once and commits one existing physical writer receipt per triggered Item
in one transaction. Its per-Item cast identity preserves replay guards. Recovery
returns the complete committed receipt batch, and an in-memory applied-event
cursor prevents repeating HP effects if a later event must retry. Nonplayers
return nil from the source Lua handler and trigger no physical Item mutation.
