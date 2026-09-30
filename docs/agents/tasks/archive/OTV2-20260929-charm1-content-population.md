# OTV2-20260929-charm1-content-population

```yaml
task_id: OTV2-20260929-charm1-content-population
title: CHARM-1 - populate content/charms/ with the 25 Charm definitions
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/charm1-content-population
issue: 162
lane_id: content population (Charm)
pr: 1296
base_sha: 54c9ca18   # main after #1293 merged; merged into this branch
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "charm content worker (claude-code-session-012nzPTz29NThWJG45F2m5fP)"
created_at: 2026-09-29
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - content/charms/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - tools/content-schema/charm-authoring/**
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - docs/agents/tasks/archive/OTV2-20260929-charm1-content-population.md
public_contracts: []
depends_on:
  - "PR #1293 (tools/content-schema/charm-authoring/)"
  - "PR #1295 (CHARM-0 decision packet, slice CHARM-1, owner answer 5a)"
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

`content/charms/` holds the 25 Charm definitions (14 major, 11 minor) from `samples/charms-candidate.json`:
`index.json` (`OTERYN_FAMILY_INDEX/v1`, replacing the READY_UNPOPULATED marker as Loot and Mount did) and one
shard. Keys `oteryn:charm.<name>` are minted here; values follow TibiaWiki (owner answer 5a). `Charm` moves
from `next_population_families` to `migrated_families` and is registered in the manifest families and
managed files. `charm_authoring.py content [--check]` writes and verifies all of it. CI runs the check through
`test_content_tree_is_current_and_registered` in the existing `charm-authoring-schema.yml` Tests step; the
workflow file is unchanged from main (D185).
No runtime loading was added; `runtime_source` is unchanged.

## Content-migration alignment (owner decision 1b)

The Charm count is in `content.lock.json` `family_counts`. `world_project_v2_to_tree.py` registers the committed
`content/charms/` family (no legacy source) when it regenerates project, manifest and lock; its validator and test
assert the Charm count. Re-running the generator leaves the tree byte-identical (`git status` clean).
Creature scripts are untouched.

## Validation (local)

- `charm_authoring.py build --check`, `validate`, `content --check`: ok. `test_charm_authoring.py`: 7 pass.
- `ruff check` / `ruff format --check` (0.16.1): pass.
- `test_world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`, `validate_materialized_game_tree.py`
  (97/97), `test_classify_content_routing.py`: pass.
- Review: independent Content review of `6e107326` (routed by the control plane, #1296 comment 5907112730) found
  the content correct and asked for FIX:
  - P1: revert `.github/workflows/charm-authoring-schema.yml` to main (D185).
  - P2: update this record.
  - P3: remove the dead `static_family_counts` pop.
  - P3: update the PR body.
  All four are addressed in the repair head. It needs a new FREEZE_SHA and re-review.

## Merge result

- #1293 was squash-merged to main, so this branch merged main twice.
- The branch keeps its Charm authoring additions on top of #1293's files.
- `content/manifest.json` and `content/content.lock.json` were regenerated with `world_project_v2_to_tree.py`
  after the creature and encounter additions of #1300.
- Base: main `54c9ca18`.
