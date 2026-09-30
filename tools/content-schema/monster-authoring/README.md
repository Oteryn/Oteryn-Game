# Monster authoring schema candidate v1

Contract and decisions: [`docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md`](../../../docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md).

| File | Purpose |
|---|---|
| `build_formal_schema.py` | Source of the schemas; regenerates the five JSON files below. |
| `monster.schema.json` | Creature, Behavior, Presentation and base Loot. |
| `monster-dependencies.schema.json` | Ability, Effect, Formula, Document, Item capability projection, nested Loot. |
| `monster-import-readiness.schema.json` | Field disposition ledger over Git and pinned MediaWiki revision sources. |
| `monster-template.json`, `monster-dependencies-template.json` | Empty field templates; placeholders are deliberately invalid data. |
| `validate_monster.py` | Structural plus semantic/reference validation (exact decimals, canonical ratios, ordering, cycles). |
| `verify_formal_schema.py` | Focused synthetic positive/negative cases; regenerates the `synthetic-*.json` fixtures. |
| `verify_source_coverage.py`, `field-census.json` | Accounts for the 242 inventoried Canary/Crystal registrar/spell paths. |
| `normalize_monster_fields.py` | Bounded helpers for already decoded source geometry and HP values. |
| `canary_batch.py` | Converts the fixed Canary test batches into bundles + manifests (needs `lupa==2.8` and a Canary checkout). |
| `wiki_compare.py` | Compares a batch's plain Canary conversion with TibiaWiki (Fandom) at the programme target date (2026-09-27, D33); its page cache is kept per target date (`<cache>/<target date>/`), so a record is never reused for another cut; records revisions, digests, wikitext lines and compared facts only, plus loot statistics and item ids for loot missing in Canary. |
| `spell_probes.py` | Runs custom-logic Canary spell scripts against stub worlds and records their behaviour, from which the converter derives D18 `summon_creature`, `remove_items` and `affects` data. |
| `test_remove_items_probe.py` | Focused `remove_items` `top_item_first_tile` probe cases (SW-2) on synthetic scripts: the Canary scan resolves; more than one removal, or a scan that goes on after the removal, stays unresolved (needs `lupa==2.8`). |
| `test_windup_template.py` | Focused SW-1 `Ability.windup` template cases on synthetic scripts: the `soulwars fear` shape matches; a missing caster check or a changed cast body stays unresolved (needs `lupa==2.8`). |
| `wiki_scenes.py` | Compares TibiaWiki ability scenes (SceneBuilder shape, effect and missile ids) with the plain Canary conversion rebuilt with the engine area rules; evidence only. |
| `population_census.py` | Converts every Canary monster file in memory with the D15 wiki values applied and records how many validate and resolve, with the blockers that remain; `--bundles DIR` writes the fully resolved bundles outside the repository and refreshes the bundle digest index. |
| `wiki_authored.py` | D44: authors monsters Tibia has at the target date and Canary lacks (Dark Merudri) from pinned Fandom and TibiaWiki BR revisions, with Canary-template values marked NEEDS VERIFICATION; `population_census.py` adds them to the bundles. `fetch --br-capture FILE` refreshes `samples/wiki-authored-2026-09-27.json`. |
| `crystal_batch.py` | Game version 15.30: selects the CrystalServer `summer-update` monsters (commit `00ce02a5`) that Canary lacks and converts them with the Crystal 15.30 item tables; `population_census.py --crystal` adds them to the census. `wiki` refreshes `samples/wiki-population-crystal-00ce02a5-2026-09-27.json`. |
| `official_library.py` | D47: Tibia.com library health and experience from TibiaData captures; `--crystal` also writes the sample for the Crystal monsters. |
| `spell_scripts.py` | Evaluates registered Canary spell scripts in a stubbed sandbox for the converter (which Combat runs, areas, conditions, variants). |
| `spell_census.py` | Classifies every registered spell script that a Canary monster references (P1–P4, NOOP, MISSING) and counts its primitives. |
| `samples/canary-47dfd51f*/` | The two 10-monster Canary test batches and their findings (`README.md`). |
| `samples/wiki-population-2026-09-27.json` | `wiki_compare.py --population`: every convertible Canary monster against TibiaWiki at the 2026-09-27 target date (counts for all facts, rows only for differences, every loot chance trimmed; one monster per line). |
| `samples/wiki-population-2026-07-28.json`, `samples/wiki-scenes-2026-07-28.json` | The same comparisons at the earlier 2026-07-28 cut, kept as the record of the admissions made before D33. |
| `samples/wiki-scenes-2026-09-27.json` | Output of `wiki_scenes.py` (one monster per line; only unmatched abilities and differences listed). |
| `samples/p4-behaviour-patterns-canary-47dfd51f.json` | Proposed D13 grouping of the registered spell scripts that still block monsters into shared parameterized native behaviours (model-assisted, with evidence lines). |
| `samples/population-canary-47dfd51f.json` | Output of `population_census.py` (1,656 files). |
| `samples/wiki-population-crystal-00ce02a5-2026-09-27.json` | `crystal_batch.py wiki`: the Crystal 15.30 monsters against TibiaWiki at the target date, in the rows of the population file. |
| `samples/official-library-2026-09-28.json`, `samples/official-library-crystal-00ce02a5-2026-09-28.json` | `official_library.py`: library health and experience for the Canary and the Crystal 15.30 monsters. |
| `samples/wiki-authored-2026-09-27.json` | The pinned wiki revisions and infobox facts `wiki_authored.py` builds from. |
| `samples/population-bundles-canary-47dfd51f.json` | One SHA-256 per fully resolved population bundle and the SHA-256 of the pinned wiki reference; the bundles themselves are not committed. |
| `samples/events-canary-47dfd51f.json` | Classification of the 193 creature events named by Canary monster files; read by `canary_batch.py`. |
| `samples/spell-census-canary-47dfd51f.json` | Output of `spell_census.py` over all 1,656 Canary monster files. |

```text
pip install -r requirements.txt
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python verify_source_coverage.py
python validate_monster.py synthetic-valid-monster.json synthetic-valid-dependencies.json --catalog synthetic-catalog.json
```
