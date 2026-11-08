# WORLDCHG checked gaps — PARTIAL

Checked against main82ff2d753 and the WORLDCHG changes in this package. These
are server gaps; no client gap is inferred from a waiting-data label. Native
activation proof does not imply that an authored journal recipe is lowered.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| `oteryn:quest.authored.fire_from_the_earth_mini_world_change` | Server: authored personal journal lowering | `content/quests/definitions/quests-00300-00351.json`, recipe s1 exploration, s2 fifty kills, s3 completion: `native_lowering.state=WAITING_IMPLEMENTATION`, all canonical refs empty. Native shared counter exists in `world_event/changes/counters/fire_earth.rs`; the authored recipe has no canonical personal transitions. | partly: native event exists, authored recipe remains unlowered |
| `oteryn:quest.authored.hive_born_world_change` | Server: authored personal recipe lowering | Same shard, s1 Orockle through s7 completion: canonical interaction/progress/reward refs empty. `crates/gameplay/quests/src/quest/hive_access.rs` implements scent purchase and native gates, not the complete authored recipe lowering. | partly: native access exists, authored recipe remains unlowered |
| `oteryn:quest.authored.hive_outpost_mini_world_change` | Server: executable outpost activation and encounter binding | Same shard, s1 Vandura outpost, s2 twenty insects, s3 unresolved survey landmark; `content/encounters/world-changes/owner-catalogue.json` entry `hive_outpost` is HELD with no binding; `owner::is_active` has no admitted outpost key. | fully for the authored event route |
| `oteryn:quest.authored.insectoid_invasion_world_change` | Server: invasion/restoration activation, threshold and spawn membership | Same shard, s1 Greenshore hole and s2 Insectoid Scouts; owner catalogue entry `insectoid_invasion` is HELD with no binding; `owner::is_active` has no admitted invasion key. | fully for the authored event route |
| `oteryn:quest.authored.jungle_camp_mini_world_change` | Server: camp variant membership and selection binding | Same shard, s1 Hunter Camp through s4 Dworc kills; owner catalogue `jungle_camp` is HELD with no binding; the closed native daily owner has no camp variant selector. | fully for the authored event route |
| `oteryn:quest.authored.kingsday_mini_world_change` | Server: celebration activation and hourly raid binding | Same shard, s1 decorated Thais, s2 King Tibianus, s3 completion; owner catalogue `kingsday` is HELD with no binding; no admitted key in `owner::is_active`. | partly: ordinary Thais/NPC presence does not implement the celebration |
| `oteryn:quest.authored.lumberjack_mini_world_change` | Server: felled-tree/Chip activation and report/reset binding | Same shard, s1 clearing, s2 Chip, s3 Queen Eloise; owner catalogue `lumberjack` is HELD with no binding; native daily selection has no lumberjack effect. | partly: dialogue and event activation remain unbound |
| `oteryn:quest.horse_station_world_change` | Server: escape/raid/stable lifecycle and journal lowering | `content/quests/definitions/quests-00100-00199.json`, `native_lowering` has no canonical interaction/progress refs; owner catalogue `horse_station` is HELD and identifies uncomposed escape, three-hour/zero-horse activation/expiry and stable transition. | partly: existing anchors do not compose the lifecycle |
| `oteryn:quest.fury_gates_mini_world_change` | Server: personal journal lowering | Same shard, `native_lowering.state=WAITING_IMPLEMENTATION` with empty canonical refs. Native layer/entry/return dispatch is already implemented in `daily_world_gates.rs`; the candidate source compiler's `require_w1_bindings` intentionally remains unavailable. | partly: native gates exist, canonical journal recipe does not |
| `oteryn:quest.grimvale_mini_world_change` | Server: personal journal lowering | Same shard, `native_lowering.state=WAITING_IMPLEMENTATION`, empty canonical refs. Native monthly spawn/raid/portal entry exists and the WORLDCHG PG case drives STEP; that is not a canonical personal quest lowering. | partly: native event exists, canonical journal recipe does not |

Shared native activation gap addressed by this lane: the generic QuestState,
callback and travel predicate previously read only `game_world_change_states`,
so native Hive/Fire Earth counters in `game_world_change_days` could not satisfy
the same predicate. The package adds current-epoch native fallback and durable,
immutable physical binding verification. Tests name the exact bounded proof;
this file does not claim all ten quests complete.
