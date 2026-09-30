# OTV2-20260930-content-registry-merge-friendly

```yaml
task_id: OTV2-20260930-content-registry-merge-friendly
title: Write content/project.json, manifest.json and content.lock.json one key per line
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/brave-cori-ou1zm1
issue: null
pr: null
allocation: "owner request in session 01KNTDGyVHkgWjcpbgSFx68w after the #1336 / #1364 lock conflict"
base_sha: 88fb9b01
owner: "Oteryn: standalone task owner (claude-code-session-01KNTDGyVHkgWjcpbgSFx68w)"
created_at: 2026-09-30
updated_at: 2026-09-30
owned_paths:
  - tools/content-migration/world_project_v2_to_tree.py  # shared: registry file format only
  - tools/content-schema/charm-authoring/charm_authoring.py  # shared: registry file format only
  - tools/content-schema/proficiency-authoring/proficiency_authoring.py  # shared: registry file format only
  - tools/content-schema/reward-claim-authoring/reward_claim_authoring.py  # shared: registry file format only
  - content/project.json
  - content/manifest.json
  - content/content.lock.json
  - docs/agents/tasks/archive/OTV2-20260930-content-registry-merge-friendly.md
public_contracts: []
```

## Outcome

The three content tree registry files were single-line JSON. Every PR that registers a family
or regenerates `content/world` rewrites that one line, so any two such PRs conflicted even when
they changed different keys. #1336 (ITEM-SEM-2b-1) conflicted with #1364 (CHEST-CONTENT 2b) this
way: one changed `legacy_blobs.reference`, the other added `family_counts.RewardClaim`.

All four writers (the tree generator and the Charm, Proficiency and RewardClaim tools) now write
these three files with `sort_keys` and `indent=2`. Keys are unchanged; only whitespace differs.
Replaying the #1336 / #1364 lock collision in the new format merges with no conflict and gives
the same bytes as the generator.

Other generated files keep their compact format. Two changes to the same key, or to adjacent
lines, still conflict; that is correct.

## Validation

- `world_project_v2_to_tree.py`, `validate_world_project_v2_to_tree.py`,
  `test_world_project_v2_to_tree.py`: PASS.
- `content --check` for Charm, Proficiency and RewardClaim: ok; their tests pass.
- `validate_materialized_game_tree.py` and `test_classify_content_routing.py`: PASS.
- ruff check and format: clean.
