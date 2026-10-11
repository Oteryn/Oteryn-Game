# OTERYN-20261011-decanary-game-2a

```yaml
task_id: OTERYN-20261011-decanary-game-2a
title: "GAME-DECANARY-2a neutral tool flags and B6 CI without external source fetch"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
lane_id: decanary
base_branch: main
branch: claude/decanary-game-2a-20261011
pr: 1986
base_sha: 3ce431413
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-11
updated_at: 2026-10-11
packet: "owner decision: remove reference-engine naming; slice 2a live couplings in tools and workflows"
owned_paths:
  - .github/workflows/content-world-b6-interaction-bindings.yml
  - tools/content-migration/npc_dialogue_stage.py
  - tools/content-schema/charm-authoring/README.md
  - tools/content-schema/charm-authoring/charm_authoring.py
  - tools/content-schema/charm-authoring/charm_mechanics.py
  - tools/content-schema/condition-authoring/README.md
  - tools/content-schema/condition-authoring/condition_authoring.py
  - tools/content-schema/encounter-authoring/README.md
  - tools/content-schema/encounter-authoring/canary_encounters.py
  - tools/content-schema/encounter-authoring/prepare_encounter_completion.py
  - tools/content-schema/monster-authoring/build_monster_presentation_closure.py
  - tools/content-schema/monster-authoring/canary_batch.py
  - tools/content-schema/monster-authoring/official_library.py
  - tools/content-schema/monster-authoring/wiki_only_candidates.py
  - tools/content-schema/monster-authoring/wiki_scenes.py
  - tools/content-schema/native-gameplay/build_canonical_monster_profiles.py
  - tools/content-schema/native-gameplay/build_monster_melee_profiles.py
  - tools/content-schema/native-gameplay/build_source_item_profiles.py
  - tools/content-schema/native-gameplay/build_spell_cues.py
  - tools/content-schema/native-gameplay/canary_generic_item_identity.py
  - tools/content-schema/npc-authoring/README.md
  - tools/content-schema/npc-authoring/promotion_candidates.py
  - tools/content-schema/npc-authoring/source_diff.py
  - tools/content-schema/npc-authoring/validate_promotion.py
  - tools/content-schema/quest-authoring/README.md
  - tools/content-schema/quest-authoring/donor_sources/npc_capture.py
  - tools/content-schema/quest-authoring/ots_chests.py
  - tools/content-schema/quest-authoring/ots_doors.py
  - tools/content-schema/quest-authoring/ots_gap_triage.py
  - tools/content-schema/quest-authoring/ots_map_check.py
  - tools/content-schema/quest-authoring/quest_donor_source_authoring.py
  - tools/content-schema/spell-authoring/README.md
  - tools/content-schema/spell-authoring/vocation_vitals.py
  - tools/content-schema/store-authoring/README.md
  - tools/content-schema/store-authoring/store_census.py
  - tools/reference-world-corridor-census/README.md
  - docs/agents/tasks/archive/OTERYN-20261011-decanary-game-2a.md
public_contracts: []
external_repositories: []
```

## Outcome

Second slice: the live couplings that can move without changing a pinned digest or golden.

- 27 content tools take `--reference-engine`, `--reference-engine-source`,
  `--reference-engine-bundles` and `--reference-engine-reference-bundles` instead of the
  engine-named flags. `args.reference_engine*` is the matching attribute. 7 authoring READMEs show
  the new commands.
- `content-world-b6-interaction-bindings.yml` no longer checks out an external server repository,
  the legacy parser or the client asset archive. The job is now `Content World B6 / offline
  self-tests` and runs the four offline self-tests. The tracked evidence is regenerated locally, as
  described in `tools/reference-world-corridor-census/README.md`.
- No generated output, data key, provenance string, digest or golden changed.

## Left in place, with reason (follow-up PRs)

- **20 tools whose file sha256 or git blob is pinned in evidence, imports, samples or
  `docs/reference`.** Editing them breaks `test_engine_items` and the byte-freeze records.
  - `content-census/item_wiki_family_capture.py`
  - `content-migration/prepare_spell_core_completion.py`
  - encounter-authoring: `prepare_custom_spell_drafts.py`, `prepare_soul_war_accepted.py`
  - monster-authoring: `crystal_batch.py`, `population_census.py`, `prepare_custom_patterns.py`,
    `prepare_deferred_crystal.py`, `spell_census.py`, `verify_source_coverage.py`, `wiki_compare.py`
  - native-gameplay: `build_companion_population_profiles.py`, `build_familiar_profiles.py`,
    `build_missing_companion_profiles.py`, `build_skeleton_profile.py`,
    `build_spell_item_identities.py`
  - quest-authoring: `ots_interactions.py`, `ots_questlog.py`, `source_text_authoring.py`
  - `spell-authoring/build_training_profile.py`
- **Tools called with the old flag by workflows that #1985 owns:**
  - `spell-authoring/spell_census.py`, `convert_spells.py` and `verify_spells.py`
  - `item-authoring/client_appearance_census.py`
- **Tools that build their flags dynamically (`'--' + name`):**
  - `complete_creature_dependencies`, `prepare_glooth_generator` and
    `build_monster_classification_evidence`, together with their two monster READMEs
- **The `imports/canary/` and `imports/crystalserver/` directory renames.** These paths are
  referenced from:
  - item-authoring and store-authoring workflows (#1985)
  - `item_admission.rs` and `native_gameplay.rs`
  - `content/manifest.json` and `content/items/taxonomy/items.json` (#1924)
  - evidence digests
  - `g4-item-crystal-bindings.yml` and `world-metadata-authoring.yml`, which must move with the
    directories
- **Other engine names that flow into goldens and workflows in flight:**
  - `ENGINES["canary"]` in `engine_items.py`
  - the `canary_*` pin outputs in `spell-wiki-capture.yml`
  - `samples/spell-census-canary-*.json`
- **Seed and vendor paths owned elsewhere:**
  - the `g4-canonical-worldproject-package-seed.yml` source fetch (#1944)
  - the game-atlas legacy vendor paths
- **Docs and data:** `docs/reference/**` READMEs and archived task records (slice 2b), and Rust
  identifiers (slice 2c).
- **Guard denylists:** kept literal, by owner decision 1a.

## Validation

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- renamed tools: `py_compile` pass, and each `--help` lists `--reference-engine` (26 of 26 with own
  flags)
- tool tests:
  - charm 13 pass
  - crystal_batch 15 OK (4 skipped)
  - npc promotion 80 OK
  - npc dialogue stage 22 OK
  - quest semantic conditions 15 OK
  - quest semantic rewards 8 OK
  - engine items PASS (599 checks)
  - vocation_vitals self-test ok
- B6 offline self-tests:
  - interaction_binding_catalog_self_test
  - corridor self_test
  - content_source_batch_self_test
  - game-atlas-fullworld-source self_test
- workflow YAML parse: pass
- `git diff --check`: clean
- Pre-existing failures, identical on base `3ce431413`:
  - `test_condition_authoring` (`test_committed_build_is_current_and_valid`)
  - `validate_repository_policy.py` (E8002 from `test_classify_post_merge_lanes`)

## Review

No paid review requested.
