# Client parity by player operation

Oteryn `a4ad4164` and OTClient `4955a3fa22d5f5456ec6c658187fd2901545b750`. Counts cover the fourteen modules below. A claim, purchase or edit includes its confirmation and cancellation controls. Local operations cite their state owner. Search scope includes the application client, native entry points, Bevy shell and client crates.

| Module | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE | Operations |
|---|---:|---:|---:|---:|---:|
| game_rewardwall | 4 | 0 | 5 | 0 | 9 |
| game_store | 9 | 0 | 3 | 0 | 12 |
| game_taskboard | 18 | 3 | 8 | 0 | 29 |
| game_spelllist | 5 | 1 | 0 | 0 | 6 |
| game_viplist | 13 | 2 | 2 | 0 | 17 |
| game_blessing | 2 | 1 | 1 | 0 | 4 |
| game_bugreport | 1 | 0 | 0 | 0 | 1 |
| game_ruleviolation | 0 | 1 | 0 | 0 | 1 |
| game_unjustifiedpoints | 3 | 0 | 0 | 0 | 3 |
| game_outfit | 9 | 1 | 11 | 10 | 31 |
| game_playermount | 1 | 0 | 0 | 0 | 1 |
| game_playerdeath | 2 | 1 | 0 | 1 | 4 |
| game_minimap | 9 | 0 | 1 | 0 | 10 |
| game_compendium | 1 | 3 | 2 | 1 | 7 |
| **Total** | 77 | 13 | 33 | 12 | 135 |

## game_rewardwall

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Inspect the reward cycle, tier and streak | PRESENT | `modules/game_rewardwall/game_rewardwall.lua:656` | `apps/client/src/bevy_shell/game_ui/reward_wall/widgets.rs:65` → `apps/client/src/features/0182-daily-reward/input.rs:54` |
| Claim the available shrine reward | PRESENT | `modules/game_rewardwall/game_rewardwall.lua:119` | `apps/client/src/bevy_shell/game_ui/reward_wall.rs:164` → `apps/client/src/features/0182-daily-reward/input.rs:14` |
| Read claim history | PRESENT | `modules/game_rewardwall/game_rewardwall.lua:1305` | `apps/client/src/bevy_shell/game_ui/reward_wall/widgets.rs:109` → `apps/client/src/features/0182-daily-reward/input.rs:64` |
| Inspect resting-area bonuses | PRESENT | `modules/game_rewardwall/game_rewardwall.lua:135` | `crates/client-ui/src/features/0277-rest-status/view.rs:32` → `apps/client/src/features/0277-rest-status/protocol.rs:31` |
| Choose reward items and quantities | MISSING | `modules/game_rewardwall/game_rewardwall.lua:1685` | Reward picker and quantity search led to reward_wall/widgets.rs:225. Each day grants fixed items. |
| Claim remotely with an instant-access token | MISSING | `modules/game_rewardwall/game_rewardwall.lua:98` | Instant access and token search led to reward_wall/widgets.rs:281. Collection requires a nearby shrine. |
| Open premium offers from the wall | MISSING | `modules/game_rewardwall/game_rewardwall.lua:1315` | Searched reward wall actions for a Store or premium-offer route. |
| Open instant reward offers from the wall | MISSING | `modules/game_rewardwall/game_rewardwall.lua:1320` | Searched reward wall actions for a joker or instant-reward purchase route. |
| Check time remaining before the reward resets | MISSING | `modules/game_rewardwall/game_rewardwall.lua:316` | Searched reward wall widgets, deadline and reset countdown. Today’s collection state is visible without a ticking deadline. |

