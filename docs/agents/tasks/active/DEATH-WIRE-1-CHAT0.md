# DEATH-WIRE-1-CHAT0

```yaml
task_id: DEATH-WIRE-1-CHAT0
title: "DEATH-WIRE-1 (CHAT-0 part): rate gate before the Channel lock for a say"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/death-wire-1-chat0-20261013
pr: null
issue: 1622
governing_issue: 1622
base_sha: 0db5c53b
head_sha: null
final_head_sha: null
created_at: 2026-10-13
updated_at: 2026-10-13
owned_paths:
  - apps/game-server/src/chat/spam.rs
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md
  - docs/agents/tasks/active/DEATH-WIRE-1-CHAT0.md
public_contracts: []
external_repositories: []
```

## Outcome

The 2026-10-12 CHAT-0 security review found two defects. The cast-path mute shipped as
CHAT-MUTE-CAST-1 (#2003). This task closes the other: a `say` took the Channel runtime lock and
cloned the visible entities before the speaker's limiter ran, so a muted or flooding character
cost the Channel a full pass per line. `ChatRuntime::pre_gate` (mute, then an empty bucket, no
line taken) now runs in `chat_speak` before the lock; CHAT-0 §6 states the rule. No wire,
protocol, migration or content change (D1021: implement, then accept, no waiver).

## Acceptance criteria

- [x] A muted or exhausted `say` is refused before the runtime lock; the gate takes no line.
- [x] An unknown session and a non-`say` intent fall through to the full path.
- [x] CHAT-0 §6 carries the rule; acceptance still waits for the independent re-review.

## Excluded scope

Durable mute row, World relay, private messages and rooms (CHAT-2).

## Validation

`cargo test --locked -p oteryn-game-server --lib -- chat` (38 passed), `cargo fmt --all --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`, `validate_governance.py`, `validate_repository_policy.py`, `python3 -m unittest discover -s tools/agents/tests` (60 passed): pass. Exact-head CI in the PR checks.

## Self-review

Whole-diff read against owned paths; no open finding.
