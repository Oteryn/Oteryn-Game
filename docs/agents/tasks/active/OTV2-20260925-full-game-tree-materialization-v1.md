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
pr: 911
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
  - apps/game-server/tests/content_world_project_repository.rs
---

# Full game tree materialization v1

Materialize every directory node from the protected Full Game Content & Ruleset Tree v1
as a Git-visible authoring surface. Existing populated family indices are preserved.
New empty-family markers explicitly say READY_UNPOPULATED and never claim content exists.

Acceptance:
- every non-world target directory is materialized now unless already populated;
- the 10 `content/world/**` successor leaves remain explicitly deferred while the legacy canonical package owns that root;
- existing populated Item/Mount data is not replaced;
- legacy content/world data is not deleted or semantically relaxed;
- rulesets/** becomes physically visible;
- the repository WorldProject capture still passes with bounded scan limits after the new sibling tree is present.

## Candidate readback

- 87/97 target directory nodes are physically materialized now.
- 86 new directory indices were added; 1 populated index was preserved.
- 10 successor world directories are intentionally deferred because legacy `content/world/**` is still the exact canonical WorldProject package root.
- existing Item/Mount and legacy WorldProject files remain outside deletion scope.
- canonical PR: #911.

The successor commit containing this PR binding is the exact candidate for validation.

## Legacy-root repair

The first candidate correctly exposed a real compatibility conflict: G4 canonical
WorldProject seed performs a recursive exact comparison of `content/world`, so adding
successor-only directory markers beneath that legacy root breaks package reproducibility.

Minimal repair:
- remove only the 10 successor world marker files from this slice;
- preserve their target paths in the protected tree contract;
- materialize all non-world Content/Ruleset/Import branches now;
- defer those 10 world paths until the separately qualified legacy-root transition.

This is an explicit deferred compatibility boundary, not a claim that the world tree is populated.

## CI repair generation

Exact head `d922261cb1092b24c51a9bf3dfb2d9b9126d7fe7` failed `Merge gate / Rust Linux workspace` because the repository-only WorldProject fixture reached `129 > 128` total scanned directory entries after adding legitimate sibling directories under `content/`. Issue #162 comment 5837429915 returns this task to AUTHORING and extends ownership only to `apps/game-server/tests/content_world_project_repository.rs`. The repair must not change the filesystem implementation, runtime semantics, per-scan limit, project evidence limits, package digests, or legacy `content/world/**` bytes.
