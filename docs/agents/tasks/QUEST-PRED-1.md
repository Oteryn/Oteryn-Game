# QUEST-PRED-1

```yaml
task_id: QUEST-PRED-1
title: QUEST-PRED-1 - read-only quest predicate API over the session copy
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/quest-pred-1
issue: 1622
lane_id: quest
pr: null
base_sha: 53a60a6
owner: "QUEST-PRED-1 worker (session_01KrMdZv73Kcud7obLddGJjy)"
created_at: 2026-10-04
updated_at: 2026-10-04
owned_paths:
  - apps/game-server/src/quest/mod.rs
  - apps/game-server/src/quest/predicate.rs
  - docs/agents/tasks/QUEST-PRED-1.md
  - docs/agents/tasks/archive/QUEST-PRED-1.md
public_contracts: []
depends_on: [QUEST-STATE-1]
jira: null   # sync pending (coordinator batch)
```

## Scope

QUEST-STATE-0 §7 (owner decision D477): `QuestPredicate` over the session copy, read-only and
advisory. No persistence, wire, migration or content change.
