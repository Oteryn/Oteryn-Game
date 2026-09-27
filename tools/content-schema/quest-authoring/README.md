# Quest authoring tools

Offline tooling for the CANDIDATE quest format
(`docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`). Evidence only: the server does not read
these files, and every source-derived output is `OTS_HYPOTHESIS_ONLY`.

| File | Purpose |
|---|---|
| `lua_tables.py` | Reads Lua table constructors (and their comments) without a Lua interpreter. |
| `ots_doors.py` | Transcribes the quest, key and level doors of both servers (`door_quest.lua`, `door_key.lua`, `door_level.lua`), joined by map position, into gates linked to the chest claims and keys. |
| `ots_chests.py` | Transcribes the reward chests of Canary and CrystalServer (`startup/tables/chest.lua` plus the text and achievement tables of `quest_reward_common.lua`), joined by map position, into reward claims, reward-only quests, a catalog and a manifest. |
| `quest_content.schema.json` | JSON Schema of reward claims, reward-only quests and door gates. |
| `validate_quest_content.py` | Schema plus semantic checks: unique keys and positions, non-empty rewards, text on a handed-out item, claim/quest links in both directions, gate conditions against the claims (progress marker, key source), catalog and manifest coverage. |
| `verify_quest_schema.py` | Focused positive/negative cases on synthetic fixtures (`--verbose` prints each case's first error). |
| `samples/quest-coverage-2026-09-27.json` | The 373 wiki quests (facts only) with their status in each server. |
| `samples/chests/` | `claims.json`, `quests.json`, `catalog.json`, `manifest.json`, `empty_containers.json`. |
| `samples/doors/` | `gates.json`, `manifest.json`. |

```sh
pip install -r ../monster-authoring/requirements.txt
python verify_quest_schema.py
python ots_chests.py --canary <opentibiabr/canary at 47dfd51f> --crystal <zimbadev/crystalserver at ff7ede59>
python ots_doors.py --canary <canary checkout> --crystal <crystalserver checkout>
python validate_quest_content.py samples/chests/claims.json samples/chests/quests.json \
  --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json \
  --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json
```

The coverage sample was built from the Fandom API (Template:Infobox Quest, retrieved 2026-09-27)
and a search of both servers' Lua sources; its `method` field records how, including the verdicts
corrected by hand.
