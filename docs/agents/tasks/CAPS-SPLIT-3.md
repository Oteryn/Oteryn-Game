# CAPS-SPLIT-3 — per-mechanic command routing

```yaml
task_id: CAPS-SPLIT-3
title: Per-mechanic command routing out of connection.rs
mode: REFACTOR
status: waiting
repository: Oteryn/Oteryn-Game
branch: claude/happy-davinci-ijz3wy-routing
base_sha: 292cd0e2cc793cf1167391014906e801293a14d4
pr: null
owner: standalone Codex worker
owned_paths:
  - crates/gameplay/dispatch/src/gameplay_transport/connection.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/routing.rs
  - crates/gameplay/dispatch/src/gameplay_transport/connection/routing/**
  - docs/agents/tasks/CAPS-SPLIT-3.md
```

## Delivered

- Extracted 26 command bodies into 25 private mechanic modules, grouping attack target and fight modes in `combat_intent.rs`.
- Kept every predicate, branch order, retained-bank handling and input/capability gate in `connection.rs`. The existing `Dispatch` enum moved unchanged to the private routing module.
- Routed bodies borrow the existing session/view state. `Option<Dispatch>` carries an early exit back to the connection, which preserves the original disconnected/control-lost result. No new clones, hot-path allocations, dependencies, public API, protocol or authority changes.
- Step remains inline because its pacing buffer owns the frame and continues the connection loop. Logout remains inline because it polls the stream and logout future together while updating held frames and disconnect flags. Retained-bank and unregistered branches remain inline as connection gates.
- Existing tests are unchanged; no test import-path updates.
- `connection.rs`: 8,722 → 8,121 lines (601 fewer, 6.89%). All 26 new Rust files are below 600 lines; the largest is `use_item.rs` at 187 lines.
- Bound META policy: version 3.1.0 at `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, read from its immutable policy path. Work is limited to local, uncommitted authoring; publication and external review are outside this task.

All body files below are under `crates/gameplay/dispatch/src/gameplay_transport/connection/routing/`:

| Branch | File |
|---|---|
| Bed sleep | `bed.rs` |
| Use intent (map tool, rune, definition, container/corpse and world object) | `use_item.rs` |
| Actor spell cast | `spell.rs` |
| Container view | `container_view.rs` |
| Depot | `depot.rs` |
| Timed item | `timed_item.rs` |
| Item move | `item_move.rs` |
| Item look | `item_look.rs` |
| House move-out | `house_move_out.rs` |
| Cyclopedia (72/73) | `cyclopedia.rs` |
| Guild | `guild.rs` |
| Staff request/receipt | `staff.rs` |
| Quest log | `quest_log.rs` |
| VIP (48–50) | `vip.rs` |
| Party intent/query | `party.rs` |
| Chat | `chat.rs` |
| NPC talk | `npc.rs` |
| NPC trade | `npc_trade.rs` |
| Attack target; fight modes (two branches) | `combat_intent.rs` |
| Account achievements | `achievements.rs` |
| Appearance | `appearance.rs` |
| House intent/query | `house.rs` |
| Daily reward claim/query | `daily_reward.rs` |
| World event query | `world_event.rs` |
| Highscores query | `highscores.rs` |

For future mechanic PRs, a new command adds one `else if` predicate line in connection.rs and its body in connection/routing/<mechanic>.rs. Keep order, capability gates and connection-loop control flow in the connection; put mechanic decoding and dispatch in its own routing module.

## Validation

`CARGO` below denotes `/home/mole/oteryn-cache/codex-bin/cargo`; this absolute wrapper was used for every Cargo invocation, including all PostgreSQL tests. No database URL was printed. Only filtered mechanic PostgreSQL cases are run, per the owner's shared-PC rule.

- `CARGO check -p oteryn-gameplay-dispatch --locked --offline --quiet`: pass.
- `CARGO fmt --all -- --check`: pass.
- `CARGO clippy -p oteryn-gameplay-dispatch --all-targets --locked --offline --quiet -- -D warnings`: pass.
- `CARGO clippy -p oteryn-game-server --all-targets --locked --offline --quiet -- -D warnings`: pass.
- `CARGO test -p oteryn-architecture-check --locked --offline --quiet`: pass, 29 unit tests and 1 integration test.
- `python3 tools/agents/validate_governance.py`: pass (22 policy documents, 9 lanes).
- `python3 -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- `git diff --check`: pass.

