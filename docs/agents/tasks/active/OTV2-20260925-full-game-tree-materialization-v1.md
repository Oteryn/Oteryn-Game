---
task_id: OTV2-20260925-full-game-tree-materialization-v1
title: Full game tree materialization v1
mode: MIGRATE
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-game-tree-materialization-v1-20260925
base_sha: 1ecdafb91f238bbdee691c08e25208d1dfef36d5
issue: 162
jira: KAN-16
allocation_comment: 5837065048
created_at: 2026-09-25T19:58:00+02:00
owned_paths:
  - docs/agents/tasks/active/OTV2-20260925-full-game-tree-materialization-v1.md
  - docs/agents/evidence/OTV2-20260925-full-game-tree-materialization-v1.json
  - tools/content-schema/validate_materialized_game_tree.py
  - target-tree leaf index.json files under content/**
  - target-tree leaf index.json files under rulesets/**
  - target-tree leaf index.json files under imports/**
---

# Full game tree materialization v1

Materialize every directory node from the protected Full Game Content & Ruleset Tree v1
as a Git-visible authoring surface. Existing populated family indices are preserved.
New empty-family markers explicitly say READY_UNPOPULATED and never claim content exists.

Acceptance:
- every target directory has an index.json;
- existing populated Item/Mount data is not replaced;
- legacy content/world data is not deleted;
- rulesets/** becomes physically visible;
- validator reports zero unmaterialized directories.
