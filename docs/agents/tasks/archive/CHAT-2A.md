# CHAT-2A

```yaml
task_id: CHAT-2A
title: "CHAT-2A: sealed World chat relay payload, key ring and replay window (CHAT-0 section 5)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-2-20261013
issue: 2074
governing_issue: 1622
pr: 2076
base_sha: b4774770
head_sha: null
final_head_sha: null
owner: claude-code-session-016UMPeTGXu2gXmPn9iAd9s1
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-13
updated_at: 2026-10-13
owned_paths:
  - apps/game-server/src/chat/relay/**
  - apps/game-server/src/chat/mod.rs
  - apps/game-server/Cargo.toml
  - Cargo.toml
  - Cargo.lock
  - content/world/pins/oteryn.json
  - docs/agents/tasks/archive/CHAT-2A.md
public_contracts: []
external_repositories: []
```

## Outcome

First reviewable batch of sub-issue #2074 (CHAT-2, parent #1622): the pure security core of the
World relay of CHAT-0 section 5. `chat::relay` holds the fixed binary payload (private and room
kinds), AES-256-GCM sealing with a random 96-bit nonce and the channel name plus `key_id` as
associated data, the per-World key ring (send with the newest activated key, open with every held key until 24 hours after its successor's activation, `CHAT0-RL-12`), the 30 s freshness check and the bounded `message_id`
de-duplication window (`CHAT0-RL-13`, a full window refuses rather than forgets), and the
`oteryn_chat_<WorldId hex>` channel name (the canonical UUID without hyphens). AES-GCM comes from `aws-lc-rs`, already in the lock through
rustls; no new crate enters `Cargo.lock`.

## Architecture and source of truth

- PROVEN: `docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md` section 5 (layout, sealing, rotation, replay) and section 8 rows `CHAT0-RL-06`, `-12`, `-13`.
- DERIVED: the channel is `oteryn_chat_<WorldId without hyphens>` (44 bytes) because production WorldIds are canonical UUIDv7 (`node/config.rs`); CHAT-0 section 5 writes `<world_id>` without fixing the encoding.
- DERIVED: a `sent_at` more than 30 s ahead of the node clock is stale too, and each de-duplication entry lives until its own line's `sent_at + 30 s`, the last instant the freshness check accepts it, so no accepted line can return after its entry is gone; `seal` draws its own random nonce and 128-bit `message_id`; the replay window keeps a monotonic high-water clock so a clock stepped back cannot re-admit an evicted id. The replay window size (8,192 lines per World) is an implementation bound, not a CHAT-0 row.
- Plaintext is at most 1,185 bytes (two-byte length prefixes), sealed at most 1,214, base64 at most 1,624, inside the section 5 figures.

## Acceptance criteria

- [x] Round trip of a room and a private line; the widest line stays within the documented bounds.
- [x] Oversized fields are `Oversized`, never truncated.
- [x] Wrong channel, altered ciphertext and a relabelled `key_id` fail authentication.
- [x] Rotation: a key sends once its cluster-wide `activate_at` has passed (the newest activated key); every held key opens, and a key retires 24 hours after its successor's activation.
- [x] Stale, future-dated and replayed lines are refused; a full window refuses.
- [x] `Debug` of a line or key never prints text or key bytes.

## Excluded scope (stays CHAT-2, follow-up batches)

The `LISTEN`/`NOTIFY` connection and `NOTIFY` send, `log_parameter_max_length = 0` on it, listener
readiness and `CHAT_UNAVAILABLE`, the `NOTIFY` queue watch (`CHAT0-RL-09`), delivery of relayed
lines to sessions, private-message and room dispatch, the durable mute row
(`game_character_chat_mutes`, needs a leased migration number) and any wire change. No protocol,
registry, migration or content change here.

## Validation

Re-run at the head before this record fix (code unchanged since `3284110`):

- `cargo fmt --all --check`: exit 0
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings` (`CARGO_BUILD_JOBS=1`): exit 0
- `cargo test --locked -p oteryn-game-server --quiet` (whole package): NOT COMPLETED locally, exit 101. rustc was killed by SIGKILL (signal 9, out of memory) while compiling the lib test target and `apps/game-server/tests/character_authority_postgres.rs`; no test result was produced. Qualification of the whole package = exact-head CI (nieukończony lokalnie, kwalifikacja = CI).
- `cargo test --locked -p oteryn-game-server --lib --quiet` with `CARGO_BUILD_JOBS=1` (partial substitute, lib target only): exit 0, 2933 passed, 0 failed, 29 ignored, including the 17 `chat::relay` tests. The integration test targets (including the Postgres-backed ones, which need PG 17.6) were not run locally.
- `cargo run --locked -p oteryn-architecture-check -- workspace .`: exit 0, `workspace-boundaries: PASS`
- `python tools/agents/validate_governance.py`: exit 0, 22 policy documents and 9 lanes validated
- `python -m unittest discover -s tools/agents/tests`: 60 tests, OK

### Exact-head CI

Recorded in the PR checks of the frozen head.

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open.

## Independent review

- required: full Codex review on the frozen head, requested by the control plane.

## PR and closeout

- `Merge gate / World bundle` failed on `6e447ee`: `Cargo.toml` and `Cargo.lock` are world-bundle inputs, so the pin `inputs_digest` was refreshed (`pin-check` passes locally).
- Refs #2074 (does not close it).
- merge commit/result: squash merge of #2076 (pending)
