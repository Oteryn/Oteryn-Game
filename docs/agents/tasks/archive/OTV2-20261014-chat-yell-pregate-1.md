# OTV2-20261014-chat-yell-pregate-1

```yaml
task_id: OTV2-20261014-chat-yell-pregate-1
title: "CHAT-0: an under-level yell takes its line and a yell flood reaches the offence before the Channel"
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-yell-pregate-1
issue: 1622
governing_issue: 1622
pr: 2103
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

## Result

- Finding (Codex security P1 on #2088, comment 4267303100, head `bdad852`): `ChatLimiter::pre_gate` exempted a yell from the empty-bucket offence, and an under-level yell took no line and started no mute, so it could force the Channel path (runtime and spell-state locks, `visible_entities()` clone) indefinitely. #2097 bounded it to one full-path attempt per 2.5 s without charging anything. Disposition: accepted and repaired here.
- `pre_gate` order: mute, yell cooldown (no line), empty bucket (offence for every mode), yell level recheck (takes its line, `LEVEL_TOO_LOW`).
- `admit_local` order: mute, yell cooldown, bucket, yell level gate. The line is taken before the level is checked.
- Effect: 16 under-level yells at one instant enter the Channel path once, then three gate refusals drain the bucket and the fifth yell starts the offence at the gate. A yell every 50 ms for 10 minutes enters the Channel path at most 8 times (7 by the mute schedule).
- CHAT-0 §6 "Gate before the Channel lock" text updated to match ("a refused yell never starts a mute" is replaced).
- High-risk authority/recovery qualification: `NOT_APPLICABLE`. In-memory per-session chat limiter; no fenced write, session, lease or recovery evidence.

## Excluded scope

- The yell cooldown refusal still takes no line. No protocol, wire, registry, migration or persistence change. The CHAT-0 status line (#2088) is untouched.

## Validation

- cargo fmt --all --check: pass
- git diff --check: pass
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60)
- ChatLimiter tests of chat/tests.rs (14), compiled against spam.rs and oteryn-simulation-determinism in an isolated crate: pass (14 passed, 0 failed); clippy -D clippy::all: pass
- cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings: not run locally (rustc OOM-killed, about 3 GB free memory); CI
- cargo test --locked -p oteryn-game-server: not run locally (same reason); CI

## Review

- Independent review: required (security semantics of CHAT-0 §6); requested by the control plane on the final head.
