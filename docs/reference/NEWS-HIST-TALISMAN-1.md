# NEWS-HIST-TALISMAN-1 reference ledger

Evidence classification: pinned OTS implementations are compatibility evidence, not proof of the
proprietary server. Official news identifies the feature but does not publish every progression
number; those absent numbers remain `UNKNOWN` as news evidence.

| Source | Retained evidence |
|---|---|
| news 8566 | Introduces Bounty Tasks and the upgradeable Bounty Ring. Exact upgrade costs are `UNKNOWN`. |
| news 8567 | Introduces Weekly Tasks as the second Task Board family. No talisman progression numbers. |
| news 8598 | Renames the ring to Bounty Talisman, moves it to the extra slot, replaces damage reduction with Life Leech, and says effect limits doubled with increasing cost. Exact progression numbers are `UNKNOWN`. |
| news 8667 | Post-release Task Board changes; no different talisman progression number is established here (`UNKNOWN`). |
| news 8831 | Later Task Board communication; no different talisman progression number is established here (`UNKNOWN`). |

The Canary pin `47dfd51f45280a59a1d3e50ba7edd573d7234446` supplies the executable
compatibility profile: four paths are damage, life leech, more loot and double Bestiary; the maximum
stored level is 355; upgrade cost is `5 + 12 * current level`; ordinary effects are 2.5% plus 0.5%
per level through level 15, 10% plus 0.25% per level through level 55, then 20% plus 0.1% per level;
double Bestiary uses twice those values. The pin also requires item 51978 to be equipped and an
active matching task creature; damage activates at 15%, life leech at 10%, loot applies directly,
and the Bestiary percentage is the chance for an extra kill.

The Crystal pin `201189f871d23597b82caf88fd611454a2ddefb8` corroborates four durable
character levels, item 51978, four effects, matching active-task and equipped-item gates, and
`5 + 12 * current level` cost. Its effect curve and caps conflict with Canary (ordinary maximum at
level 166 and Bestiary at level 180), so those Crystal curve numbers are retained as `CONFLICT` and
are not mixed into the Canary-compatible executable profile.

Pinned file locations inspected:

- Canary `data/modules/scripts/taskboard/settings.lua:145-157` and
  `data/modules/scripts/taskboard/rules.lua:39-67,103-112,643-668,769-785,1188-1238`.
- Crystal `src/io/iobountytasks.cpp:569-598,668-780`,
  `src/io/functions/iologindata_load_player.cpp:1092-1100`, and
  `data/migrations/62.lua:19-22`.
