# OTV2-20261014-chat-pregate-unserved-1

```yaml
task_id: OTV2-20261014-chat-pregate-unserved-1
title: "CHAT-0: refuse unserved chat intents at the pre-gate, before the Channel locks"
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-pregate-unavailable-1
issue: 1622
governing_issue: 1622
pr: null
created_at: 2026-10-14
updated_at: 2026-10-14
execution_policy: continuous_progress
depends_on: ["#2103 (CHAT-YELL-PREGATE-1), merged to main"]
blocks: ["#2088 (ACCEPT-CHAT-0)"]
owned_paths:
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/agents/tasks/archive/OTV2-20261014-chat-pregate-unserved-1.md
public_contracts: ["CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1 §6 (CANDIDATE)"]
external_repositories: []
```

## Result

- Finding (Codex security P1 on #2088, comment 4270680712, head `ad3f185`): `ChatRuntime::pre_gate` returned `None` for `Private`, `Room`, `OpenRoom` and `CloseRoom`, so `chat_speak` took the runtime and spell-state locks before `chat_in_channel` answered `CHAT_UNAVAILABLE`. These intents take no line, so an admitted client could repeat them against both locks without limit. Disposition: accepted and repaired here.
- `pre_gate` now refuses every non-`Say` intent `CHAT_UNAVAILABLE` before any lock or map lookup, for known and unknown sessions. It takes no line. The full-path refusal in `chat_in_channel` stays as a second check.
- CHAT-0 §6 "Gate before the Channel lock" records the rule; CHAT-2 gates these intents at the pre-gate when it implements them.
- High-risk authority/recovery qualification: `NOT_APPLICABLE`. In-memory gate only; no fenced write, session, lease or recovery evidence.

## Excluded scope

- No protocol, wire, registry, migration or persistence change. The CHAT-0 status line (#2088) is untouched.

## Validation

- cargo fmt --all --check: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests)
- python tools/agents/validate_governance.py: pass
- cargo clippy / cargo test -p oteryn-game-server chat_intent: run in CI (the local build hit the RAM limit)

## Review

- Independent review: required (security semantics of CHAT-0 §6); requested by the control plane on the final head.
