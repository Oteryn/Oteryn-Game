# PRESENT-CONTENT-1

```yaml
task_id: PRESENT-CONTENT-1
title: "PRESENT-CONTENT-1: Presentation content (client ids, hit table, creature races)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/present-content-1
issue: 1622
governing_issue: 1622
pr: 2130
base_sha: 7a61f366
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
execution_policy: continuous_progress
owned_paths:
  - content/presentations/bindings/client-ids.json
  - content/presentations/bindings/hit-table.json
  - content/creatures/definitions/creature-races.json
  - content/creatures/definitions/index.json
  - content/world/pins/oteryn.json
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
  - docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md
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
- `content/creatures/definitions/creature-races.json`: the race of each of the 1,863 Creatures,
  keyed by definition identity and registered on the Creature family index, from the monster file
  its source binding names at that binding's revision, with the file's sha256. Three take Canary's
  `MonsterInfo::race = RACE_BLOOD` default: `bonelord_totem` and `soul_sphere` set no race, and
  `dark_merudri` is wiki-authored.
- `validate_presentation_bindings.py` rejects a content key without a binding, a stale binding, an
  effect or missile id `appearances.dat` lacks, a colour outside `PresentationColor`, a missing race
  or damage type, a Creature without a known race, and a disagreement with `spell-cues.json`.

## Rulings and deviations (reported to the control plane)

- Race is a Creature family overlay (`native_profile_overlay`, like `spell-native-profiles.json`),
  not a field written into the shards: the shards are regenerated from the World declarations and a
  round-trip check rejects hand edits. It joins by definition identity and revision; the runtime
  reader is COMBAT-PRESENT-1's. The World Bundle `inputs_digest` pin moves with the family
  directory; the bundle digest is unchanged.
- The 15.30 sound files and the sound-id check against them are not in this PR: the files are in
  neither the repository nor the container. On the control plane's ruling (option b) they are
  PRESENT-CONTENT-1b, `BLOCKED_OWNER_INPUT` in the mechanics completion plan; the control plane asks
  the owner for the `sounds/` folder. SPELL-PRESENT-1 and COMBAT-PRESENT-1 emit no sound id
  before PRESENT-CONTENT-1b; PRESENT-CLIENT-1 needs it for sound.

## Validation

- `python tools/content-migration/regenerate_content.py`: pass
- `python -m unittest test_presentation_bindings test_spell_cues` (native-gameplay): OK (13)
- `python tools/content-migration/register_spell_families.py --check`: pass
- `python tools/content-migration/test_import_spell_families.py`: OK
- `cargo run --locked --release -p oteryn-world-bundle-compiler -- pin-check . <out>`: pass
- `cargo test --locked -p oteryn-game-server --test content_world_project_repository`: pass (5)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- `python tools/repository/validate_repository_policy.py`: pass
- `git diff --check`: pass

## Review

- Codex on e25e316d: P2 dynamic keys now must equal the one runtime-resolved weapon missile; P2
  race moved from a presentation sidecar to a Creature family overlay keyed by definition
  identity and revision; P1 sound-id check is PRESENT-CONTENT-1b on the control plane's ruling,
  and the plan gates every sound emitter on it.