Non-PostgreSQL dispatch lib: **884 passed, 0 failed, 14 existing ignored, 135 PostgreSQL cases filtered out** (68.20s). The existing ignored cases require retained external captures or explicit performance qualification; no ignore attribute or test was changed. The following selection runs the non-PostgreSQL suite while keeping unrelated database mechanics off this shared PC:

```bash
/home/mole/oteryn-cache/codex-bin/cargo test -p oteryn-gameplay-dispatch --lib --locked --offline -- :: \
  --skip postgres \
  --skip gameplay_transport::chat_relay::tests::two_nodes_of_a_world_exchange_room_and_private_lines \
  --skip gameplay_transport::chat_relay::tests::a_listening_relay_stores_its_world_lease \
  --skip gameplay_transport::chat_relay::tests::a_relay_role_without_the_set_grant_fails_closed \
  --skip gameplay_transport::chat_relay::tests::every_new_relay_connection_pins_logging_off \
  --skip gameplay_transport::chat_relay::tests::a_send_on_a_session_that_may_log_bind_parameters_sends_nothing
```

Filtered dispatch PostgreSQL command: **38 passed, 1 failed, 0 ignored** (747.30s). The failure is `gameplay_transport::monk_save::postgres_tests::world_event::world_event_postgres_real_dispatch_reset_restart_and_fences`, at its unchanged line 128: the no-capability query received no result instead of one rejection. Isolated execution and the original unmodified `HEAD` connection both reproduce it. No source/test repair made because the task requires identical behavior and unchanged tests.

```bash
/home/mole/oteryn-cache/codex-bin/cargo test -p oteryn-gameplay-dispatch --lib --locked --offline -- \
  gameplay_transport::monk_save::postgres_tests::bed_sleep_dispatch:: \
  gameplay_transport::monk_save::postgres_tests::depot_dispatch:: \
  gameplay_transport::monk_save::postgres_tests::item_look:: \
  gameplay_transport::monk_save::postgres_tests::guild_wire:: \
  gameplay_transport::monk_save::postgres_tests::guild_bank:: \
  gameplay_transport::monk_save::postgres_tests::guild_chat:: \
  gameplay_transport::monk_save::postgres_tests::house_wire:: \
  gameplay_transport::monk_save::postgres_tests::house_move_out:: \
  gameplay_transport::monk_save::postgres_tests::cyclopedia:: \
  gameplay_transport::monk_save::postgres_tests::daily_reward:: \
  gameplay_transport::monk_save::postgres_tests::appearance_change_postgres:: \
  gameplay_transport::monk_save::postgres_tests::party_membership:: \
  gameplay_transport::monk_save::postgres_tests::vip:: \
  gameplay_transport::monk_save::postgres_tests::staff:: \
  gameplay_transport::monk_save::postgres_tests::promotion_npc:: \
  gameplay_transport::monk_save::postgres_tests::mining:: \
  gameplay_transport::monk_save::postgres_tests::timed_item:: \
  gameplay_transport::monk_save::postgres_tests::world_event:: \
  gameplay_transport::monk_save::postgres_tests::quest_log_postgres_tests:: \
  gameplay_transport::monk_save::postgres_tests::logout_e2e:: \
  gameplay_transport::monk_save::postgres_tests::rune_use_bridge:: \
  gameplay_transport::monk_save::postgres_tests::session_ingress:: \
  gameplay_transport::monk_save::postgres_tests::map_pickup::map_pickup_dispatch_ \
  gameplay_transport::monk_save::postgres_tests::creature_move:: \
  gameplay_transport::item_use::item_use_postgres_tests:: \
  --test-threads=1 --nocapture
```

