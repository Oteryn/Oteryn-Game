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
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: null   # stacked on claude/happy-cannon-4gzw0y (PR #1293)
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "charm content worker (claude-code-session-012nzPTz29NThWJG45F2m5fP)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - content/charms/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - tools/content-schema/charm-authoring/**
  - .github/workflows/charm-authoring-schema.yml
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
managed files. `charm_authoring.py content [--check]` writes and verifies all of it; CI runs the check.
No runtime loading was added; `runtime_source` is unchanged.

## Deviation and follow-up (not in owned paths)

`tools/content-migration` (generator, validator, test) asserts `content.lock.json` `family_counts` verbatim,
so the Charm count is `static_family_counts.Charm`, not `family_counts.Charm`. That generator also rewrites
project, manifest and lock and would drop the Charm registration if re-run: teach it Charm, then move the
count. Both are in `tools/content-migration/**`, owned by another lane.

## Validation (local)

- `charm_authoring.py build --check`, `validate`, `content --check`: ok. `test_charm_authoring.py`: 7 pass.
- `ruff check` / `ruff format --check` (0.16.1): pass.
- `test_world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`, `validate_materialized_game_tree.py`
  (97/97), `test_classify_content_routing.py`: pass.
- Review: none required (static catalogue, no runtime or authority change); review packet in the worker report.
