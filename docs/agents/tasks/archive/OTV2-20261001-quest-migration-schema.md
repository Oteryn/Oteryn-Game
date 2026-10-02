# OTV2-20261001-quest-migration-schema

```yaml
task_id: OTV2-20261001-quest-migration-schema
title: Type quest migration packets and retain registered Quest data
mode: WORK
status: completed
repository: Oteryn/Oteryn-Game
base_branch: codex/quest-catalogue-progress-20261001
branch: codex/quest-migration-schema-20261001
pr: null
base_sha: bbbe81d51197a9a70e0478af7ee2f53e09ecb142
head_sha: null
owner: codex-quest-data-parent
created_at: 2026-10-01T13:26:00Z
updated_at: 2026-10-01T13:26:00Z
execution_policy: continuous_progress
owned_paths:
  - tools/content-schema/quest-authoring/
  - tools/content-migration/
  - docs/agents/tasks/archive/OTV2-20261001-quest-migration-schema.md
public_contracts: []
depends_on: [OTV2-20261001-quest-progress-catalogue]
blocks: []
external_repositories: []
```

Scope: #162 comment5931946946, one parent writer. JSON schemas describe the
accepted first reward_only data batch and the complete source-catalogue packet;
existing source Quest vocabulary resolves offline, without copying or extending it.
Readiness fields explicitly retain their source/canonical/runtime boundaries.

Tree regeneration now preserves registered Quest index, shards and counts, rejects
inconsistent registration, and retains baseline behavior before Quest population.
Five focused preservation tests and migration regression/validator checks PASS;
both schemas pass Draft202012 checks. The existing full-tree
SOURCE_ID_BOUNDARY_MISSING failure is not bypassed or changed.
GitHub records the frozen head/PR and CI. Coordinator retains review routing/MQ.