Isolated world-event rerun: **0 passed, 1 failed** (29.65s), same assertion. This does not meet the owner's pass-on-isolated-rerun condition for classifying shared-PC contention:

```bash
/home/mole/oteryn-cache/codex-bin/cargo test -p oteryn-gameplay-dispatch --lib --locked --offline -- \
  gameplay_transport::monk_save::postgres_tests::world_event::world_event_postgres_real_dispatch_reset_restart_and_fences \
  --exact --test-threads=1 --nocapture
```

Baseline comparison: ran the identical isolated command with `connection.rs` temporarily containing exactly `git show HEAD:crates/gameplay/dispatch/src/gameplay_transport/connection.rs`; no tests changed. **Same failure, 0 passed, 1 failed** (29.04s), at the same line 128. A `finally` block restored the refactor byte-for-byte and removed its temporary backup (verified SHA-256 `fe04cf9df04af47f4bd05da09903868b72928879d2112a7a0f5a39aea3eba828`). This is a proven baseline validation failure, not a routing regression or a pass-on-retry contention result. Source trace: the first `exchange` advances the session owner's result-sequence floor; the next exchange changes `next_command_id` but reuses the original `server_sequence`. The unchanged `SessionCommandStream::open` guard refuses that stale continuity before command routing, producing no result. Fixing the fixture is outside the task's explicit no-test-changes boundary.

After byte-for-byte restoration, `CARGO test -p oteryn-gameplay-dispatch --lib --locked --offline --quiet -- gameplay_transport::connection::tests:: --test-threads=1`: **52 passed, 0 failed, 0 ignored** (2.44s), recompiling the refactored source. Final format, governance validation, 129 governance unit tests and whitespace checks also passed.

Additional existing highscores and NPC-trade PostgreSQL store/handler cases: **pass**, both selected cases executed on PostgreSQL 18.6 (`server_version_num=180006`). Highscores also has the unchanged real-dispatch `connection::tests::highscores_query_reads_only_the_admitted_session_under_capability_9` test in the non-PostgreSQL run:

```bash
/home/mole/oteryn-cache/codex-bin/cargo test -p oteryn-game-server --test character_authority_postgres --locked --offline -- \
  highscores_postgres_cases::experience_gain_reaches_the_client_through_the_snapshot_job \
  npc_trade_postgres_cases::buy_with_change_then_partial_and_whole_sell_replay_and_conflicts \
  --test-threads=1 --nocapture
```

Whole-diff self-review against `/home/mole/oteryn-cache/CODEX-CHECKLIST.md`: pass. Re-read every new module and the full connection diff. Token comparison against the clean starting `HEAD` verifies all 30 chain headers in order, retained-bank and input/capability gating, both inline mechanic bodies and all 26 extracted bodies. Only explicit parameter/reborrow/module-path adaptations, five equivalent `let Some` → `?` early exits, the `Option<Dispatch>` wrapper and rustfmt's redundant match-block normalization differ. The `Dispatch` variants and fields are unchanged. Code outside the extracted enum/chain is unchanged apart from the module declaration/import, including all tests. Workspace `rg` over `apps/`, `crates/`, `tools/` confirmed the enum's consumers and all 26 production route callers. No enum variants, struct fields, trait methods or public API functions were added.

## Unfinished

- Baseline world-event test failure at `postgres_tests/world_event.rs:128` prevents a fully green PostgreSQL validation result. Reproduced with both refactor and original `HEAD`; requires a separate fixture repair permitted to change tests. All other selected checks and 40 PostgreSQL cases passed.
- Existing coverage limitation: no dedicated NPC-trade real-dispatch PostgreSQL case exists in dispatch. The existing durable BUY/SELL PostgreSQL case, NPC-trade unit tests, and the mechanically verified branch move provide the available qualification. No tests were added or changed, as required by the task.
- Changes must remain uncommitted; PR, CI and publication are left to the owner/control plane.
