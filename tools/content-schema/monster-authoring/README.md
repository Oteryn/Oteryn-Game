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
| `wiki_compare.py` | Compares a batch's plain Canary conversion with TibiaWiki (Fandom) at the 2026-07-28 cut; records revisions, digests, wikitext lines and compared facts only, plus loot statistics and item ids for loot missing in Canary. |
| `population_census.py` | Converts every Canary monster file in memory and records how many validate and resolve, with the blockers that remain. |
| `spell_scripts.py` | Evaluates registered Canary spell scripts in a stubbed sandbox for the converter (which Combat runs, areas, conditions, variants). |
| `spell_census.py` | Classifies every registered spell script that a Canary monster references (P1–P4, NOOP, MISSING) and counts its primitives. |
| `samples/canary-47dfd51f*/` | The two 10-monster Canary test batches and their findings (`README.md`). |
| `samples/population-canary-47dfd51f.json` | Output of `population_census.py` (1,656 files). |
| `samples/events-canary-47dfd51f.json` | Classification of the 193 creature events named by Canary monster files; read by `canary_batch.py`. |
| `samples/spell-census-canary-47dfd51f.json` | Output of `spell_census.py` over all 1,656 Canary monster files. |

```text
pip install -r requirements.txt
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python verify_source_coverage.py
python validate_monster.py synthetic-valid-monster.json synthetic-valid-dependencies.json --catalog synthetic-catalog.json
```
