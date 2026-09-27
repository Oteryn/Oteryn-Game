# Encounter authoring tools

Offline tooling for the CANDIDATE Encounter authoring format
(`docs/architecture/OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md`). Evidence only: the server does not
read these files, and every Canary-derived output is `OTS_HYPOTHESIS_ONLY`.

| File | Purpose |
|---|---|
| `build_schema.py` | Generates `encounter.schema.json` (the closed v1 vocabulary, D28). |
| `validate_encounter.py` | JSON Schema plus semantic checks (roles, anchors, state, phases, outcomes, `prevent_death` placement, killer scope, declared references) and the import manifest. |
| `verify_encounter_schema.py` | Focused positive/negative cases on synthetic fixtures. |
| `canary_encounters.py` | Transcribes Canary creature events into encounters with a manifest that maps every source line; participants come from the monster files that register the event. |
| `samples/<encounter>/` | `encounter.json`, `catalog.json` and `manifest.json` per transcribed encounter. |

```sh
python build_schema.py
python verify_encounter_schema.py
python canary_encounters.py --canary <Canary checkout at 47dfd51f>
python validate_encounter.py samples/soul_war_taint_zones/encounter.json \
  --catalog samples/soul_war_taint_zones/catalog.json --manifest samples/soul_war_taint_zones/manifest.json
```

The monster converter (`../monster-authoring/canary_batch.py`) reads the sample manifests: a
monster whose registered event is transcribed here, and which the encounter lists as a
participant, records that event as relocated to the encounter instead of unresolved.
