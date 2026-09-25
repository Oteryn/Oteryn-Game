---
task_id: OTV2-20260925-full-content-tree-migration-v1
title: Full content tree migration v1
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-tree-migration-v1-20260925
base_sha: 2389c6671000b8b0efe341540a62e303e307ad15
issue: 162
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

## Outcome

Materialize the successor human-readable physical tree without deleting or changing
`content/world/**`.

This slice migrates the currently populated families only:

- all 38,157 Item definitions;
- all 252 Mount declarations;
- 165 Item editor entries and exact TibiaWiki source bindings;
- 252 Mount editor entries and exact TibiaWiki source bindings;
- the exact CrystalServer/TibiaWiki import batch/source metadata needed to reproduce
  those two families.

## Compatibility strategy

The old WorldProject/v2 package remains the runtime/source-of-compatibility input.
The new tree is generated deterministically from it.

The new tree stores exact legacy definition/declaration payloads inside family shards
plus attached authoring metadata. The validator reconstructs family identity/order and
compares the resulting semantic payloads/cardinalities against the protected legacy
package. No identity is reminted and no UNKNOWN field is guessed.

## Exclusions

- no `content/world/**` deletion or mutation;
- no server runtime/compiler/protocol/persistence change;
- no world-placement migration;
- no Store/Platform write;
- no new gameplay semantics;
- no source/page/client numeric identifier promoted to canonical gameplay identity.

## Publication

The generated delta spans many deterministic shard files. After a fresh predecessor
readback, the complete generated candidate must be one API-native Git Data successor
commit on this single-writer branch, followed by exact-head qualification.
