# OTV2-20260930-crystal-batch-widen-ordinary

```yaml
task_id: OTV2-20260930-crystal-batch-widen-ordinary
title: crystal_batch.py reads the 28 ordinary CrystalServer monsters outside summer_update_2026
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
owned_paths:
  - docs/agents/tasks/archive/OTV2-20260930-crystal-batch-widen-ordinary.md
  - tools/content-schema/monster-authoring/**
  - tools/content-migration/creature_admission_stage.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - content/**
  - imports/**
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
public_contracts: []
external_repositories:
  - "zimbadev/crystalserver@00ce02a57ca5a12e48f32a3476e37471167e4c3f (branch summer-update, read-only reference)"
```

## Outcome

Owner decision (2026-09-30): the client 15.30 creatures Canary lacks that CrystalServer keeps in another directory come
through a widened `crystal_batch.py`, with the reference-date TibiaWiki values (D15, D33) adopted over the Crystal values
and no wiki ability lists (D18); ordinary monsters first.

- `crystal_batch.EXTRA_MONSTERS` is an explicit allow-list of the 28 `kind == "real monster"` rows with a
  `crystal_other_dir` in `samples/wiki-only-candidates-2026-09-30.json` (inkborn 9, humanoids 4, undeads 2,
  winter_update_2025 13). No directory is widened; `files()` fails if an entry is untracked at the pin or a Canary file
  already creates it, and `require_revision` checks the listed files are clean. Quest, event, raid and summon-like rows
  stay out.
- The converter root is now `data-global/monster` (`MONSTER_ROOT`), so the source file of a converted monster is its real
  Crystal path; the 49 summer files keep `data-global/monster/summer_update_2026/<name>.lua`, so their bundles and
  bindings are unchanged.
- Wiki values are adopted through the existing `adopt_wiki` path, so provenance is the usual mediawiki source and
  `mapped` manifest rows of the D15 wiki reference. `wiki-population-crystal-00ce02a5-2026-09-27.json` gained the 28 rows
  (49 rows byte-identical to before; a re-read differs only in `retrieved_at`).
- No new D-number; the Tibia.com library sample (D47) is not extended: its 2026-09-28 capture is not repeated here.
- `canary_batch.py` and the ITEM-ID-1 logic are unchanged.
- `test_crystal_batch.py` (8 tests; 2 need the pinned checkouts via `OTERYN_CANARY`/`OTERYN_CRYSTAL`).

## Census of the 28 (in memory, before staging)

- 19 fully resolved: bluebeak, bramble wyrmling, cinder wyrmling, crusader, hawk hopper, headwalker, ink splash, lion
  hydra, shell drake, norcferatu nightweaver, dworc shadowstalker, orclops bloodbreaker, creepy crawler, crypt fiend,
  cyclursus, haunted hunter, raubritter chastener, raubritter marksman, stag.
- 7 blocked, all `unresolved_semantics attacks[]` (D10: Crystal-only spell names that are neither inline kinds nor
  registered spells): gloom maw, norcferatu heartless, varg, crypt construct, crypt mage, night harpy, raubritter
  skirmisher.
- 2 structure-invalid: roaming dread, walking dread (corpse decay cycle on `canary:item/52559`; the converter's item
  resolution is out of scope here).

## Staging (local dry run, regenerated in the second commit)

Of the 19 resolved, 17 are admitted (creatures 1476 -> 1493, Crystal bindings 13 -> 30). Deferred: hawk hopper
(unregistered items 51428 and 51560) and haunted hunter (unresolved reference `oteryn:creature.bone_bear`). Nothing of
the 28 is deferred as encounter or initial-health. Pins updated for the regeneration: staged file digest, census index
digest and counts in the materializer, the validator and test of `world_project_v2_to_tree`, the Rust repository test
(`TREE_SHA256`, document digests, Crystal binding prefix `data-global/monster/`, count 30).

## Acceptance and evidence

Local only: `python -m unittest test_crystal_batch` (with checkouts: 8 pass), `wiki_only_candidates.py --check` ok,
`population_census.py`, `creature_admission_stage.py`, the materializer, `world_project_v2_to_tree.py` and its validator
and test pass, `cargo test -p oteryn-game-server --test content_world_project_repository` (3 pass), `cargo fmt --check`
ok. The regenerated outputs are a separate commit and are regenerated on top of the current `main` before the PR.
