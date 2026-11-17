# SRX-3 Malice native owner evidence

Status: in progress; W11 is not certified DONE. Wiki checks: wiki-pending.

Donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
All paths and line numbers below refer to that immutable revision.

| Rule | Donor evidence | Production consumer |
| --- | --- | --- |
| Five staging tiles, level 250, exact boss/entry/exit coordinates and 20-hour cooldown | `data-global/lib/quests/soul_war.lua:332–354` | Source-qualified activity entry, current Instance room |
| Cage at (33709,31596,14), initial delay 23 seconds | `data-global/lib/quests/soul_war.lua:349–350,931` | `SoulWarCallbacks::initialize_malice`, `MaliceRoom::entered` |
| If Cage exists at its 40-second callback, remove it, schedule another in 23 seconds, and add 10 defense plus 10 reflection for each of seven elements to a surviving Malice | `data-global/lib/quests/soul_war.lua:936–944,979–1004` | `soul_war_malice::drive`, current carrier mutation |
| Killing Cage before removal stops the chain; failed forced creation still schedules the removal callback | `data-global/lib/quests/soul_war.lua:979–1004` | Presence re-read at callback execution, independent timer state |
| Count actual one-second native think occurrences; at 40 seconds reset the counter and create white ground | `data-global/monster/quests/soul_war/goshnars_malice.lua:135–147` | Reserved native think batch, before generic attack selection |
| Choose from the 15×15 square around the source center; create four connected groups of up to three, not an imposed eleven-tile limit | `data-global/lib/quests/soul_war.lua:1006–1064,1100–1147` | `select_white_cells`, deterministic bounded draws |
| After three seconds, top player on each nonwhite grounded zone tile loses 8000 Death health | `data-global/lib/quests/soul_war.lua:1078–1093,1147` | Current top-stack actor, current session, existing shield/lethal HP owner |
| Restore only saved ground that is still 409; preserve pending restoration otherwise | `data-global/lib/quests/soul_war.lua:1095–1099` | Actual Instance Bundle facts, retained source-ground keys |
| Ground 410 emits red smoke only without excluded Items and source top orders 1/3 | `data-global/lib/quests/soul_war.lua:1068–1075,1091–1093`; `src/utils/utils_definitions.hpp:135` | Captured immutable source top order, existing native effect queue, effect 167 |

Mutable closure state and timers belong to the independently owned party Instance,
following `INSTANCE_SCOPE_AND_RUNTIME_OWNER_BASELINE.md:97–111,131` and
`BOSS_RAID0` per-party isolation. A retained, nonforgeable source continuation
survives its spawning creature, while every use rechecks the current Instance
owner and content/frame/map binding. It grants no source-Channel or foreign
Instance authority.

Observed RED: `/tmp/srx3-w11-malice-spell-red.log` contains the missing Malice
clock/selection APIs and missing direct-Lua health ingress. Those APIs are now
implemented. Native gameplay integration remains pending the focused case
`srx3_malice_actual_source_entry_cage_and_retained_buff_postgres`, which uses
actual source staging, real command 178, the scope barrier, reserved native
turns, the genuine Cage, timed removal, and an exactly-once retained reflection
buff. It does not seed a catalogue recipe or fabricate a boss policy.

Additional validation still required: source entry eligibility for every roster
member; nearby-player paralyze removal by Heal Malice; actual white-tile
projection, damage and restoration; idle native effect delivery; owner retirement
and retry cases. No unexecuted case is reported green.
