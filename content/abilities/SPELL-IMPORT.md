# Spell data in the content families

The r25 source-attested spell import is registered in `content/manifest.json`,
`content/content.lock.json` and each affected family's `index.json` under
`spell_imports`. These are schema-preserving authoring collections, separate from
the legacy ProjectV2 shards. Their counts do not replace legacy family counts.

| Data | Repository path |
| --- | --- |
| Complete 246-definition catalog | `content/abilities/definitions/player-spells.json` |
| Source selection and five inactive aliases | `content/abilities/definitions/player-spell-selection.json` |
| 128 Ability dependencies | `content/abilities/definitions/player-spell-abilities.json` |
| 167 Effect dependencies | `content/abilities/effects/player-spell-effects.json` |
| 88 Formula dependencies | `content/abilities/formulas/player-spell-formulas.json` |
| Creature profiles and familiar configuration | `content/creatures/definitions/spell-*.json` |
| Presentation profiles | `content/presentations/definitions/spell-native-profiles.json` |
| Appearance and source cue bindings | `content/presentations/bindings/spell-*.json` |
| Item execution policies | `content/items/definitions/spell-native-profiles.json` |
| Training profile | `content/abilities/definitions/spell-build-training.json` |
| Wheel profile | `rulesets/progression/wheel-of-destiny/spell-profile.json` |
| Reference-only source facts, gaps and Thalom source-world input | `imports/spells/r25/` |

The formal collection schema is
`tools/content-schema/spell-authoring/spell-family-import.schema.json`. It reuses
`spell.schema.json`, `spell-dependencies.schema.json` and the existing monster
authoring/import schemas. Native profiles retain their original schema tags and
existing Rust serde contracts, recorded in the index descriptors. Import does not
invent a new runtime schema or convert unsupported expressions into legacy ones.

`content/spells.manifest.json` is the existing native loader's v5 input. Its 11
provider locators now resolve to these content families, rulesets and imports;
loading it needs no test-pack, documentation or source checkout. The qualification
runner's default `map` and `server` modes select it. Ordinary node configuration
may select it through `OTERYN_NATIVE_GAMEPLAY_MANIFEST`, with normal artifact
issuance, scope assignment and admission still required.

Native profile overlays do not create new canonical identities or replace existing
Creature, Presentation or Item definitions. Approximate melee, unsupported attacks,
source aliases and missing Item targets retain their original flags. The 20,640
source monster slots in the gzip sidecar remain reference data. This import does
not activate presentation playback or missing effect dispatchers.

Regenerate and check with:

```sh
python tools/content-migration/register_spell_families.py
python tools/content-migration/register_spell_families.py --check
python tools/content-migration/test_import_spell_families.py
python tools/content-migration/validate_world_project_v2_to_tree.py
bash tools/qualification/spells/run.sh map
```

The normal `world_project_v2_to_tree.py` generator also registers the import, so
regeneration preserves it. Inputs remain pinned to the preserved r25 test pack;
altered providers, sidecars, cue registry or conflicting identities are rejected
before writes. Runtime source switching and deployment are separate operations.
