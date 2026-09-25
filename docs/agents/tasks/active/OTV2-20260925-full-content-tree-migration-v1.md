---
task_id: OTV2-20260925-full-content-tree-migration-v1
title: Full content tree migration v1
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-tree-migration-v1-20260925
base_sha: 2389c6671000b8b0efe341540a62e303e307ad15
issue: 162
pr: 905
jira: KAN-16
allocation_comment: 5835516048
created_at: 2026-09-25T18:00:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260925-full-content-tree-migration-v1.md
  - docs/agents/evidence/OTV2-20260925-full-content-tree-migration-v1.json
  - tools/content-migration/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - content/items/**
  - content/cosmetics/mounts/**
  - imports/crystalserver/**
  - imports/tibiawiki/**
---

# Full content tree migration v1

Candidate contains 38,157 Item definitions in 77 deterministic shards and 252 Mount declarations in one shard. Exact legacy definition/declaration objects plus Item/Mount editor/source bindings are preserved. Legacy content/world remains unchanged and runtime remains legacy_until_separately_qualified.

Validation must prove exact Item/Mount semantic round-trip, editor/source-binding equality, identity uniqueness/cardinality and import-binding equality.
