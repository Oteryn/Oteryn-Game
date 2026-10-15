# CHAT-2B

```yaml
task_id: CHAT-2B
title: "CHAT-2B: World chat relay over LISTEN/NOTIFY, rooms and private messages (CHAT-0 sections 5 and 7)"
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/chat-2
issue: 2074
governing_issue: 1622
pr: null
base_sha: ed2b1180
head_sha: null
final_head_sha: null
owner: claude-code-session-013fCsCyaQjFiyBr4JFPBfrF
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
owned_paths:
  - apps/game-server/migrations/0088_chat_relay_leases.sql
  - apps/game-server/src/chat/relay/mod.rs
  - apps/game-server/src/chat/spam.rs
  - apps/game-server/src/chat/tests.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/character_chat_recipient.rs
  - apps/game-server/src/gameplay_transport/chat_relay.rs
  - apps/game-server/src/gameplay_transport/chat_relay/tests.rs
  - apps/game-server/src/gameplay_transport/chat_intent.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/capabilities_tests.rs
  - docs/agents/tasks/active/CHAT-2B.md
  - docs/agents/tasks/archive/CHAT-2B.md
public_contracts: []
external_repositories: []
```

## Outcome

The second batch of CHAT-2 (#2074, parent #1622, plan row 129). It wires the sealed relay core of
CHAT-2A (#2076) to a live World relay and serves rooms and private messages.

- **Relay.** `gameplay_transport::chat_relay` gives each World one PostgreSQL `LISTEN`/`NOTIFY`
  channel, `oteryn_chat_<hex>`. Lines are sealed with the World key ring and sent with
  `pg_notify` in autocommit. Each listener opens a line through its `ReplayWindow` and hands it
  to `ChatRuntime::deliver`. Delivery is at most once.
- **Lease.** The listener keeps the per-World relay lease in the new table
  `game_chat_relay_leases` (migration 0088, leased to CHAT-2B by the control plane). It is
  stored as `GREATEST`, and a trigger refuses to lower it. On restart the stored lease is read
  back into `ReplayWindow::resume`.
- **Readiness.** The relay is ready only when all of these hold:
  - it is listening;
  - it has a database clock reading;
  - `pg_notification_queue_usage()` is at most 0.5;
  - its connections assert `log_parameter_max_length = 0`. Every relay connection also sets
    `log_parameter_max_length_on_error = 0`.

  A relay that is not ready answers `CHAT_UNAVAILABLE`.
- **Rooms.** World, English, Help and Advertising. Opening and closing a room is answered before
  the Channel lock. A changed room set follows the result as one domain 12 delta type 2
  (`DELTA_TYPE_CHAT_ROOMS_V1`). A line to a room that is not open is `ROOM_NOT_OPEN`. World,
  English and Advertising need level 20 or Premium; Help needs neither. Advertising needs a
  vocation and allows one line per 2 minutes.
- **Private messages.** The recipient is found by `name_key` in the same World, using the
  `find_person` join as a DurabilityRoot semantic pass. A recipient who is not online
  (`session_state = 2`) gives `NOT_ONLINE`. A message to oneself is `REJECTED`. Sending needs
  level 3. More than 20 distinct recipients in 10 minutes mutes the sender.
- **Rate gate.** Room and private lines pass the same pre-gate as local speech, before the
  Channel lock (#2026). A muted session cannot cast (#2003).
- **Wire.** No new protocol numbers. The rooms delta and dispositions are those CHAT-0 already
  fixed.

## Architecture and source of truth

- PROVEN: `docs/architecture/reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md`
  sections 5 and 7 (CHAT-0 ACCEPTED, #2088).
- DERIVED (WP3 deviation, recorded here): the relay runs on its own small pool (4 connections),
  outside the WP3 durability root. The root is one physical connection on its own runtime and
  carries durability work only. A `LISTEN` connection that stays open would hold that connection
  for good, and chat sends must not queue behind durability passes. The relay pool reuses the
  root's verified endpoint and credentials. Its options are cloned before
  `oteryn_wp3_resource_budget`, so they do not carry the WP3 root budget. Recipient resolution
  still runs on the root as a semantic pass.
- DERIVED: an Advertising line claims its 2-minute interval before the send. A send that then
  fails still uses the interval, which fails closed against a flood.
- DERIVED: the relay key file is an environment secret, named by `OTERYN_CHAT_RELAY_KEYS_FILE`.
  It is read with the secret-file checks and holds one `<key_id> <activate_at_micros> <base64 32
  bytes>` per line. No key material is in the repository; tests generate their own keys.
  Provisioning keys for protected or production environments is out of scope.
- No chat text, sender or recipient is logged or stored (FND-02 §20). Relay failures are
  reported only as fixed reasons and counts (`chat_relay_disabled reason=...`,
  `chat_relay_dropped lines=N`). No database error text is logged.

## Excluded scope

- CHAT-2C: the durable mute row `game_character_chat_mutes`, migration 0089, leased to CHAT-2C.
  It opens after this PR merges.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test -p oteryn-game-server`, against local PostgreSQL 17 with
  `OTERYN_TEST_POSTGRES_ADMIN_URL`: pending
- `python3 tools/agents/validate_governance.py`: pending
- `python3 -m unittest discover -s tools/agents/tests`: pending

## Self-review

- method: the implementing agent read the whole diff against the owned paths.
- material findings: none open.

## Independent review

- required: Codex review on the frozen head.

## PR and closeout

- Refs #2074 (CHAT-2C follows).
