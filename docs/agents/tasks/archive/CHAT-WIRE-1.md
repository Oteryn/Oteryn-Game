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

## 2026-10-27 — CHAT-MUTE-1 verification

The current server already contains the requested end-to-end staff-mute slice under the existing
staff and chat owners, so this pass did not allocate migration 0292, capability 0145 or command
0167. Migration 0170 stores signed, duration-bound character sanctions; the composed dispatch
owner reads that durable truth through the current session/item fence on every chat and spell
attempt. The shared chat entry covers local speech, private messages, rooms and private-channel
commands, while the guild-room entry applies the same gate. Storage, fence or trusted-clock
uncertainty fails closed. Expired or explicitly lifted sanctions stop gating without deleting the
immutable sanction evidence. The existing PostgreSQL dispatch qualification executes a signed
mute through command 60, sends command 13 and command 3 through real connection dispatch, rebuilds
the owner against the same database, verifies enforcement on a node without operator trust, then
executes and verifies an unmute.

### Parity

| behaviour | Canary/Crystal ref | Oteryn | status |
|---|---|---|---|
| A muted player cannot speak; spam mutes and staff mutes use the same muted refusal semantics | Canary pin `47dfd51f45280a59a1d3e50ba7edd573d7234446`; CHAT-0 manual-derived rule. Crystal pin `201189f871d23597b82caf88fd611454a2ddefb8`: Not verified. | All `CHAT_INTENT` speak paths return `MUTED {seconds}` before delivery; guild chat uses the same durable gate. | Complete |
| A muted player cannot cast an instant spell | Canary pin `47dfd51f45280a59a1d3e50ba7edd573d7234446`; CHAT-0 manual-derived rule. Crystal: Not verified. | Actor-spell dispatch checks the durable staff sanction before all cast dispatchers and returns the existing rejected disposition. | Complete |
| Mute survives relog, process restart and channel transfer | Canary/Crystal persistence implementation: Not verified. | Sanction truth is character-scoped PostgreSQL state, read with the current session-generation/item fence on every attempt and independently of `ChannelId`. | Complete |
| Mute ends at its duration or explicit staff unmute | Canary/Crystal exact staff-duration policy: Not verified. | Trusted time compares the persisted end; expiry opens the gate and a signed unmute lifts the sanction atomically. | Complete |

### Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy -p oteryn-gameplay-entities -p oteryn-gameplay-dispatch --all-targets --all-features -- -D warnings`: pass.
- `cargo test -p oteryn-gameplay-entities --all-features`: pass.
- `cargo test -p oteryn-gameplay-dispatch --all-features staff`: pass (the PostgreSQL case returns
  `Ok(())` when `OTERYN_TEST_POSTGRES_ADMIN_URL` is absent, via the existing mounted-case guard).

### Remaining

- Richer native-client channel UI parity remains outside this server-only slice.
