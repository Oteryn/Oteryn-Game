# Package W: client-to-server test map

Scope: every `SCENARIO_MATRIX` row with an id starting `W` that `PW.md` marks BOTH-WIRED
(W14 is owner-excluded). Each row lists the test that drives real client code
(`oteryn_session::Session` through `tools/dev-client`, or a client-ui/client-runtime reducer)
against a real server path.

Paths are shortened:

- `QP/` = `apps/game-server/src/qualification/client_phases/`
- `PG/` = `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/`
- `ST/` = `crates/session/src/tests/`

## How the existing chain tests run

- **Node-boot phases.** `QP/full_row_*.rs` and `QP/client_row_*.rs` drive the dev-client
  `Session` over TCP+TLS against a booted node. The client's decoder or reducer applies the
  replies, and durable rows are read after a relog where the row has durable state.
  - The phases run from `client_world::journeys` (`QP/world.rs`), which reports
    `verdict::indexed("<row>.full" | "<row>.client")`.
  - That journey lives inside the `#[ignore]` test `server_seam_real_owners_over_tcp_tls`
    (`apps/game-server/src/qualification.rs`). It needs the WP5 Platform + PostgreSQL topology,
    so this worker did not run it. These rows are **not verified here**.
- **Replay tests.** `ST/e2e_w*.rs` and the client-ui/client-runtime `e2e_w*.rs` tests feed
  captured server bytes through the real client decoder or reducer. The bytes are real server
  bytes only when the node run sets `E2E_ALL_CAPTURE_DIR`; otherwise the tests use
  encoder-built fixtures, and a local run proves decoder wiring only.
