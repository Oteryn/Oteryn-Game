# OTV2-20260927-br-health-experience

```yaml
task_id: OTV2-20260927-br-health-experience
title: TibiaWiki BR health and experience where Fandom is uncertain (D43)
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: 1051
jira: KAN-16
base_sha: a911e667e012005b0c41a3b0974280c4535db69c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude/nice-edison-h9aqh0
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-br-health-experience.md
  - docs/agents/tasks/active/OTV2-20260927-br-population-capture.md
  - docs/agents/tasks/archive/OTV2-20260927-br-population-capture.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/agents/evidence/OTV2-20260927-creature-admission-wave-a-staged.json
  - tools/content-schema/monster-authoring/**
  - tools/content-migration/creature_admission_stage.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - content/items/index.json
  - content/cosmetics/mounts/index.json
  - content/npcs/**
  - content/services/**
  - apps/game-server/**
  - content/world/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/creatures/definitions/**
  - content/presentations/definitions/**
  - content/behaviors/**
  - content/loot/**
  - content/abilities/**
  - imports/canary/**
  - imports/tibiawiki/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner decision D43 ("Tylko HP i doświadczenie"): TibiaWiki BR fills health and experience where the 2026-09-27
Fandom page is missing or gives no certain value. The owner's source order puts BR after Fandom. BR element
modifiers and speed are not used.

- The BR capture now records the page id, the SHA-256 of the revision text and the line of each infobox field. These
  are the pins a MediaWiki manifest source needs.
- `wiki_br_fill.py` selects the fills into `samples/wiki-br-fill-2026-09-27.json`.
- The converter applies them after the Fandom values.
- The census is recomputed, and wave A is restaged and rematerialized.

Authority: owner answer in this session. Runtime behaviour stays unallocated beyond the admitted Creature values.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: content values of admitted creatures change within the existing admission route; no protocol,
persistence or authority change.

## Acceptance and evidence

- `wiki_br_capture.py self-test` and `wiki_br_fill.py self-test` pass.
- The census, staging, tree regeneration, the Rust tests and the governance and policy validators pass.
- The PR gets an exact-head review because `content/world` changes.
