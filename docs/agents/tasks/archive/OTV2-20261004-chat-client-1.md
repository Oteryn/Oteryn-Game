# OTV2-20261004-chat-client-1

```yaml
task_id: OTV2-20261004-chat-client-1
title: "CHAT-CLIENT-1: capability 7 chat in the client session, the dev client and the live harness"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-client-1-20261004
issue: 1622
pr: 1762
head_sha: "exact frozen head in the FREEZE entry to the control plane"
final_head_sha: "exact frozen head in the FREEZE entry to the control plane"
owner: claude-code worker for control plane session_013KJX6mv8LQveCKKXYgAX94
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - crates/session/src/**
  - tools/dev-client/src/**
  - tools/synthetic-client-harness/src/live/**
  - docs/agents/tasks/archive/OTV2-20261004-chat-client-1.md
public_contracts: []
depends_on: [ENTITY-CLIENT-1]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_CORE_CLIENT_PACKETS_2026-10-04.md` §2.3, §1.4-§1.6.

- `CLIENT_SUPPORTED_CAPABILITIES` is `[6, 7, 13]`. Capability 7 requires nothing, so the set stays closed.
- `Session::chat(&ChatIntent)` sends command 13 and returns `ChatOutcome` (status, disposition,
  `wait_seconds`, result sequence) at the result. A codec error (empty or over 1,020 bytes of text)
  is refused before anything is sent; capability 7 unselected is `CapabilityNotSelected`; an
  accepted status with a `Rejected` disposition (or the reverse) poisons the session.
- Domain 12: snapshot type 1 sets the open rooms (absent means none); delta type 1 is one line and
  type 2 replaces the room set, through the fail-closed store (revision, registered type). Lines
  are kept in a ring of the last 64 (`ChatLog`). Events `ChatLine` and `ChatRooms`. Without
  capability 7 a domain 12 snapshot or delta is refused. Chat no longer uses the raw gated table.
- `dev-client` re-exports the chat types and adds `chat()` and `chat_log()`.
- Harness: input lines `say`, `yell`, `whisper`, `pm NAME TEXT`, `room N TEXT`, `open N`, `close N`;
  a chat pane rendering `Name says:`, `whispers:`, `yells:`, `(private):`, `[Room] Name:` and a
  `DROPPED` marker; `MUTED` and `EXHAUSTED` show their wait and change nothing else.
- Carried P2 4177563323 (#1760): cells outside the i32 world no longer alias `i32::MAX`.
- Tests: every intent variant at the text bound reaches the wire; over-bound and empty refused
  unsent; pushed lines of every kind apply while idle; the ring keeps 64; `DROPPED`; `MUTED` and
  `EXHAUSTED` change no state; rooms delta; revision, delta type and codec poisoning; snapshot
  type, unselected domain; harness parsing, rendering and viewport edge.
- No protocol, registry, proto, server or `apps/client` file touched. The server does not offer
  capability 7 yet (CHAT-1b-2b), so the Server Seam is not applicable and live-play evidence
  follows CHAT-1b-2b.

## Validation

- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-session -p oteryn-dev-client -p oteryn-synthetic-client-harness --quiet`: pass
- `cargo check --locked -p oteryn-game-server --tests`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