- **New dispatch PostgreSQL case (W09).** The real `Session` admits over a tokio duplex through
  the production `admit_frame` and `serve_admitted` (the listener's sequence). It advertises its
  own `CLIENT_SUPPORTED_CAPABILITIES` ids 4, 6, 15 and 31, sends the USE, and applies the pushed
  inventory delta in its own decoder. A second admission then reads the durable result back from
  PostgreSQL through the join snapshot. Two fixture choices, both stated in the code:
  - The owner offers the production `BUNDLE_WORLD_OFFERED_CAPABILITIES`. The fixture World has no
    bundle map, and the non-bundle Item-view offer lacks Item use (15), which gates the Cube route
    (`connection/routing/use_item.rs`). The older drome_cube cases bypass admission with a
    hand-built offer instead.
  - The cube sits directly in the main backpack. The client's inventory view lists only the
    backpack's own entries, and container views (14) need capability 12.

## Row map

Every row's full phase sits in `QP/full_row_<row>.rs` and reports at `QP/world.rs:<line>`; the
table gives only those line numbers.

| row | client-to-server test | new/existing | verified how | gap |
|---|---|---|---|---|
| W01a | full phase `:79` (world.rs 846); `QP/client_row_w01a.rs` (world.rs 1975); `ST/e2e_w01a.rs:141` | existing | not run here (node topology) | none known |
| W01b | full phase `:55` (world.rs 762); client_row (world.rs 1982); `ST/e2e_w01b.rs:131` | existing | not run here | none known |
| W01c | full phase `:73` (world.rs 951); client_row (world.rs 1989); `ST/e2e_w01c.rs:58` | existing | not run here | none known |
| W05a | full phase `:29` (world.rs 832); client_row (world.rs 2017); `ST/e2e_w05a.rs:92` | existing | not run here | conditional: needs a server-save reset in flight during the walk |
| W05b | full phase `:24` (world.rs 2704); server wire `PG/daily_world_gates.rs:16` | existing | not run here | content/precondition: day-change effects fall outside the walk budget, so the full phase reports FAIL by design |
| W05c | full phase `:89` (world.rs 930); client_row (world.rs 2024); `crates/client-ui/src/features/0081-npc-shop/tests/e2e_w05c.rs:51` | existing | not run here | conditional: day/night NPC rotation is unimplemented |
| W05d | full phase `:16` (world.rs 2711) | existing | not run here | conditional: passes only on an active event date |
| W06 | full phase `:18` (world.rs 2718); server wire `PG/echo_warden.rs:31` | existing | not run here | content: the Echo portal is a 0.05% draw, so the walk cannot reach it |
| W09 | `PG/drome_cube.rs:86` (`drome_cube_client_session_use_applies_reward_and_relog_reads_it_back`, body `PG/drome_cube/session_client.rs`); full phase `:23` (world.rs 2732) | **new** + existing | run locally against PostgreSQL 18.6: seen red (USE Rejected while Item use 15 was unselected), then green; route-less run returns Ok | content: a player has no route to a Drome Cube or concoction, so the node full phase reports FAIL. The new PG case seeds the cube. |
| W11 | full phase `:24` (world.rs 825); client_row (world.rs 2038); `crates/client-ui/src/features/0117-boss-encounter/tests/e2e_w11.rs:60` | existing | not run here | conditional: the lever is refused for a starter character, so the combat chain is not reached |
| W15a | full phase `:57` (world.rs 783); client_row (world.rs 2157); `crates/client-runtime/src/features/0016-quest-log/tests/e2e_w15a.rs:182` | existing | not run here | none known |
| W15b | full phase `:185` (world.rs 818); client_row (world.rs 2164); `ST/e2e_w15b.rs:130`; server wire `PG/quest_catalogue_chests.rs:28` | existing | not run here | none known |
| W15c | full phase `:24` (world.rs 2739) | existing | not run here | conditional: refused by task level or premium state |
| W15d | full phase `:42` (world.rs 776); client_row (world.rs 2171); `crates/client-runtime/src/features/0016-quest-log/tests/e2e_w15d.rs:85` | existing | not run here | none known |
| W16 | full phase `:26` (world.rs 909); client_row (world.rs 2178); `ST/e2e_w16.rs:17` | existing | not run here | conditional: boss profiles are unreachable for the walk |
| W19a | full phase `:350` (world.rs 755); client_row (world.rs 2192); `ST/e2e_w19a.rs:48`; server wire `PG/reward_bag.rs:14` | existing | not run here | none known |
| W19b | full phase `:22` (world.rs 2746); server wire `PG/item_seed_planting.rs:32` | existing | not run here | content: no seeds for a starter character; the full phase reports FAIL. No Session-driven PG case: the fixture takes `ComposedFreshAdmission` directly. |
| W19c | full phase `:19` (world.rs 2753); server wire `PG/novelty_items.rs:83` | existing | not run here | none known |
| W19d | full phase `:22` (world.rs 2760); server wire `PG/item_cosmetics.rs:22` | existing | not run here | content: no outfit or mount items for a starter character; the full phase reports FAIL. No Session-driven PG case, same reason as W19b. |
| W19e | full phase `:22` (world.rs 2767); server wire `PG/rust_lottery.rs:23` | existing | not run here | content: no lottery ticket or rust remover; the full phase reports FAIL. No Session-driven PG case, same reason as W19b. |
| W20a | `QP/client_row_w20a.rs:62` (world.rs 2199); `ST/e2e_w20a.rs:66` | existing | not run here | no durable state, so no relog check applies; no `.full` phase |
| W20b | `QP/client_row_w20b.rs:99` (world.rs 2206); `ST/e2e_w20b.rs:79` | existing | not run here | no durable state, so no relog check applies; no `.full` phase |
| W22a | full phase `:309` (world.rs 811); client_row (world.rs 2213); `ST/e2e_w22a.rs:73` | existing | not run here | none known |
| W22b | full phase `:56` (world.rs 720); client_row (world.rs 2220); `crates/client-ui/src/features/0140-prey/tests/e2e_w22b.rs:86` | existing | not run here | conditional: multi-modifier steps (Wealth Duplex and others) |
| W23 | full phase `:27` (world.rs 769); client_row (world.rs 2227); `ST/e2e_w23.rs:43`; server wire `PG/character_home_town.rs:56`, `PG/citizen_qualification.rs:31` | existing | not run here | none known |
| W24 | full phase `:58` (world.rs 804); client_row (world.rs 2234); `ST/e2e_w24.rs:70`; server wire `PG/ambient_b24.rs:10` | existing | not run here | none known |

## Counts

- **BOTH-WIRED rows:** 26.
- **Coverage:**
  - 26 rows have an existing node-boot client phase.
  - 24 of them have a `.full` phase; W20a and W20b are `.client` only.
  - 1 new Session-driven PostgreSQL case (W09).
- **Gaps:**
  - **Content (the full phase reports FAIL by design):** W05b, W06, W09, W19b, W19d, W19e.
  - **Conditional:** W05a, W05c, W05d, W11, W15c, W16, W22b.

## Verification

Run on this branch with local PostgreSQL 18.6 (pgdg), 2026-11-16:

- **Dispatch PostgreSQL cases:**
  - `cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib`, run as
    `postgres_tests::drome_cube` with `OTERYN_TEST_POSTGRES_ADMIN_URL` set: 7 passed, 0 failed.
    That covers the new W09 case and the 5 existing drome_cube cases.
  - The same new case with the URL unset: passed (returns `Ok(())`).
- **Clippy:** `cargo clippy --locked -p oteryn-gameplay-dispatch -p oteryn-dev-client --all-features --all-targets -- -D warnings`: clean.
- **Client replay tests:**
  - `cargo test --locked -p oteryn-session --lib e2e_w`: 12 passed.
  - `cargo test --locked -p oteryn-client-ui --lib e2e_w`: 8 passed (includes W05c, W11 and W22b).
  - `cargo test --locked -p oteryn-client-runtime --lib e2e_w`: 2 passed (W15a, W15d).
  - These use encoder fixtures when `E2E_ALL_CAPTURE_DIR` is unset, so they prove client
    decoder wiring only.
- **Not run:** the node-boot journey `server_seam_real_owners_over_tcp_tls`, which needs the
  WP5 Platform + PostgreSQL topology. Every `.full` and `.client` verdict above is therefore
  unverified in this run.
