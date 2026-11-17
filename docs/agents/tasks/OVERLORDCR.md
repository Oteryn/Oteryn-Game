# OVERLORDCR

Base: e2152d050038cecdb3728c222af2ac184e1cac5a. Branch: claude/happy-davinci-ijz3wy-overlordcr.

Crystal reference checkout: /workspace/crystal at 201189f871d23597b82caf88fd611454a2ddefb8; the original packet revision 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d was also inspected.

`data-global/scripts/quests/elemental_spheres/creaturescripts_overlord_kill.lua:2-5` maps energy, fire, ice and earth overlords to the same player `Storage.Quest.U8_2.ElementalSpheres.BossStorage`. Lines 24-26 call `onDeathForDamagingPlayers`, check `< 1` and set `1`. `data-global/lib/core/storages.lua:1084` defines BossStorage as 41652. The distinct globalStorage keys in lines 2-5 control spawn availability, not per-character kill credit.

Implementation plan: register four source-death rules against one quest track. Use the existing lethal damage-share capture, quest recipient freeze, native death queue and fenced quest-state transaction. Do not introduce a second storage owner or infer nearby/party participants. Verify two damaging characters, a nearby observer, other creature deaths, duplicate settlement and all four source mappings through PostgreSQL.

WIP: production rules are registered in `crates/gameplay/quests/src/quest/source_deaths.rs`. Mounted dispatch regressions cover shared donor mapping, two damaging Characters, a nearby observer, a non-overlord death and duplicate projection/settlement. The pre-change quest dependency was compiled before production registration; its initial red run is still compiling dispatch. Formatting passed. Governance validation and all 135 governance unit tests passed. PostgreSQL execution, clippy and pin verification remain pending.
