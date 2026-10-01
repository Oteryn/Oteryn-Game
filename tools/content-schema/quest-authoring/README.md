# Quest authoring tools

Offline tooling for the CANDIDATE quest format
(`docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md`). Evidence only: the server does not read
these files, and every source-derived output is `OTS_HYPOTHESIS_ONLY`.

| File | Purpose |
|---|---|
| `lua_tables.py` | Reads Lua table constructors (and their comments) without a Lua interpreter. |
| `ots_doors.py` | Transcribes the quest, key and level doors of both servers (`door_quest.lua`, `door_key.lua`, `door_level.lua`), joined by map position, into gates linked to the chest claims and keys, plus the `QuestDoorUnique` doors each opened by their own dedicated script (the Katana Quest lever door). |
| `ots_questlog.py` | Transcribes the quest logs of both servers into storyline quests with staged missions (D34) and their transitions (D35), joined by quest and mission name; writes the whole quest catalogue and the progress tracks with per-server transition sources. |
| `lua_writers.py` | Finds every storage write in the Lua sources and reads it as a candidate transition (D35): owner, callback, effect, `from` stage through its if-block, script registrations. |
| `lua_blocks.py` | Splits a Lua callback body into if/elseif/else branches, loops and statements (early `return` makes the rest an implicit `else`). |
| `ots_interactions.py` | Transcribes every quest script of both servers into interaction definitions (D36): edge, read-only conditions and children for the Quest, Ability, Item, Achievement and Presentation owners; Movement children are D37 relocations (to a named anchor or the previous tile; a computed target stays blocked) and WorldObject children are D38 overlay operations (`TRANSFORM`/`CREATE`/`REMOVE`/`RETAG`, optionally `revert_after_ms`; a call the converter cannot yet classify by kind stays blocked); joined by quest and script path. |
| `ots_chests.py` | Transcribes the reward chests of Canary and CrystalServer (`startup/tables/chest.lua` plus the text and achievement tables of `quest_reward_common.lua`), joined by map position, into reward claims, reward-only quests, a catalog and a manifest. |
| `quest_content.schema.json` | JSON Schema of reward claims, door gates, reward-only and storyline quests. |
| `conflict_decisions.json` | D25 decisions for every Canary/CrystalServer conflict of the chest, door, quest-log and interaction transcriptions: chosen server, basis, the difference in our own words and the wiki revision when it decides; the converters apply it and fail on a stale decision. |
| `interaction_overrides.json` | Curated replacements for an interaction condition `ots_interactions.py` cannot read statically because it lives in a sibling `lib/quests/*.lua` table indexed by a role field or a world state: interaction key and source line to the resolved condition (existing D36 vocabulary only) plus a basis citing the table and its registration; `ots_interactions.py` applies it and fails on a stale entry (a line no longer unresolved). |
| `track_owners.json` | Owning quest of the progress tracks quest scripts write outside missions, where no mission-track prefix or script directory names it; the quest-log converter fails on a missing or stale record. |
| `script_quests.json` | Wiki quests confidently matched to a script directory and/or the `wiki_quest` of an auxiliary progress track, added to the catalogue as `kind: script_only` (a quest the servers implement in scripts with no quest-log entry); each entry gives its basis. |
| `chest_quest_links.json` | Curated chest-to-quest links for claims `ots_chests.py`'s own kv_quest_name/storage_key/label/section match could only find a section-header candidate for, or no link at all: each cites the Fandom wiki (an item, key or quest page) or the quest-coverage sample's own recorded source evidence; applied with `quest_link_basis: curated`, and the converter fails on a stale link. |
| `interaction.schema.json` | JSON Schema of interaction definitions (D36), including the D37 relocation and D38 world-object overlay child shapes. |
| `ots_map_check.py` | Checks chest, door and interaction positions against Canary's `otservbr.otbm` and CrystalServer's `world.otbm` (not committed; sha256 pinned), reading Canary's startup id tables; writes `samples/map-check/report.json`. |
| `ots_readiness.py` | Per quest, the engine features it needs and its data gaps, and the unlock order; reads only the committed samples. |
| `ots_gap_triage.py` | Classifies every unresolved interaction line and unresolved condition into `owner_pending` (an already-named or same-shape missing owner), `shared_mechanism` (a recurring >=3-quest pattern with an explicit rule) or `bespoke`; per quest and a ranked, unlock-order summary; reads the committed samples plus the pinned checkouts for line text; writes `samples/gap-triage/triage.json`. |
| `validate_quest_content.py` | Schema plus semantic checks: unique keys and positions, non-empty rewards, text on a handed-out item, claim/quest links in both directions, gate conditions against the claims (progress marker, key source), one identity per quest, mission ranges and stages against the progress tracks, catalog and manifest coverage; for interactions: anchors, blocked reasons, named transitions against the missions, undeclared progress tracks, manifest status. |
| `verify_quest_schema.py` | Focused positive/negative cases on synthetic fixtures (`--verbose` prints each case's first error). |
| `refresh_quest_source_checks.py` | Refreshes exact inventories of missing gate/condition references after conversion; unresolved and blocked definitions never become `mapped` merely because their JSON shape is valid. `--check` verifies reproducibility. |
| `test_quest_completeness.py` | Regression cases for quest ownership, blocked children, missing reads/gates and stale diagnostic inventories. |
| `samples/quest-coverage-2026-09-27.json` | The 373 wiki quests (facts only) with their status in each server. |
| `samples/chests/` | `claims.json`, `quests.json`, `catalog.json`, `manifest.json`, `empty_containers.json`. |
| `samples/doors/` | `gates.json`, `manifest.json`. |
| `samples/questlog/` | `quests.json` (the whole quest catalogue), `progress.json`, `manifest.json`. |
| `samples/interactions/` | `interactions.json`, `manifest.json`. |
| `samples/gap-triage/` | `triage.json`. |

```sh
pip install -r ../monster-authoring/requirements.txt
python verify_quest_schema.py
python ots_chests.py --canary <opentibiabr/canary at 04b83b51> --crystal <zimbadev/crystalserver at 9f5a72c6>
python ots_doors.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_questlog.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_interactions.py --canary <canary checkout> --crystal <crystalserver checkout>
python refresh_quest_source_checks.py
python ots_readiness.py
python ots_gap_triage.py --canary <canary checkout> --crystal <crystalserver checkout>
python ots_map_check.py <otservbr.otbm> --crystalserver <decompressed world.otbm> --canary <canary checkout>
python validate_quest_content.py samples/chests/claims.json samples/questlog/quests.json \
  --catalog samples/chests/catalog.json --manifest samples/chests/manifest.json \
  --gates samples/doors/gates.json --gates-manifest samples/doors/manifest.json \
  --progress samples/questlog/progress.json \
  --interactions samples/interactions/interactions.json --interactions-manifest samples/interactions/manifest.json
python test_quest_completeness.py
python refresh_quest_source_checks.py --check
```

The coverage sample was built from the Fandom API (Template:Infobox Quest, retrieved 2026-09-27)
and a search of both servers' Lua sources; its `method` field records how, including the verdicts
corrected by hand.
