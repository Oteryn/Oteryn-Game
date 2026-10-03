# OTV2-20261001-monster-mitigation-list-round5

```yaml
task_id: OTV2-20261001-monster-mitigation-list-round5
title: Complete the creature list and repair accepted monster schema and mechanics defects
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/monster-list-mitigation-round5-20261001
pr: null
base_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
head_sha: d3cfb2468510255d2495514ec975c23c1d76f32a
final_head_sha: null
final_head_frozen_at: null
owner: codex-user-directed-monster-completion
created_at: 2026-10-01T19:20:00Z
updated_at: 2026-10-01T21:38:28.036041Z
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/OTV2-20261001-monster-mitigation-list-round5.md
  - tools/content-schema/monster-authoring/wiki_compare.py
  - tools/content-schema/monster-authoring/samples/wiki-population-2026-09-27.json
  - tools/content-schema/monster-authoring/test_wiki_mitigation_variants.py
  - tools/content-schema/monster-authoring/test_wiki_horse_mitigation.py
  - tools/content-schema/encounter-authoring/prepare_soul_war_accepted.py
  - tools/content-schema/encounter-authoring/test_prepare_soul_war_accepted.py
  - tools/content-schema/encounter-authoring/samples/soul_war_taint_zones/encounter.json
  - tools/content-schema/encounter-authoring/samples/soul_war_taint_zones/manifest.json
  - tools/content-schema/encounter-authoring/samples/soul_war_taint_zones/catalog.json
  - tools/content-schema/encounter-authoring/samples/round2-soul-war-accepted.json
  - tools/content-schema/monster-authoring/build_formal_schema.py
  - tools/content-schema/monster-authoring/monster.schema.json
  - tools/content-schema/monster-authoring/verify_formal_schema.py
  - apps/game-server/src/content/project/v2/creature.rs
  - apps/game-server/tests/content_world_project_v2_creature_admission.rs
  - tools/content-schema/monster-authoring/crystal_batch.py
  - tools/content-schema/monster-authoring/samples/wiki-only-candidates-2026-09-30.json
  - tools/content-schema/monster-authoring/samples/wiki-population-crystal-00ce02a5-2026-09-27.json
  - tools/content-schema/monster-authoring/test_crystal_batch.py
  - tools/content-schema/monster-authoring/canary_batch.py
  - tools/content-schema/monster-authoring/test_wiki_bestiary_matrix.py
  - tools/content-schema/monster-authoring/samples/source-bestiary-matrix-2026-09-27.json
  - tools/content-schema/monster-authoring/test_wiki_staticdata_identity.py
  - tools/content-schema/monster-authoring/test_prepare_deferred_crystal.py
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authorization

The owner requests continuous completion of all repairable monster data and schema
issues, including statistics, loot, Bestiary and spell/mechanics authoring. The
849-row public mitigation list is an accepted numeric observation source, not
proof that every source registration has the same identity or executable rules.
The owner explicitly requires parallel subagents and records architecture and
coordinator decisions in issue #162. The active coordinator's D252 publication
question stays pending; this work produces a local reviewable successor.

## Candidate and ownership

This isolated local successor applies the complete qualified Round4 patch from
base d3cfb246. Round4 archive SHA256
bb62b6512da9a342ad21e84e264033c81271a7dad3ae600bf2427d0aa522f389 and predecessor
source bytes remain immutable. Root owns integration and this task record.

Disjoint workers own exact Wiki identity/test routes, the primary Wiki data sample,
Crystal selections/data and the existing Bestiary adopter, six accepted Soul War
preparation paths, and five accepted AI-limit schema/native paths. Independent
reviewers own no changed project paths. Statistics/loot audits, 16 Wiki-only
source-fact drafts, reference-gap research and native qualification write only
outside the repository, under /workspace/monster-round5-output. Further bounded
identity routes may only proceed from exact accepted staticdata and canonical
Creature facts; a contradictory race identifier is never treated as absent.

## Acceptance and qualification

- Reconcile every supplied numeric mitigation observation against actual generated
  bundles and native projection, keeping prepared/import/identity boundaries apart.
- Correct both exact horse mitigation bindings (2/25 percent), preserving unrelated
  fields and their misleading donor color labels; keep mitigation-only scope.
- Prepare all 25 additional Crystal Creature candidates from exact dated pages;
  select 18 ordinary registrations while retaining independent event/boss and
  unresolved mechanics boundaries. Use actual Imperial staticdata and loot proof.
- Compare all 786 existing Bestiary profiles with separately revision-pinned public
  difficulty/rarity/threshold/charm facts. Never infer a field from compact DIFF
  absence or promote unqualified identities. Very Rare thresholds are 2/3/5.
- Repair existing formal/native AI limits RL07/RL08: attacks16, defences8,
  summon entries8, max_summons16, per-entry count16; reject each first excess.
- Preserve accepted Soul War source roster, boxes, holes, teleport roles, vocation
  predicates and closed participant dependencies; leave unresolved SW6/E4 runtime
  ownership and gameplay semantics explicit.
- Recheck all statistics, loot rows and effective accepted Item aliases against
  exact source/data/native profiles after the complete source freeze. Batch
  preservation checks never claim complete Tibia Global enrichment.
- Prepare 16 missing-donor Wiki-only drafts with fact-level provenance and explicit
  unverified template values; they must remain outside native admission.
- Bind appropriate tests, actual generated outputs and independent review to exact
  current source bytes. Package source/fact/evidence files only; exclude raw Wiki
  prose, proprietary captures and third-party binary assets.

## Evidence and remaining boundaries

Generated data remains external. Artifact SHA256 is a local byte qualification,
not a remote FREEZE_SHA. Previous horse-only receipts are historical component
checks and cannot certify this expanded cohort. The complete final population,
source freeze, module review and validation receipts supersede those checks.

No remote publication, production activation or new architecture is selected.
Actual source-unspecified data, missing sprite/Item admission, source ID conflicts,
and unavailable public drop statistics remain explicit. Existing accepted runtime
consumers requiring allocation stay with their owning lanes and #162. Local
structural/native materialization does not prove all production spell execution
or complete Global parity.

## Local candidate byte freeze

All disjoint source writers finished authoring. The external final-source-byte-freeze.json
binds every changed path against the qualified Round4 predecessor. Formal270 and
governance validation passed; full current population regeneration, appropriate
Python/source checks and bounded native qualification bind their final receipts
in final-validation-summary.json. Final runtime and production status stays false.

The final regression run found obsolete deferred-roster expectations after the 18
ordinary additions. Only their existing regression file was corrected to assert
the exact retained four and accepted eighteen paths, event admission separation,
and default roster stability. Source data and all 371 Rust inputs stayed frozen.
