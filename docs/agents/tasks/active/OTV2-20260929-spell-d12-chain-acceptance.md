# OTV2-20260929-spell-d12-chain-acceptance

```yaml
task_id: OTV2-20260929-spell-d12-chain-acceptance
title: D12 Ability.chain extension for independent review (spell rule S23)
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
pr: null
allocation_comment: "#162 5884614275"
base_branch: main
branch: claude/spell-d12-chain-acceptance
base_sha: 8dfd069594e0b6bd4d4cd9df7226b4b8dcd69d8c
head_sha: null
owner: "Oteryn: content" (Claude Code)
created_at: 2026-09-29T00:00:00Z
updated_at: 2026-09-29T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - docs/architecture/OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md
  - docs/agents/tasks/active/OTV2-20260929-spell-d12-chain-acceptance.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

The D12 `Ability.chain` wording in `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` states the S23 extension (owner,
2026-09-28) for independent review through #162:
- `max_targets` counts the further creatures after the first (a Canary chain hits the value plus one; no monster
  data changes);
- the optional `shape` (`sequential` default | `fork`), `initial_range_tiles` (absent = `range_tiles`) and
  `damage_step_percent` (integer, absent = 0);
- the added `target_filter` value `ranged_monsters`.

`OTERYN_SPELL_CHAIN_BEHAVIOUR_CANDIDATE_V1.md` records in its status line that the §5 shape and the D12 wording are
accepted when that review closes with no P1 open, that Q1-Q3 remain for the owner's in-game tests, and that Q4 is
addressed by this PR.

## Excluded scope

The chain runtime, the schema JSON, `chain-behaviours.json` and any monster data (already carried by #1162), Wheel
augments, and the Harmony and support-chain behaviours.

## Validation

- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`,
  `git diff --check`
