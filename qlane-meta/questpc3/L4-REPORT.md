# questpc3 round L4 report

The 22-entry request is incomplete. 1 qualified / 21 unfinished / 0 blocked. Stampede is qualified; no whole 3–6-entry batch has completed. The other entries retain their exact implementation gaps.

## Changed native files

- `apps/game-server/src/node/raid_scheduler.rs`
- `apps/game-server/src/node/serve.rs`
- `apps/game-server/src/node/serve/daily_world_changes.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/quest_l4_occurrences.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/raid_live.rs`
- `crates/gameplay/dispatch/src/gameplay_transport/quest_kills.rs`
- `crates/gameplay/durability/src/durability/quest_state.rs`
- `crates/gameplay/durability/src/durability/quest_state/stampede.rs`
- `crates/gameplay/durability/src/durability/quest_state/writer.rs`
- `crates/gameplay/durability/src/durability/raid_runner.rs`
- `crates/gameplay/durability/src/durability/raid_runner/effects.rs`
- `crates/gameplay/durability/src/durability/raid_schedule/scheduler.rs`
- `crates/gameplay/encounter-rules/src/raid/catalogue.rs`
- `crates/gameplay/quests/src/quest/loader.rs`
- `crates/gameplay/quests/src/quest/mod.rs`
- `crates/gameplay/quests/src/quest/stampede.rs`
- `crates/gameplay/world-change/src/daily.rs`

Task state: `.questpc3/QUEST-STATUS.json` (`round_L4`) and `.questpc3/RESUME.md`. Evidence/check manifest: `.questpc3/L4-CHECKS.json`, `.questpc3/l4-native-tree-final.json`, constructor audit files, and `.questpc3/reference/l4/` (three pinned Crystal files plus bounded tree metadata). Owner input `.questpc3/LANE-L4.json` was preserved.

## Exact focused test commands

```sh
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-quests --lib stampede -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-world-change --lib daily::tests -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-encounter-rules --lib stampede -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-game-server --lib stampede_activation -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked -p oteryn-gameplay-dispatch --lib quest_l4_stampede -- --nocapture
```

Results: 8 pure unit cases passed (1 Quest, 5 daily, 1 raid planner, 1 node selector). Two PostgreSQL cases passed with completion markers: native schedule/spawn 34988ms; personal counter/achievement 36202ms. The pure daily test timing of 0.00s is not used as PostgreSQL qualification. Clippy passed with -D warnings for all six changed packages, then passed again for durability/dispatch after the narrow scheduling repair. Formatting and diff whitespace checks passed. No cargo command remains running.

## Entry status and exact remaining gaps

