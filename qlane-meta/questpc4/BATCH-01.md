# T..Z reward batch

12 source reward cohorts/26 placements have completion markers and durations under mechanic-named tests. Waterfall also passed three access destinations through real STEP, with a non-empty quest-state control and unchanged receipts. This is reward/access qualification; full quest paths are not inferred.

| Quest | Placements | Current test | Duration | Remaining path |
|---|---:|---|---|---|
| Thais Lighthouse Quest | 2 | `source_quest_rewards::thais_lighthouse_rewards` | 149.054779408s | Pressure-plate step-out and paired portals are blocked on missing engine support. |
| Throwing Star Quest | 1 | `source_quest_rewards::throwing_star_rewards` | 53.207217314s | Ordinary pick/rope traversal is outside the chest fixture. |
| Time Ring Quest | 3 | `source_quest_rewards::time_ring_rewards` | 171.245491229s | All three source claims passed; ordinary traversal is outside the fixture. |
| To Blind the Enemy Quest | 4 | `source_quest_rewards::to_blind_the_enemy_rewards` | 214.635653662s | Eroth start, mirror on scrying ball, return and final NPC reward remain. |
| To Outfox a Fox Quest | 1 | `source_quest_rewards::to_outfox_a_fox_rewards` | 56.383084253s | Budrik start, quest door, helmet hand-in and final NPC reward remain. |
| Tomes of Knowledge Quest | 5 | `source_quest_rewards::tomes_of_knowledge_rewards` | 314.41999983s | New Frontier access, all 12 tomes, Cael hand-ins and unlocks remain. |
| Triangle Tower Quest | 1 | `source_quest_rewards::triangle_tower_rewards` | 64.176743015s | Asymmetric UID30034 entrance lever and basement exit remain. |
| Troll Cave Quest | 2 | `source_quest_rewards::troll_cave_rewards` | 88.188581216s | Both orientations prove cross-box refusal/replay and successful fixture cleanup; ordinary traversal is outside the fixture. |
| Vampire Hunter Quest | 1 | `source_quest_rewards::vampire_hunter_rewards` | 53.405000251s | Inquisition prerequisite, Storkus ranks/XP/doll/achievement and skull access remain. |
| Vampire Shield Quest | 3 | `source_quest_rewards::vampire_shield_rewards` | 137.767884403s | Level 70 gate is separate from these claim fixtures. |
| Waterfall Quest | 1 | `source_quest_rewards::waterfall_rewards` | 74.623925073s | Three STEP destinations passed; source facing/splash remains unqualified. |
| Wedding Ring Quest | 2 | `source_quest_rewards::wedding_ring_rewards` | 126.491829499s | Both source chests passed; ordinary route traversal is outside the fixture. |

The current continuation ran the reward/access cases in groups of five, five and three, with zero PostgreSQL failures. All used the absolute cargo wrapper with a named filter and -j4; no absent-URL early return was counted. The final three-line cleanup repair changed only successful test teardown. A focused Troll Cave rerun then passed both orientations with cleanup before its cohort marker. Earlier qualification logs predate that teardown-only repair; production content and every reward assertion are identical.

The only new unit failure found in this continuation was the native container Item count130→131, exactly Garlic Necklace i3083 from Triangle Tower. It was repaired with an exact131 assertion plus an explicit necklace check; the isolated test passed. See SHARED-FIXES.md.

Source fixtures remain generated from the pinned Crystal table and handler. Exact commands, unit results and historical unchanged-content checks are recorded in VALIDATION.md. Full quest limitations and engine-feature blocks are in PHASE-A.md and RESUME.md. No migration, command/capability number, commit or publication was introduced.
