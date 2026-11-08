# Retained request after fresh main/engine inspection

ENGINE-REQUEST questpc-pc2-n2: atomic multiple root item rewards in one NPC receipt, including capacity refusal and replay | needed by: An Interest In Botany Quest; Bigfoot's Burden Quest | where: crates/gameplay/quests/src/quest/reward_callbacks.rs

This renews the existing request from the frozen pc2 branch, without adding a
second engine implementation. Checked mainf5b584ef1 and QENGINE2
64b3597c1ae76c936bb80dd3b5b85a01e0761fb6. Each retains give:Option<ItemReward>.
Rabaz source consumes11699x1 and independently grants11700x1 plus3035x10,
3000XP and Questline4. ItemReward.contents is not equivalent to two roots.
Source and exact hash are in QUEST-STATUS.json.
