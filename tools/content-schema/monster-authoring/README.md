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
| `canary_batch.py` | Converts a fixed 10-monster Canary batch into bundles + manifests (needs `lupa==2.8` and a Canary checkout). |
| `samples/canary-47dfd51f/` | That test batch and its findings (`README.md`). |
| `samples/canary-47dfd51f-batch-2/` | Second 10-monster batch, uncommon mechanics and explicit readiness gaps. |
| `verify_canary_batch.py`, `verify_sample_batches.py` | Converter boundaries and validation of all 20 samples, including expected blocked manifests. |
| `wiki_current_comparison.py`, `verify_wiki_current_comparison.py` | Latest TibiaWiki BR structured comparison, revision-bound facts and focused offline checks. |

```text
pip install -r requirements.txt
python build_formal_schema.py && git diff --exit-code -- .
python verify_formal_schema.py
python verify_source_coverage.py
python validate_monster.py synthetic-valid-monster.json synthetic-valid-dependencies.json --catalog synthetic-catalog.json
python verify_canary_batch.py
python verify_sample_batches.py
python verify_wiki_current_comparison.py
python wiki_current_comparison.py --cache <local-cache.json> --refresh
```

`canary_batch.py` additionally requires `lupa==2.8`; only the exact, clean pinned
Canary checkout is accepted. `--batch second` preserves the first batch unchanged.
Wiki comparison requests the latest revision, without a historical date selector;
the raw local cache is not committed, and retained output excludes narrative prose/artwork.
