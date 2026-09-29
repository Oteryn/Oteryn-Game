# OTV2-20260929-item-client-census-in-repo-appearances

```yaml
task_id: OTV2-20260929-item-client-census-in-repo-appearances
title: B3 census reads the in-repo 15.30 appearances and CI checks drift
mode: MIGRATE
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/compassionate-albattani-s29syw
issue: 162
pr: null
base_sha: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: owner-launched Claude Code session (Item authoring lane)
created_at: 2026-09-29T18:10:00Z
updated_at: 2026-09-29T18:10:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/item-authoring/client_appearance_census.py
  - tools/content-schema/item-authoring/test_client_appearance_census.py
  - tools/content-schema/item-authoring/README.md
  - .github/workflows/item-authoring-schema.yml
  - docs/agents/tasks/archive/OTV2-20260929-item-client-census-in-repo-appearances.md
public_contracts: []
depends_on:
  - "#1251-#1253: owner-staged 15.30 client assets in content/assets/files"
blocks: []
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@ff7ede59 and @00ce02a5, opentibiabr/canary@47dfd51f (items.xml, digest-verified)"
jira: KAN-16
```

## Outcome

The owner asked for this in session ("tak").

- `client_appearance_census.py` defaults `--appearances` to
  `content/assets/files/appearances-<pinned sha>.dat`. The pinned size and sha256 guard is unchanged, and a new test
  pins the default path.
- The CI lane `Item Authoring Schema` fetches the three pinned `items.xml` files at their revisions. It then runs
  `--check` against the committed B3 sample. The census verifies every input digest, so it fails closed.
- The census output is unchanged.

## Validation

- `test_client_appearance_census.py`: 29 checks PASS.
- `--check` with the `items.xml` files fetched as CI does: `ok`.
- ruff 0.16.1: PASS.
- Governance and repository-policy validators: PASS.
