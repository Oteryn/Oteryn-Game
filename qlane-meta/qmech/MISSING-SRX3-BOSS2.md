# SRX3-BOSS2 — missing quest mechanics

Status: **PARTIAL** — initial report requested by the owner; production-hook tracing continues.

| quest | missing mechanic (server or client) | evidence (quest file and step, missing hook/table/command) | blocks fully or partly |
|---|---|---|---|

No missing-mechanic assertion is made in this initial partial report.

Checked in the local boss lane (`f6da4c350` plus its pending repairs):
- `content/quests/definitions/quests-00100-00199.json`, quest `oteryn:quest.primal_ordeal_quest` (record 87): `native_lowering.state` is `WAITING_IMPLEMENTATION`, and its interaction/progress reference lists are empty.
- The same file, `oteryn:quest.rotten_blood_quest` (record 93), has the same lowering metadata.
- `content/quests/definitions/quests-00200-00299.json`, `oteryn:quest.soul_war_quest` (record 6), has the same lowering metadata.

That metadata alone does **not** prove a missing production mechanic: separate native death, encounter and quest callbacks must be traced before adding a blocker row. No guessed blockers, quest counts or completed-mechanic claims are included here. The next update will identify verified quest steps and missing production hooks, or record that the checked paths are already wired.