## game_store

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Browse categories and their offers | PRESENT | `modules/game_store/game_store.lua:733` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:103` → `apps/client/src/features/0154-store/input.rs:45` |
| Inspect a product and its description | PRESENT | `modules/game_store/game_store.lua:975` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:116` → `apps/client/src/features/0154-store/input.rs:27` |
| Search offers | PRESENT | `modules/game_store/game_store.lua:1282` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:130` → `apps/client/src/features/0154-store/input.rs:31` |
| Buy an offer and confirm the purchase | PRESENT | `modules/game_store/game_store.lua:975` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:93` → `apps/client/src/features/0154-store/input.rs:115` |
| Get coins through checkout | PRESENT | `modules/game_store/game_store.lua:939` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:79` → `apps/client/src/features/0154-store/coin_shop.rs:23` |
| Transfer coins to a named recipient | PRESENT | `modules/game_store/game_store.lua:1204` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:150` → `apps/client/src/features/0154-store/input/gift.rs:10` |
| Browse transaction history by page | PRESENT | `modules/game_store/game_store.lua:962` | `apps/client/src/bevy_shell/game_ui/store/actions.rs:172` → `apps/client/src/features/0154-store/input.rs:98` |
| Check coin balance | PRESENT | `modules/game_store/game_store.lua:543` | `apps/client/src/bevy_shell/game_ui/store/widgets.rs:94` → `apps/client/src/bevy_shell/game_ui/store/widgets.rs:91` |
| Browse recently added offers from Store home | MISSING | `modules/game_store/game_store.lua:1145` | Searched Store browser for recently added, featured and home offer IDs. |
| Cycle through promotional banners | MISSING | `modules/game_store/game_store.lua:1154` | Searched Store home, banner and carousel controls. |
| Supply a character name for a name-change purchase | MISSING | `modules/game_store/game_store.lua:1175` | Searched Store checkout and purchase payload for new character name. |
| Name a purchased hireling | PRESENT | `modules/game_store/game_store.lua:1192` | `apps/client/src/bevy_shell/game_ui/store/widgets.rs:351` → `apps/client/src/features/0154-store/input.rs:126` |

