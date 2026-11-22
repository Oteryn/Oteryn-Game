# Official client settings parity

Oteryn `a4ad4164` and OTClient `4955a3fa22d5`. The official page observations in `apps/client/SETTINGS.md:66` and `apps/client/REFERENCE-AUDIT.md:11` set the scope. The preference-field crosswalk in the first-pass review supplies candidate editors. The unit is a distinct player operation. Repeated Basic shortcuts are counted on their detailed page. A single selector, position or size can cover several saved keys. Contextual preferences are grouped with the nearest settings page. The catalogue also contains `emergencyDialogVersion`, a version marker without an identified player control.

All source paths are repository-relative. A dash in the OTClient column means no matching control was identified in the pinned reference. The official inventory still applies. For standard Bevy controls the widget dispatches a typed edit through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` and Apply publishes the draft at `apps/client/src/bevy_shell/owner/options_draft.rs:80`. Evidence below is source-level.

| Page | Operations | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE |
|---|---:|---:|---:|---:|---:|
| Basic Options | 5 | 5 | 0 | 0 | 0 |
| Controls | 7 | 3 | 1 | 3 | 0 |
| General Hotkeys | 11 | 10 | 0 | 1 | 0 |
| Action Bar Hotkeys | 2 | 1 | 1 | 0 | 0 |
| Custom Hotkeys | 5 | 5 | 0 | 0 | 0 |
| Interface | 11 | 1 | 2 | 8 | 0 |
| HUD | 25 | 15 | 3 | 7 | 0 |
| Console | 13 | 2 | 10 | 1 | 0 |
| Game Window | 22 | 8 | 1 | 13 | 0 |
| Action Bars | 32 | 25 | 6 | 1 | 0 |
| Shortcuts | 3 | 0 | 0 | 3 | 0 |
| Graphics | 13 | 6 | 2 | 5 | 0 |
| Effects | 8 | 7 | 1 | 0 | 0 |
| Sound | 9 | 8 | 0 | 1 | 0 |
| Battle Sounds | 16 | 10 | 0 | 6 | 0 |
| UI Sounds | 15 | 3 | 9 | 3 | 0 |
| Miscellaneous | 15 | 5 | 0 | 7 | 3 |
| Gameplay | 10 | 2 | 0 | 8 | 0 |
| Screenshots | 23 | 7 | 2 | 14 | 0 |
| Help | 6 | 6 | 0 | 0 | 0 |
| **Total** | 251 | 129 | 38 | 81 | 3 |

## Basic Options

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Show advanced options (`optionsShowAdvanced`) | PRESENT | `modules/client_options/options.otui:254` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:184` `apps/client/src/bevy_shell/game_ui/options/projection.rs:81` |
| Apply edits | PRESENT | `modules/client_options/options.otui:271` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:192` → `apps/client/src/bevy_shell/game_ui/options.rs:181` |
| Apply edits and close | PRESENT | `modules/client_options/options.otui:260` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:192` → `apps/client/src/bevy_shell/game_ui/options.rs:182` |
| Cancel edits | PRESENT | `modules/client_options/options.otui:281` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:192` → `apps/client/src/bevy_shell/game_ui/options.rs:183` |
| Reset page controls | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:146` → `apps/client/src/bevy_shell/game_ui/options.rs:167` |

