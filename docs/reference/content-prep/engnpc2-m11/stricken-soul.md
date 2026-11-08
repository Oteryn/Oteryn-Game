# M11 — Stricken Soul name response

Quest: `content/quests/definitions/quests-00200-00299.json#/records/21/definition/oteryn_recipe/payload/recipe/stages/2` (`s3`, Haunted House access).

Source: https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/stricken_soul.lua

Downloaded and verified SHA-256: `506efb1fc93172d5a9fef91360686e1cb080d6830897d44e5cb8dfd86ea8b39c`.
The same file at revision `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d` is byte-identical.

Lines 52–55 select the greeting from HauntedHouse.Questline. Lines 71–78 require two `yes` responses (topics 2, then 3). Lines 79–93 compare MsgContains against `player:getName()`, emit eight paragraphs, write Questline=1, and reset the topic. The `yes` branch takes precedence over the name branch. Lines 95–98 reset the topic only for unmatched messages. The authored no-op branches preserve the source's topic behavior.

`engnpc2_player_name.py` reproduces the programme offline. The runtime resolves the current name from the Character owner under the same transaction/root lock as the quest write. The native Haunted House track retains its existing bounds and initial value; no quest progress is preseeded. This wires the Haunted House access conversation, not the remaining Dream Courts mechanics.

## Validation

The selector regression first failed with the closed-schema rejection `unknown field match_player_name`, then passed after implementation. The real PostgreSQL case `engnpc2_m11_current_name_is_fenced_and_replay_bound` executed in 6.72541493 s; it uses the embedded production programme and native quest catalogue without preseeded quest tracks.

The shared QMECH base `44748cc34` already fails `published_ungated_quest_hash_baseline` for `oteryn:quest.outfit_and_addon_quests` (`bd44ba2f…` versus the test's `87f64767…`). Reproduced with all tracked M11 quest-code changes removed and the new source programme absent, then restored the working changes. That separate outfit baseline is outside M11; no hash was hand-edited. Focused NPC tests cover the additional programme, defaults, selector, and legacy activation compatibility.

After merging M08's live eligibility API, M11 keeps eligibility and the current-name hint together in private evaluation facts. The writer still reloads the actual name under its Character fence. The merged PostgreSQL case executed in **5.769369521 s**; all **37** focused NPC tests passed, including the combined activation/default inventory.
