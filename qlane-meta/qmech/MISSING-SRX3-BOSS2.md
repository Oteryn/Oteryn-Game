# SRX3-BOSS2 — checked quest mechanics (M25)

Status: **M25 resolved in this lane's tested code**. This report covers the assigned Ahau BossLever/exit and Mitmah callback closure; it does not assert that unrelated quest stages are complete.

| quest | missing mechanic (server or client) | evidence (the quest file and step, plus the missing hook/table/command) | blocks the quest fully or partly |
|---|---|---|---|
| Adventures of Galthen Quest | none in the checked M25 scope | `content/quests/definitions/quests-00100-00199.json#/records/8/definition/oteryn_recipe/payload/recipe/stages/4` is s5 (Ahau); stage 7 is s8 (Mitmah Vanguard). `tools/content-schema/encounters/import_boss_levers.py` qualifies the exact pinned Ahau exit configuration and audited callback closure. The production BossLever activity caller and `crates/gameplay/dispatch/src/gameplay_transport/source_boss_exit.rs` share the durable return owner. Native lethal-combat PG coverage consumes both gated quest kills and checks actual corpse/loot settlement. `docs/reference/content-prep/M25-boss2-sources.md` records the pinned donor evidence, including why the unregistered `iksupanBossesDeath` attachment is inert. | Neither of these checked M25 hooks remains a blocker. Other quest stages were not audited here. |

The real two-player Ahau PG regression now passes entry, native-coordinate physical PUSH onto the configured exit, return of only the pushed participant, preserved control loss with no controller, and one destination teleport effect. Its measured case duration is **234.26252329 s**; the prior implementation returned `NotPossible`/`Rejected` after a real PG run. The ordinary input-control rejection remains covered independently. Native client coverage replays actual server entry/STEP/return frames through the native UI with real assets and verifies the player sprite and destination effect.

No CLAIMS.md change is made: the coordinator owns assignment completion.
