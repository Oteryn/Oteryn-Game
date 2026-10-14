# OTV2-20261014-chat-yell-pregate-1

```yaml
task_id: OTV2-20261014-chat-yell-pregate-1
title: "CHAT-0: an under-level yell takes its line and a yell flood reaches the offence before the Channel"
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-yell-pregate-1
issue: 1622
governing_issue: 1622
pr: null
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
depends_on: ["#2097 (pre-gate recheck window for under-level yells), merged to main"]
blocks: ["#2088 (ACCEPT-CHAT-0)"]
owned_paths:
  - apps/game-server/src/chat/spam.rs
  - apps/game-server/src/chat/tests.rs
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/agents/tasks/active/OTV2-20261014-chat-yell-pregate-1.md
  - docs/agents/tasks/archive/OTV2-20261014-chat-yell-pregate-1.md
public_contracts: ["CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1 §6 (CANDIDATE)"]
external_repositories: []
```

## Outcome

Repeated under-level yells cost bucket lines and reach the offence and mute at the pre-gate, before the Channel runtime and spell-state locks and the `visible_entities()` clone (Codex security P1 on #2088, comment 4267303100).

## Next action

Run the local checks, push, open the PR, archive this record with its PR number.
