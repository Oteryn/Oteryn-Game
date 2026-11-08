# SRVENC checked missing quest mechanics — PARTIAL

Checked on qmech `aca848557`: this is a bounded inspection of boss-death quest
continuations, not a completeness claim for every quest. No missing mechanic
below is inferred solely from the completion matrix.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Heart of Destruction Quest | Server: source room-recipient boss-death progress adapter | `content/quests/definitions/quests-00100-00199.json`, key `oteryn:quest.heart_of_destruction_quest`; `content/quests/native-death-bindings/quest-batch3-c-heart_of_destruction.json:1`, six rules from Anomaly through World Devourer, including `heart-destruction/anomaly-room-death` and `heart-destruction/world-devourer-room-death`. The retained packet has `runtime_enabled:false` and explicitly names missing `quest_batch3_c.rs::{capture,settle}`. That dispatch file does not exist. Live `quest_kills.rs::settle` consumes only the counters in `quests/src/quest/native_kills.rs`; none of these room-death transitions are registered there. | Partly: named boss-death continuation steps are not consumed by this adapter. |
| The Order of the Stag Quest | Server: Court Warlock room-recipient death progress | `content/quests/definitions/quests-00200-00299.json`, key `oteryn:quest.the_order_of_the_stag_quest`; `content/quests/native-death-bindings/quest-batch3-c-the_order_of_the_stag.json:1`, rule `order-of-the-stag/court-warlock-room-death`, questline 11 → 12. Packet disabled; `quest_batch3_c.rs::{capture,settle}` absent; transition absent from live native kill counter registration. | Partly: Court Warlock completion step. |
| Tower Defence Quest | Server: Goblin Leader quest kill transition | `content/quests/definitions/quests-00200-00299.json`, key `oteryn:quest.tower_defence_quest`; `content/quests/native-death-bindings/quest-batch3-c-tower_defence_quest.json:1`, `questline/creature_event_1`, questline 2 → 3 on `oteryn:creature.goblin_leader`. Packet disabled; `quest_batch3_c.rs::{capture,settle}` absent; no Goblin Leader binding in live `native_kills.rs`. | Partly: named final kill step. |
| The Primal Ordeal Quest | Server: Magma Bubble killed quest flag | `content/quests/definitions/quests-00100-00199.json`, key `oteryn:quest.primal_ordeal_quest`; `content/quests/native-death-bindings/quest-batch3-c-the_primal_ordeal.json:1`, rule 0 `magma_bubble_killed/creature_event_1`, flag < 1 → 1. Packet disabled; `quest_batch3_c.rs::{capture,settle}` absent. Rust search finds no writer/binding for `magma_bubble_killed`. This report does **not** claim Primal Menace is missing: `durability/character_hazard/primal.rs::apply_hazard_kill_in_transaction` already writes its completion independently. | Partly: Magma Bubble flag only. |

## Finished work transfer (B)

SRVENC PR #3034 is MERGED, merge commit
`c68395ccc0029cb3c8511ec804dfe3536afd4cc6` (live GitHub readback).
Its finished W02/W03/W17 and combined encounter work is therefore already on
main; there is no additional unpublished finished SRVENC work to transplant.
The old SRVENC branch is left unchanged. Migration range remains **1100–1109**;
no migration is allocated by this report.