## game_taskboard

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Select a bounty creature | PRESENT | `modules/game_taskboard/controller/tab_bounty.lua:158` | `apps/client/src/bevy_shell/game_ui/task_board.rs:86` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Claim a completed bounty | PRESENT | `modules/game_taskboard/controller/tab_bounty.lua:163` | `apps/client/src/bevy_shell/game_ui/task_board.rs:88` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Change bounty difficulty | PRESENT | `modules/game_taskboard/controller/tab_bounty.lua:187` | `apps/client/src/bevy_shell/game_ui/task_board.rs:90` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Reroll bounty choices | PRESENT | `modules/game_taskboard/controller/tab_bounty.lua:130` | `apps/client/src/bevy_shell/game_ui/task_board.rs:98` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Upgrade a talisman attribute | PRESENT | `modules/game_taskboard/controller/tab_bounty.lua:225` | `apps/client/src/bevy_shell/game_ui/task_board.rs:129` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Claim the daily bounty reroll token | PARTIAL | `modules/game_taskboard/controller/tab_bounty.lua:196` | The reward wall grants bounty tokens (reward_wall/widgets.rs:292). The board lacks the independent daily claim and cap confirmation. |
| Start weekly tasks at selected difficulty | PRESENT | `modules/game_taskboard/controller/tab_weekly.lua:236` | `apps/client/src/bevy_shell/game_ui/task_board.rs:115` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Deliver weekly task items | PRESENT | `modules/game_taskboard/controller/tab_weekly.lua:247` | `apps/client/src/bevy_shell/game_ui/task_board.rs:112` → `apps/client/src/features/0072-task-board/input.rs:31` |
| Look up a delivery item in Cyclopedia | MISSING | `modules/game_taskboard/controller/tab_weekly.lua:303` | Searched weekly delivery rows and item context actions for a destination carrying the selected item. |
| Open market offers for a delivery item | MISSING | `modules/game_taskboard/controller/tab_weekly.lua:319` | Searched weekly delivery rows and item context actions for a destination carrying the selected item. |
| Buy a task-shop reward | PARTIAL | `modules/game_taskboard/controller/tab_shop.lua:42` | task_board.rs:120 sends BuyShop through features/0072-task-board/input.rs:31. The row spends points without the reference price confirmation. |
| Assign a preferred monster | PRESENT | `modules/game_taskboard/controller/modal_preferred.lua:537` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:156` → `apps/client/src/features/0072-task-board/input.rs:136` |
| Assign an unwanted monster | PRESENT | `modules/game_taskboard/controller/modal_preferred.lua:556` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:159` → `apps/client/src/features/0072-task-board/input.rs:136` |
| Clear a preferred monster | PRESENT | `modules/game_taskboard/controller/modal_preferred.lua:575` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:84` → `apps/client/src/features/0072-task-board/input.rs:136` |
| Clear an unwanted monster | PRESENT | `modules/game_taskboard/controller/modal_preferred.lua:609` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:84` → `apps/client/src/features/0072-task-board/input.rs:136` |
| Unlock another preference slot | PRESENT | `modules/game_taskboard/controller/modal_preferred.lua:643` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:163` → `apps/client/src/features/0072-task-board/input.rs:136` |
| Search monster preferences by name | MISSING | `modules/game_taskboard/controller/modal_preferred.lua:491` | Searched preference entries, creature choices and text filters. The board enumerates available choices. |
| Browse Soulseal fights and costs | PRESENT | `modules/game_taskboard/controller/modal_soulseal.lua:390` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:29` → `apps/client/src/features/0072-task-board/input/soulseals.rs:24` |
| Start a Soulseal fight | PRESENT | `modules/game_taskboard/controller/modal_soulseal.lua:627` | `apps/client/src/bevy_shell/game_ui/task_board/extras.rs:165` → `apps/client/src/features/0072-task-board/input/soulseals.rs:45` |
| Search Soulseals by name | MISSING | `modules/game_taskboard/controller/modal_soulseal.lua:606` | Searched Soulseal fight rows, catalogue query and paging controls for name or category filters. |
| Filter Soulseals by category | MISSING | `modules/game_taskboard/controller/modal_soulseal.lua:616` | Searched Soulseal fight rows, catalogue query and paging controls for name or category filters. |
| Open task-related Store offers | MISSING | `modules/game_taskboard/tasks.lua:304` | Searched task-board actions for a useful-things or Store redirect. |
| Track bounty kills | PRESENT | `modules/game_taskboard/trackers/miniwindows_tracker.lua:319` | `apps/client/src/bevy_shell/game_ui/task_tracker.rs:149` → `apps/client/src/bevy_shell/game_ui/task_tracker.rs:47` |
| Track weekly progress | PRESENT | `modules/game_taskboard/trackers/miniwindows_tracker.lua:470` | `apps/client/src/bevy_shell/game_ui/task_tracker.rs:149` → `apps/client/src/bevy_shell/game_ui/task_tracker.rs:47` |
| Track prey duration | PRESENT | `modules/game_taskboard/trackers/miniwindows_tracker.lua:237` | `apps/client/src/bevy_shell/game_ui/task_tracker.rs:149` → `apps/client/src/bevy_shell/game_ui/task_tracker.rs:47` |
| Open bounty board from tracker | PRESENT | `modules/game_taskboard/trackers/miniwindows_tracker.lua:296` | `apps/client/src/bevy_shell/game_ui/task_tracker.rs:226` → `apps/client/src/features/0072-task-board/view.rs:14` |
| Open weekly tasks from tracker | PARTIAL | `modules/game_taskboard/trackers/miniwindows_tracker.lua:414` | task_tracker.rs:226 opens the task board without selecting weekly tasks. |
| Open prey from tracker | MISSING | `modules/game_taskboard/trackers/miniwindows_tracker.lua:219` | Searched tracker click actions for a prey-panel route. The single action opens the task board. |
| Show or hide the kill tracker independently | MISSING | `modules/game_taskboard/trackers/miniwindows_tracker.lua:132` | Searched tracker visibility, toolbar shortcuts and sidebar controls. task_tracker.rs:221 attaches the tracker directly to the sidebar. |

## game_spelllist

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Browse spells and inspect a spell’s formula, requirements and details | PRESENT | `modules/game_spelllist/spelllist.lua:286` | `apps/client/src/bevy_shell/game_ui/journal/model.rs:86` → `apps/client/src/features/0287-actor-spell-catalog/book.rs:65` |
| Hide spells above current level | PRESENT | `modules/game_spelllist/spelllist.lua:374` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:255` → `apps/client/src/features/0287-actor-spell-catalog/book.rs:35` |
| Filter by selected vocation | PRESENT | `modules/game_spelllist/spelllist.lua:374` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:267` → `apps/client/src/features/0287-actor-spell-catalog/book.rs:35` |
| Filter by premium requirement | PRESENT | `modules/game_spelllist/spelllist.lua:374` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:269` → `apps/client/src/features/0287-actor-spell-catalog/book.rs:35` |
| Filter by spell group | PRESENT | `modules/game_spelllist/spelllist.lua:374` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:271` → `apps/client/src/features/0287-actor-spell-catalog/book.rs:35` |
| Restrict spells to the current character’s vocation | PARTIAL | `modules/game_spelllist/spelllist.lua:340` | journal/actions.rs:267 cycles vocations manually. The selector lacks an automatic current-vocation filter. |

