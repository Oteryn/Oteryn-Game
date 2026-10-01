# OTV2-20260930-crystal-only-spells

```yaml
task_id: OTV2-20260930-crystal-only-spells
title: Crystal-only monster spells and the D47 library capture for the 28 ordinary Crystal monsters
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
base_sha: f38d7f3
branch: claude/friendly-albattani-rfzvku
issue: 162
pr: null  # the PR on this branch is authoritative
jira: KAN-16
head_sha: null
final_head_sha: null
final_head_frozen_at: null
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
depends_on:
  - OTV2-20260930-crystal-batch-widen-ordinary
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260930-crystal-only-spells.md
  - tools/content-schema/monster-authoring/**
  - tools/content-migration/**
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/**
  - imports/**
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
public_contracts: []
external_repositories:
  - "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f (branch summer-update, read-only reference)"
```

## Outcome (owner answers 1a and 3b, 2026-09-30)

Crystal-only spells (1a). The 7 monsters left blocked by `OTV2-20260930-crystal-batch-widen-ordinary` named 13 spells that
only CrystalServer registers, all in `data-global/scripts/spells/monster/`. Twelve are plain area combats
(`Combat` + `createCombatArea` + `combat:execute`); `clouds chain` is a plain combat with a fixed chain value callback
(3, 3, false). None has custom logic, so no D18 probe pattern is needed and no wiki ability list is used.

- `spell_scripts.SpellScripts` and `index_spells` take `extra_roots`; a root adds only the spells of
  `EXTRA_SCRIPT_DIRS` (`data-global/scripts/spells/monster`) whose name Canary does not register. Canary wins on a name
  clash. `crystal_batch.converter` passes the Crystal checkout, and `require_revision` requires that directory clean and
  untracked-free at the pin.
- The existing converter route (`registered_spell`, tier P2) evaluates them exactly as it does a Canary script; the
  manifest names the Crystal script path. The Canary census is unchanged (no extra root). `canary_batch.py` and
  `spell_probes.py` are not edited, so the SW-1 change merges cleanly.
- All 13 spells convert; all 7 monsters (gloom maw, norcferatu heartless, varg, crypt construct, crypt mage, night
  harpy, raubritter skirmisher) become fully resolved. The manifest note still says "as Canary does" for these.
- Side effect: the 49 summer monsters use the same spell index; Phosphorus final, blocked before, is now resolved too.

D47 library for the 28 (3b). `official-library-crystal-extra-00ce02a5-2026-09-30.json` is a new dated sample (the
2026-09-28 captures are not repeated), written by `official_library.py --extra-captures <dir> --crystal <checkout>`,
which fetches the TibiaData records itself. The news check covers 2026-09-27 to 2026-09-30: 8980 (Fixes and Changes)
and 8989 (a fan-site ticker) change no creature; 8947 and 8979 were checked on 09-28. 27 of 28 have a library entry
(Ink Splash has none); the 10 without wiki health all have library health. `population_census.py` merges the sample into
the Crystal official facts and records its hash; the converter adopts it as `official_capture` as for the 647 others.

## Result

Census (Crystal group): 75 fully resolved, 0 blocked, 2 structure-invalid (roaming dread, walking dread: corpse decay
cycle on `canary:item/52559`, item-resolution logic out of scope). Stage: creatures 1493 -> 1500, Crystal bindings
30 -> 37; all 7 monsters are admitted. Deferred among the 28: hawk hopper (items 51428, 51560) and haunted hunter
(`oteryn:creature.bone_bear`).

## Acceptance and evidence

Local only: `test_crystal_batch.py` 13 tests pass (with checkouts), census, stage, materializer, tree generator, its
validator and test, `content_world_project_repository` (3 pass) and `cargo fmt --check` pass. The regenerated outputs
are a separate commit and are regenerated on top of the current `main` before the PR.

## Publication (restaged on main f38d7f3, after SW-1)

Both task commits are published in one PR with the restage on top of SW-1. `SpellScripts` keeps main's
`accepted_guards` and takes `extra_roots`; a spell found under an extra root carries `extra_root`, and the converter
note then reads "resolved by name as CrystalServer does" instead of Canary (owner answer 2a). Owner answer 1a on
Ink Splash: it has no Tibia.com library entry and keeps its wiki values (hp 1950, exp 1450).

E4 restage: 1,503 creatures, 21,069 records, 20,097 profiles (was 1,479 / 20,693 / 19,745); Crystal bindings 13 to
37. Tree families: Creature 1,503, Ability 6,000, Effect 4,599, Formula 4,905, Loot 1,056, Presentation and Behavior
2,605. Census: Crystal 75 fully resolved, 2 structure-invalid (roaming dread, walking dread: corpse decay cycle on
item 52559, a separate task after ITEM-ID-1, owner answer 2a of the earlier batch).
