# OTV2-20261004-quest-log-wire-1

```yaml
task_id: OTV2-20261004-quest-log-wire-1
title: QUEST-LOG-WIRE-1 Capability 16 QUEST_LOG_V1, domain 16 QUEST_LOG, command 22 QUEST_LOG_QUERY
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/otv2-20261004-quest-log-wire-1
issue: 1622
pr: 1741
decision: QUEST-GATE-0 §7 and §9 (QUESTGATE0-RL-06 to RL-09); owner D482 (2a); leases D485; wire lane D486
builds_on: QUEST-PRED-1, QUEST-LOWER-1 (#1727), QUEST-XP-1; pattern BAGS-WIRE-1 (#1720), BAGS-WIRE-1b (#1730)
migration_lease: none
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json        # capability 16, command 22, domain 16
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json           # own QUESTGATE0-RL-06..09 rows only
  - docs/contracts/protocol-oteryn/v1/quest_log_v1.proto   # new
  - crates/protocol-oteryn/src/{lib,quest_log,quest_log_tests}.rs
  - apps/game-server/src/gameplay_transport/{capabilities,capabilities_tests,connection}.rs
  - apps/game-server/src/gameplay_transport/{quest_log,quest_log_tests}.rs   # new
  - apps/game-server/src/quest/{log,log_tests}.rs          # new
  - apps/game-server/src/quest/mod.rs                      # the mod line only
  - docs/agents/tasks/archive/OTV2-20261004-quest-log-wire-1.md
```

## Outcome

- Protocol: capability 16 `QUEST_LOG_V1` (`offered: false`, no requirement), command 22
  `QUEST_LOG_QUERY` (oneof list, quest, track; 62 bytes; no result payload) and domain 16
  `QUEST_LOG` (snapshot and delta type 1, the whole domain, 180,268 bytes). Fail-closed codecs with
  max and max+1 tests in `quest_log_tests.rs`.
- Resource rows: `QUESTGATE0-RL-06` (10 tracked), `-07` (1,024 listed), `-08` (128 missions,
  measured 87), `-08-NAME` (128, measured 49), `-08-TEXT` (1,024, measured 463), `-08-BYTES`
  (16,384 per line, measured 13,057), `-08-DOMAIN-BYTES` (180,268) and `-09` (2 queries per
  second per GameSession).
- Projection (`quest/log.rs`, std only): the log catalogue per content revision, checked against
  that revision's state catalogue; quests indexed by ascending Oteryn key. Listed once the start
  track reaches its value (storyline, shown, with a start); completed by the completed receipt;
  `hide_when_completed` leaves the list; missions in `[start, end]`, done at the end; per-stage,
  fixed and template journals. An in-progress quest resolves against its pinned revision, a
  completed or stateless one against the active one; an unloadable pin hides the quest; an
  undeclared track never reads as an initial value.
- Server (`gameplay_transport/quest_log.rs`, a child module of `connection.rs`): content is used
  only when every worst-case line fits 16,384 bytes. The revision, tracked set and query window
  are per GameSession in `SessionContinuity`; the requested view is per connection and closes at
  every reconnect and transfer. Command 22: rate, decode, copy read, apply; anything refused is
  `REJECTED` with an empty payload and changes nothing. A 1 s refresh asks the owner for a copy
  changed since the projected version and sends a delta when the domain changes.
  `observe_quest_log` defaults to `Unavailable`.

## Assumptions

- Mission names and journal texts travel as strings (a pinned revision's missions cannot be
  resolved from the client's active content); quest names do not (the client resolves them by
  canonical index).
- The tracked set is carried across reconnect and transfer; the view is not.
- A per-stage journal with no entry for the current value shows an empty text.

## Excluded

- The production `observe_quest_log` (the session copy source in `gameplay_transport/mod.rs`) and
  the quest log content loader (missions, journal text and `hide_when_completed`,
  QUEST-CONTENT-2); offering capability 16.
- The domain 16 entry in the `resume.rs` FND-02 fence (not an owned path); the revision travels in
  the carried continuity.
- The client half.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-protocol-oteryn -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-protocol-oteryn -p oteryn-game-server`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
