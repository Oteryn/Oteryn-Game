# COIN-EXCHANGE-1 — banker coin exchange

Owner authority: decision 27 in `OTERYN_GAME_NIGHT_W1_OWNER_DECISIONS_2026-10-25.md`.
Reference: Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`.
Implementation candidate; the focused dispatch scenario passed against live local PostgreSQL.

## Audit

Production routing is command 7 `NPC_TALK_INTENT` → `vocation_talk` → `npc_bank_talk`.
`NpcConversations::current_bank` qualifies the current NPC generation, conversation, range and
Bank service. The existing bank lane retains accepted commands and resumes them before interpreting
replacement input (`npc_command.rs`, `npc_command/premium_purchase.rs`, connection authority).
`npc_purchase.rs` is the separate promotion/blessing purchase owner; it burns a fee and advances
CharacterRevision, so coin exchange does not call it or modify its pricing.

The inventory owner is `item_loop_move.rs`: equipped main backpack, direct container entries,
nested entries, depth eight, at most 20 entries per container and 500 carried entries. Definitions,
stack limits, weights and capacities come from the admitted content generation. The exchange uses
its `ItemLoopContent`, `carried_weight`, derived capacity and existing equipment/item fencing.
Migration 0317 adds only `game_coin_exchange_receipts`; its proof functions extend existing custody
guards without replacing other owners' proof arms. `git log --all --oneline -- '*0317*'` found only
unrelated resource-limit / asset history, no prior 0317 SQL migration.

## Dialogue at the pin

`data/npclib/npc_system/bank_system.lua:328-444` owns these handlers. Neither
`data/libs/functions/player.lua` nor `data/npclib/npc.lua` implements another exchange handler;
the latter supplies `MsgContains` at lines 13-17. The requested
`data/libs/functions/npc.lua` does not exist at this pin.

- `change gold`: `How many platinum coins would you like to get?`
- `change platinum`: `Would you like to change your platinum coins into {gold} or {crystal}?`
- Its `gold` branch: `How many platinum coins would you like to change into {gold}?`
- Its `crystal` branch: `How many crystal coins would you like to get?`
- `change crystal`: `How many crystal coins would you like to change into platinum?`

Confirmation strings, with N the parsed amount:

- Gold → platinum: `So you would like me to change {100*N} of your gold coins into {N} platinum coins?`
- Platinum → gold: `So you would like me to change {100*N} of your platinum coins into {N} gold coins for you?`
- Platinum → crystal: `So you would like me to change {100*N} of your platinum coins into {N} crystal coins for you?`
- Crystal → platinum: `So you would like me to change {100*N} of your crystal coins into {N} platinum coins for you?`

The two downgrade confirmation strings reverse the actual quantities in Canary. This candidate
preserves the text; the transaction uses N input coins and 100*N output coins for downgrades.
Success is `Here you are.` Cancellation is `Well, can I help you with something else?`
Invalid amounts and insufficient funds use `Sorry, you do not have enough gold coins.`,
`Sorry, you do not have enough platinum coins.`, or `Sorry, you do not have enough crystal coins.`

`getMoneyCount` (`data/libs/functions/functions.lua:137-144`) extracts the first ASCII digit run,
including from `1 platinum` or `-7`. `isValidMoney` at lines 306-308 permits 1 through 4294967295.
`MsgContains` matches case-insensitively with a non-alphanumeric boundary on each side; `yes please`
confirms, `yesterday` does not. A non-yes confirmation cancels. Oteryn additionally fences the
pending topic to the session, conversation/catalogue revision, NPC generation, node incarnation and
current item fence with a 30-second expiry, using the existing NPC confirmation convention.

Canary removes input and calls `addItem` without an explicit no-room dialogue/refund branch here.
Decision 27 requires stronger atomicity. Oteryn plans every consume, merge and new slot before
writing, checks carried weight/capacity, and uses the existing banker refusal text
`There is not enough room in your backpack.` for NoRoom. This sentence is an Oteryn banker message,
not an exact string from Canary's exchange handlers.

## Parity table

All Canary locations below are at `47dfd51f`.

| Mechanic | Canary file:lines @47dfd51f | Oteryn | Test |
|---|---|---|---|
| Gold → platinum, 100:1 | `data/npclib/npc_system/bank_system.lua:328-351` | `CoinExchange::GoldToPlatinum`; real banker dispatch | `coin_exchange_postgres_dispatch_rates_refusals_replay_and_custody`, round 0 |
| Platinum → gold, 1:100 | `data/npclib/npc_system/bank_system.lua:352-390` | Destination choice, quote, fenced carried conversion | Same, round 1 |
| Platinum → crystal, 100:1 | `data/npclib/npc_system/bank_system.lua:360-407` | Destination choice, quote, fenced carried conversion | Same, round 2 |
| Crystal → platinum, 1:100 | `data/npclib/npc_system/bank_system.lua:408-444` | Quote and fenced carried conversion | Same, round 3 |
| IDs 3031 / 3035 / 3043 | `src/utils/utils_definitions.hpp:608-610` | Existing `Coin` definition keys | Economy `banker_rates_match_both_reference_scripts`; four PG rounds |
| Amount parsing, bounds, word matching | `data/libs/functions/functions.lua:137-144,306-308`; `data/npclib/npc.lua:13-17` | First digit run, 1..=u32::MAX; bounded word matches | `coin_exchange_canary_amount_parser_and_prompts`; PG round 2 / 4 / 7 |
| Insufficient carried coins | `data/npclib/npc_system/bank_system.lua:342-348,380-386,397-403,435-441` | Exact denomination only, no bank/depot lookup | PG round 5; nonzero bank and depot snapshots throughout |
| Freed slots and existing stack merge | `data/npclib/npc_system/bank_system.lua:343-344,381-382,398-399,436-437` | Complete post-consumption placement plan across carried containers | PG rounds 0 / 4; economy `coin_exchange_whole_input_stack_and_existing_receiver_make_room` |
| No room changes nothing | Same `removeItem` / `addItem` calls; no atomic room refusal here | Decision 27 override; no item, bank or depot mutation | PG round 6, exact inventory snapshot |
| Zero and arithmetic overflow | `data/libs/functions/functions.lua:137-144,306-308` | Checked input/output multiplication; zero rejected | Economy `qualification_is_all_or_nothing_for_funds_overflow_and_room`, `coin_exchange_whole_input_stack_and_existing_receiver_make_room`; PG round 7 |
| Receipt and original-command replay | No durable command receipt in Lua handlers | Migration 0317; acceptance then atomic finalization; receipt before live dialogue/custody on resume | Every PG confirmation replay; injected finalization failure, stale-generation hold, nested input retirement, conversation retirement and migration reapplication |

## Scope and qualification

No client-visible command, server domain or wire code is added: existing domain 7 conversation
and the existing inventory-dirty projection are used. No client handler fragment is needed.
The PG case is automatically mounted from dispatch's `postgres_tests/`; it does not override
`SeamRooms`, so the construction inventory does not change.

Required independent persistence review and repository CI remain integration work. After the local
service restart, the exact `coin_exchange_postgres_dispatch` filter passed through the absolute cargo
wrapper, with POSTGRES_STARTED and POSTGRES_COMPLETED markers. Migration CASE parsing, SQL alias
ambiguity and JSON operator precedence were repaired; the fixture uses Paulie's authored Bank
service. No route-less early return is counted as qualification. See `COINX-FINAL.md` for results.
