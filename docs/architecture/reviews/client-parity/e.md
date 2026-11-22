# Client parity audit for analytics, trade and storage

| Module | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE | Operations |
| --- | --- | --- | --- | --- | --- |
| game_analyser | 11 | 5 | 37 | 0 | 53 |
| game_market | 13 | 4 | 5 | 1 | 23 |
| game_npctrade | 12 | 3 | 7 | 0 | 22 |
| game_playertrade | 4 | 1 | 1 | 0 | 6 |
| game_lootsplitter | 0 | 1 | 3 | 0 | 4 |
| game_stash | 8 | 0 | 7 | 0 | 15 |
| Total | 48 | 14 | 60 | 1 | 123 |

The comparison uses Oteryn `a4ad4164` and OTClient `4955a3fa22d5f5456ec6c658187fd2901545b750`. Each row counts a player outcome. A quantity editor, its shortcuts and its confirmation form one transaction. Independently selectable filters and display switches count separately. Evidence comes from source inspection across `apps/client/src` and `crates/client-*/src`, including the Bevy presenters, native entry paths and client UI state.

## game_analyser

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Review loot value and hourly return | PRESENT | `modules/game_analyser/analyser.otui:69` | `crates/client-ui/src/features/0010-analyser/view.rs:135` → `apps/client/src/features/0010-analyser/protocol.rs:42` |
| Review supply use and hourly cost | PRESENT | `modules/game_analyser/analyser.otui:85` | `crates/client-ui/src/features/0010-analyser/view.rs:205` → `apps/client/src/features/0010-analyser/protocol.rs:42` |
| Review received damage by type and source | PRESENT | `modules/game_analyser/analyser.otui:117` | `crates/client-ui/src/features/0010-analyser/view.rs:367` → `apps/client/src/features/0010-analyser/protocol.rs:42` |
| Review raw and adjusted XP gain, rates and level ETA | PRESENT | `modules/game_analyser/analyser.otui:133` | `crates/client-ui/src/features/0010-analyser/view.rs:98` → `apps/client/src/features/0010-analyser/protocol.rs:42` |
| Monitor tracked drops | PRESENT | `modules/game_analyser/analyser.otui:149` | `crates/client-ui/src/features/0010-analyser/view.rs:41` → `apps/client/src/features/0010-analyser/protocol.rs:42` |
| Review party members’ hunt contributions | PRESENT | `modules/game_analyser/analyser.otui:165` | `crates/client-ui/src/features/0010-analyser/state/party.rs:198` → `apps/client/src/features/0010-analyser/protocol.rs:22` |
| Check boss cooldowns | PRESENT | `modules/game_analyser/analyser.otui:181` | `apps/client/src/bevy_shell/game_ui/analyser/widgets.rs:183` → `crates/client-ui/src/features/0010-analyser/state/bosses.rs:72` |
| Review a consolidated hunt session | PARTIAL | `modules/game_analyser/analyser.otui:53` | The hunting view shows kills and kills/hour. XP, loot, supplies and combat measurements occupy separate reports in `crates/client-ui/src/features/0010-analyser/view.rs:17`. |
| Review outgoing damage and healing | PARTIAL | `modules/game_analyser/analyser.otui:101` | Current and session-average DPS/HPS and damage types appear in `crates/client-ui/src/features/0010-analyser/view.rs:332`. The report lacks all-time highs. |
| Start a new hunt session | PARTIAL | `modules/game_analyser/classes/HuntingAnalyser.lua:199` | Reset acts on the selected tool in `crates/client-ui/src/features/0010-analyser/protocol.rs:249`. Starting a reference hunt also resets the other analysers and, for the leader, the shared party interval. |
| Reset the selected analyser’s measurements | PRESENT | `modules/game_analyser/classes/ImpactAnalyser.lua:552` | `apps/client/src/bevy_shell/game_ui/analyser.rs:240` → `apps/client/src/features/0010-analyser/input.rs:30` |
| Remove a tracked item | PRESENT | `modules/game_analyser/classes/DropTrackerAnalyser.lua:488` | `apps/client/src/bevy_shell/game_ui/analyser/tracker.rs:42` → `apps/client/src/features/0010-analyser/input.rs:38` |
| Choose an item to track | PRESENT | `modules/game_analyser/classes/DropTrackerAnalyser.lua:132` | `apps/client/src/bevy_shell/game_ui/analyser/tracker.rs:42` → `apps/client/src/features/0010-analyser/input.rs:38` |
| Remove all tracked items | MISSING | `modules/game_analyser/classes/DropTrackerAnalyser.lua:492` | Searched for a clear-all control in tracker actions. |
| Copy the hunt report | MISSING | `modules/game_analyser/classes/HuntingAnalyser.lua:203` | Searched for clipboard output from the analyser. |
| Save the hunt report to a file | MISSING | `modules/game_analyser/classes/HuntingAnalyser.lua:204` | Searched for production callers of analyser_save_session and a destination picker. |
| Export the hunt report as JSON | MISSING | `modules/game_analyser/classes/HuntingAnalyser.lua:205` | Searched for a session JSON encoder and export action. |
| Reset all-time high | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:553` | Searched for a persistent peak and a reset-peak action. |
| Set a DPS target | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:562` | Searched for an editable damage-per-second goal. |
| Set an HPS target | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:574` | Searched for an editable healing-per-second goal. |
| Set a loot-per-hour target | MISSING | `modules/game_analyser/classes/LootAnalyser.lua:149` | Searched for a loot rate goal in configuration and measurements. |
| Set a supply-per-hour target | MISSING | `modules/game_analyser/classes/SupplyAnalyser.lua:240` | Searched for a cost rate goal in supply configuration. |
| Set an XP-per-hour target | MISSING | `modules/game_analyser/classes/XPAanalyser.lua:446` | Searched for an XP rate goal; Set XP goal takes remaining XP. |
| Reset the shared party session | MISSING | `modules/game_analyser/classes/PartyHuntAnalyser.lua:482` | Searched for a leader reset request alongside party refresh. |
| Switch the party valuation price source | MISSING | `modules/game_analyser/classes/PartyHuntAnalyser.lua:487` | Searched party configuration and requests for the leader-price versus market-price switch. Per-item value entry does not select a shared price source. |
| Send party report to Loot Splitter | MISSING | `modules/game_analyser/classes/PartyHuntAnalyser.lua:497` | Searched for analyser-to-splitter handoff. |
| Search boss cooldowns by name | MISSING | `modules/game_analyser/styles/boss.otui:120` | Searched for a boss name editor and cooldown list predicate. |
| Open the item Cyclopedia from the drop tracker | MISSING | `modules/game_analyser/styles/droptracker.otui:163` | Searched for a Cyclopedia action in the tracker controls. |
| Sort bosses by remaining cooldown | MISSING | `modules/game_analyser/classes/BossCooldown.lua:443` | Searched for a live analyser control or report field to sort bosses by remaining cooldown. |
| Sort bosses by name | MISSING | `modules/game_analyser/classes/BossCooldown.lua:447` | Searched for a live analyser control or report field to sort bosses by name. |
| Open Bosstiary from a cooldown row | MISSING | `modules/game_analyser/classes/BossCooldown.lua:297` | Searched for a live analyser control or report field to open bosstiary from a cooldown row. |
| Automatically track drops above a value threshold | MISSING | `modules/game_analyser/classes/DropTrackerAnalyser.lua:351` | Searched for a live analyser control or report field to automatically track drops above a value threshold. |
| Identify which monster dropped a tracked item | MISSING | `modules/game_analyser/classes/DropTrackerAnalyser.lua:235` | Searched for a live analyser control or report field to identify which monster dropped a tracked item. |
| Copy received-damage report | MISSING | `modules/game_analyser/classes/InputAnalyser.lua:564` | Searched for a live analyser control or report field to copy received-damage report. |
| Copy party report | MISSING | `modules/game_analyser/classes/PartyHuntAnalyser.lua:493` | Searched for a live analyser control or report field to copy party report. |
| Show or hide DPS gauge | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:563` | Searched for a visibility setting for DPS gauge in analyser widgets and report state. |
| Show or hide DPS graph | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:566` | Searched for a visibility setting for DPS graph in analyser widgets and report state. |
| Show or hide outgoing damage types | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:570` | Searched for a visibility setting for outgoing damage types in analyser widgets and report state. |
| Show or hide HPS gauge | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:575` | Searched for a visibility setting for HPS gauge in analyser widgets and report state. |
| Show or hide HPS graph | MISSING | `modules/game_analyser/classes/ImpactAnalyser.lua:578` | Searched for a visibility setting for HPS graph in analyser widgets and report state. |
| Show or hide incoming damage graph | MISSING | `modules/game_analyser/classes/InputAnalyser.lua:554` | Searched for a visibility setting for incoming damage graph in analyser widgets and report state. |
| Show or hide incoming damage types | MISSING | `modules/game_analyser/classes/InputAnalyser.lua:557` | Searched for a visibility setting for incoming damage types in analyser widgets and report state. |
| Show or hide incoming damage sources | MISSING | `modules/game_analyser/classes/InputAnalyser.lua:560` | Searched for a visibility setting for incoming damage sources in analyser widgets and report state. |
| Show or hide loot rate gauge | MISSING | `modules/game_analyser/classes/LootAnalyser.lua:150` | Searched for a visibility setting for loot rate gauge in analyser widgets and report state. |
| Show or hide loot rate graph | MISSING | `modules/game_analyser/classes/LootAnalyser.lua:151` | Searched for a visibility setting for loot rate graph in analyser widgets and report state. |
| Show or hide supply rate gauge | MISSING | `modules/game_analyser/classes/SupplyAnalyser.lua:241` | Searched for a visibility setting for supply rate gauge in analyser widgets and report state. |
| Show or hide supply rate graph | MISSING | `modules/game_analyser/classes/SupplyAnalyser.lua:242` | Searched for a visibility setting for supply rate graph in analyser widgets and report state. |
| Show or hide XP rate gauge | MISSING | `modules/game_analyser/classes/XPAanalyser.lua:447` | Searched for a visibility setting for XP rate gauge in analyser widgets and report state. |
| Show or hide XP rate graph | MISSING | `modules/game_analyser/classes/XPAanalyser.lua:448` | Searched for a visibility setting for XP rate graph in analyser widgets and report state. |
| Show or hide raw XP | MISSING | `modules/game_analyser/classes/XPAanalyser.lua:444` | Searched for a visibility setting for raw XP in analyser widgets and report state. |
| Switch between current and session outgoing rates | PARTIAL | `modules/game_analyser/classes/ImpactAnalyser.lua:557` | The report displays current and session-average rates together. It lacks the mode selector (`crates/client-ui/src/features/0010-analyser/view.rs:332`). |
| Switch between current and session received rates | PARTIAL | `modules/game_analyser/classes/InputAnalyser.lua:549` | The report displays current and session-average rates together. It lacks the mode selector (`crates/client-ui/src/features/0010-analyser/view.rs:367`). |
| Select an analyser from the HUD | PRESENT | `modules/game_analyser/analyser.otui:4` | `apps/client/src/bevy_shell/game_ui/hud.rs:208` → `apps/client/src/features/0010-analyser/input.rs:88` |

## game_market

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Create a buy or sell offer | PRESENT | `modules/game_market/market_helpers.lua:20` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:219` → `apps/client/src/features/0063-market/input.rs:104` |
| Cancel an active offer | PRESENT | `modules/game_market/market_helpers.lua:24` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:248` → `apps/client/src/features/0063-market/input.rs:104` |
| Accept a buy or sell offer for a chosen quantity | PRESENT | `modules/game_market/market_helpers.lua:16` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:275` → `apps/client/src/features/0063-market/input.rs:104` |
| Review current personal offers | PRESENT | `modules/game_market/t_market.otui:1571` | `apps/client/src/bevy_shell/game_ui/market/widgets.rs:292` → `apps/client/src/features/0063-market/input.rs:104` |
| Review personal offer history | PRESENT | `modules/game_market/t_market.otui:2127` | `apps/client/src/bevy_shell/game_ui/market/widgets.rs:294` → `apps/client/src/features/0063-market/input.rs:104` |
| Browse the selected item’s order book | PRESENT | `modules/game_market/market_helpers.lua:3` | `apps/client/src/bevy_shell/game_ui/market/actions.rs:110` → `apps/client/src/features/0063-market/protocol/details.rs:10` |
| Filter catalogue by category | PRESENT | `modules/game_market/t_market.lua:1146` | `apps/client/src/bevy_shell/game_ui/market/actions.rs:98` → `apps/client/src/features/0063-market/protocol/details.rs:10` |
| Search items by name | PRESENT | `modules/game_market/t_market.otui:620` | `apps/client/src/bevy_shell/game_ui/market/actions.rs:93` → `apps/client/src/features/0063-market/protocol/details.rs:10` |
| Filter by level | PARTIAL | `modules/game_market/t_market.otui:480` | The player can enter a level range in `apps/client/src/bevy_shell/game_ui/market/widgets/filters.rs:17`. The reference checkbox derives it from the current character automatically. |
| Filter by vocation | PARTIAL | `modules/game_market/t_market.otui:491` | The player can enter a vocation in `apps/client/src/bevy_shell/game_ui/market/widgets/filters.rs:17`. The reference checkbox derives it from the current character automatically. |
| Filter by forge tier | MISSING | `modules/game_market/t_market.otui:532` | Searched for a tier selector and tier field in the catalogue query. The forge field selects classification. |
| Review completed-trade statistics | PRESENT | `modules/game_market/t_market.otui:700` | `apps/client/src/bevy_shell/game_ui/market/actions.rs:150` → `apps/client/src/features/0063-market/protocol/details.rs:10` |
| Post anonymously | PRESENT | `modules/game_market/t_market.otui:1434` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:408` → `apps/client/src/features/0063-market/input.rs:78` |
| Preview fee, gross value and net proceeds | PRESENT | `modules/game_market/t_market.otui:1373` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:342` → `apps/client/src/bevy_shell/game_ui/market/offers.rs:331` |
| Close market | PRESENT | `modules/game_market/t_market.otui:1565` | `apps/client/src/bevy_shell/game_ui/market/actions.rs:65` → `apps/client/src/features/0063-market/input.rs:53` |
| Inspect catalogue item details | PARTIAL | `modules/game_market/t_market.otui:664` | The catalogue exposes requirements and name. Verified item artwork is unavailable (`apps/client/src/bevy_shell/game_ui/market/README.md:19`). |
| Filter one-handed weapons | MISSING | `modules/game_market/t_market.otui:502` | Searched for one-handed catalogue predicate. |
| Filter two-handed weapons | MISSING | `modules/game_market/t_market.otui:514` | Searched for two-handed catalogue predicate. |
| Show locker holdings only | MISSING | `modules/game_market/t_market.otui:541` | Searched for locker-only filter in the catalogue query. |
| Open the potion store from market | MISSING | `modules/game_market/t_market.otui:1469` | Searched for a potion store route in market actions. |
| Purchase store points | NOT-APPLICABLE | `modules/game_market/t_market.otui:1545` | Real-money currency checkout belongs to Platform. |
| Filter by forge classification | PRESENT | `modules/game_market/t_market.lua:1149` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:527` → `apps/client/src/features/0063-market/protocol/details.rs:10` |
| Check available market funds | PARTIAL | `modules/game_market/t_market.lua:89` | The coins panel exposes a coin balance (`crates/client-ui/src/features/0063-market/state/coins/presentation.rs:110`). The item market lacks the combined cash/bank total and breakdown. |

## game_npctrade

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Buy item | PRESENT | `modules/game_npctrade/controllers/npc_legacy_ui.lua:160` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:141` → `apps/client/src/features/0081-npc-shop/input.rs:16` |
| Sell item | PRESENT | `modules/game_npctrade/controllers/npc_legacy_ui.lua:163` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:141` → `apps/client/src/features/0081-npc-shop/input.rs:16` |
| Inspect shop item | PRESENT | `modules/game_npctrade/controllers/npc_legacy_ui.lua:181` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:143` → `apps/client/src/features/0081-npc-shop/view.rs:18` |
| Browse buy and sell lists | PRESENT | `modules/game_npctrade/templates/npctrade_legacy.otui:42` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:93` → `crates/client-ui/src/features/0081-npc-shop/state.rs:56` |
| Search goods by name | PRESENT | `modules/game_npctrade/templates/npctrade_legacy.otui:185` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:301` → `crates/client-ui/src/features/0081-npc-shop/state.rs:56` |
| Ignore capacity when ordering | PARTIAL | `modules/game_npctrade/templates/npctrade_legacy.otui:214` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:114` toggles a free-backpack-slot check in `crates/client-ui/src/features/0081-npc-shop/state/options.rs:30`. It does not implement the reference’s weight-based capacity override. |
| Toggle display of unowned goods | PRESENT | `modules/game_npctrade/templates/npctrade_legacy.otui:236` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:115` → `apps/client/src/features/0081-npc-shop/input.rs:77` |
| Sell all eligible goods | PARTIAL | `modules/game_npctrade/templates/npctrade_legacy.otui:248` | The action sells the selected quote in bulk. It requires one selected sell quote and does not traverse every eligible item (`apps/client/src/bevy_shell/game_ui/npc_shop.rs:145`). |
| Sort by name | MISSING | `modules/game_npctrade/controllers/npc_options.lua:4` | Searched for a player-selectable name sort. |
| Sort by price | MISSING | `modules/game_npctrade/controllers/npc_options.lua:7` | Searched for a quote-price comparator control. |
| Sort by weight | MISSING | `modules/game_npctrade/controllers/npc_options.lua:10` | Searched for a weight-order control. |
| Buy items packed in backpacks | MISSING | `modules/game_npctrade/templates/npctrade_legacy.otui:204` | Searched for a purchase packaging option; Open Backpack opens the existing bag. |
| Toggle exclusion of equipped items | MISSING | `modules/game_npctrade/templates/npctrade_legacy.otui:224` | Searched for an enabled ignore-equipped switch; npc_shop_options rejects false. |
| Close NPC trade | PRESENT | `modules/game_npctrade/templates/npctrade_legacy.otui:270` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:76` → `apps/client/src/features/0003-npc-service/view.rs:14` |
| Open Cyclopedia inspection for a shop item | MISSING | `modules/game_npctrade/controllers/npc_trader.lua:231` | Searched for a Cyclopedia inspection request bound to the quote; Look requests NPC shop details. |
| Check available money | PRESENT | `modules/game_npctrade/templates/npctrade_legacy.otui:126` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:437` → `apps/client/src/features/0081-npc-shop/protocol.rs:58` |
| Preview the price for the selected quantity | PRESENT | `modules/game_npctrade/templates/npctrade_legacy.otui:115` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:420` → `crates/client-ui/src/features/0081-npc-shop/state.rs:118` |
| Check weight against available capacity | PARTIAL | `modules/game_npctrade/templates/npctrade_legacy.otui:150` | The shop validates backpack slots in `crates/client-ui/src/features/0081-npc-shop/state.rs:136`. The reference displays item weight and remaining carrying capacity. |
| Send a typed NPC message | PRESENT | `modules/game_npctrade/controllers/npc_dialog.lua:181` | `apps/client/src/bevy_shell/game_ui/npc_shop/conversation.rs:86` → `apps/client/src/features/0003-npc-service/input.rs:29` |
| Reply with a dialogue keyword | PRESENT | `modules/game_npctrade/controllers/npc_dialog.lua:81` | `apps/client/src/bevy_shell/game_ui/npc_shop/conversation.rs:87` → `apps/client/src/features/0003-npc-service/input.rs:33` |
| Read the NPC conversation transcript | PRESENT | `modules/game_npctrade/controllers/npc_dialog.lua:101` | `apps/client/src/bevy_shell/game_ui/npc_shop/conversation.rs:145` → `apps/client/src/features/0003-npc-service/view.rs:3` |
| Toggle chat input mode from NPC dialogue | MISSING | `modules/game_npctrade/controllers/npc_dialog.lua:238` | Searched for a dialogue control bound to the global chat-mode toggle. |

## game_playertrade

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Accept trade | PRESENT | `modules/game_playertrade/tradewindow.otui:98` | `apps/client/src/bevy_shell/game_ui/trade.rs:36` → `apps/client/src/features/0060-player-trade/presentation.rs:178` |
| Reject trade | PRESENT | `modules/game_playertrade/playertrade.lua:30` | `apps/client/src/bevy_shell/game_ui/trade.rs:37` → `apps/client/src/features/0060-player-trade/presentation.rs:178` |
| Identify an offered item visually | PARTIAL | `modules/game_playertrade/playertrade.lua:59` | `apps/client/src/bevy_shell/game_ui/trade/widgets.rs:346` renders a question mark and stack count. The offered item has no usable appearance in this slot. |
| Inspect the selected entry on either side | PRESENT | `modules/game_playertrade/playertrade.lua:64` | `apps/client/src/bevy_shell/game_ui/trade.rs:38` → `apps/client/src/features/0060-player-trade/presentation.rs:146` |
| Review both sides’ offered contents and counts | PRESENT | `modules/game_playertrade/playertrade.lua:57` | `apps/client/src/bevy_shell/game_ui/trade/widgets.rs:395` → `apps/client/src/features/0060-player-trade/protocol.rs:22` |
| Verify the counterparty’s name in the trade panel | MISSING | `modules/game_playertrade/playertrade.lua:55` | Searched for a participant-name caption in trade widgets and status projection. The panel labels the two sides Your offer and Their offer. |

## game_lootsplitter

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Calculate transfers from a pasted hunt log | PARTIAL | `modules/game_lootsplitter/lootsplitter.lua:207` | The party view computes and displays transfers for confirmed measurements in `crates/client-ui/src/features/0010-analyser/state/party.rs:243`. It lacks a paste field and parser for the reference session log. |
| Copy settlement instructions | MISSING | `modules/game_lootsplitter/lootsplitter.lua:201` | Searched for clipboard writes for calculated member transfers. |
| Exclude a member from the split | MISSING | `modules/game_lootsplitter/advanced.otui:27` | Searched for a live member inclusion control; analyser/input.rs:127 gates the adjustment dispatcher on tests. |
| Adjust member supply expenses | MISSING | `modules/game_lootsplitter/lootsplitter.lua:257` | Searched for a live per-member cost editor in party controls. |

## game_stash

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Withdraw chosen amount | PRESENT | `modules/game_stash/game_stash.lua:474` | `apps/client/src/bevy_shell/game_ui/stash.rs:119` → `apps/client/src/features/0062-stash/input.rs:104` |
| Search stash items | PRESENT | `modules/game_stash/game_stash.otui:35` | `apps/client/src/bevy_shell/game_ui/stash.rs:191` → `apps/client/src/features/0062-stash/input.rs:75` |
| Filter by item category | PRESENT | `modules/game_stash/game_stash.lua:609` | `apps/client/src/bevy_shell/game_ui/stash.rs:192` → `apps/client/src/features/0062-stash/input.rs:75` |
| Filter by trader | PRESENT | `modules/game_stash/game_stash.lua:558` | `apps/client/src/bevy_shell/game_ui/stash.rs:193` → `apps/client/src/features/0062-stash/input.rs:75` |
| Close supply stash | PRESENT | `modules/game_stash/game_stash.otui:131` | `apps/client/src/bevy_shell/game_ui/stash.rs:84` → `apps/client/src/features/0062-stash/input.rs:99` |
| Inspect stash item in Cyclopedia | MISSING | `modules/game_stash/game_stash.lua:269` | Searched for a stash row action carrying the definition into Cyclopedia. |
| Show stash item in market | MISSING | `modules/game_stash/game_stash.lua:288` | Searched for a stash-to-market book query. |
| Add stash item to loot list | MISSING | `modules/game_stash/game_stash.lua:303` | Searched for a stash row action updating the loot allowlist. |
| Remove stash item from loot list | MISSING | `modules/game_stash/game_stash.lua:307` | Searched for a stash row action deleting a loot rule. |
| Manage destination containers from stash | MISSING | `modules/game_stash/game_stash.otui:117` | Searched for a container-assignment route from the stash presenter. |
| Sort by name ascending | PRESENT | `modules/game_stash/game_stash.lua:59` | `apps/client/src/bevy_shell/game_ui/stash.rs:131` → `apps/client/src/features/0062-stash/input.rs:22` |
| Sort by name descending | MISSING | `modules/game_stash/game_stash.lua:60` | Searched for a reverse-direction flag; stash requests encode only name or stock order. |
| Sort by quantity descending | PRESENT | `modules/game_stash/game_stash.lua:65` | `apps/client/src/bevy_shell/game_ui/stash.rs:131` → `apps/client/src/features/0062-stash/input.rs:22` |
| Sort by quantity ascending | MISSING | `modules/game_stash/game_stash.lua:69` | Searched for a reverse-direction flag; stash requests encode only name or stock order. |
| Browse stored items and quantities | PRESENT | `modules/game_stash/game_stash.otui:3` | `apps/client/src/bevy_shell/game_ui/stash.rs:278` → `apps/client/src/features/0062-stash/input.rs:141` |

## Gaps ranked by player impact

### 1. Trade verification and party settlement

- Reset the shared party session (missing)
- Switch the party valuation price source (missing)
- Send party report to Loot Splitter (missing)
- Copy party report (missing)
- Identify an offered item visually (partial)
- Verify the counterparty’s name in the trade panel (missing)
- Calculate transfers from a pasted hunt log (partial)
- Copy settlement instructions (missing)
- Exclude a member from the split (missing)
- Adjust member supply expenses (missing)

### 2. Trading, purchasing and inventory workflows

- Filter by level (partial)
- Filter by vocation (partial)
- Filter by forge tier (missing)
- Inspect catalogue item details (partial)
- Filter one-handed weapons (missing)
- Filter two-handed weapons (missing)
- Show locker holdings only (missing)
- Open the potion store from market (missing)
- Check available market funds (partial)
- Ignore capacity when ordering (partial)
- Sell all eligible goods (partial)
- Sort by name (missing)
- Sort by price (missing)
- Sort by weight (missing)
- Buy items packed in backpacks (missing)
- Toggle exclusion of equipped items (missing)
- Open Cyclopedia inspection for a shop item (missing)
- Check weight against available capacity (partial)
- Toggle chat input mode from NPC dialogue (missing)
- Inspect stash item in Cyclopedia (missing)
- Show stash item in market (missing)
- Add stash item to loot list (missing)
- Remove stash item from loot list (missing)
- Manage destination containers from stash (missing)
- Sort by name descending (missing)
- Sort by quantity ascending (missing)

### 3. Hunt records, goals and navigation

- Review a consolidated hunt session (partial)
- Review outgoing damage and healing (partial)
- Start a new hunt session (partial)
- Remove all tracked items (missing)
- Copy the hunt report (missing)
- Save the hunt report to a file (missing)
- Export the hunt report as JSON (missing)
- Reset all-time high (missing)
- Set a DPS target (missing)
- Set an HPS target (missing)
- Set a loot-per-hour target (missing)
- Set a supply-per-hour target (missing)
- Set an XP-per-hour target (missing)
- Search boss cooldowns by name (missing)
- Open the item Cyclopedia from the drop tracker (missing)
- Open Bosstiary from a cooldown row (missing)
- Automatically track drops above a value threshold (missing)
- Identify which monster dropped a tracked item (missing)
- Copy received-damage report (missing)

### 4. Report display preferences

- Sort bosses by remaining cooldown (missing)
- Sort bosses by name (missing)
- Show or hide DPS gauge (missing)
- Show or hide DPS graph (missing)
- Show or hide outgoing damage types (missing)
- Show or hide HPS gauge (missing)
- Show or hide HPS graph (missing)
- Show or hide incoming damage graph (missing)
- Show or hide incoming damage types (missing)
- Show or hide incoming damage sources (missing)
- Show or hide loot rate gauge (missing)
- Show or hide loot rate graph (missing)
- Show or hide supply rate gauge (missing)
- Show or hide supply rate graph (missing)
- Show or hide XP rate gauge (missing)
- Show or hide XP rate graph (missing)
- Show or hide raw XP (missing)
- Switch between current and session outgoing rates (partial)
- Switch between current and session received rates (partial)
