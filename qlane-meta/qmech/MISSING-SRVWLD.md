# SRVWLD — PARTIAL checked missing mechanics

PARTIAL report against qmech `3786efb97`; checked server paths only. No client or full-quest qualification is inferred.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Nomads Land Quest | Server: literal readable text carried through an NPC quest Item reward | `content/quests/definitions/quests-00300-00351.json`, `/records/20/definition/recipe`, `s7` return to Muhad, reward intent `The Sacred Casket (Book)`. `crates/gameplay/quests/src/quest/reward_callbacks.rs::ItemReward` has no text field; `crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs` constructs `RewardClaimMintRequest` with `root_text: None`. The existing mint/text owner accepts authored text, but this quest adapter cannot supply it. | Partly: readable reward delivery only. Book identity remains unresolved in the checked recipe; adding an adapter alone cannot complete this quest. |

## Checked original lane scope

None missing in the checked Dryad Garden enter/leave slice: `content/quests/world-callbacks` admits the two existing callbacks; `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks/execute.rs:720` commits teleport before destination effect 54. `postgres_tests/srvwld_dryad_garden.rs` contains both real-USE PG regressions. This is a world-interaction slice, not certification of an entire quest.

The Scavenge consumer is present at `crates/gameplay/durability/src/durability/item_craft/skinning.rs:177`: it consumes `CharmEffect::SkinningChanceBonus` from the fenced persisted assignment. Canonical monster callback closure is covered by `srvwld_canonical_schedules_resolve_every_callback_at_its_source_index`. These are existing checked implementations, not fresh PG runs for this report.

## Transfer boundary

PR #3024 is merged as `e1ca3e003`; this package contains that squash merge, incorporating its earlier tested fixes. Do not replay the pre-squash history at head `648f1bcdaeb8d68fa9fab470ed61f27405fafdb2`. No finished unmerged quest or mechanic is available to transfer. The last Royal Costume planning experiment was explicitly WIP/red and is not submitted as a tested fix. No frozen PR branch is modified.

Migration range retained from the original SRVWLD packet: **1110–1119**, none allocated here. This partial contribution changes documentation only; no content, source code, generated pin or pinned PG test changes. Publication follows the owner's urgent partial-report instruction; the workspace check is running and has no result yet. `git diff --check` is the focused validation for this report.

## Concurrent package update

Before this report could publish, ENGITEM supplied the readable-text adapter on qmech: `ItemReward.text`, the `reward.text` → `root_text` bridge, migration `1641_quest_literal_reward_text.sql`, unit tests and `authority_cases/engitem_reward_text.rs`. The Nomads Land row above records the checked earlier snapshot; that adapter gap is now RESOLVED in the merged package and must not be claimed again. The unresolved book identity remains quest-data work, not evidence of another missing text engine. No mechanic has been claimed by SRVWLD yet.

## Queue item selected after report publication

| quest | missing mechanic (server or client) | evidence (quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| The First Dragon Quest | Server: annual UTC month/day window for source NPC dialogue | `content/quests/definitions/quests-00200-00299.json`, record 23, `s1` (Tamoril). `content/dialogues/quest-sources/the_first_dragon.json`, Tamoril source `data-global/npc/tamoril.lua:44-60`, gates dialogue to Jan 14–Feb 12 inclusive. `npc_rules.rs::NpcRuleGuard` exposes only `CalendarMonth`; `evaluate.rs` cannot represent the day boundary. | Partly: seasonal opening; timed encounters and full native binding remain separate. |