## game_viplist

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Add contact | PRESENT | `modules/game_viplist/viplist.lua:233` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:307` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Edit contact description, icon and login notification | PRESENT | `modules/game_viplist/viplist.lua:329` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:299` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Remove contact | PRESENT | `modules/game_viplist/viplist.lua:408` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:148` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Assign contact to groups | PRESENT | `modules/game_viplist/viplist.lua:329` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:176` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Create a group | PRESENT | `modules/game_viplist/viplist.lua:969` | `apps/client/src/bevy_shell/game_ui/hud/vip/groups.rs:215` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Rename a group | PRESENT | `modules/game_viplist/viplist.lua:989` | `apps/client/src/bevy_shell/game_ui/hud/vip/groups.rs:216` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Remove a group | PRESENT | `modules/game_viplist/viplist.lua:792` | `apps/client/src/bevy_shell/game_ui/hud/vip/groups.rs:200` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Sort contacts by name | PRESENT | `modules/game_viplist/viplist.lua:672` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:50` → `apps/client/src/bevy_shell/game_ui/hud/vip/presentation.rs:247` |
| Sort contacts by icon type | PRESENT | `modules/game_viplist/viplist.lua:678` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:50` → `apps/client/src/bevy_shell/game_ui/hud/vip/presentation.rs:249` |
| Sort contacts by status | PRESENT | `modules/game_viplist/viplist.lua:684` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:50` → `apps/client/src/bevy_shell/game_ui/hud/vip/presentation.rs:248` |
| Hide or show offline contacts | PRESENT | `modules/game_viplist/viplist.lua:421` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:51` → `apps/client/src/bevy_shell/game_ui/hud/vip/presentation.rs:238` |
| Read contact presence | PRESENT | `modules/game_viplist/viplist.lua:627` | `apps/client/src/bevy_shell/game_ui/hud/vip/presentation.rs:225` → `apps/client/src/features/0258-vip-presence/protocol.rs:10` |
| Message a contact privately | PRESENT | `modules/game_viplist/viplist.lua:765` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:56` → `apps/client/src/features/0007-chat/console.rs:95` |
| Invite contact to own private chat | PARTIAL | `modules/game_viplist/viplist.lua:776` | Private-channel management accepts a name in apps/client/src/features/0080-chat-private-channel/input.rs:12. VIP context actions lack the selected-contact shortcut. |
| Exclude contact from own private chat | PARTIAL | `modules/game_viplist/viplist.lua:779` | Private-channel management accepts a name in apps/client/src/features/0080-chat-private-channel/input.rs:12. VIP context actions lack the selected-contact shortcut. |
| Toggle grouped contact display | MISSING | `modules/game_viplist/viplist.lua:1017` | Searched VIP presentation, group management and menu actions for a grouped/flat switch. |
| Copy a contact name | MISSING | `modules/game_viplist/viplist.lua:838` | Searched VIP context menu, clipboard and copy-name actions. |

## game_blessing

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Inspect blessing holdings and Store copies | PARTIAL | `modules/game_blessing/blessing.lua:109` | hud/blessings.rs:170 displays authoritative holdings. The holdings list has no separate per-blessing Store copy count. |
| Inspect death-loss protection | PRESENT | `modules/game_blessing/blessing.lua:120` | `apps/client/src/bevy_shell/game_ui/hud/blessings.rs:170` → `crates/client-ui/src/features/0199-cyclopedia-character/protocol/blessings.rs:59` |
| Read dated blessing history | PRESENT | `modules/game_blessing/blessing.lua:57` | `apps/client/src/bevy_shell/game_ui/hud/blessings.rs:170` → `crates/client-ui/src/features/0199-cyclopedia-character/protocol/blessings.rs:63` |
| Open premium offers from blessings | MISSING | `modules/game_blessing/blessing.lua:150` | Searched blessing HUD and Cyclopedia blessing actions for a premium Store link. |

## game_bugreport

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Compose and send a bug report | PRESENT | `modules/game_bugreport/bugreport.lua:29` | `apps/client/src/bevy_shell/game_ui/bug_report.rs:182` → `apps/client/src/features/0076-player-report/bug.rs:36` |

## game_ruleviolation

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Submit a rule violation for a named character | PARTIAL | `modules/game_ruleviolation/ruleviolation.lua:124` | rule_violation.rs:373 and features/0076-player-report/input.rs:102 submit player reports. Reference reasons and sanction choices come from GM permissions. The player form lacks sanction and IP-banishment selection. |

## game_unjustifiedpoints

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Inspect unjustified kill counts across penalty windows | PRESENT | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:278` | `apps/client/src/bevy_shell/game_ui/unjustified_points.rs:94` → `apps/client/src/features/0134-party-skull/pvp_status.rs:17` |
| Check remaining skull time | PRESENT | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:182` | `apps/client/src/bevy_shell/game_ui/unjustified_points.rs:74` → `apps/client/src/features/0134-party-skull/pvp_status.rs:35` |
| Inspect open PvP situations | PRESENT | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:200` | `apps/client/src/bevy_shell/game_ui/unjustified_points.rs:247` → `apps/client/src/features/0134-party-skull/pvp_status.rs:21` |

