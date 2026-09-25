# OTV2-20260925-item-enrichment-wave1

```yaml
task_id: OTV2-20260925-item-enrichment-wave1
title: Item enrichment Wave 1 (165 EXACT TibiaWiki bindings)
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/eloquent-volta-p6174v
issue: 162
pr: null
base_sha: 08a8d5d49e767476df7be10949042e539db414ca
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (lease request #162 comment 5838752108)
jira: KAN-16
created_at: 2026-09-25T20:15:00Z
updated_at: 2026-09-25T20:15:00Z
execution_policy: continuous_progress
owned_paths:
  - content/world/**
  - content/items/**
  - content/manifest.json
  - content/content.lock.json
  - content/project.json
  - imports/tibiawiki/**
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - tools/content-census/g4_item_wave1_*.py
  - tools/content-migration/*.py
  - .github/workflows/g4-item-wave1-capture.yml
  - docs/agents/evidence/OTV2-20260925-item-enrichment-wave1*.json
  - docs/agents/tasks/active/OTV2-20260925-item-enrichment-wave1.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The first real Item enrichment wave. The 165 Items with protected EXACT TibiaWiki
bindings gain typed facts, source taxonomy, Forge profiles, canonical family profiles
and capability relations, each with per-fact provenance. No identity is minted or
rematched, and the runtime source stays legacy.

## Architecture and source of truth

- `docs/architecture/OTERYN_ITEM_AUTHORING_MASTER_SCHEMA_V1.md` and the protected field census `OTV2-20260925-tibiawiki-item-master-field-census-v1.json` (PROVEN mapping).
- Frozen source: the recorded `revision_id` and `source_digest` of each EXACT page (`OTV2-20260925-g4-item-exact-165-selected.json`). CI refetches every revision and verifies the wikitext SHA-256 (PROVEN).
- Weight (`TYPED_WEIGHT_UNIT`), equipment slot patterns (`SLOT_SEMANTICS`) and `stackable=true` (`STACK_MAX`) stay UNKNOWN.

## High-risk authority/recovery qualification

NOT_APPLICABLE: static content authoring only. No session, lease, fence, persistence or production mutation.
