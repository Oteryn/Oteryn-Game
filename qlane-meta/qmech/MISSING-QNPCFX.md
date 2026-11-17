# QNPCFX — checked NPC effect gaps

Checked QMECH base `e1aa08e81` and the production source planner, not just the
presence of effect enum variants. No client defect was established by this check.
This is a bounded report of checked steps, not a qualification of whole quests.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Koshei the Deathless Quest | Server: atomic NPC carried/bank payment composed with item hand-in and quest progress | `content/dialogues/quest-sources/koshei_the_deathless_quest.json`, entry `a_sweaty_cyclops`, retained Lua line 322: `removeMoneyBank(5000)` before the four amulet-piece checks/removals. `crates/gameplay/quests/src/quest/npc_rules/item_exchange.rs::item_exchange_keys` rejects `TakeMoney`; `evaluate.rs::plan` storage-only whitelist also rejects it. `quest_state/npc_program.rs::evaluate_source` selects only those bounded planners (or two special item owners); there is no NPC payment participant there. Source entry itself remains non-admitted, so lowering/binding is also required. | Partly: the paid amulet-assembly step; no claim that payment alone qualifies the entire quest. |
| Kilmaresh Quest | Server: achievement delivery composed with NPC progress/item rewards | `content/dialogues/quest-sources/kilmaresh.json`, entry `alyxo`, retained Lua line 197: final mission answer grants `Sculptor Apprentice`, item 31574 and advances `Storage.Kilmaresh.Fourteen.Remains`. `npc_rules/evaluate.rs::plan` storage whitelist and `npc_rules/item_exchange.rs::item_exchange_keys` reject `Achievement`; `quest_state/npc_program/rewards.rs` handles only XP, and the NPC mint request in `reward_claim_mint/npc_program.rs` sets `achievement: None`. The source entry also needs admission/lowering. | Partly: that finale; no whole-quest certification. |

## Existing work and transfer

- Shared NPC calendar input already comes from the transaction/server epoch and
  `utc_calendar_month`; delayed conversation-end, bounded plain item exchange,
  and XP obligations already exist in production. They are not reported missing.
- QNPCFX production work landed in main via #3027; follow-up fixes #3035,
  #3038 and #3040 also merged. MAINRED48 is merged as #3044 (live PR state
  checked). There is no finished unmerged QNPCFX delta to copy into this package.
- Outfit/addon grants remain outside this lane; achievement ownership is already
  listed as `engclaim` in `qlane-meta/qmech/PACKAGE.md`.
