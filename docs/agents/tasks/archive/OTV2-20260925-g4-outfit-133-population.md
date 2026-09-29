# OTV2-20260925-g4-outfit-133-population

```yaml
task_id: OTV2-20260925-g4-outfit-133-population
title: Populate 133 exact TibiaWiki Outfit identities
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-outfit-g4-133-population-20260925
issue: 162
issue_comment: 5834920273
jira_story: KAN-16
pr: null
base_sha: 2389c6671000b8b0efe341540a62e303e307ad15
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: work-coordinator
created_at: 2026-09-25T15:04:00Z
updated_at: 2026-09-25T15:04:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/examples/materialize_content_world_project_v2.rs
  - apps/game-server/tests/content_world_project_repository.rs
  - docs/agents/evidence/OTV2-20260925-g4-outfit-133-selected.json
  - docs/agents/tasks/active/OTV2-20260925-g4-outfit-133-population.md
  - content/world/definitions/declarations.json
  - content/world/editor/author.json
  - content/world/provenance/sources.json
  - content/world/manifest.json
  - content/world/content.lock.json
  - content/world/project.json
public_contracts: []
```

Allocation: #162 comment 5834920273, timestamp correction 5834936971.

Bounded product outcome:
- source family: Outfit;
- protected exact page-ID cohort: 134;
- CREATE at target cut: 133;
- BLOCKED_POST_CUT: page ID 68724, `Captain's Outfits`, revision 441933 at `2026-08-04T13:38:39Z`;
- source bindings written target: 133;
- editor entries written target: 133;
- typed Outfit semantic fields promoted: 0;
- relationships/presentations/assets/placements/runtime activation: 0.

Reuse WorldProject/v2 and the existing canonical materializer. No new crawl, crosswalk, schema, importer or framework. Preserve `definitions/reference.json`, `worlds/world.json`, `presentations/bindings.json` and `assets/catalog.json` byte-identically.

Qualification requires deterministic source packet checks, 133 unique canonical keys/page IDs, preserved Item 165 + Mount 252 population, two-pass generator equality, tracked package comparison, focused tests, fmt, strict Clippy, governance, exact-head full `game-gate`, then coordinator-owned governed Merge Queue and protected-main readback.
