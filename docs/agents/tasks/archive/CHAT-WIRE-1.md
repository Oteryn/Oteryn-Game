# CHAT-WIRE-1

```yaml
task_id: CHAT-WIRE-1
title: "CHAT-WIRE-1: dispatch CHAT_INTENT for local say, whisper and yell and offer CHAT_V1"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-wire-1-20261008
pr: 1953
base_sha: null
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
merge_commit: "squash merge of #1953"
owner: claude-code-session-01J2hk7k6xarVoxJ9aASiLfA
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-08
updated_at: 2026-10-10
owned_paths:
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/capabilities.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - apps/game-server/src/chat/**
  - apps/game-server/src/durability/character_chat_name.rs
  - apps/game-server/src/durability/mod.rs
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
  - crates/protocol-oteryn/src/chat_tests.rs
  - docs/agents/tasks/archive/CHAT-WIRE-1.md
public_contracts:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json
external_repositories: []
```

## Outcome

A client that selects capability 7 `CHAT_V1` can send command 13 `CHAT_INTENT`. A local say, whisper
or yell passes the existing limiter, yell gate and egress rules and reaches every session of the
Channel that hears it as a domain 12 line delta. Private messages, rooms and the World relay answer
`CHAT_UNAVAILABLE` (CHAT-2); the open-room set is empty.

## Architecture and source of truth

- PROVEN: CHAT-0 decision `docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md`; plan #1622 §2.1 row N1, decisions T2, T3, T4.
- DERIVED: the speaker name is read once per session from the durable Character root under the existing recovery fence; the domain 12 revision is cumulative per GameSession in `SessionContinuity`.

## Acceptance criteria

- [x] Command 13 dispatches through a thin arm to `chat_intent.rs` (connection-level test, two sessions).
- [x] A say is heard in range and not out of range (`a_say_reaches_the_session_in_range_and_not_the_one_out_of_range`).
- [x] `CHAT_V1` is offered; capability tests and the protocol registry test updated.

## Excluded scope

No dispatch framework, spell words, NPC talk, map, client or session crates; no protocol crate change except one registry assertion (CP decision D607 option A).

## Implementation / findings

`ChatRuntime` holds per-session name, limiter and egress queue; a 250 ms tick drains it into line
deltas. A malformed intent is unregistered; `Rejected` maps to `CommandStatus::Rejected`. A session
without a readable name is not admitted to chat. Lock order: runtime, spell states, chat mutex.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server`: pass (2647 passed, 28 ignored)
- `cargo test --locked -p oteryn-protocol-oteryn`: pass (213 passed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

### Exact-head CI

Recorded in the PR checks of the frozen head.

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: squash merge of #1953
