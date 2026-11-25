# GMOTD — Guild message of the day

## Existing Oteryn owners

- **Guild durability (S1).** `game_guilds.motd` defaults to empty. `game_guild_ranks` and `game_guild_members.rank_level` hold ranks and assignments. The native `SetMessage` writer permits level 1 (Leader) and 2 (Vice Leader), journals the edit and advances the Guild revision.
  `apps/game-server/migrations/0132_guild_core.sql:4–24`, `crates/gameplay/domain/src/domain/guild.rs:8`, `crates/gameplay/durability/src/durability/guild/text_mutation.rs:20–32,70–98`.
- **Guild chat (S2/S5/S6).** Admission reads current membership and MOTD. World relay delivery rechecks membership at dequeue. Opening Guild chat is a local client action.
  `crates/gameplay/dispatch/src/gameplay_transport/guild_chat/admission.rs:6–24`, `crates/gameplay/dispatch/src/gameplay_transport/guild_chat/dequeue.rs:53–90`, `apps/client/src/features/0036-chat-guild-room/input.rs:20–28`.
- **Read wire and UI.** The login chat snapshot carries `GuildRoomState::Member.motd` (field 5). Guild Own also returns `GuildSummary.message` and its revision. The client projects Own over the join snapshot and renders `Guild — {motd}`.
  `crates/protocol-oteryn/src/chat/guild_snapshot.rs:9–21,49–74`, `crates/protocol-oteryn/src/guild.rs:72–80`, `apps/client/src/features/0036-chat-guild-room/protocol.rs:21–29`, `crates/client-ui/src/features/0036-chat-guild-room/view.rs:7–8`, `crates/client-ui/src/components/localization/locales/formats.json:90`.
- **Edit wire.** Current Guild intents lack SetMessage. The separate admin command `world.motd.set` is a World operation under `game.world.ops`.
  `crates/protocol-oteryn/src/guild.rs:8–45`, `crates/admin/contracts/src/catalog.rs:171`.

## Parity

Primary donor is CrystalServer `main` at `201189f871d23597b82caf88fd611454a2ddefb8` (OTS_HYPOTHESIS_ONLY). Donor paths below are relative to that revision.

| Behavior | Crystal | Planned Oteryn |
| --- | --- | --- |
| Authority | In-game Guild management registers Guild-channel `!setmotd`. Only `GUILDLEVEL_LEADER` may edit. `data/scripts/talkactions/player/guild.lua:18–47` | Rank level 1 only. Vice and member edits are refused. Revise the existing level-1-or-2 contract at `docs/contracts/OTERYN_GAME_GUILDS_V1.md:155`. |
| Length and text | Lua `#newMotd` limits the parameter to 255 bytes after the command parser strips leading spaces. `data/scripts/talkactions/player/guild.lua:29–34`, `src/lua/creature/talkaction.cpp:59–65` | At most 255 UTF-8 bytes for new input. Reject controls, trim with existing chat text rules and reject over-limit input without truncation. Preserve reads of older stored values. |
| Persistence and clear | Per-guild `motd` defaults to empty. Bare `!setmotd` writes empty, and empty suppresses the notice. `schema.sql:387–398`, `data/scripts/talkactions/player/guild.lua:29–38`, `src/io/ioguild.cpp:87–103,323–328`, `src/game/game.cpp:9563–9569` | Same Guild row and empty sentinel. Empty input explicitly clears. Whitespace-only nonempty input is invalid. |
| Open and login | A successful join schedules a nonempty `Message of the Day: …` for the Guild channel after 150 ms. Login reloads membership and rank. **PARITY_PENDING** whether the donor client automatically opens Guild chat at login. `src/server/network/protocol/protocolgame.cpp:1851–1853,5629–5638`, `src/creatures/interactions/chat.cpp:113–139`, `src/game/game.cpp:9557–9572`, `src/io/iologindata.cpp:137–138`, `src/io/functions/iologindata_load_player.cpp:446–485` | Populate the Guild chat view at login from the existing snapshot and keep the tab closed until the player opens it. Show one localized system notice on each closed-to-open Guild tab transition with a current nonempty MOTD. Wire order replaces the donor timer. |
| Change | The setter persists and reopens only the editor's channel. **PARITY_PENDING** live fan-out in official Tibia beyond this donor path. `data/scripts/talkactions/player/guild.lua:36–39`, `src/game/game.cpp:9462–9470` | Refresh only the editor's Guild channel. Other members see the new MOTD through Guild Own when they next open the Guild tab or from their next login snapshot. A clear removes the current MOTD and suppresses future open notices. |

## Design and implementation units

1. **Durability.** Reuse `commit_guild_text`, the Guild revision and its receipt/audit transaction. Use **one forward migration (3317)** for the operation text CHECK (empty or bounded) and both SQL role guards. Keep the existing column and immutable receipts. Allow historical wider receipts to stand while enforcing the tighter bound on new writes. Align the Guild contract, request and audit validation with the new rules.
   `apps/game-server/migrations/0132_guild_core.sql:53–56,205–217,441–442`, `crates/gameplay/durability/src/durability/guild_audit/text.rs:37–47`.
   Reuse the runtime actor check and durable recovery, node and session-generation fence. Lock the Guild, reread current same-World membership/rank, then compare the expected revision. Commit MOTD, receipt and audit together. Identical command ID and bytes return the prior receipt with no second write. Conflicting reuse and stale sessions cannot write.
   `crates/gameplay/dispatch/src/gameplay_transport/guild_intent.rs:39–63,108–127`, `crates/gameplay/durability/src/durability/guild/operation.rs:46–72,104–105`, `crates/gameplay/durability/src/durability/guild/journal.rs:13–29,103–121`.
2. **Command.** Add one protocol command type under existing Guild support. Route through Guild dispatch to `SetMessage`. The current Guild envelope caps requests at 128 bytes, so give the new command its own bound. Require existing Guild and Guild-chat support. Capability 618 stays unused because delivery uses the existing receipt and projections. Send Guild ID, expected revision and text (empty for clear). Map stale revision to a refresh and explicit retry. Reuse the Guild-tab composer for `!setmotd [text]`, consuming it as an edit before the normal speech path.
3. **Delivery.** Refresh the editor's view through Guild Own after an edit. Other members refresh through Guild Own at tab open or see the message in their next login snapshot. There is no relay invalidation, change delta or egress recheck. Clear the projection and close the Guild tab when Own reports membership loss.

## PostgreSQL acceptance

Mount the live-dispatch cases in `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/` for the dispatch PG partition. Drive the production command, login and Guild Own paths. Verify the tab-open presentation through the client adapter with the relog payload.

- `gmotd_leader_sets` and `gmotd_leader_clears` check the stored value, receipt and empty projection.
- `gmotd_member_refused`, `gmotd_vice_refused` and `gmotd_stale_session_refused` prove no mutation.
- `gmotd_other_member_sees_on_next_open` covers another Channel, a removed member and a nonmember.
- `gmotd_shown_on_channel_open_after_relog` checks the login payload and the fenced Own refresh after relog.
- `gmotd_length_limit` covers 255/256 UTF-8 bytes, multibyte input and rejected controls.
- `gmotd_replay_same_command_id` checks the original result, one revision across retry, plus rejection of changed bytes.
