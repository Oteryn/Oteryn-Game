# QUEST-PRED-1

```yaml
task_id: QUEST-PRED-1
title: QUEST-PRED-1 - read-only quest predicate API over the session copy
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/quest-pred-1
issue: 1622
lane_id: quest
pr: 1723
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

## Validation

- cargo fmt --all --check: pass
- cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings: pass
- cargo test --locked -p oteryn-game-server --lib quest: pass (32 tests)
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass
- Review: Codex round 1 P1s (own completion outside the account gates; undeclared quests fail closed) fixed; the new head is routed by the control plane.
