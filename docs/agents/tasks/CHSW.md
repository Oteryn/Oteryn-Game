# CHSW — in-game Channel switch (GAME-CHANNEL-01)

```yaml
task_id: chsw
title: "chsw: in-game Channel switch under GAME-CHANNEL-01"
mode: IMPLEMENT
status: in_progress
repository: Oteryn/Oteryn-Game
contract: docs/architecture/GAME-CHANNEL-01_CHANNEL_PRODUCT_POLICY_CONTRACT.md
branch: claude/happy-davinci-ijz3wy-chsw
pr: null
numbers:
  migrations: [2858, 2859]    # 2860 reserved, unused: load reads the reconnect journal
  capabilities: [512, 513]
  commands: [6247, 6248, 6251] # 6251 CHANNEL_LOAD under capability 512
merge_authority: CP_ONLY
```

## Owner decisions (2026-11-17)

- Change Channel in game is a safe exit plus a fresh admission into the target Channel of the
  same World: same login, same position, never a teleport. The selection-screen picker stays; a
  relog into another Channel is the same switch.
- Five minutes between voluntary switches, timed by the server clock (§10). The guard is keyed by
  CharacterId and WorldId and stores the last Channel and the cooldown state (§9).
- Hard locks (§8): combat or protection-zone block, a red or black skull toward a Channel that is
  not Open PvP, trade, a pending value transaction, boss/raid/event, house/instance/travel
  transition, checkpoint/handoff, destination state.
- First admission is the baseline; the same Channel is not a switch; a failed target leaves the
  guard unchanged and uses no cooldown; a crash and retry gives exactly one outcome.
- Every Channel has a PvP type (Optional PvP / Open PvP); the Channel list shows the current World
  only; the recommended Channel keeps the current type; a move to Open PvP warns.
- World Chat is shared by every Channel of a World and never reaches another World; Local Chat
  stays in its Channel.
- Social views are Channel-aware within one World: Party chat, Guild chat and private messages
  reach any Channel of the World; the Party and its chat survive a switch; shared experience,
  loot and Party spells still need the same Channel; VIP, Party and Guild rows show the Channel.

## Gap list and state

| # | Gap | State |
|---|---|---|
| 1 | Durable guard table and migration | `apps/game-server/migrations/2858_channel_switch_guard.sql` |
| 2 | Classification and guard update in the final admission boundary | AFTER INSERT trigger on `game_durability_fresh_admission_receipts` (same transaction as admission) |
| 3 | Lock enforcement | runtime locks in `crates/gameplay/dispatch/src/gameplay_transport/channel_switch.rs`; database locks in `game_channel_switch_lock` |
| 4 | protocol-oteryn command and server flow | capability 512, commands 6247/6248/6251, `docs/contracts/protocol-oteryn/v1/channel-switch-v1.md` |
| 5 | Client window, ping, selection-screen columns | window model in `crates/client-ui/src/features/0512-channel-switch/`; "Change Channel" menu entry and dialog (`apps/client/src/bevy_shell/gameplay_dialogs.rs`); the accepted switch re-enters through `gateway_login` with the target `channel_id` (`apps/client/src/bevy_shell/owner.rs`, `reentry.rs`); PvP type from `DirectoryChannel.pvp_type`; online count from command 6251. ping per client-channel-probe-v1 |
| 6 | E2E with two Channels | `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/channel_switch_two_channels.rs`: two assigned Channel runtimes of one World; through the real connection the player reads 6251 (both Channels, one online each), 6247 (ready), 6248 to the same Channel (refused) and to the other (safe exit, session ended), then a fresh admission into the other Channel counts the switch and starts the cooldown |
| 7 | Party, Guild and private chat across Channels of a World | World-keyed relay; test `party_guild_and_private_lines_cross_channels_of_a_world_and_never_another_world` |
| 8 | Party survives a switch | `apps/game-server/migrations/2859_channel_switch_party_hold.sql`; switch release holds the membership (`world_party::hold_terminal_member_in_transaction`), admission takes it back, an expired hold leaves through the World Party drain |
| 9 | Channel shown in VIP, Party and Guild rows; live update on a switch | capability 513: Party member field 7, VIP Online field 6, Guild member field 5; client rows in `crates/client-ui/src/features/0513-social-channel/`; a Party member on another Channel is greyed; VIP login/logout/channel-change notices |

## Platform fields needed

`DirectoryChannel` (`crates/platform-contracts/src/lib.rs`) carries `pvp_type` and `region`.
Ping: the current Channel is the live session round trip; every other Channel is measured
under client-channel-probe-v1 (Oteryn-Platform d2e54f2e, PR #1939; fixtures vendored in
`docs/contracts/fixtures/client-channel-probe-v1/`): `GET /v1/client/channel-probes`, then per
edge one warm-up and three timed `HEAD` requests on one connection, the median shown, 2 s per
request, at most every 30 s and only while the picker is open (`crates/platform-client/src/channel_probe.rs`).
The Platform list handler and the edge targets are not deployed yet (a T36b follow-up); until
then the list request fails and other Channels show "—". Capacity and `full` wait for an owner
decision; no capacity refusal exists. The Game-side PvP type of a Channel is stored in
`game_channel_pvp_types` (migration 2858); the online count comes from the durable reconnect
journal (command 6251), so there is no heartbeat.

## Validation

- `cargo test -p oteryn-protocol-oteryn --lib`
- `cargo test -p oteryn-gameplay-durability --lib channel_switch`
- `cargo test -p oteryn-gameplay-dispatch --lib -- channel_switch chat_relay` (with PostgreSQL 18.6
  this includes `postgres_tests::channel_switch_party` and `postgres_tests::channel_switch_two_channels`)
- `cargo test -p oteryn-client-ui --lib channel_switch`
- `cargo test -p oteryn-platform-client --lib channel_probe`
- `cargo test -p oteryn-client --features bevy-shell --lib channel_switch`
- PostgreSQL 18.6: `cargo test -p oteryn-game-server --test character_authority_postgres channel_switch_guard`
  and the dispatch chat relay case `world_chat_reaches_another_channel_of_the_world_and_never_another_world`
  (run locally on PostgreSQL 18.6 in Docker, `postgres:18.6-bookworm`).
