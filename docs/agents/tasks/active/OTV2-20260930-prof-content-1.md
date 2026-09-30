# OTV2-20260930-prof-content-1

```yaml
task_id: OTV2-20260930-prof-content-1
title: PROF-CONTENT-1 Proficiency definitions in content/ and Item profile_binding
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/prof-content-1-proficiency-definitions
issue: 162
pr: null
base_sha: null   # stacked on #1327 (claude/inspiring-lamport-nc623x) until it merges
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "owner-launched Claude Code session (session_012FuykcnY5errmsP3T1E4Nm)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - content/proficiencies/**
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - tools/content-schema/proficiency-authoring/**   # except .github/workflows (unchanged here)
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-migration/validate_world_project_v2_to_tree.py
  - tools/content-migration/test_world_project_v2_to_tree.py
  - docs/agents/tasks/active/OTV2-20260930-prof-content-1.md
  # 1b adds: apps/game-server/src/content/project/v2.rs, the DefinitionFamily in
  # apps/game-server/src/content/reference_playable.rs, apps/game-server/tests/content_world_project_v2.rs,
  # tools/content-schema/item-authoring/** (PROFICIENCY-0 brief)
public_contracts: []
depends_on:
  - "#1327 (proficiency authoring schema)"
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md
blocks:
  - PROF-2
cross_repository_coordination_id: null
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome (target)

- **1a (this branch):** `content/proficiencies/` holds the 443 definitions from the #1327 candidate
  (index + 3 shards). Keys `oteryn:proficiency.tibia.p<id>` are minted here. `Proficiency` is registered
  in project, manifest and lock, and `world_project_v2_to_tree.py` knows it, so its output stays
  byte-identical. No runtime loading.
- **1b (next PR):**
  - Items carry `proficiency.profile_binding` only. The inline `levels`/`shaping` are removed from
    `item.schema.json` and `ProjectV2WeaponProficiencyProfile`, and the Rust definition family is
    added.
  - The threshold class comes from the rule (owner decision 2 in the #1327 record).
  - The crosswalk-rule amendment and the coverage report.

## Decisions carried

Owner, 2026-09-30 (recorded in `docs/agents/tasks/archive/OTV2-20260930-proficiency-authoring-schema.md`):
- threshold tables per class at catalogue level;
- the threshold class comes from a rule applied when content is built and checked in CI;
- the TibiaWiki point table (revid 1192598) is admitted;
- #1327 ships separately.

Threshold tables and the point table are progression rules, so they are not written to `content/`.

## Validation (1a, local)

- `proficiency_authoring.py build --check`, `validate`, `content --check`: ok. Tests: 6 pass. Ruff: clean.
- `world_project_v2_to_tree.py` regenerates project, manifest and lock byte-identically.
- `validate_world_project_v2_to_tree.py`: PASS (`proficiency_records=443`).
- `test_world_project_v2_to_tree.py`: PASS (160 managed files).
- `validate_materialized_game_tree.py`: 97/97.
- `test_classify_content_routing.py`: PASS.