## Controls

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Always face the direction of movement (`alwaysTurnTowardsMoveDirection`) | MISSING | — | Searched for always turn towards move direction in option editors, contextual controls and the owning feature. No player control found. |
| Choose mouse control scheme (`controlSchemeIndex`) | PARTIAL | `modules/client_options/styles/controls/general.otui:43` | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:6` Right/left handed presets only. No official interaction-scheme selector. |
| Move the entire stack by default when dragging (`dragAndDropDefaultActionIsMoveAll`) | MISSING | `modules/client_options/styles/controls/general.otui:110` | Searched for drag and drop default action is move all in option editors, contextual controls and the owning feature. No player control found. |
| Keyboard Delay (ms) (`keyboardDelayMs`) | PRESENT | `modules/client_options/styles/controls/general.otui:156` | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:64` → `apps/client/src/features/local-reference-preferences/settings_actions.rs:367` |
| Use the default keyboard delay (`keyboardDelayUseDefault`) | MISSING | — | Searched for keyboard delay use default in option editors, contextual controls and the owning feature. No player control found. |
| Select rotation modifier (`rotateWithAlt`, `rotateWithCtrl`, `rotateWithShift`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:39` `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:49` |
| Set movement keys | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:25` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |

## General Hotkeys

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Select chat-on or chat-off binding context (`chatModeOn`) | PRESENT | `modules/client_options/styles/controls/keybinds.otui:154` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:64` `apps/client/src/bevy_shell/game_ui/options/profiles.rs:111` |
| Choose hotkey profile | PRESENT | `modules/client_options/styles/controls/keybinds.otui:92` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:16` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:107` |
| Add profile | PRESENT | `modules/client_options/styles/controls/keybinds.otui:98` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:16` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:108` |
| Rename profile | PRESENT | `modules/client_options/styles/controls/keybinds.otui:110` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:16` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:109` |
| Remove profile | PRESENT | `modules/client_options/styles/controls/keybinds.otui:116` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:16` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:110` |
| Copy current bindings into a named profile | PRESENT | `modules/client_options/styles/controls/keybinds.otui:104` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:108` adds a named snapshot by cloning the selected preset in `apps/client/src/features/local-reference-preferences/profile_contexts.rs:156`. |
| Search actions by name | MISSING | `modules/client_options/styles/controls/keybinds.otui:187` | Searched the binding editor for search, filter and query input. `apps/client/src/bevy_shell/game_ui/options/keys.rs:68` lists all actions. Its text field edits the chord. |
| Assign primary key | PRESENT | `modules/client_options/styles/controls/keybinds.otui:237` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:91` → `apps/client/src/bevy_shell/game_ui/options/keys.rs:164` |
| Assign secondary key | PRESENT | `modules/client_options/styles/controls/keybinds.otui:243` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:91` → `apps/client/src/bevy_shell/game_ui/options/keys.rs:170` |
| Clear secondary key | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/keys.rs:91` → `apps/client/src/bevy_shell/game_ui/options/keys.rs:173` |
| Restore default binding | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/keys.rs:91` → `apps/client/src/bevy_shell/game_ui/options/keys.rs:176` |

## Action Bar Hotkeys

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Assign a key to an action-bar slot | PRESENT | `modules/client_options/styles/controls/keybinds.otui:237` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:91` → `apps/client/src/bevy_shell/game_ui/options/keys.rs:164` |
| Assign secondary action-bar key | PARTIAL | `modules/client_options/styles/controls/keybinds.otui:243` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:87` excludes extra action-bar slots from secondary bindings. |

## Custom Hotkeys

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Assign text to a chord | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:91` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:41` |
| Send bound text automatically | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:109` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:360` |
| Assign an object and target mode | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:126` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:45` |
| Capture chord | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:220` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:314` |
| Remove custom binding | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:63` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:361` |

## Interface

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Confirm before copying a link (`linkCopyWarningEnabled`) | MISSING | — | Searched for link copy warning enabled in option editors, contextual controls and the owning feature. No player control found. |
| Animate the mouse cursor (`mouseAnimatedCursor`) | MISSING | `modules/client_options/styles/interface/interface.otui:36` | Searched for mouse animated cursor in option editors, contextual controls and the owning feature. No player control found. |
| Use a large mouse cursor (`mouseBigCursor`) | MISSING | — | Searched for mouse big cursor in option editors, contextual controls and the owning feature. No player control found. |
| Highlight the object under the pointer (`mouseHighlightTarget`) | MISSING | `modules/client_options/styles/interface/interface.otui:12` | Searched for mouse highlight target in option editors, contextual controls and the owning feature. No player control found. |
| Use the operating system cursor (`mouseSystemCursor`) | MISSING | `modules/client_options/styles/interface/interface.otui:24` | Searched for mouse system cursor in option editors, contextual controls and the owning feature. No player control found. |
| Show item expiry in containers (`showExpireInContainers`) | MISSING | `modules/client_options/styles/interface/interface.otui:170` | Searched for show expire in containers in option editors, contextual controls and the owning feature. No player control found. |
| Show item expiry in inventory (`showExpireInInventory`) | MISSING | `modules/client_options/styles/interface/interface.otui:163` | Searched for show expire in inventory in option editors, contextual controls and the owning feature. No player control found. |
| Show expiry for unused items (`showExpireWhenUnused`) | MISSING | `modules/client_options/styles/interface/interface.otui:177` | Searched for show expire when unused in option editors, contextual controls and the owning feature. No player control found. |
| Set interface scale | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:14` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |
| Resize NPC dialog (`height`, `width`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/npc_shop/widgets.rs:37` registers shop geometry with `apps/client/src/bevy_shell/ui/window_layout/persistence.rs:152`. No separate saved conversation-dialog geometry found. |
| Move NPC dialog (`x`, `y`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/npc_shop/widgets.rs:37` registers shop geometry with `apps/client/src/bevy_shell/ui/window_layout/persistence.rs:152`. No separate saved conversation-dialog geometry found. |

## HUD

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Show NPC Icons (`creatueShowNpcIcon`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:42` → `apps/client/src/features/0023-presentation/actor_hud.rs:50` |
| Toggle all other-creature HUD elements (`creatureHudEnabled`) | MISSING | — | Searched for creature hud enabled in option editors, contextual controls and the owning feature. No player control found. |
| Show other creatures’ health bars (`creatureShowHealth`) | PARTIAL | `modules/client_options/styles/interface/HUD.otui:34` | `apps/client/src/features/0023-presentation/view/creature_bars.rs:55` draws actor bars. No independent other-creature health switch. |
| Show Other Creatures' Marks (`creatureShowMarks`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:38` → `apps/client/src/features/0023-presentation/actor_hud.rs:49` |
| Show Creature Names (`creatureShowName`) | PRESENT | `modules/client_options/styles/interface/HUD.otui:23` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:35` → `apps/client/src/features/local-world-view/view.rs:174` |
| Arc Distance (`playerHudArcDistance`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:57` → `apps/client/src/features/local-reference-preferences/arcs.rs:42` |
| Set HUD arc height (`playerHudArcHeight`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:64` One size value scales both axes. No independent dimension. |
| Arc Opacity (%) (`playerHudArcOpacity`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:66` → `apps/client/src/features/local-reference-preferences/arcs.rs:42` |
| Set HUD arc width (`playerHudArcWidth`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:64` One size value scales both axes. No independent dimension. |
| Toggle the player HUD (`playerHudEnabled`) | MISSING | — | Searched for player hud enabled in option editors, contextual controls and the owning feature. No player control found. |
| Show HUD arcs (`playerHudShowArcs`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:49` Health and mana arc switches at lines 49 and 55 edit arcs.health and arcs.mana. |
| Show player HUD bars (`playerHudShowBars`) | MISSING | — | Searched for player hud show bars in option editors, contextual controls and the owning feature. No player control found. |
| Place the harmony indicator on the left (`playerHudShowHarmonyLeft`) | MISSING | — | Searched for player hud show harmony left in option editors, contextual controls and the owning feature. No player control found. |
| Show Serene Effects (`playerShowHarmonySerene`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:103` → `apps/client/src/features/0023-presentation/resources.rs:27` |
| Show Health (`playerShowHealth`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:27` → `apps/client/src/features/0023-presentation/resources.rs:24` |
| Show Mana (`playerShowMana`) | PRESENT | `modules/client_options/styles/interface/HUD.otui:45` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:28` → `apps/client/src/features/0023-presentation/resources.rs:25` |
| Show Own Marks (`playerShowMarks`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:36` → `apps/client/src/features/0023-presentation/actor_hud.rs:48` |
| Show Own Name (`playerShowName`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:29` → `apps/client/src/features/local-world-view/view.rs:174` |
| Show status bars (`statusBarEnabled`) | MISSING | `modules/client_options/styles/interface/HUD.otui:265` | Searched for status bar enabled in option editors, contextual controls and the owning feature. No player control found. |
| Show Status Panel (`statusPanelEnabled`) | PRESENT | `modules/client_options/styles/interface/HUD.otui:254` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:44` → `apps/client/src/bevy_shell/game_ui/projection.rs:197` |
| Set per-condition visibility in bar (`visibleInBar`) | PRESENT | `modules/client_options/styles/interface/HUD.otui:182` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:73` updates the hidden set at `apps/client/src/bevy_shell/game_ui/options/conditions.rs:110`. |
| Set per-condition visibility in HUD (`visibleInHUD`) | PRESENT | `modules/client_options/styles/interface/HUD.otui:159` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:73` updates the hidden set at `apps/client/src/bevy_shell/game_ui/options/conditions.rs:110`. |
| Change condition display order (`specialConditionsOrder`) | PRESENT | `modules/client_options/styles/interface/HUD.otui:222` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:27` `apps/client/src/bevy_shell/game_ui/options/conditions.rs:115` |
| Toggle all conditions in bar (`showInBar`) | MISSING | `modules/client_options/styles/interface/HUD.otui:187` | Searched condition matrix and master visibility. `apps/client/src/bevy_shell/game_ui/options/conditions.rs:55` builds per-condition toggles only. |
| Toggle all conditions in HUD (`showInHUD`) | MISSING | `modules/client_options/styles/interface/HUD.otui:166` | Searched condition matrix and master visibility. `apps/client/src/bevy_shell/game_ui/options/conditions.rs:55` builds per-condition toggles only. |

## Console

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Show events in console (`consoleShowEventMessages`) | PARTIAL | `modules/client_options/styles/interface/console.otui:23` | `crates/client-ui/src/components/console_filters.rs:78` filters notices by class. No live editor for events independently. |
| Show information in console (`consoleShowInfoMessages`) | PARTIAL | `modules/client_options/styles/interface/console.otui:12` | `crates/client-ui/src/components/console_filters.rs:78` filters notices by class. No live editor for information independently. |
| Show join and leave messages (`consoleShowJoinLeaveMessages`) | MISSING | — | Searched for console show join leave messages in option editors, contextual controls and the owning feature. No player control found. |
| Show speaker levels (`consoleShowLevels`) | PARTIAL | `modules/client_options/styles/interface/console.otui:67` | `crates/client-ui/src/components/console_display.rs:24` Formatter supports this field. No player-facing switch in Console or chat menus. |
| Show other status in console (`consoleShowStatusMessagesOfOthers`) | PARTIAL | `modules/client_options/styles/interface/console.otui:45` | `crates/client-ui/src/components/console_filters.rs:78` filters notices by class. No live editor for other status independently. |
| Show own status in console (`consoleShowStatusMessagesOwn`) | PARTIAL | `modules/client_options/styles/interface/console.otui:34` | `crates/client-ui/src/components/console_filters.rs:78` filters notices by class. No live editor for own status independently. |
| Show timestamps (`consoleShowTimestamps`) | PARTIAL | `modules/client_options/styles/interface/console.otui:56` | `crates/client-ui/src/components/console_display.rs:24` Formatter supports this field. No player-facing switch in Console or chat menus. |
| Show timestamp seconds (`consoleShowTimestampsSeconds`) | PARTIAL | — | `crates/client-ui/src/components/console_display.rs:24` Formatter supports this field. No player-facing switch in Console or chat menus. |
| Open incoming private messages in a new tab (`openNewPrivateMessagesInNewTab`) | PARTIAL | — | `apps/client/src/features/0007-chat/console.rs:95` opens a conversation manually. No player switch for automatic opening. |
| Keep the loot channel open (`lootChannelOpen`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/chat/widgets/transcript.rs:80` recognizes the open Loot tab. No persistent loot-channel-open preference found. |
| Open and close chat channels (`openChannels`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/chat/widgets.rs:426` Channel dialog and close-current handlers exist. No saved open-channel set found. |
| Set console text size | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:125` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |
| Change chat input mode | PRESENT | — | `apps/client/src/bevy_shell/game_ui/chat/widgets.rs:404` → `apps/client/src/features/local-reference-preferences/chat_context.rs:70` |

## Game Window

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Choose how loot values are colored (`colorizeLootStyle`) | MISSING | `modules/client_options/styles/interface/interface.otui:134` | Searched for colorize loot style in option editors, contextual controls and the owning feature. No player control found. |
| Show combat frames (`combatShowFrames`) | MISSING | — | Searched for combat show frames in option editors, contextual controls and the owning feature. No player control found. |
| Show PvP Frames (`combatShowPvpFrames`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:168` → `apps/client/src/features/0023-presentation/actor_hud.rs:79` |
| Resize the lower console pane (`gameWindowLowerPaneHeight`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/layout.rs:367` updates height and `apps/client/src/bevy_shell/game_ui/layout.rs:187` stores it. |
| Choose viewport resizing mode (`gameWindowResizingMode`) | MISSING | — | Searched for game window resizing mode in option editors, contextual controls and the owning feature. No player control found. |
| Show attack animation in the game window (`gameWindowShowAttackAnimation`) | MISSING | — | Searched for game window show attack animation in option editors, contextual controls and the owning feature. No player control found. |
| Show boosted creature messages in the game window (`gameWindowShowBoostedCreatureMessages`) | MISSING | — | Searched for game window show boosted creature messages in option editors, contextual controls and the owning feature. No player control found. |
| Show hotkey usage messages in the game window (`gameWindowShowHotkeyUsageMessages`) | MISSING | — | Searched for game window show hotkey usage messages in option editors, contextual controls and the owning feature. No player control found. |
| Show Banner (`gameWindowShowInfoBanner`) | PRESENT | `modules/client_options/styles/interface/interface.otui:100` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:174` → `apps/client/src/features/0092-zones-nologout/view.rs:23` |
| Highlight Loot (`gameWindowShowLootHighlighting`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:170` → `apps/client/src/features/0023-presentation/actor_hud.rs:80` |
| Show Loot Messages (`gameWindowShowLootMessages`) | PRESENT | `modules/client_options/styles/interface/console.otui:100` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:159` → `apps/client/src/features/0098-quick-loot/view.rs:6` |
| Show messages in the game window (`gameWindowShowMessages`) | MISSING | — | Searched for game window show messages in option editors, contextual controls and the owning feature. No player control found. |
| Show offline training messages in the game window (`gameWindowShowOfflineTrainingMessages`) | MISSING | — | Searched for game window show offline training messages in option editors, contextual controls and the owning feature. No player control found. |
| Show other players’ spells (`gameWindowShowOthersSpells`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:149` The other-spells filter also affects creatures. |
| Show Own Spells (`gameWindowShowOwnSpells`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:144` → `apps/client/src/features/local-reference-preferences/game_window.rs:36` |
| Show potion messages in the game window (`gameWindowShowPotionMessages`) | MISSING | — | Searched for game window show potion messages in option editors, contextual controls and the owning feature. No player control found. |
| Show private messages in the game window (`gameWindowShowPrivateMessages`) | MISSING | `modules/client_options/styles/interface/console.otui:89` | Searched for game window show private messages in option editors, contextual controls and the owning feature. No player control found. |
| Show store messages in the game window (`gameWindowShowStoreMessages`) | MISSING | — | Searched for game window show store messages in option editors, contextual controls and the owning feature. No player control found. |
| Show target highlight in the game window (`gameWindowShowTargetHighlight`) | MISSING | — | Searched for game window show target highlight in option editors, contextual controls and the owning feature. No player control found. |
| Show Value Text (`gameWindowShowTextualEffects`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:154` → `apps/client/src/features/local-reference-preferences/game_window.rs:39` |
| Show Target Frame (`gameWindowShowTragetFrame`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:164` → `apps/client/src/features/local-world-view/view.rs:173` |
| Resize the upper pane (`gameWindowUpperPaneHeight`) | MISSING | — | Searched for game window upper pane height in option editors, contextual controls and the owning feature. No player control found. |

## Action Bars

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Lock all bottom bars (`actionBarBottomLocked`) | PARTIAL | `modules/client_options/data_options.lua:1137` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:76` Locks individual rows. No side-wide lock control. |
| Lock all left bars (`actionBarLeftLocked`) | PARTIAL | `modules/client_options/data_options.lua:1138` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:76` Locks individual rows. No side-wide lock control. |
| Lock all right bars (`actionBarRightLocked`) | PARTIAL | `modules/client_options/data_options.lua:1139` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:76` Locks individual rows. No side-wide lock control. |
| Show Bottom Action Bar 1 (`actionBarShowBottom1`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:36` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:241` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Bottom Action Bar 2 (`actionBarShowBottom2`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:45` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:246` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Bottom Action Bar 3 (`actionBarShowBottom3`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:53` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:251` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Left Action Bar 1 (`actionBarShowLeft1`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:76` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:257` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Left Action Bar 2 (`actionBarShowLeft2`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:84` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:262` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Left Action Bar 3 (`actionBarShowLeft3`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:91` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:267` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Right Action Bar 1 (`actionBarShowRight1`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:113` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:279` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Right Action Bar 2 (`actionBarShowRight2`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:121` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:284` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Right Action Bar 3 (`actionBarShowRight3`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:129` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:289` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show All Bottom Action Bars (`actionBarsShowBottom`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:28` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:234` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show All Left Action Bars (`actionBarsShowLeft`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:69` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:255` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show All Right Action Bars (`actionBarsShowRight`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:106` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:272` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:125` |
| Show Action Tooltips (`actionButtonAllowTooltip`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:204` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:224` → `apps/client/src/bevy_shell/game_ui/actions/projection.rs:215` |
| Automatically Insert New Actions (`actionButtonAutoInsertSpells`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:229` → `apps/client/src/features/local-spell-action-bar/view.rs:133` |
| Show Item Amounts (`actionButtonShowAmount`) | PRESENT | `modules/client_options/styles/interface/actionbars.otui:156` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:214` → `apps/client/src/features/local-spell-action-bar/view.rs:169` |
| Show numeric cooldowns (`actionButtonShowCooldownNumbers`) | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:192` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:219` Shared cooldown switch lacks independent format selection. |
| Show graphical cooldowns (`actionButtonShowGraphicalCooldown`) | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:180` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:219` Shared cooldown switch lacks independent format selection. |
| Show assigned hotkeys on action buttons (`actionButtonShowHotkey`) | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:144` | `apps/client/src/bevy_shell/game_ui/actions/projection.rs:188` renders the assigned chord without a visibility switch. |
| Show assigned spell parameters (`actionButtonShowSpellParameters`) | MISSING | `modules/client_options/styles/interface/actionbars.otui:168` | Searched for action button show spell parameters in option editors, contextual controls and the owning feature. No player control found. |
| Show Cooldown Bar (`cooldownBarEnabled`) | PRESENT | `modules/client_options/styles/interface/interface.otui:88` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:204` → `apps/client/src/features/local-spell-action-bar/view.rs:232` |
| Clear bottom row 1 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:241` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear bottom row 2 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:251` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear bottom row 3 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:261` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear left row 1 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:279` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear left row 2 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:289` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear left row 3 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:299` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear right row 1 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:317` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear right row 2 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:327` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |
| Clear right row 3 | PRESENT | `modules/client_options/styles/interface/actionbars.otui:337` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:16` → `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` |

## Shortcuts

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Add sidebar shortcut (`disabledButtons`, `enabledButtons`) | MISSING | — | Searched edit_shortcut, shortcut_choices and live options navigation. `apps/client/src/features/local-reference-preferences/shortcuts.rs:27` has an edit method without a player-facing editor. |
| Remove sidebar shortcut | MISSING | — | Searched edit_shortcut, shortcut_choices and live options navigation. `apps/client/src/features/local-reference-preferences/shortcuts.rs:27` has an edit method without a player-facing editor. |
| Reorder sidebar shortcuts | MISSING | — | Searched edit_shortcut, shortcut_choices and live options navigation. `apps/client/src/features/local-reference-preferences/shortcuts.rs:27` has an edit method without a player-facing editor. |

## Graphics

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Select anti-aliasing mode (`antialiasingMode`) | MISSING | `modules/client_options/styles/graphics/graphics.otui:6` | Searched antialiasing in graphics settings and the renderer. `apps/client/src/bevy_shell/ui.rs:432` identifies the host limitation. |
| Frame Rate Limit (0 = unlimited) (`frameRateLimit`) | PRESENT | `modules/client_options/styles/graphics/graphics.otui:53` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:63` → `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:72` |
| Toggle the foreground frame-rate limit (`frameRateLimitEnabled`) | PARTIAL | `modules/client_options/styles/graphics/graphics.otui:53` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:63` Zero disables the limit. Re-enabling requires setting the rate again. |
| Scale by even multiples (`gameWindowScaleOnlyByEvenMultiples`) | PARTIAL | `modules/client_options/styles/graphics/graphics.otui:113` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:26` Integral scaling also admits odd factors. |
| Choose the rendering engine (`rendererIndex`) | MISSING | — | Searched engine selection in graphics settings and the renderer. `apps/client/src/bevy_shell/ui.rs:432` identifies the host limitation. |
| Show FPS and lag (`showFpsLagIndicator`) | PRESENT | `modules/client_options/styles/graphics/graphics.otui:48` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:83` FPS and command latency have separate switches at lines 83 and 92. Both use the pacing state written at line 6. |
| Use the FreeType text renderer (`useFreetypeRenderer`) | MISSING | — | Searched for use freetype renderer in option editors, contextual controls and the owning feature. No player control found. |
| Vertical Synchronisation (V-Sync) (`vSyncEnabled`) | PRESENT | `modules/client_options/styles/graphics/graphics.otui:41` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:21` → `apps/client/src/features/local-reference-preferences/graphics.rs:34` |
| Resize the main window (`clientWindowHeight`, `clientWindowWidth`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/layout.rs:191` saves desktop bounds and `apps/client/src/bevy_shell/game_ui/layout.rs:65` restores them on launch. |
| Use borderless fullscreen (`clientWindowIsBorderless`) | PRESENT | `modules/client_options/styles/graphics/graphics.otui:29` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:16` → `apps/client/src/bevy_shell/mod.rs:272` |
| Remember maximized window state (`clientWindowIsMaximized`) | MISSING | — | Searched for client window is maximized in option editors, contextual controls and the owning feature. No player control found. |
| Move the main window (`clientWindowPositionX`, `clientWindowPositionY`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/layout.rs:191` saves desktop bounds and `apps/client/src/bevy_shell/game_ui/layout.rs:65` restores them on launch. |
| Select a monitor (`clientWindowScreenName`) | MISSING | — | Searched for client window screen name in option editors, contextual controls and the owning feature. No player control found. |

## Effects

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Minimum Ambient Brightness (%) (`lightAmbientLevel`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:17` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:39` → `apps/client/src/features/local-reference-preferences/graphics.rs:80` |
| Attenuate clouds and indoor light (`lightAttenuationCloudsIndoor`) | PARTIAL | — | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:47` Indoor dimming has a slider. Cloud attenuation lacks an editor. |
| Show Lights (`lightEffectsEnabled`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:14` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:101` → `apps/client/src/features/0019-world-light/view.rs:11` |
| Lower-Floor Dimming (%) (`lightLevelSeparatorLevel`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:27` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:55` → `apps/client/src/features/0019-world-light/view.rs:26` |
| Boss Area Opacity (%) (`monsterBossAreaEffectsOpacity`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:181` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:74` → `apps/client/src/features/0023-presentation/view/floors.rs:27` |
| Creature Effects Opacity (%) (`monsterEffectsOpacity`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:170` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:121` → `apps/client/src/features/0023-presentation/view/floors.rs:27` |
| Other Players' Effects Opacity (%) (`otherPlayersEffectsOpacity`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:160` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:113` → `apps/client/src/features/0023-presentation/view/floors.rs:27` |
| Own Effects Opacity (%) (`ownEffectsOpacity`) | PRESENT | `modules/client_options/styles/graphics/effects.otui:150` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:105` → `apps/client/src/features/0023-presentation/view/floors.rs:27` |

## Sound

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Ambient Volume (%) (`soundAmbienceVolume`) | PRESENT | `modules/client_options/styles/sound/sound.otui:74` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:20` → `apps/client/src/features/0023-presentation/state.rs:40` |
| Play the anthem (`soundAnthemEnabled`) | MISSING | `modules/client_options/styles/sound/sound.otui:63` | Searched for sound anthem enabled in option editors, contextual controls and the owning feature. No player control found. |
| Event Volume (%) (`soundEventsVolume`) | PRESENT | `modules/client_options/styles/sound/sound.otui:126` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:44` → `apps/client/src/features/0023-presentation/mixer.rs:24` |
| Item Volume (%) (`soundItemsVolume`) | PRESENT | `modules/client_options/styles/sound/sound.otui:95` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:36` → `apps/client/src/features/0023-presentation/mixer.rs:24` |
| Master Volume (%) (`soundMasterVolume`, `soundMasterVolumeOld`) | PRESENT | `modules/client_options/styles/sound/sound.otui:30` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:12` → `apps/client/src/features/0023-presentation/state.rs:57` |
| Music Volume (%) (`soundMusicVolume`) | PRESENT | `modules/client_options/styles/sound/sound.otui:49` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:28` → `apps/client/src/features/0023-presentation/mixer.rs:24` |
| UI Volume (%) (`soundUiVolume`) | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:22` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:52` → `apps/client/src/features/0023-presentation/mixer.rs:24` |
| Select output device (`usedSoundDevice`) | PRESENT | `modules/client_options/styles/sound/sound.otui:14` | `apps/client/src/bevy_shell/game_ui/options/audio_device.rs:25` `apps/client/src/bevy_shell/game_ui/options/audio_device.rs:58` Device selection reaches `apps/client/src/bevy_shell/audio/driver.rs:72`. |
| Enable sound | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:5` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |

## Battle Sounds

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Creature Attack Sounds (`soundBattleCreaturesAttacksEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:154` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:116` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Enable creature death sounds (`soundBattleCreaturesDeathEnabled`) | MISSING | `modules/client_options/styles/sound/battleSounds.otui:150` | Searched for sound battle creatures death enabled in option editors, contextual controls and the owning feature. No player control found. |
| Enable creature noises (`soundBattleCreaturesNoisesEnabled`) | MISSING | `modules/client_options/styles/sound/battleSounds.otui:144` | Searched for sound battle creatures noises enabled in option editors, contextual controls and the owning feature. No player control found. |
| Creature Sounds Volume (%) (`soundBattleCreaturesVolume`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:131` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:78` → `apps/client/src/features/0023-presentation/state.rs:47` |
| Other Players' Attack Sounds (`soundBattleOthersAttackSpellsEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:107` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:101` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Other Players' Healing Sounds (`soundBattleOthersHealingSpellsEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:111` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:106` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Toggle all other-player spell sounds (`soundBattleOthersSpellsEnabled`) | MISSING | `modules/client_options/styles/sound/battleSounds.otui:88` | Searched for sound battle others spells enabled in option editors, contextual controls and the owning feature. No player control found. |
| Other Players' Support Sounds (`soundBattleOthersSupportSpellsEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:115` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:111` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Other Players' Sounds Volume (%) (`soundBattleOthersVolume`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:75` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:70` → `apps/client/src/features/0023-presentation/state.rs:46` |
| Enable other-player weapon sounds (`soundBattleOthersWeaponsEnabled`) | MISSING | `modules/client_options/styles/sound/battleSounds.otui:121` | Searched for sound battle others weapons enabled in option editors, contextual controls and the owning feature. No player control found. |
| Own Attack Sounds (`soundBattleOwnAttackSpellsEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:51` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:86` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Own Healing Sounds (`soundBattleOwnHealingSpellsEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:55` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:91` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Toggle all own spell sounds (`soundBattleOwnSpellsEnabled`) | MISSING | `modules/client_options/styles/sound/battleSounds.otui:34` | Searched for sound battle own spells enabled in option editors, contextual controls and the owning feature. No player control found. |
| Own Support Sounds (`soundBattleOwnSupportSpellsEnabled`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:59` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:96` → `apps/client/src/features/0023-presentation/mixer.rs:51` |
| Own Sounds Volume (%) (`soundBattleOwnVolume`) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:21` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:62` → `apps/client/src/features/0023-presentation/state.rs:45` |
| Enable own weapon sounds (`soundBattleOwnWeaponsEnabled`) | MISSING | `modules/client_options/styles/sound/battleSounds.otui:65` | Searched for sound battle own weapons enabled in option editors, contextual controls and the owning feature. No player control found. |

## UI Sounds

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Enable console message sounds (`ChatEnabled`) | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:69` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` gates the chat tone in `apps/client/src/features/0023-presentation/ui_sound.rs:62`. |
| Enable global messages sounds (`GlobalMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:110` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable guild messages sounds (`GuildMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:86` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable npc messagese sounds (`NpcMessageseEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:106` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable party-message sounds (`PartyMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:82` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:137` gates party messages at `apps/client/src/features/0023-presentation/ui_sound.rs:26`. The party category doubles as the party notification setting. |
| Enable private messages sounds (`PrivateMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:94` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable private messages without tab sounds (`PrivateMessagesWithoutTabEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:90` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable raid messages sounds (`RaidMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:126` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable system messages sounds (`SystemMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:130` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Enable team finder messages sounds (`TeamFinderMessagesEnabled`) | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:122` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` The shared chat sound switch cannot select this channel independently. |
| Play eating sounds (`soundEatingEnabled`) | MISSING | `modules/client_options/styles/sound/sound.otui:110` | Searched for sound eating enabled in option editors, contextual controls and the owning feature. No player control found. |
| Play sounds when moving items (`soundMoveItemEnabled`) | MISSING | `modules/client_options/styles/sound/sound.otui:115` | Searched for sound move item enabled in option editors, contextual controls and the owning feature. No player control found. |
| Play a sound when someone joins or leaves the party (`soundPartyEnabled`) | MISSING | `modules/client_options/styles/sound/uiSounds.otui:53` | Searched party membership audio and UiSound::Party. `apps/client/src/features/0023-presentation/ui_sound.rs:26` assigns this cue to party speech. |
| Interaction Sounds (`soundUiInteractionsEnabled`) | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:42` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:133` → `apps/client/src/features/0023-presentation/ui_sound.rs:62` |
| VIP Sounds (`soundVipEnabled`) | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:57` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:138` → `apps/client/src/features/0023-presentation/ui_sound.rs:62` |

## Miscellaneous

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Show the character-trade payment reminder (`charTradePaymentRemindAgain`) | MISSING | — | Searched for char trade payment remind again in option editors, contextual controls and the owning feature. No player control found. |
| Show hidden characters (`characterSelectionShowHidden`) | MISSING | `modules/client_entergame/characterlist.otui:660` | Searched for character selection show hidden in option editors, contextual controls and the owning feature. No player control found. |
| Show outfits in character selection (`characterSelectionShowOutfits`) | MISSING | — | `apps/client/src/bevy_shell/selection.rs:233` explicitly leaves the outfit cells empty. Searched OwnerCharacter and toggle_outfits. |
| Reverse character sort order (`characterSelectionSortAscendingOrder`) | MISSING | `modules/client_entergame/characterlist.lua:46` | Searched for character selection sort ascending order in option editors, contextual controls and the owning feature. No player control found. |
| Choose the character sort column (`characterSelectionSortColumn`) | MISSING | `modules/client_entergame/characterlist.lua:45` | Searched for character selection sort column in option editors, contextual controls and the owning feature. No player control found. |
| Optimise Connection Stability (`connectionStabilityOptimizationEnabled`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:45` → `apps/client/src/native_entry.rs:252` |
| Enter login email (`loginEmailAddress`) | NOT-APPLICABLE | — | Platform owns browser account identity and credential presentation. |
| Quick login: use the last available character (`quickLogin`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:30` → `apps/client/src/device_login/attempt.rs:228` |
| Remember login email (`rememberLoginEmail`) | NOT-APPLICABLE | — | Platform owns browser account identity and credential presentation. |
| Stay logged in for the session (`sessionRemainLoggedIn`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:12` `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:15` |
| Show login email as plain text (`showEMailAsPlainText`) | NOT-APPLICABLE | — | Platform owns browser account identity and credential presentation. |
| Confirm before stowing container contents (`stashAskBeforeStowContainerContent`) | MISSING | — | Searched for stash ask before stow container content in option editors, contextual controls and the owning feature. No player control found. |
| Confirm store purchases (`storeAskBeforeBuyingProducts`) | MISSING | — | Searched the store purchase flow and confirm-purchase setting. `apps/client/src/bevy_shell/game_ui/store/actions.rs:101` confirms purchases but exposes no preference for suppressing the prompt. |
| Confirm logout | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:36` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |
| Confirm exit | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:41` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |

## Gameplay

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Stop chasing on manual movement (`autoChaseEnabled`) | PRESENT | `modules/client_options/styles/controls/general.otui:76` | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:58` `apps/client/src/features/local-reference-preferences/gameplay.rs:24` |
| Move nested contents to managed containers (`containerMoveToManagedContainerRecursive`) | MISSING | — | Searched for container move to managed container recursive in option editors, contextual controls and the owning feature. No player control found. |
| Warn before recursively moving container contents (`containerMoveToManagedContainerRecursiveShowWarningAgain`, `containerMoveToManagedContainerRecursiveWarning`) | MISSING | — | Searched for container move to managed container recursive show warning again in option editors, contextual controls and the owning feature. No player control found. |
| Sort backpacks first (`containerSortBackpacksFirst`) | MISSING | — | Searched for container sort backpacks first in option editors, contextual controls and the owning feature. No player control found. |
| Include nested containers when sorting (`containerSortRecursive`) | MISSING | — | Searched for container sort recursive in option editors, contextual controls and the owning feature. No player control found. |
| Warn before sorting recursively (`containerSortRecursiveShowWarningAgain`) | MISSING | — | Searched for container sort recursive show warning again in option editors, contextual controls and the owning feature. No player control found. |
| Allow others to inspect my character (`inspectPlayerAllowAllEnabled`) | MISSING | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:53` Local context-menu visibility only. Searched inspect permission and allow-inspection protocol. |
| Choose the looting control scheme (`lootSchemeIndex`) | MISSING | `modules/client_options/styles/controls/general.otui:58` | Searched for loot scheme index in option editors, contextual controls and the owning feature. No player control found. |
| Restrict market results to locker items (`marketShowLockerOnly`) | MISSING | — | Searched for market show locker only in option editors, contextual controls and the owning feature. No player control found. |
| Enable nearby-corpse looting (`quickLootAllCorpsesInAreaEnabled`) | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:63` enables the LootAll request at `apps/client/src/features/local-reference-preferences/gameplay.rs:45`. |

## Screenshots

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Capture on achievement (`screenshotsAchievementEnabled`) | PRESENT | `modules/game_notifications/controllers/screenshot.lua:26` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:203` → `apps/client/src/features/local-reference-preferences/capture_options.rs:66` |
| Enable automatic screenshots (`screenshotsAutoScreenshotsEnabled`) | MISSING | `modules/game_notifications/templates/screenshot.otui:52` | Searched for screenshots auto screenshots enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture completed bestiary entries (`screenshotsBestiaryFullEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:28` | Searched for screenshots bestiary full enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture newly unlocked bestiary entries (`screenshotsBestiaryPartlyEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:27` | Searched for screenshots bestiary partly enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture boss defeats (`screenshotsBossKillEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:31` | Searched for screenshots boss kill enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture on PvE death (`screenshotsDeathPvEEnabled`) | PARTIAL | `modules/game_notifications/controllers/screenshot.lua:32` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:188` Both use on_death. No choice by cause of death. |
| Capture on PvP death (`screenshotsDeathPvPEnabled`) | PARTIAL | `modules/game_notifications/controllers/screenshot.lua:33` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:188` Both use on_death. No choice by cause of death. |
| Capture Gift of Life activation (`screenshotsGiftOfLifeEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:40` | Searched for screenshots gift of life enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture on level up (`screenshotsLevelUpEnabled`) | PRESENT | `modules/game_notifications/controllers/screenshot.lua:24` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:193` → `apps/client/src/features/local-reference-preferences/capture_options.rs:64` |
| Capture low health (`screenshotsLowHitPointsEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:39` | Searched for screenshots low hit points enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture a new damage record (`screenshotsNewMaxDamageEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:37` | Searched for screenshots new max damage enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture a new healing record (`screenshotsNewMaxHealingEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:38` | Searched for screenshots new max healing enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture a player kill assist (`screenshotsPlayerKillAssistEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:35` | Searched for screenshots player kill assist enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture a player kill (`screenshotsPlayerKillFullEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:34` | Searched for screenshots player kill full enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture an incoming PvP attack (`screenshotsPvPAttackEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:36` | Searched for screenshots pv pattack enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture reward-chest treasure (`screenshotsRewardChestEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:29` | Searched for screenshots reward chest enabled in option editors, contextual controls and the owning feature. No player control found. |
| Capture on skill up (`screenshotsSkillUpEnabled`) | PRESENT | `modules/game_notifications/controllers/screenshot.lua:25` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:198` → `apps/client/src/features/local-reference-preferences/screenshot_backlog.rs:121` |
| Capture tracked loot (`screenshotsTrackedLootEnabled`) | MISSING | `modules/game_notifications/controllers/screenshot.lua:30` | Searched for screenshots tracked loot enabled in option editors, contextual controls and the owning feature. No player control found. |
| Screenshot backlog (seconds, 0 disables) (`screenshotsUseBacklog`) | PRESENT | `modules/game_notifications/templates/screenshot.otui:39` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:208` → `apps/client/src/features/local-reference-preferences/screenshot_backlog.rs:167` |
| Capture game area only (`screnshotsOnlyGameWindow`) | PRESENT | `modules/game_notifications/templates/screenshot.otui:27` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:183` → `apps/client/src/features/local-reference-preferences/capture_options.rs:87` |
| Set screenshot folder | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/screenshots.rs:11` → `apps/client/src/bevy_shell/game_ui/options/screenshots.rs:70` |
| Open the screenshots folder | MISSING | `modules/game_notifications/templates/screenshot.otui:87` | Searched folder launcher and screenshots page. The path editor changes the destination only. |
| Take a manual screenshot | PRESENT | `modules/game_notifications/controllers/screenshot.lua:167` | `apps/client/src/bevy_shell/audit.rs:62` → `apps/client/src/bevy_shell/screenshots/automatic.rs:129` |

## Help

| Operation | Status | OTClient file:line | Oteryn evidence or gap |
|---|---|---|---|
| Read help topic | PRESENT | `modules/client_options/styles/misc/help.otui:17` | `apps/client/src/bevy_shell/game_ui/options/guide.rs:9` → `apps/client/src/bevy_shell/game_ui/options/guide.rs:35` |
| Export options | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/transfer.rs:25` → `apps/client/src/bevy_shell/game_ui/options/transfer.rs:86` |
| Import options | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/transfer.rs:25` → `apps/client/src/bevy_shell/game_ui/options/transfer.rs:87` |
| Reset saved options | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/transfer.rs:25` → `apps/client/src/bevy_shell/game_ui/options/transfer.rs:89` |
| Export minimap | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/minimap_transfer.rs:25` → `apps/client/src/bevy_shell/game_ui/options/minimap_transfer.rs:60` |
| Import minimap | PRESENT | — | `apps/client/src/bevy_shell/game_ui/options/minimap_transfer.rs:25` → `apps/client/src/bevy_shell/game_ui/options/minimap_transfer.rs:60` |

## Outstanding operations by player impact

Within each tier, operations are grouped by page. Combat visibility and input affect decisions during play. Communication and inventory settings affect repeated tasks. Presentation and capture settings primarily affect customization or records.

### 1. Combat visibility, targeting and input

- **Controls** — Always face the direction of movement (missing), Choose mouse control scheme (partial), Move the entire stack by default when dragging (missing), Use the default keyboard delay (missing).
- **General Hotkeys** — Search actions by name (missing).
- **Action Bar Hotkeys** — Assign secondary action-bar key (partial).
- **HUD** — Toggle all other-creature HUD elements (missing), Show other creatures’ health bars (partial), Set HUD arc height (partial), Set HUD arc width (partial), Toggle the player HUD (missing), Show player HUD bars (missing), Place the harmony indicator on the left (missing), Show status bars (missing), Toggle all conditions in bar (missing), Toggle all conditions in HUD (missing).
- **Game Window** — Choose how loot values are colored (missing), Show combat frames (missing), Choose viewport resizing mode (missing), Show attack animation in the game window (missing), Show boosted creature messages in the game window (missing), Show hotkey usage messages in the game window (missing), Show messages in the game window (missing), Show offline training messages in the game window (missing), Show other players’ spells (partial), Show potion messages in the game window (missing), Show private messages in the game window (missing), Show store messages in the game window (missing), Show target highlight in the game window (missing), Resize the upper pane (missing).
- **Action Bars** — Lock all bottom bars (partial), Lock all left bars (partial), Lock all right bars (partial), Show numeric cooldowns (partial), Show graphical cooldowns (partial), Show assigned hotkeys on action buttons (partial), Show assigned spell parameters (missing).
- **Gameplay** — Move nested contents to managed containers (missing), Warn before recursively moving container contents (missing), Sort backpacks first (missing), Include nested containers when sorting (missing), Warn before sorting recursively (missing), Allow others to inspect my character (missing), Choose the looting control scheme (missing), Restrict market results to locker items (missing).

### 2. Communication, navigation and session workflow

- **Interface** — Confirm before copying a link (missing), Animate the mouse cursor (missing), Use a large mouse cursor (missing), Highlight the object under the pointer (missing), Use the operating system cursor (missing), Show item expiry in containers (missing), Show item expiry in inventory (missing), Show expiry for unused items (missing), Resize NPC dialog (partial), Move NPC dialog (partial).
- **Console** — Show events in console (partial), Show information in console (partial), Show join and leave messages (missing), Show speaker levels (partial), Show other status in console (partial), Show own status in console (partial), Show timestamps (partial), Show timestamp seconds (partial), Open incoming private messages in a new tab (partial), Keep the loot channel open (partial), Open and close chat channels (partial).
- **Shortcuts** — Add sidebar shortcut (missing), Remove sidebar shortcut (missing), Reorder sidebar shortcuts (missing).
- **Miscellaneous** — Show the character-trade payment reminder (missing), Show hidden characters (missing), Show outfits in character selection (missing), Reverse character sort order (missing), Choose the character sort column (missing), Confirm before stowing container contents (missing), Confirm store purchases (missing).

### 3. Display, sound and capture

- **Graphics** — Select anti-aliasing mode (missing), Toggle the foreground frame-rate limit (partial), Scale by even multiples (partial), Choose the rendering engine (missing), Use the FreeType text renderer (missing), Remember maximized window state (missing), Select a monitor (missing).
- **Effects** — Attenuate clouds and indoor light (partial).
- **Sound** — Play the anthem (missing).
- **Battle Sounds** — Enable creature death sounds (missing), Enable creature noises (missing), Toggle all other-player spell sounds (missing), Enable other-player weapon sounds (missing), Toggle all own spell sounds (missing), Enable own weapon sounds (missing).
- **UI Sounds** — Enable global messages sounds (partial), Enable guild messages sounds (partial), Enable npc messagese sounds (partial), Enable party-message sounds (partial), Enable private messages sounds (partial), Enable private messages without tab sounds (partial), Enable raid messages sounds (partial), Enable system messages sounds (partial), Enable team finder messages sounds (partial), Play eating sounds (missing), Play sounds when moving items (missing), Play a sound when someone joins or leaves the party (missing).
- **Screenshots** — Enable automatic screenshots (missing), Capture completed bestiary entries (missing), Capture newly unlocked bestiary entries (missing), Capture boss defeats (missing), Capture on PvE death (partial), Capture on PvP death (partial), Capture Gift of Life activation (missing), Capture low health (missing), Capture a new damage record (missing), Capture a new healing record (missing), Capture a player kill assist (missing), Capture a player kill (missing), Capture an incoming PvP attack (missing), Capture reward-chest treasure (missing), Capture tracked loot (missing), Open the screenshots folder (missing).
