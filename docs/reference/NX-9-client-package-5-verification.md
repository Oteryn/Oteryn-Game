# NX-9: client package 5 verification

Base: `81504ec792c186a3adcdc95aef8610d0862334cf`.

The base already contains the requested server projections and native controls.
No migration, capability bit, command ID, content pin or golden is added.
Reserved capability bits 390–394 and commands 680–689 remain unused.

Headless native play exercises the production input/mailbox/session-loop and UI reducers.
Physical Windows window/GPU interaction is not executed on Linux.
Server PostgreSQL checks exercise admitted production routing separately; these are
complementary checks, not a claim that a mocked Stepper is a real server.

## Base traces

Server paths below start at `crates/gameplay/dispatch/src/gameplay_transport/`;
client paths start at `apps/client/src/features/`. Line numbers identify the base.

| Row | Server | Native client | Exact client test filter |
| --- | --- | --- | --- |
| M01 | `connection/serve/target_move.rs:46` | `0064-target-move/input.rs:11` | `feature_0064_target_move` |
| M04 | `creature_push.rs:16` | `0082-creature-push/input.rs:12` | `feature_0082_creature_push` |
| I17a | `connection/routing/hotkey_equip.rs:47` | `0179-hotkey-equip/input.rs:26` | `feature_0179_hotkey_equip` |
| I17b | `hotkey_equip.rs:143` | `0179-hotkey-equip/input.rs:26` | `feature_0179_hotkey_equip` |
| I17c | `hotkey_equip.rs:395` | `0179-hotkey-equip/input.rs:26` | `feature_0179_hotkey_equip` |
| I17d | `hotkey_equip.rs:422` | `0179-hotkey-equip/input.rs:26` | `feature_0179_hotkey_equip` |
| I18b | `connection/routing/item_move.rs:57` | `0230-browse-field/input.rs:36` | `feature_0230_browse_field` |
| I18c | `container_view.rs:339` | `0230-browse-field/tests/play.rs:28` | `feature_0230_browse_field` |
| E12 | `connection/routing/hireling_management.rs:5` | `0250-hireling-management/native.rs:90` | `feature_0250_hireling_management` |
| ADD-0066 | `connection/serve/result.rs:10` | `0011-world-interaction/protocol.rs:13` | `every_server_detail_reaches_the_native_status_without_rewording` |
| P18 | `item_forge.rs:49` | `0152-item-forge/protocol.rs:6` | `feature_0152_item_forge` |
| P05 | `task_board.rs:93` | `0072-task-board/tests/play.rs:306` | `feature_0072_task_board` |
| P04 | `task_board.rs:93` | `0072-task-board/tests/play.rs:232` | `feature_0072_task_board` |

I17a–d use one assigned item-definition key, command 348/capability 179,
with carried selection capability 180. The server selects equip, unequip or swap;
the client does not predict jewelry timing. The key test checks repeat suppression
and refusal presentation. Server toggle/reload, nested exchange and ordinary/jewelry
clock tests provide the corresponding authority evidence.

I18b uses ordinary session item moves from the Browse window. I18c preserves the
ground-container view ID in capability 14/command 21 Up; the server resolves its field (`container_view.rs:254` preserves the BrowseField
parent; `postgres_tests/browse_field/wire.rs:152` proves Up returns both windows to the field).
P18 decodes the existing result/history payload in
`crates/client-ui/src/features/0152-item-forge/protocol/reply.rs`; no new message is needed.
P04/P05 use the existing full Task Board protocol, including weekly, delivery,
shop and talisman actions. E12 uses capability 250/command 470 and server-published
identity, outfit entitlements and colors.

## Repairs and verification scope

The isolated movement control test reproduced the timeout in 3.00 s. The Say
reply reached local chat but did not release the movement panel's shared pending
request. A distinct local movement-speech command now routes its reply to the panel that
owns the pending request, without completing an unrelated public-channel request. No casting
rule or S17 source changes. The unchanged reproducer passed in 0.05 s.

The E12 play test now publishes a valid server identity and entitled outfit,
selects it, submits the outfit through the session loop, verifies the decoded
request identity/colors, and checks the resulting panel and save feedback.

## Donor evidence

Fetched read-only:

- Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`, `src/server/network/protocol/protocolgame.cpp`.
- Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, same path: Follow at 2226, Browse at 3817, forge history parser registration at 1455.
- Existing pinned implementation evidence for hireling outfits: Crystal
  `data/libs/systems/hireling.lua:30-41,205-227`, cited by the existing UI codec.

No new donor gameplay rule is adopted. No Dawnport, OT-only feature, P23,
quest, encounter-server, starting-equipment or casting change.
Wiki checks for the requested mechanics remain **wiki-pending** (known 402);
no refused donor URL and no NEED-FETCH.

## Executed client checks

All filters below use `cargo test --locked -p oteryn-client --lib FILTER -- --nocapture`.
Durations are the test runner durations, excluding build/lock time.

| Filter | Passed | Duration |
| --- | ---: | ---: |
| `feature_0064_target_move` | 5 | 0.00s |
| `feature_0082_creature_push` | 4 | 0.01s |
| `feature_0179_hotkey_equip` | 3 | 0.00s |
| `feature_0230_browse_field` | 6 | 0.00s |
| `feature_0250_hireling_management` | 1 | 0.04s |
| `every_server_detail_reaches_the_native_status_without_rewording` | 1 | 0.00s |
| `feature_0152_item_forge` | 3 | 0.00s |
| `feature_0072_task_board` | 7 | 0.26s |

Additional checks: `feature_0011_world_interaction`: 7 passed / 0.01s;
`cargo test --locked -p oteryn-client-runtime use_flow_presents_every_negotiated_refusal_detail -- --nocapture`: 1 passed / 0.00s.
The final isolated movement reproducer passed / 0.06s and expanded E12 / 0.04s.

Final crate validation: `cargo test --locked -p oteryn-client --quiet`
(all targets; 312 library tests plus integration targets);
`cargo clippy --locked -p oteryn-client --all-targets --quiet -- -D warnings`;
`cargo fmt --all --check`; `oteryn-architecture-check workspace .`; `git diff --check`.
All passed. Existing vendor deprecation/documentation warnings were emitted; vendor is unchanged.

Self-review: read the entire source delta. The movement command remains local
and uses the same CHAT_V1 wire caller; only the owning UI result reducer changes.
E12 checks valid identity provenance, wire decode, entitlement-compatible outfit
and all four colors. No persistence/session fence changes, generator edits,
skipped tests or weakened assertions.

## Executed PostgreSQL authority checks

Each filter uses `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib FILTER -- --nocapture`, with the supplied PostgreSQL URL present.
These are existing production-route cases, not newly authored PostgreSQL cases.
Durations below exclude compilation.

| Exact filter | Passed | Duration |
| --- | ---: | ---: |
| `target_move_real_dispatch_walk_turn_follow_and_cancel_on_postgres` | 1 | 50.62s |
| `creature_push_real_dispatch_moves_and_refuses_on_postgres` | 1 | 90.61s |
| `browse_field_postgres` | 5 | 95.65s |
| `hireling_postgres_client_use_management_durable_replay_and_lost_ingress_refusal` | 1 | 51.71s |
| `refusal_detail_bundle_world_door_too_far_with_and_without_capability_66` | 1 | 42.51s |
| `forge_postgres_dust_sliver_core_conversion_and_history_replay` | 1 | 46.62s |
| `task_board_real_dispatch_weekly_bounty_custody_reset_and_replay` | 1 | 124.10s |
| `hotkey_equip_postgres_dispatch_toggle_replay_and_durable_reload` | 1 | 41.98s |
| `hotkey_equip_postgres_nested_exchange_has_one_receipt_and_exact_edges` | 1 | 52.01s |
| `ordinary_exhaust_waits_but_jewelry_refuses_without_waiting` | 1 | 0.00s |
| `equip_timers_share_ordinary_slots_and_isolate_ring_necklace_and_actors` | 1 | 0.00s |

Hotkey execution markers: `PG_MARKER hotkey-real-route` reported 40.694637063s
for toggle/reload and 50.834110696s for nested exchange. PostgreSQL startup
confirmed the retained PostgreSQL 18.6 cluster. Browse ran five real cases;
none of these reported a skipped/no-URL success.

## Final disposition

| Row | Result | Evidence |
| --- | --- | --- |
| M01 | DONE | Native turn/stop/follow controls; target-move real PostgreSQL route |
| M04 | DONE | Native drag push/refusal controls; real push PostgreSQL route |
| I17a | DONE | Assigned item key; PostgreSQL equip and reload |
| I17b | DONE | Same key toggles worn item; PostgreSQL toggle/reload |
| I17c | DONE | Same key invokes server exchange; nested exchange PostgreSQL receipt/custody test |
| I17d | DONE | Key waits for server result/refusal; independent ordinary/ring/necklace clocks verified |
| I18b | DONE | Native item move controls; five Browse PostgreSQL cases |
| I18c | DONE | Native Up preserves view ID; ground-container Up wire PostgreSQL case |
| E12 | DONE | Expanded identity/outfit session-loop play; real management PostgreSQL case |
| ADD-0066 | DONE | All details render; use detail decoder/native play and real capability-66 refusal PostgreSQL case |
| P18 | DONE | Result/history native decoding play; real conversion/history replay PostgreSQL case |
| P05 | DONE | Native purchase/refusal play; real task-board purchase/upgrade PostgreSQL scenario |
| P04 | DONE | Native weekly/delivery/shop session-loop play; real task-board PostgreSQL scenario |

No row stopped. No commits or pushes. No new numbers.
