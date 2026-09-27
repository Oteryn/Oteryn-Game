# Quest authoring tools

Offline tooling for the CANDIDATE quest format
(`docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`). Evidence only: the server does not read
these files, and every source-derived output is `OTS_HYPOTHESIS_ONLY`.

| File | Purpose |
|---|---|
| `lua_tables.py` | Reads Lua table constructors (and their comments) without a Lua interpreter. |
| `ots_doors.py` | Transcribes the quest, key and level doors of both servers (`door_quest.lua`, `door_key.lua`, `door_level.lua`), joined by map position, into gates linked to the chest claims and keys. |
| `ots_questlog.py` | Transcribes the quest logs of both servers into storyline quests with staged missions (D34) and their transitions (D35), joined by quest and mission name; writes the whole quest catalogue and the progress tracks with per-server transition sources. |
| `lua_writers.py` | Finds every storage write in the Lua sources and reads it as a candidate transition (D35): owner, callback, effect, `from` stage through its if-block, script registrations. |
| `lua_blocks.py` | Splits a Lua callback body into if/elseif/else branches, loops and statements (early `return` makes the rest an implicit `else`). |
| `ots_interactions.py` | Transcribes every quest script of both servers into interaction definitions (D36): edge, read-only conditions and children for the Quest, Ability, Item, Achievement and Presentation owners, with Movement and WorldObject children blocked; joined by quest and script path. |
| `ots_chests.py` | Transcribes the reward chests of Canary and CrystalServer (`startup/tables/chest.lua` plus the text and achievement tables of `quest_reward_common.lua`), joined by map position, into reward claims, reward-only quests, a catalog and a manifest. |
| `quest_content.schema.json` | JSON Schema of reward claims, door gates, reward-only and storyline quests. |
| `conflict_decisions.json` | D25 decisions for every Canary/CrystalServer conflict of the chest, door, quest-log and interaction transcriptions: chosen server, basis, the difference in our own words and the wiki revision when it decides; the converters apply it and fail on a stale decision. |
| `interaction.schema.json` | JSON Schema of interaction definitions (D36). |
| `validate_quest_content.py` | Schema plus semantic checks: unique keys and positions, non-empty rewards, text on a handed-out item, claim/quest links in both directions, gate conditions against the claims (progress marker, key source), one identity per quest, mission ranges and stages against the progress tracks, catalog and manifest coverage; for interactions: anchors, blocked reasons, named transitions against the missions, undeclared progress tracks, manifest status. |
| `verify_quest_schema.py` | Focused positive/negative cases on synthetic fixtures (`--verbose` prints each case's first error). |
| `samples/quest-coverage-2026-09-27.json` | The 373 wiki quests (facts only) with their status in each server. |
| `samples/chests/` | `claims.json`, `quests.json`, `catalog.json`, `manifest.json`, `empty_containers.json`. |
| `samples/doors/` | `gates.json`, `manifest.json`. |
| `samples/questlog/` | `quests.json` (the whole quest catalogue), `progress.json`, `manifest.json`. |
| `samples/interactions/` | `interactions.json`, `manifest.json`. |

```sh
pip install -r ../monster-authoring/requirements.txt
python verify_quest_schema.py
python ots_chests.py --canary <opentibiabr/canary at 47dfd51f> --crystal <zimbadev/crystalserver at ff7ede59>
python ots_doors.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_questlog.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_interactions.py --canary <canary checkout> --crystal <crystalserver checkout>
python validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json \
  --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json \
  --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json \
  --progress samples/questlog/progress.json \
  --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json
```

The coverage sample was built from the Fandom API (Template:Infobox Quest, retrieved 2026-09-27)
and a search of both servers' Lua sources; its `method` field records how, including the verdicts
corrected by hand.
