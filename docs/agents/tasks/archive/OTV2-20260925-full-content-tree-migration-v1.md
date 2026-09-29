---
task_id: OTV2-20260925-full-content-tree-migration-v1
title: Full content tree migration v1
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-tree-migration-v1-20260925
base_sha: 9728d30669a85579d333f826ebe7f812c76337ad
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

Candidate materializes 38,157 Item definitions in 77 deterministic shards and 252 Mount declarations in one shard. Legacy `content/world/**` remains untouched and runtime remains `legacy_until_separately_qualified`.

## Material repair

Exact-head readback found that attached source bindings are grouped by definition in the successor tree, while the legacy provenance document uses a different global list order. The first validator therefore produced a false order-sensitive Mount failure.

The repaired validator compares editor/binding collections canonically while requiring every attached binding/editor target to equal the containing definition identity. The migration script is also complete enough to regenerate shards, imports, family indices, manifest, lock and project controls from a clean protected WorldProject/v2 input.

Fresh exact-head validation is required after this repair.

## Current invariant target

- Items: 38,157 / 77 shards
- Item editor entries: 165
- Item source bindings: 165
- Mounts: 252 / 1 shard
- Mount editor entries: 252
- Mount source bindings: 252
- legacy `content/world/**`: byte-preserved / not mutated
- runtime switch: forbidden in this slice

## Current-main refresh

Refreshed onto `main@9728d30669a85579d333f826ebe7f812c76337ad`. The protected declarations file now also contains 133 Outfit records. The migration and validator explicitly filter `kind == "Mount"`; the 252 Mount declaration payloads are unchanged, so no Item/Mount shard data was reminted.
