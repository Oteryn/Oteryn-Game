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
  migrations: [2858]          # 2859-2860 reserved, unused
  capabilities: [512]         # 513 reserved, unused
  commands: [6247, 6248]
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

## Gap list and state

| # | Gap | State |
|---|---|---|
| 1 | Durable guard table and migration | `apps/game-server/migrations/2858_channel_switch_guard.sql` |
| 2 | Classification and guard update in the final admission boundary | AFTER INSERT trigger on `game_durability_fresh_admission_receipts` (same transaction as admission) |
| 3 | Lock enforcement | runtime locks in `crates/gameplay/dispatch/src/gameplay_transport/channel_switch.rs`; database locks in `game_channel_switch_lock` |
| 4 | protocol-oteryn command and server flow | capability 512, commands 6247/6248, `docs/contracts/protocol-oteryn/v1/channel-switch-v1.md` |
| 5 | Client window, ping, selection-screen columns | window model in `crates/client-ui/src/features/0512-channel-switch/`; Bevy shell wiring, ping probe and selection-screen columns open |
| 6 | E2E with two Channels | open |

## Platform fields needed

`DirectoryChannel` (`crates/platform-contracts/src/lib.rs`) carries only `channel_ref` and
`display_name`. The window needs per Channel, from Platform: `pvp_type` (Optional PvP / Open
PvP), `ping_endpoint` (host and port the client probes for RTT), `region`, `load_percent` and
`full`. Until they exist the window shows "—" and nothing is guessed. The Game-side PvP type of a
Channel is stored in `game_channel_pvp_types` (migration 2858).

## Validation

- `cargo test -p oteryn-protocol-oteryn --lib`
- `cargo test -p oteryn-gameplay-durability --lib channel_switch`
- `cargo test -p oteryn-gameplay-dispatch --lib -- channel_switch chat_relay`
- `cargo test -p oteryn-client-ui --lib channel_switch`
- PostgreSQL 18.6: `cargo test -p oteryn-game-server --test character_authority_postgres channel_switch_guard`
  and the dispatch chat relay case `world_chat_reaches_another_channel_of_the_world_and_never_another_world`
  (CI PostgreSQL legs; no PostgreSQL 18.6 was available in the authoring session).
