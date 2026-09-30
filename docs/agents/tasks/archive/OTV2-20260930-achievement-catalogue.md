# OTV2-20260930-achievement-catalogue

```yaml
task_id: OTV2-20260930-achievement-catalogue
title: ACHIEVEMENT - populate content/achievements/ (contract step 2)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/achievement-catalogue
issue: 162
lane_id: ACHIEVEMENT
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 970e30a
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "achievement catalogue worker (claude-code-session-01L687XUJAozgAPkNZ7B8GMG)"
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - content/achievements/**
  - tools/content-schema/achievement-authoring/**
  - .github/workflows/achievement-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260930-achievement-catalogue.md
public_contracts: []
depends_on: [OTV2-20260929-achievement-owner-contract]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Owner direction (2026-09-30, this session): continue with contract §5 steps 2 and 3; question 29 answered `a`
(the achievement-authoring workflow is authorized). This change is step 2: `content/achievements/` holds 571
catalogue records in two shards, built by `build_catalogue.py` from the committed staticdata and TibiaWiki
observations and `owner_resolutions.json` (the 2026-09-29 resolutions already recorded in contract §2.2). The
directory marker is `POPULATED`. No runtime, migration or protocol change; step 3 (persistence) is a separate
change.

## Evidence

- 571 records: 368 with client staticdata provenance, 203 secret wiki-only, 1 retired (The More the Merrier);
  Achievement 563 excluded. Grades 432/121/17/1.
- Every record validates; keys are unique and allocated once (`allocate_key`); a rebuild keeps committed keys.
- Spot checks: Allow Cookies? (1, 2 points, premium), Taskaholic (3, 7 points), Hell Rider (2 points),
  Something Smells (client text "extinguished", not the wiki's typo).

## Validation (local)

- `build_catalogue.py --check`: ok (571 records, 2 files); `validate_achievements.py` on both shards: 571/571.
- `test_validate_achievements.py`: 8 tests pass; ruff 0.16.1 and 0.15 check and format pass.
- `validate_materialized_game_tree.py`, `validate_governance.py`, `validate_repository_policy.py`,
  `git diff --check`: pass.
- Review: content population under an accepted contract; the workflow change extends the owner-authorized
  (29a) workflow's path filter and adds the `--check` step.
