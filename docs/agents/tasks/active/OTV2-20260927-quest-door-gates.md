# OTV2-20260927-quest-door-gates

```yaml
task_id: OTV2-20260927-quest-door-gates
title: Quest format slice 2 - Canary + CrystalServer door gates and wiki decisions for the chest conflicts
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 984
base_sha: ec0e12a7927dcd4d98f7d1151f6b8ee100c1b65c
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk
created_at: 2026-09-27
updated_at: 2026-09-27
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260927-quest-door-gates.md
  - docs/architecture/OTERYN_QUEST_AUTHORING_FORMAT_V1.md
  - tools/content-schema/quest-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Second slice of the CANDIDATE quest format (D32-D33, PR 976): quest, key and level doors of both
servers become gates. A quest gate reads quest progress, and names the reward claim when a chest
records the same marker. A level gate reads the character level. A key gate is a lock shared by
everyone and lists the chests that hand out its key. The reference-date wiki decides the Thieves
Guild goblet conflict (D25) and confirms the Secret Library chest cooldown. Runtime, persistence
and `content/**` stay unchanged.

## Architecture and source of truth

- `PROVEN`: the pinned Canary and CrystalServer revisions of slice 1; door tables and the three
  door scripts (identical in both servers), blob ids in `samples/doors/manifest.json`.
- `DERIVED`: wiki decisions at Fandom revisions 1086330 (Thieves Guild spoiler), 1115203 (Stolen
  Golden Goblet) and 1200697 (Brass-Shod Chest).
- `CONFLICT`: ten door storage names (Kilmaresh, Order of the Lion, King Zelos) and one corpse chest
  appearance.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- `verify_quest_schema.py` 39/39 with the cause of each negative case checked.
- `ots_chests.py` and `ots_doors.py` are deterministic; the chest and door samples validate
  together.
- `validate_governance.py` and `validate_repository_policy.py` pass.
- Jira: mapping to a programme Story not resolved in this session (pending).
