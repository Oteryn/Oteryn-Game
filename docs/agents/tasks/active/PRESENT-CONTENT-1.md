# PRESENT-CONTENT-1

```yaml
task_id: PRESENT-CONTENT-1
title: "PRESENT-CONTENT-1: Presentation content (client ids, hit table, creature races)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/present-content-1
issue: 1622
governing_issue: 1622
pr: null
base_sha: 7a61f366
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - content/presentations/bindings/client-ids.json
  - content/presentations/bindings/hit-table.json
  - content/presentations/bindings/creature-races.json
  - content/presentations/bindings/index.json
  - content/presentations/definitions/index.json
  - content/manifest.json
  - content/content.lock.json
  - tools/content-schema/native-gameplay/build_presentation_bindings.py
  - tools/content-schema/native-gameplay/build_creature_races.py
  - tools/content-schema/native-gameplay/validate_presentation_bindings.py
  - tools/content-schema/native-gameplay/test_presentation_bindings.py
  - tools/content-migration/import_spell_families.py
  - tools/content-migration/test_monster_seven_spell_overlay.py
  - .github/workflows/content-tree-migration.yml
  - docs/agents/tasks/active/PRESENT-CONTENT-1.md
  - docs/agents/tasks/archive/PRESENT-CONTENT-1.md
public_contracts: []
external_repositories: []
```

## Outcome

SPELL-PRESENT-0 §3 and §5 content (accepted with conditions, #2099), generated from pinned sources
and checked without them:

- `client-ids.json`: each of the 421 effect, missile and sound keys used in `content/` (the hit
  table included) bound to its 15.30 client id and the enum member that supplies it. Canary at the
  cue PIN supplies 401; Crystal 00ce02a5 supplies the 18 newer effects Canary's PIN lacks.
  `canary.appearance:effect/poison` (harpy) and `effect/redspark` (Izcandar) are defined by neither
  source; Lua reads nil there, so they bind to no effect (null) with a note. The weapon missile stays
  dynamic.
- `hit-table.json`: §5's colour and hit effect per damage type and per race, parsed from Canary
  `Game::combatGetTypeInfo`, `combatChangeHealth` and `combatChangeMana`, with source lines. Players
  are blood.
- `creature-races.json`: the race of each of the 1,863 Creatures from the monster file its source
  binding names at that binding's revision, with the file's sha256. Three take Canary's
  `MonsterInfo::race = RACE_BLOOD` default: `bonelord_totem` and `soul_sphere` set no race, and
  `dark_merudri` is wiki-authored.
- `validate_presentation_bindings.py` rejects a content key without a binding, a stale binding, an
  effect or missile id `appearances.dat` lacks, a colour outside `PresentationColor`, a missing race
  or damage type, a Creature without a known race, and a disagreement with `spell-cues.json`.

## Rulings and deviations (reported to the control plane)

- Race is an overlay beside the other bindings, not a field in the Creature shards: the shards are
  regenerated from the World declarations and a round-trip check rejects hand edits (precedent:
  `spell-native-profiles.json`).
- The 15.30 sound files and the sound-id check against them are not in this PR: the files are not in
  the repository or the container (BLOCKER sent to the control plane).

## Validation

## Review
