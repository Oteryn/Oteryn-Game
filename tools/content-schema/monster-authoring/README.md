# Monster authoring schema candidate v1

Contract and decisions: [`docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md`](../../../docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md).

| File | Purpose |
|---|---|
| `build_formal_schema.py` | Source of the schemas; regenerates the five JSON files below. |
| `monster.schema.json` | Creature, Behavior, Presentation and base Loot. |
| `monster-dependencies.schema.json` | Ability, Effect, Formula, Document, Item capability projection, nested Loot. |
| `monster-import-readiness.schema.json` | Git-source field disposition ledger. |
| `monster-template.json`, `monster-dependencies-template.json` | Empty field templates; placeholders are deliberately invalid data. |
| `validate_monster.py` | Structural plus semantic/reference validation (exact decimals, canonical ratios, ordering, cycles). |
| `verify_formal_schema.py` | Focused synthetic positive/negative cases; regenerates the `synthetic-*.json` fixtures. |
| `verify_source_coverage.py`, `field-census.json` | Accounts for the 242 inventoried Canary/Crystal registrar/spell paths. |
| `normalize_monster_fields.py` | Bounded helpers for already decoded source geometry and HP values. |
| `canary_batch.py` | Converts the fixed Canary test batches into bundles + manifests (needs `lupa==2.8` and a Canary checkout). |
| `samples/canary-47dfd51f*/` | The two 10-monster Canary test batches and their findings (`README.md`). |

```text
pip install -r requirements.txt
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python verify_source_coverage.py
python validate_monster.py synthetic-valid-monster.json synthetic-valid-dependencies.json --catalog synthetic-catalog.json
```
