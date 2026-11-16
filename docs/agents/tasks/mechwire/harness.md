# mechwire-0 A: do the phases and the synthetic harness use the real client?
Mostly yes at the `crates/session` level (the real client transport), partly via `client-runtime`
typed adapters, in places via raw protocol-oteryn codecs; never via the `apps/client` mailbox/reducer.
Paths from repo root. Phases = apps/game-server/src/qualification/client_phases (CP).
Two harnesses: CP drive `oteryn-dev-client` (tools/dev-client, game-server dev-dep; wraps
`oteryn_session::Session`, re-exports `client_runtime`/`client_ui`, lib.rs:47-48). The crate
`oteryn-synthetic-client-harness` (tools/synthetic-client-harness) runs the production apps/client
login form, mailbox and `play::run_session` in its tests.
## Evidence per family
- login: CP call `oteryn_dev_client::connect_session` (CP/dev_client.rs:62, readmission.rs:27,
  world.rs:3045) -> `Session::admit` with real `CLIENT_SUPPORTED_CAPABILITIES`
  (tools/dev-client/src/lib.rs:235-262). The UI login form runs only in the harness
  (tools/synthetic-client-harness/tests/native_login.rs:2-3).
- walk: `client.step` (CP/client_row_m01.rs:40) -> dev-client lib.rs:415 -> `Session::step`
  (crates/session/src/session_commands.rs:42). Own encoding in three CP files: use_wire.rs:48,
  admission.rs:68, full_e2ef7.rs:2245 (`encode_step_intent` from protocol-oteryn, hand-built
  frame, no client builder). The paced `Stepper` is only in harness tests (native_vitals.rs:3).
- chat: `client.chat(&ChatIntent::Say ..)` (CP/client_row_c17a.rs:55, client_row_m11a.rs:20) ->
  `Session::chat` (session_commands.rs:330). client-runtime chat is re-exports of session types
  (crates/client-runtime/src/features/0007-chat/protocol.rs:1-6): no separate builder. The client-ui
  chat panel reducer runs only in the harness (tests/native_public_channels.rs:3).
- items: `client.move_item` (CP/client_row_i05.rs:35, full_row_i18c.rs:320) and `use_item`
  (client_row_w19a.rs:109) -> `Session::move_item`/`use_item` (session_commands.rs:109/97).
- others: typed client-runtime adapters via `oteryn_dev_client::client_runtime::*` (CP/full_row_s14c.rs:14,49
  party; full_row_m09.rs:200 npc_shop; full_row_w11.rs:18 boss_encounter); raw-feature caps go through
  `Session::gated_command` (crates/session/src/gated_command.rs:13). About 134 CP files mention client_runtime.
## Reducers
CP read results and deltas through `Session` (`apply_state_delta`, crates/session/src/domain_dispatch.rs:5;
`chat_log`, dev-client lib.rs:351), not client-runtime reducers (e.g. features/0011-world-interaction/state.rs,
0016-quest-log/state.rs) and not the apps/client mailbox/projection.
## "Step 0" slice (login, walk, chat through the real builder and reducer) needs
1. No new dependency for client-runtime/client-ui (re-exported by dev-client). Reaching apps/client from
   game-server needs an `oteryn-client` entry under `[dev_edges]` in workspace-boundaries.toml; the harness
   crate already has it, so put the slice in tools/synthetic-client-harness/tests if that file stays untouched.
2. Boot the node as the CP do, then drive apps/client `play::run_session` + `PlayView`/`Stepper` over the real
   TCP+TLS listener: login via `native_login::LoginForm`, walk via `Stepper`, chat via the client-ui chat intent.
3. Assert on the client reducer output (PlayView / chat panel state), not only on `CommandResult`.
4. Pair with apps/game-server/src/lifted_tests/auto/command_wiring_registry.rs so a server route with no client
   adapter fails before a scenario is written.
