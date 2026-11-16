# Mechwire package M: client-vs-server test per M row (base main 60e40aae)

Every M row of `apps/game-server/src/qualification/SCENARIO_MATRIX.md` is BOTH-WIRED in `IMS.md`.
This file maps each row to its existing test of the whole client -> server -> client chain.

Paths: `CP` = `apps/game-server/src/qualification/client_phases`, `SES` = `crates/session/src/tests`.

## Test shape

- **Node-boot phases (`CP`).** These drive the real client transport (`oteryn_session::Session` through
  `oteryn_dev_client::DevClientSession`; `client_runtime::target_move::send` for M01 autowalk) against the
  real node over TCP+TLS. The reply goes through `Session::apply_state_delta`, and the assertions read
  client state: `world_spatial`, `world_entities`, `world_object_overlay`, `actor_vitals` and inventory.
  - `full_e2ef1.rs` (run as the provisioning phase, `world.rs:2933` -> `e2ef1::run`) checks M01-M05 and
    M07. After each row it logs out, admits again and compares the saved position, inventory and new
    session id (`full_e2ef1.rs:1114-1144`). This is the `relogin` check.
  - `full_row_*.rs` does the same for M06 and M08-M11b, each with its own relog.
  - `client_row_*.rs` write the capture files.
- **Replays (`SES`, `client-runtime`).** These feed the captured server payloads through the client
  decoder or reducer. When `E2E_ALL_CAPTURE_DIR` is unset they use built-in payloads.

## Row map

| row | chain test (file:line) | new/existing | relog | verified how (this session) | gap |
|---|---|---|---|---|---|
| M01 | `CP/full_e2ef1.rs:288` (`m01`: paced step, autowalk via `client_runtime::target_move::send` :246, stop, turn asserted on own entity facing, follow with a second client); capture `CP/client_row_m01.rs:32`; replay `crates/client-runtime/src/features/0064-target-move/tests/e2e_m01.rs:9` | existing | `full_e2ef1.rs:1114-1144` | replay run: pass (built-in payloads); node boot not run | `client_row_m01` checks only `Accepted` status; the client-state checks live in `full_e2ef1` |
| M02 | `CP/full_e2ef1.rs:403` (`m02`: far use and far move); capture `CP/client_row_m02.rs:61`; replay `SES/e2e_m02.rs:13` | existing | `full_e2ef1.rs:1114-1144` | replay pass; node boot not run | content: no plain door (appearance 1638 with door facts) reachable from town (packet; not re-verified) |
| M03 | `CP/full_e2ef1.rs:554` (`m03`: stairs, teleport, aggressive exhaust); capture `CP/client_row_m03.rs:22` (automatic floor change and exhaust refusal); replay `SES/e2e_m03.rs:8` | existing | `full_e2ef1.rs:1114-1144` | replay pass; node boot not run | content: the teleport case needs a character level the starter does not have (packet; not re-verified) |
| M04 | `CP/full_e2ef1.rs:670` (`m04`: player push :700, creature push :761, collision); capture `CP/client_row_m04.rs:58`; replay `SES/e2e_m04.rs:9` | existing | `full_e2ef1.rs:1114-1144` (pushed peer too) | replay pass; node boot not run | none known |
| M05 | `CP/full_e2ef1.rs:899` (`m05`: basic door, unlocked key door, locked refusal); capture `CP/client_row_m05.rs:98`; replay `SES/e2e_m05.rs:10` | existing | not required (door overlay is not saved per character) | replay pass; node boot not run | content: no matching door family reachable from town (packet; not re-verified) |
| M06 | `CP/full_row_m06.rs:133`; capture `CP/client_row_m06.rs:62`; replay `SES/e2e_m06.rs:10` | existing | `full_row_m06.rs:185-186` | replay pass; node boot not run | matrix: locked/level/quest predicate metadata unresolved |
| M07 | `CP/full_e2ef1.rs:950` (`m07`: carried rope, rope ascent); capture `CP/client_row_m07.rs:16`; replay `SES/e2e_m07.rs:11` | existing | `full_e2ef1.rs:1114-1144` | replay pass; node boot not run | content: no reachable rope spot; the starter has no rope (packet and matrix; not re-verified) |
| M08 | `CP/full_row_m08.rs:387`; capture `CP/client_row_m08.rs:380`; replay `SES/e2e_m08.rs:8` | existing | `full_row_m08.rs:461-477` | replay pass; node boot not run | matrix: row PARTIAL (not every terrain tool is covered) |
| M09 | `CP/full_row_m09.rs:349`; capture `CP/client_row_m09.rs:366`; replay `SES/e2e_m09.rs:12` | existing | `full_row_m09.rs:526-527` | replay pass; node boot not run | matrix: row PARTIAL |
| M10 | `CP/full_row_m10.rs:102` (bounded drowning); capture `CP/client_row_m10.rs:65`; replay `SES/e2e_m10.rs:111` | existing | `full_row_m10.rs:176-185` | replay pass; node boot not run | swimming, traps and special tiles are not covered (`full_row_m10.rs` header) |
| M11a | `CP/full_row_m11a.rs:70`; capture `CP/client_row_m11a.rs:14`; replay `SES/e2e_m11a.rs:19` | existing | `full_row_m11a.rs:85-93` | replay pass; node boot not run | none known |
| M11b | `CP/full_row_m11b.rs:105`; capture `CP/client_row_m11b.rs:58`; replay `SES/e2e_m11b.rs:9` | existing | `full_row_m11b.rs:182-185` | replay pass; node boot not run | none known |

Counts: 12 rows; 12 existing, 0 new. Content gaps: 4 rows (M02, M03 teleport, M05, M07); 3 more rows
have partial coverage (M06, M08-M10).

## Why no new test

- Each row already has a node-boot test of the whole chain that asserts client state, and a relog check
  wherever position or inventory changes.
- Nothing connects a real `oteryn_session::Session` to dispatch's `serve_admitted` in the PostgreSQL lanes:
  `s17_client_watch.rs` uses its own `Peer`, and `nx2_arena_native.rs` drives a client-ui reducer without
  `Session`. A Session-level PostgreSQL case would first need that admission harness. That harness is a
  separate task, and this session has no PostgreSQL route to verify it.
- The content gaps need authored content or bootstrap changes, not tests.

## Evidence (this session, head 1fce9546)

- `cargo test -p oteryn-session --lib e2e_m`: 11 passed, 0 failed (M02-M11b replays, built-in payloads).
- `cargo test -p oteryn-client-runtime --lib e2e_m01`: 1 passed (M01 replay, built-in payloads).
- Not run: node boot (`tools/qualification/node_boot/run.sh` needs Platform, PostgreSQL and the built
  world bundle) and PostgreSQL cases (`OTERYN_TEST_POSTGRES_ADMIN_URL` absent). The node-boot pass state
  of these rows is therefore unverified here.
