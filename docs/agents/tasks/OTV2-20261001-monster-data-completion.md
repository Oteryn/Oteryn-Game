# OTV2-20261001-monster-data-completion

```yaml
task_id: OTV2-20261001-monster-data-completion
title: Audit and complete monster data under accepted source decisions
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/monster-completion-round3-20261001
pr: null
base_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
head_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
final_head_sha: null
final_head_frozen_at: null
owner: codex-user-directed-monster-completion
created_at: 2026-10-01T11:17:00Z
updated_at: 2026-10-01T16:58:10Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/src/ai_think.rs
  - apps/game-server/src/ai_think/profile_schedule.rs
  - apps/game-server/src/ai_think/profile_schedule_tests.rs
  - apps/game-server/src/content/item_identity.rs
  - apps/game-server/src/content/project/v2.rs
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/src/content/project/v2/encounter.rs
  - apps/game-server/src/spell/tests.rs
  - apps/game-server/tests/content_item_identity.rs
  - apps/game-server/tests/content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - apps/game-server/tests/content_world_project_v2_encounter_admission.rs
  - docs/agents/tasks/OTV2-20261001-monster-data-completion.md
  - tools/content-migration/creature_admission_stage.py
  - tools/content-migration/prepare_canonical_corpse_item_candidates.py
  - tools/content-migration/prepare_creature_item_fact_packets.py
  - tools/content-migration/prepare_noncorpse_item_candidates.py
  - tools/content-migration/qualify_creature_assets.py
  - tools/content-migration/test_creature_admission_stage.py
  - tools/content-migration/test_creature_condition_preservation.py
  - tools/content-migration/test_creature_initial_health.py
  - tools/content-migration/test_creature_reference_closure.py
  - tools/content-migration/test_prepare_canonical_corpse_item_candidates.py
  - tools/content-migration/test_prepare_creature_item_fact_packets.py
  - tools/content-migration/test_prepare_noncorpse_item_candidates.py
  - tools/content-migration/test_qualify_creature_assets.py
  - tools/content-migration/test_verify_spyrat_corpse_source_boundaries.py
  - tools/content-migration/verify_spyrat_corpse_source_boundaries.py
  - tools/content-schema/encounter-authoring/build_schema.py
  - tools/content-schema/encounter-authoring/encounter.schema.json
  - tools/content-schema/encounter-authoring/prepare_custom_spell_drafts.py
  - tools/content-schema/encounter-authoring/prepare_soul_war_accepted.py
  - tools/content-schema/encounter-authoring/samples/round2-soul-war-accepted.json
  - tools/content-schema/encounter-authoring/samples/round3-custom-spell-drafts.json
  - tools/content-schema/encounter-authoring/test_prepare_custom_spell_drafts.py
  - tools/content-schema/encounter-authoring/test_prepare_soul_war_accepted.py
  - tools/content-schema/encounter-authoring/validate_encounter.py
  - tools/content-schema/encounter-authoring/verify_encounter_schema.py
  - tools/content-schema/monster-authoring/build_formal_schema.py
  - tools/content-schema/monster-authoring/canary_batch.py
  - tools/content-schema/monster-authoring/custom-pattern-preparation.schema.json
  - tools/content-schema/monster-authoring/monster.schema.json
  - tools/content-schema/monster-authoring/prepare_custom_patterns.py
  - tools/content-schema/monster-authoring/prepare_deferred_crystal.py
  - tools/content-schema/monster-authoring/prepare_missing_visible_evidence.py
  - tools/content-schema/monster-authoring/samples/custom-pattern-preparation-canary-47dfd51f.json
  - tools/content-schema/monster-authoring/samples/wiki-only-candidates-2026-09-30.json
  - tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json
  - tools/content-schema/monster-authoring/test_corpse_decay.py
  - tools/content-schema/monster-authoring/test_display_name.py
  - tools/content-schema/monster-authoring/test_faction_preservation.py
  - tools/content-schema/monster-authoring/test_familiar_source_costs.py
  - tools/content-schema/monster-authoring/test_independent_source_coverage.py
  - tools/content-schema/monster-authoring/test_loot_source_semantics.py
  - tools/content-schema/monster-authoring/test_partial_source_preservation.py
  - tools/content-schema/monster-authoring/test_prepare_custom_patterns.py
  - tools/content-schema/monster-authoring/test_prepare_deferred_crystal.py
  - tools/content-schema/monster-authoring/test_prepare_missing_visible_evidence.py
  - tools/content-schema/monster-authoring/test_source_disposition_closure.py
  - tools/content-schema/monster-authoring/test_verify_runtime_readiness.py
  - tools/content-schema/monster-authoring/test_visible_fields.py
  - tools/content-schema/monster-authoring/test_wiki_creature_identity.py
  - tools/content-schema/monster-authoring/test_wiki_empty_source_binding.py
  - tools/content-schema/monster-authoring/validate_monster.py
  - tools/content-schema/monster-authoring/verify_formal_schema.py
  - tools/content-schema/monster-authoring/verify_loot_source_semantics.py
  - tools/content-schema/monster-authoring/verify_monster_source_values.py
  - tools/content-schema/monster-authoring/verify_runtime_readiness.py
  - tools/content-schema/monster-authoring/verify_source_coverage.py
  - tools/content-schema/monster-authoring/verify_visible_fields.py
  - tools/content-schema/monster-authoring/wiki_authored.py
  - tools/content-schema/monster-authoring/wiki_compare.py
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Continuation scope and authority

Owner-directed continuation with six subagents completes source-backed data locally in
/workspace/monster-round3; outputs /workspace/monster-round3-output. Root/server AGENTS,
bound META3.1.0 and live#162 apply. Architecture, control-plane, data-admission and production
integration remain with owners; no remote writes/publication are authorized by this work.

## Qualified predecessor

The complete unmodified Round2 artifact is /workspace/monster-round2-output/monster-completion-review.tar.gz,
SHA256 9fa088e8986721269b624cbe140218c505934bdaeeb937891d3ea04267bbf41a.
Its65-path patch,14244 verified members and qualified checks are historical. They certify
unchanged predecessor bytes within their scopes; new Round3 deltas require fresh qualification.

## Source and research restrictions

Pinned sources remain Canary47dfd51f45280a59a1d3e50ba7edd573d7234446 and Crystal
00ce02a57ca5a12e48f32a3476e37471167e4c3f, with D15/D18/D25/D33/D43/D44/D47 source order and
2026-09-27 wiki cut preserved. Normal Git/HTTP is preferred. Fandom402/WikiBR403 may use real
Chrome/CDP through Remote Desktop Commander ONLY for public internet research. No project files,
private files or administrative operations use the owner's computer. Exact URLs/methods/uncertainty
remain in receipts; raw wiki caches are excluded from redistributed evidence.

## Round3 implementation and frozen data

Current generated population is1642 authoring
bundles and1600 native-stage Creature candidates,22390 records,21338 profiles and61 Encounters.
The42 defers remain26 Encounter,15 unresolved references and1 Item. Stage SHA256
745fce7d98e02227b178be0720e0e365fd7a943eed2c7d176ed2273e055b306c; census index SHA256
e9d49f92c182afe72d7b7f274061a7245dffc056b22d33b42ad57d991d6af97d.

- Exact source/context identities recover18 wiki records:1574 COMPARED,12 identity UNKNOWN,
  64 missing pages in1650 observations. Sample SHA256
  77d3b596ff91dc3e3accbbe2c6ae209f5dd2fe21aca451c8710bb576a0dc1fb6;1632 rows unchanged.
- Familiar generic Wiki summoning no longer changes special familiar eligibility. Existing
  special carrier gets exact registered player costs knight1000,druid3000,paladin2000,
  sorcerer3000,monk1500, with source proof. Knight/Paladin/
  Monk generated bundles remain reference-deferred; Druid/Sorcerer source drafts remain blocked.
  Paladin/Monk generated cost changes are not native admission or production summon execution.
- Fifteen generated bundles changed;1627 are byte-identical to the qualified predecessor.
  All1642 pass exact-Decimal local structure/semantics/reference and census-digest validation.
- All1600 candidates preserve HP/XP/armor/speed and summon/convince flags; mitigation993 present/
  607 source-unspecified. Bestiary773 present/827 undeclared in source; this does not prove
  Global ineligibility. Every absent value remains explicit, never silently zero.
- The accepted prepared SUMMON component covers167 actual profiles when callers supply exact
  eligibility/count facts. Retry/clock/snapshot checks are explicit; no-facts legacy still refuses.
  Production dispatch/birth/link/caps commitment and1 defensive-melee profile remain open.
- Forty custom Spell identities/43 references have source-bound boundary packets. Three
  schema-valid partial cores (generator,Maxxen teleport,Zamulosh teleport),7 Ability definitions,
  4 presentation definitions and2 Encounter successor copies preserve RNG order/positions/effects.
  No complete conversions,new covers or admission. targetfirering actual nil->TYPE0->
  UNDEFINEDDAMAGE4 is proven separately from unverified intended wiki behavior.
- Twenty-five schema-valid noncorpse Item drafts are prepared from46 exact candidates;18
  existing owner routes and3 unresolved cases stay separate. All25 lack admitted sprite atlas
  binding;0 native admissions. Canonical/projection source packets are regenerated independently.
- Four Spyrat raw lookTypeEx IDs are retired without successor; no alias/remapping is invented.
  CandyHorror48267 has a5s source decay target48296 absent in Item/appearance bindings. Compiled
  original-source guard witnesses prove missing-target transformation returns the same item
  and stops decay; target removal or cross-source48268 alias is not asserted.
- Canary91 blocked monster drafts are structurally valid, plus1 NON_MONSTER helper. Crystal22
  source preparations retain19 valid/3 invalid source profiles;3 exact [2,3,5]/VeryRare correction
  drafts pass schema and remain pending owner threshold/quest-admission decisions. Two Muglex
  full Bestiary proposals likewise remain pending the D15 taxonomy/class scope decision.

## Current qualification and limitations

Exact source values and all16743 authored loot entries have bounded consistency proof. Mapper
checks1642 bundles/582860 leaves with0 mapping findings or unread nonempty fields. Fresh strict
registrar retains unused missing MonsterType.skull API (0 actual assignments); arbitrary/contextual
Lua meaning remains UNKNOWN. Appearance availability1592 present/4missing/4invisible is not native
Asset admission or full effect/missile qualification.

Full Rust package:16316 top-level PASS/0FAIL/7 existing external-topology ignored;1 nested replay
PASS separately. Formatting/clippy, actual native materialization, unchanged tree-writer export,
Item checks and complete-path independent review are bound by the terminal
/workspace/monster-round3-output/final-validation-summary.json. Only the exact completed receipt
certifies current local bytes. Full schema INCOMPLETE (3/11 original groups resolved); production
Creature/Ability/Condition/Encounter execution UNKNOWN. Architecture/control-plane work and
taunt/HP0/permanent/condition/future-dependency admission remain with owners.

Local branch is uncommitted, no remote frozen head/PR. Current main aad17f99 has84 changed paths,
0 owned overlap; base remains d3cf. Live#162 D252 applies; no remote project writes. Complete
patch includes all new paths and readonly baseline apply-check/whole-patch whitespace checks.
Final archive must verify all member hashes and exclude raw wiki prose, third-party assets and
ephemeral engine-source extraction.
