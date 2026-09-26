# OTV2-20260926-monster-authoring-batch-2

```yaml
task_id: OTV2-20260926-monster-authoring-batch-2
title: Second Canary monster batch and wiki reference-date comparison
mode: CONTRACT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/nice-edison-h9aqh0
issue: 162
pr: null
jira: KAN-16
base_sha: c72925db52ee8cc19265b63d76e8e3ca863d93fc
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: claude-code-session-01UiMEDawVAZ3kxLCLepWZnG
created_at: 2026-09-26
updated_at: 2026-09-26
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260926-monster-authoring-batch-2.md
  - docs/agents/tasks/archive/OTV2-20260926-monster-authoring-schema-v1.md
  - docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md
  - tools/content-schema/monster-authoring/**
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Owner request after PR #938: (1) a second batch of 10 Canary monsters covering the cases the
first batch did not exercise, (2) a comparison of the first batch against wiki pages at the
2026-07-28 reference boundary, (3) archival of the #938 task record.
Authority: direct owner request in this session; product/runtime implementation stays unallocated.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: offline authoring evidence and Python tooling only; no production mutation,
fence, session, authority or persisted-recovery evidence is touched.

## Acceptance and evidence

- #938 task record archived as completed (merge `c72925d`).
- Batch 2 (`samples/canary-47dfd51f-batch-2/`): 10/10 bundles validate, 7/10 manifests resolve;
  the 3 blocked monsters need registered-spell, familiar-look and Encounter decisions.
- Owner decisions D8 (per-stage bosstiary points) and D9 (boss = monster, mechanics in Encounter)
  recorded; `verify_formal_schema.py` 173/173.
- Creature events are classified only from their read script; batch 1 events re-verified as quest
  counters. Batch 1 still resolves 10/10.
- Wiki / 2026-07-28 comparison: pending.
