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
runner's default `map`, `server` and node-boot modes select it. Ordinary node configuration
selects it through `OTERYN_NATIVE_GAMEPLAY_MANIFEST` (the single selector), with normal
artifact issuance, scope assignment and admission still required. The steps are under
"Activating the book on a node" below.

Native profile overlays do not create new canonical identities or replace existing
Creature, Presentation or Item definitions. Approximate melee, unsupported attacks,
source aliases and missing Item targets retain their original flags. The 20,640
source monster slots in the gzip sidecar remain reference data. This import does
not activate presentation playback or missing effect dispatchers.

## Activating the book on a node

`OTERYN_NATIVE_GAMEPLAY_MANIFEST` is the single selector, for the node and for
`oteryn-game-ops`. There is no path default, no embedded catalogue and no code default; an
unset variable boots the baseline. The issued digests are always computed over the room the
node will actually boot, so set the same value for both.

1. Deploy `content/`, `imports/spells/r25/source-world.json` and
   `rulesets/progression/wheel-of-destiny/spell-profile.json` from one commit under one root
   `<R>`, with their repository-relative paths kept. The manifest's `source_world` and
   `wheel_profile` locators leave `content/`. A missing or unreadable referenced file refuses
   the decode and the node does not boot. Set
   `OTERYN_NATIVE_GAMEPLAY_MANIFEST=<R>/content/spells.manifest.json`.
2. Issue the activation under that variable (operator, root, Content-activation grant):

   ```sh
   oteryn-game-ops --config <ops-config> content activate --world <W> --channel <C> \
     --sequence <N+1> --previous <N> --request content-activation-<W>-<C>-<N+1>.json
   ```

   `<N>` is the scope's current activation sequence (`empty` if none).
3. If the outcome of step 2 is unknown, retry with the request file only:

   ```sh
   oteryn-game-ops --config <ops-config> content activate --request <file>
   ```

   Do not repeat `--world`, `--channel`, `--sequence` or `--previous`; an existing request
   file is replayed exactly and the command refuses them.
4. Restart the node with the variable set. If the digests or frame binding do not match the
   newest activation, the node refuses readiness (`DigestMismatch`, `FrameBindingMismatch`)
   and stays down until the configuration and the activation match. To go back, unset the
   variable, issue the baseline digests at the next sequence with a new request file, and
   restart. Try all steps on a test node first.

The content is `baseline_test` magnitude with source approximation flags: test and
preproduction content, not production numerical parity.

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
