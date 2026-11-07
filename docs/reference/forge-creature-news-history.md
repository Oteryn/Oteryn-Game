# Forge creature news-history evidence

Task: `NEWS-HIST-FORGE-XP-1`.

The runtime values below were checked against both selected OTS hypotheses at the exact task
pins. They are compatibility evidence, not claims that the projects are Global truth.

| Mechanic | Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446` | Crystal `201189f871d23597b82caf88fd611454a2ddefb8` | Native disposition |
|---|---|---|---|
| Influenced stacks | `src/creatures/monsters/monster.cpp:3875-3880` chooses `1..5` | `src/creatures/monsters/monster.cpp:2769-2773` chooses `1..5` | present |
| XP | `src/creatures/monsters/monster.cpp:873-875` multiplies base XP by `(stack + 10) / 10`; both operands are integers, so stacks `1..5` retain `1x` and stack `15` yields `2x` | `src/creatures/monsters/monster.cpp:747-749` is identical | present |
| Ordinary-loot multiplier | No forge-dependent ordinary-loot probability or quantity multiplier was found at the pin | No forge-dependent ordinary-loot probability or quantity multiplier was found at the pin | **UNKNOWN**; no multiplier guessed |
| Fiendish slivers | `src/config/configmanager.cpp:268-269` supplies inclusive bounds `3..7`; `src/creatures/monsters/monster.cpp:3415-3425` applies them only to fiendish corpses | `src/config/configmanager.cpp:315-316` and `src/creatures/monsters/monster.cpp:2526-2536` agree | present |
| Population caps | `src/config/configmanager.cpp:264,266` defaults to `3` fiendish and `300` influenced | `src/config/configmanager.cpp:310,312` agrees | present |
| Creature eligibility | `src/creatures/monsters/monster.cpp:3891-3893` excludes summons and reward bosses and requires loot, Forge admission and a positive race id | `src/creatures/monsters/monster.cpp:2783-2785` agrees | present |
| Safe tile selection | `src/game/game.cpp:11933-11940` excludes `TILESTATE_NOLOGOUT`; it does not name the distinct protection-zone flag | `src/game/game.cpp:11672-11683` agrees | **UNKNOWN** for the requested protection-zone rule; no numeric or flag equivalence guessed |

The task's official news inputs are archive IDs `7842`, `7944`, `7994`, `8025`, `8092`, `8128`,
`8237`, `8270`, and `8794`. They motivate the historical lane but do not override a conflicting or
missing pinned runtime fact.
