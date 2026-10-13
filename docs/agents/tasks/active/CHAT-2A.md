# CHAT-2A

```yaml
task_id: CHAT-2A
title: "CHAT-2A: sealed World chat relay payload, key ring and replay window (CHAT-0 section 5)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-2-20261013
issue: 2074
governing_issue: 1622
pr: null
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
  - docs/agents/tasks/active/CHAT-2A.md
public_contracts: []
external_repositories: []
```

## Outcome

First reviewable batch of sub-issue #2074 (CHAT-2, parent #1622): the pure security core of the
World relay of CHAT-0 section 5. `chat::relay` holds the fixed binary payload (private and room
kinds), AES-256-GCM sealing with a random 96-bit nonce and the channel name plus `key_id` as
associated data, the per-World key ring (send with the newest key, open with any key retired
within 24 hours, `CHAT0-RL-12`), the 30 s freshness check and the bounded `message_id`
de-duplication window (`CHAT0-RL-13`, a full window refuses rather than forgets), and the
`oteryn_chat_<WorldId hex>` channel name (the canonical UUID without hyphens). AES-GCM comes from `aws-lc-rs`, already in the lock through
rustls; no new crate enters `Cargo.lock`.

## Architecture and source of truth

- PROVEN: `docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md` section 5 (layout, sealing, rotation, replay) and section 8 rows `CHAT0-RL-06`, `-12`, `-13`.
- DERIVED: the channel is `oteryn_chat_<WorldId without hyphens>` (44 bytes) because production WorldIds are canonical UUIDv7 (`node/config.rs`); CHAT-0 section 5 writes `<world_id>` without fixing the encoding.
- DERIVED: a `sent_at` more than 30 s ahead of the node clock is stale too, and a de-duplication entry is kept for twice the window, so no line the freshness check accepts can return after its entry is gone. The replay window size (8,192 lines per World) is an implementation bound, not a CHAT-0 row.
- Plaintext is at most 1,185 bytes (two-byte length prefixes), sealed at most 1,214, base64 at most 1,624, inside the section 5 figures.

## Acceptance criteria

- [x] Round trip of a room and a private line; the widest line stays within the documented bounds.
- [x] Oversized fields are `Oversized`, never truncated.
- [x] Wrong channel, altered ciphertext and a relabelled `key_id` fail authentication.
- [x] Rotation: newest key sends; the old key opens for 24 hours after its successor was added.
- [x] Stale, future-dated and replayed lines are refused; a full window refuses.
- [x] `Debug` of a line or key never prints text or key bytes.

## Excluded scope (stays CHAT-2, follow-up batches)

The `LISTEN`/`NOTIFY` connection and `NOTIFY` send, `log_parameter_max_length = 0` on it, listener
readiness and `CHAT_UNAVAILABLE`, the `NOTIFY` queue watch (`CHAT0-RL-09`), delivery of relayed
lines to sessions, private-message and room dispatch, the durable mute row
(`game_character_chat_mutes`, needs a leased migration number) and any wire change. No protocol,
registry, migration or content change here.

## Validation

- `cargo fmt -p oteryn-game-server --check`: pass
- `cargo clippy --offline -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --offline -p oteryn-game-server --lib -- chat`: pass (55 passed, 0 failed; 14 are `chat::relay`)

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
- merge commit/result: pending
