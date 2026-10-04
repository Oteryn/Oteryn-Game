# OTV2-20261004-bags-wire-1b

```yaml
task_id: OTV2-20261004-bags-wire-1b
title: BAGS-WIRE-1b keep the BAGS0-RL-04 view command rate per GameSession
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/bags-wire-1b-20261004
issue: 1622
origin: Codex P2 on PR #1720 (head 9f00472), deferred under D245 to this follow-up
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/container_view.rs
  - apps/game-server/src/gameplay_transport/container_view_tests.rs
  - apps/game-server/src/gameplay_transport/item_view.rs          # ItemViewContinuity field only
  - apps/game-server/src/gameplay_transport/item_view_tests.rs    # one struct literal
  - apps/game-server/src/gameplay_transport/connection.rs         # FRESH literal and command 21 rate check
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json                  # BAGS0-RL-04 consumers and notes
  - docs/agents/tasks/archive/OTV2-20261004-bags-wire-1b.md
```

## Outcome

- `BAGS0-RL-04` (10 view commands per second, sliding window) is per GameSession. The window
  (`ViewCommandWindow`, ten `Option<Instant>` slots, `Copy`) moved from the connection's
  `ContainerViewState` into `ItemViewContinuity`, so a reconnect, resume or transfer no longer
  resets it. An over-rate command is still `REJECTED` with an empty payload before decoding.
- Tests: the window unit test, the window carried across reconnect and transfer, and a connection
  test that serves 10 commands, reconnects with the carried continuity and gets command 11
  `REJECTED`.

## Excluded

- Any protocol, wire or limit value change; BAGS-1 work.

## Validation

- `cargo test --locked -p oteryn-game-server --quiet`
- `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`
- `python3 tools/agents/validate_governance.py`; `git diff --check`