## game_outfit

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Choose an outfit | PRESENT | `modules/game_outfit/outfit.lua:1275` | `apps/client/src/bevy_shell/game_ui/outfit.rs:64` → `apps/client/src/features/0025-appearance/input.rs:42` |
| Select outfit addons | PRESENT | `modules/game_outfit/outfit.lua:1534` | `apps/client/src/bevy_shell/game_ui/outfit.rs:63` → `apps/client/src/features/0025-appearance/input.rs:42` |
| Set colours for each body part | PRESENT | `modules/game_outfit/outfit.lua:1566` | `apps/client/src/bevy_shell/game_ui/outfit.rs:89` → `apps/client/src/features/0025-appearance/input.rs:42` |
| Choose a mount appearance | PRESENT | `modules/game_outfit/outfit.lua:1293` | `apps/client/src/bevy_shell/game_ui/outfit.rs:77` → `apps/client/src/features/0025-appearance/input.rs:42` |
| Apply appearance changes | PRESENT | `modules/game_outfit/outfit.lua:1853` | `apps/client/src/bevy_shell/game_ui/outfit.rs:185` → `apps/client/src/features/0025-appearance/input.rs:45` |
| Search the appearance catalogue | PRESENT | `modules/game_outfit/outfit.lua:1740` | `apps/client/src/bevy_shell/game_ui/outfit.rs:310` → `apps/client/src/bevy_shell/game_ui/outfit/search.rs:10` |
| Rotate the preview | PRESENT | `modules/game_outfit/outfit.lua:1694` | `apps/client/src/bevy_shell/game_ui/outfit.rs:455` → `apps/client/src/bevy_shell/game_ui/outfit/animation.rs:33` |
| Preview walking animation | PRESENT | `modules/game_outfit/outfit.lua:173` | `apps/client/src/bevy_shell/game_ui/outfit.rs:457` → `apps/client/src/bevy_shell/game_ui/outfit/animation.rs:35` |
| Create a named appearance preset | MISSING | `modules/game_outfit/outfit.lua:602` | Searched appearance state, gallery and preferences for saved outfit presets. |
| Delete a preset | MISSING | `modules/game_outfit/outfit.lua:639` | Searched appearance state, gallery and preferences for saved outfit presets. |
| Update a saved preset | MISSING | `modules/game_outfit/outfit.lua:675` | Searched appearance state, gallery and preferences for saved outfit presets. |
| Rename a preset | MISSING | `modules/game_outfit/outfit.lua:740` | Searched appearance state, gallery and preferences for saved outfit presets. |
| Apply a saved preset | MISSING | `modules/game_outfit/outfit.lua:1203` | Searched appearance state, gallery and preferences for saved outfit presets. |
| Select familiar appearance | MISSING | `modules/game_outfit/outfit.lua:1311` | Searched appearance catalogue and change payload for a familiar look. |
| Filter gallery by ownership | PARTIAL | `modules/game_outfit/outfit.lua:1726` | outfit.rs:310 filters the server-owned catalogue by name. There is no switch between owned and all looks. |
| Toggle preview floor | MISSING | `modules/game_outfit/outfit.lua:191` | Searched appearance playback controls and preview renderer for this visibility switch. |
| Toggle mount preview independently | MISSING | `modules/game_outfit/outfit.lua:244` | Searched appearance playback controls and preview renderer for this visibility switch. |
| Toggle familiar preview | MISSING | `modules/game_outfit/outfit.lua:249` | Searched appearance playback controls and preview renderer for this visibility switch. |
| Toggle character visibility in preview | MISSING | `modules/game_outfit/outfit.lua:254` | Searched appearance playback controls and preview renderer for this visibility switch. |
| Equip a cosmetic aura | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:1346` | OTClient custom appearance layers extend the official outfit, addon, colour and mount selection. |
| Equip cosmetic wings | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:1372` | OTClient custom appearance layers extend the official outfit, addon, colour and mount selection. |
| Choose an outfit shader | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:1399` | OTClient custom appearance layers extend the official outfit, addon, colour and mount selection. |
| Select a custom health bar | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:1432` | OTClient custom appearance layers extend the official outfit, addon, colour and mount selection. |
| Select an attached cosmetic effect | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:1450` | OTClient custom appearance layers extend the official outfit, addon, colour and mount selection. |
| Select a displayed title | PRESENT | `modules/game_outfit/outfit.lua:1477` | `apps/client/src/features/0097-character-info/view.rs:170` → `apps/client/src/features/0097-character-info/view.rs:183` |
| Toggle aura preview | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:266` | Visibility control for an OTClient-specific cosmetic layer. |
| Toggle wing preview | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:271` | Visibility control for an OTClient-specific cosmetic layer. |
| Toggle shader preview | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:276` | Visibility control for an OTClient-specific cosmetic layer. |
| Toggle custom bar preview | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:286` | Visibility control for an OTClient-specific cosmetic layer. |
| Toggle attached effect preview | NOT-APPLICABLE | `modules/game_outfit/outfit.lua:291` | Visibility control for an OTClient-specific cosmetic layer. |
| Toggle title preview | MISSING | `modules/game_outfit/outfit.lua:281` | Searched outfit preview controls for title visibility. |

## game_playermount

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Mount or dismount | PRESENT | `modules/game_playermount/playermount.lua:40` | `apps/client/src/bevy_shell/game_ui/outfit.rs:177` → `apps/client/src/features/0502-actor-motion/protocol.rs:90` |

## game_playerdeath

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Continue playing after death | PRESENT | `modules/game_playerdeath/playerdeath.lua:81` | `apps/client/src/bevy_shell/game_ui/feature_dialogs/death_controls.rs:80` → `apps/client/src/features/local-death-respawn/protocol.rs:103` |
| Leave the session after death | PRESENT | `modules/game_playerdeath/playerdeath.lua:85` | `apps/client/src/bevy_shell/game_ui/feature_dialogs/death_controls.rs:78` → `apps/client/src/features/0021-logout/input.rs:5` |
| Read the death type and penalty | PARTIAL | `modules/game_playerdeath/playerdeath.lua:56` | crates/client-ui/src/features/0382-death-protection/view.rs:20 identifies consumed blessings. The notice omits the unfair-fight percentage. |
| Automatically reconnect after death | NOT-APPLICABLE | `modules/game_playerdeath/playerdeath.lua:96` | OTClient autoReconnect preference schedules a new login after two seconds. |

## game_minimap

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Change map floor | PRESENT | `modules/game_minimap/minimap.lua:205` | `apps/client/src/bevy_shell/game_ui/minimap.rs:293` → `apps/client/src/features/local-minimap/input.rs:146` |
| Zoom the map | PRESENT | `modules/game_minimap/minimap.lua:154` | `apps/client/src/bevy_shell/game_ui/minimap.rs:292` → `apps/client/src/features/local-minimap/input.rs:144` |
| Pan the map | PRESENT | `modules/game_minimap/minimap.lua:225` | `apps/client/src/bevy_shell/game_ui/minimap/mouse.rs:63` → `apps/client/src/features/local-minimap/input.rs:130` |
| Recenter the map on the player | PRESENT | `modules/game_minimap/minimap.lua:245` | `apps/client/src/bevy_shell/game_ui/minimap.rs:242` → `apps/client/src/features/local-minimap/input.rs:140` |
| Open the large map overview | PRESENT | `modules/game_minimap/minimap.lua:162` | `apps/client/src/bevy_shell/game_ui/minimap.rs:368` → `apps/client/src/bevy_shell/ui/toolbar/shortcuts.rs:10` |
| Walk to a map position | PRESENT | `modules/gamelib/ui/uiminimap.lua:294` | `apps/client/src/bevy_shell/game_ui/minimap.rs:160` → `apps/client/src/features/local-minimap/input.rs:6` |
| Set or edit a map marker | PRESENT | `modules/gamelib/ui/uiminimap.lua:300` | `apps/client/src/bevy_shell/game_ui/minimap/markers.rs:268` → `apps/client/src/features/local-minimap/input.rs:191` |
| Remove a map marker | PRESENT | `modules/gamelib/ui/uiminimap.lua:116` | `apps/client/src/bevy_shell/game_ui/minimap/markers.rs:211` → `apps/client/src/features/local-minimap/input.rs:176` |
| Check world time on the minimap | MISSING | `modules/game_minimap/minimap.lua:46` | Searched minimap projection and controls for world time, clock and day/night indicator. |
| Show or hide the minimap | PRESENT | `modules/game_minimap/minimap.lua:284` | `apps/client/src/bevy_shell/ui/toolbar.rs:36` → `apps/client/src/bevy_shell/game_ui/minimap.rs:406` |

## game_compendium

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Browse the player guide | PRESENT | `modules/game_compendium/game_compendium.lua:15` | `apps/client/src/bevy_shell/game_ui/feature_dialogs/guide.rs:66` → `apps/client/src/bevy_shell/game_ui/feature_dialogs/guide.rs:49` |
| Browse major update articles | MISSING | `modules/game_compendium/game_compendium.lua:14` | Searched guide topics, update history and news categories. The guide enumerates five topics at feature_dialogs/guide.rs:5. |
| Browse client feature articles | PARTIAL | `modules/game_compendium/game_compendium.lua:16` | Guide topics cover controls and help at feature_dialogs/guide.rs:5. There is no corresponding article index. |
| Browse useful information articles | PARTIAL | `modules/game_compendium/game_compendium.lua:17` | Guide topics cover controls and help at feature_dialogs/guide.rs:5. There is no corresponding article index. |
| Browse support articles | PARTIAL | `modules/game_compendium/game_compendium.lua:18` | Guide topics cover controls and help at feature_dialogs/guide.rs:5. There is no corresponding article index. |
| Read the OTClient Redemption project page | NOT-APPLICABLE | `modules/game_compendium/game_compendium.lua:304` | Project-specific OTClient news. |
| Follow a link from a compendium article | MISSING | `modules/game_compendium/game_compendium.lua:653` | Searched the guide content renderer and activation handlers for article links. |

## Gaps ranked by player impact

The first group affects rewards or purchases. The second affects decisions and access during play. The third affects customization and discovery.

### Reward and purchase outcomes

- Choose reward items and quantities — MISSING
- Claim remotely with an instant-access token — MISSING
- Supply a character name for a name-change purchase — MISSING
- Claim the daily bounty reroll token — PARTIAL
- Buy a task-shop reward — PARTIAL

### Decisions and access during play

- Open premium offers from the wall — MISSING
- Open instant reward offers from the wall — MISSING
- Check time remaining before the reward resets — MISSING
- Browse recently added offers from Store home — MISSING
- Look up a delivery item in Cyclopedia — MISSING
- Open market offers for a delivery item — MISSING
- Search monster preferences by name — MISSING
- Search Soulseals by name — MISSING
- Filter Soulseals by category — MISSING
- Open task-related Store offers — MISSING
- Open weekly tasks from tracker — PARTIAL
- Open prey from tracker — MISSING
- Restrict spells to the current character’s vocation — PARTIAL
- Invite contact to own private chat — PARTIAL
- Exclude contact from own private chat — PARTIAL
- Toggle grouped contact display — MISSING
- Inspect blessing holdings and Store copies — PARTIAL
- Open premium offers from blessings — MISSING
- Submit a rule violation for a named character — PARTIAL
- Select familiar appearance — MISSING
- Read the death type and penalty — PARTIAL

### Customization and discovery

- Cycle through promotional banners — MISSING
- Show or hide the kill tracker independently — MISSING
- Copy a contact name — MISSING
- Create a named appearance preset — MISSING
- Delete a preset — MISSING
- Update a saved preset — MISSING
- Rename a preset — MISSING
- Apply a saved preset — MISSING
- Filter gallery by ownership — PARTIAL
- Toggle preview floor — MISSING
- Toggle mount preview independently — MISSING
- Toggle familiar preview — MISSING
- Toggle character visibility in preview — MISSING
- Toggle title preview — MISSING
- Check world time on the minimap — MISSING
- Browse major update articles — MISSING
- Browse client feature articles — PARTIAL
- Browse useful information articles — PARTIAL
- Browse support articles — PARTIAL
- Follow a link from a compendium article — MISSING