| Canonical | Title | Status | Outstanding gap |
| --- | --- | --- | --- |
| oteryn:quest.the_beginning_quest | The Beginning Quest | UNFINISHED | Santiago's three-cockroach occurrence counter and Zirella's wood hand-in lack native completion/delivery bindings; no scalar dialogue completion may substitute. |
| oteryn:quest.the_first_dragon_quest | The First Dragon Quest | UNFINISHED | Tamoril's dragon-kill/treasure conditions and final boss admission/rewards remain unbound; the four admitted exit portals are only the return route. |
| oteryn:quest.the_tainted_soul | The Tainted Souls Quest | UNFINISHED | The talisman-on-live-Tainted-Soul operation,100-soul gate and20-hour Ekatrix reset lack native source-qualified creature transformation/cooldown bindings. |
| oteryn:quest.threatened_dreams_quest | Threatened Dreams Quest | UNFINISHED | Maelyrra's summed counters and item/final bag reward branches are explicitly held in threatened_dreams.rs; scalar branches and source book/mirror chests do not cover them. |
| unresolved:questpc2:pilgrimage_of_ashes_quest | Pilgrimage of Ashes Quest | UNFINISHED | The four free regular blessing Service identities (Spiritual Shielding, Embrace of Tibia, Fire of the Suns, Spark of the Phoenix) are absent from the canonical Service.Blessing family. The ruleset price names are not canonical service declarations. The closed native blessing purchase owner admits premium services only and its fee/receipt guards lack those bindings. A lane-local mission handler cannot grant unadmitted Services; changing the selected recipe to dialogue-only would remove the source blessings. |
| oteryn:quest.authored.rathleton_quest | Rathleton Quest | UNFINISHED | Atomic voting-point debit/world tally and fortnightly map/spawn selection owner. |
| oteryn:quest.authored.river_runs_deep_mini_world_change | River Runs Deep Mini World Change | UNFINISHED | Native daily river-layer/fishing/achievement binding. |
| oteryn:quest.roshamuul_quest | Roshamuul Quest | UNFINISHED | Shared contribution-to-world-stage/map/encounter composition. |
| oteryn:quest.authored.spider_nest_mini_world_change | Spider Nest Mini World Change | UNFINISHED | Daily nest spawn/portal and three-kill-to-entry/achievement binding. |
| oteryn:quest.authored.spirit_grounds_mini_world_change | Spirit Grounds Mini World Change | UNFINISHED | Native daily portal/layer and independent monster selection binding. |
| oteryn:quest.authored.spring_into_life | Spring into Life | UNFINISHED | Seasonal activation and native nest/reset/hatching/spawn composition. |
| oteryn:quest.stampede_mini_world_change | Stampede Mini World Change | QUALIFIED | None; qualified by the evidence above. |
| oteryn:quest.authored.the_fire_feathered_serpent_world_change | The Fire-Feathered Serpent World Change | UNFINISHED | Renegade Quara kills must change world-owned sea state/travel responses; the retained authored event has no selected native global-state occurrence binding. |
| oteryn:quest.authored.the_great_expedition | The Great Expedition | UNFINISHED | The expedition requires world donation totals, daily event epochs and shared cargo/generator progress; the selected authored recipe has no exact native global event binding. |
| oteryn:quest.authored.the_lightbearer | The Lightbearer | UNFINISHED | The world-wide light timers, loss condition and event reward epoch lack a selected native world-event binding; ordinary once-only character claims cannot represent them. |
| oteryn:quest.the_mages_tower_world_change | The Mage's Tower World Change | UNFINISHED | The250-Yielothax personal condition and500-kill global mage-state transition lack composed native counters/encounter ownership. |
| oteryn:quest.the_mummys_curse_world_change | The Mummy's Curse World Change | UNFINISHED | Breaking the Ornate Canopic Jars must atomically advance the world Horestis release cycle; the selected recipe has no exact native jar/reset occurrence binding. |
| oteryn:quest.their_masters_voice | Their Master's Voice World Change | UNFINISHED | Slime Gobbler fungal cleanup and per-event servant raid totals lack a native world reset/occurrence binding; the selected recipe has no admitted delivery path. |
| oteryn:quest.thornfire_world_change | Thornfire World Change | UNFINISHED | Prepared Bucket use, world fire extinction state and Crystal Wolf transformation lack source-qualified native item/world-event bindings. |
| oteryn:quest.authored.twisted_waters_world_change | Twisted Waters World Change | UNFINISHED | Lake Equivocolao's corpse-fed world state and20-hour Shimmer Swimmer harvest need event-owned state, exact native harvest selectors and character receipts. |
| oteryn:quest.authored.war_against_the_hive_quest | War Against the Hive Quest | UNFINISHED | Orockle's powder/corpse hand-ins and shared Hive stage transitions lack native item/world-event bindings; the authored access reward is not an executable gate. |
| oteryn:quest.authored.warpath_mini_world_change | Warpath Mini World Change | UNFINISHED | The Jakundaf orc encampment state and combat-driven event reset have no selected native world occurrence/delivery binding. |

## Implemented scope and review

Stampede uses the accepted W10 reset probability, the existing source-bound raid runner, an occurrence-deduplicated personal counter and an atomic fifth-kill achievement grant. The native scheduler now works during a busy legacy raid, without advancing legacy delay clocks. Both terminal clock-update paths were repaired. A source-wide rg sweep checked the new DailyWorldChanges field and all constructors/consumers. The only added struct field is DailyWorldChanges.stampede. No enum variant, trait method or protocol API was added. All added internal functions have production callers; the native dispatch and real scheduler/raid owner are exercised against PostgreSQL.

The whole L4 diff was read against the owner CODEX-CHECKLIST. Two concrete findings were repaired test-first: dropped row-less native deaths and legacy raid interference. No test assertion was removed or weakened. Ordinary corpse/loot admission is unchanged; the quest-only fallback grants no ordinary XP or loot. No schema, dependency, protocol code, qengine2 engine, excluded quest or CI/protection file was changed. All worker Git operations were read-only; no subagent or external review was used. The checkout remains AUTHORING, without a worker-created freeze, commit, push or PR.
