# Client parity audit D

Oteryn `a4ad4164` and OTClient `4955a3fa22d5f5456ec6c658187fd2901545b750`. Counts measure operations within each module. A shared operation can have an entry in each module that offers it. Mouse, keyboard and alternate layout bindings within a module share an entry. Sorting directions and independently selectable capture events have separate entries. The review traces source paths and does not exercise a live server.

| Module | Operations | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE |
| --- | ---: | ---: | ---: | ---: | ---: |
| game_interface | 58 | 42 | 5 | 9 | 2 |
| game_console | 34 | 23 | 3 | 8 | 0 |
| game_containers | 18 | 12 | 2 | 4 | 0 |
| game_inventory | 13 | 11 | 1 | 1 | 0 |
| game_textmessage | 4 | 2 | 2 | 0 | 0 |
| game_textwindow | 3 | 2 | 1 | 0 | 0 |
| game_modaldialog | 1 | 1 | 0 | 0 | 0 |
| game_notifications | 23 | 6 | 4 | 13 | 0 |
| game_inspect | 7 | 5 | 0 | 2 | 0 |
| game_walk | 4 | 4 | 0 | 0 | 0 |
| game_joystick | 1 | 0 | 0 | 1 | 0 |
| game_tutorial | 2 | 0 | 2 | 0 | 0 |
| **Total** | 168 | 108 | 20 | 38 | 2 |

## game_interface

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Log out | PRESENT | `modules/game_interface/gameinterface.lua:328` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:356` → `apps/client/src/features/0021-logout/input.rs:6` |
| Look at a target | PRESENT | `modules/game_interface/gameinterface.lua:673` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:252` → `apps/client/src/features/0011-world-interaction/input.rs:199` |
| Inspect a character | PRESENT | `modules/game_interface/gameinterface.lua:681` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:271` → `apps/client/src/features/0132-inspection/input.rs:7` |
| Inspect an item | PRESENT | `modules/game_interface/gameinterface.lua:687` | `apps/client/src/features/0132-inspection/input.rs:26` → `apps/client/src/features/0132-inspection/input.rs:14` |
| Open the item’s Cyclopedia entry | PRESENT | `modules/game_interface/gameinterface.lua:693` | `apps/client/src/features/0132-inspection/input/related.rs:6` → `apps/client/src/features/0132-inspection/input/related.rs:72` |
| Open the inspected weapon’s proficiency | PRESENT | `modules/game_interface/gameinterface.lua:698` | `apps/client/src/features/0132-inspection/input/related.rs:6` → `apps/client/src/features/0132-inspection/input/related.rs:56` |
| Open a container | PRESENT | `modules/game_interface/gameinterface.lua:712` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:173` → `apps/client/src/features/0014-container-tree/input.rs:126` |
| Use an item on another target | PRESENT | `modules/game_interface/gameinterface.lua:725` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:260` → `apps/client/src/features/0015-item-use/input.rs:161` |
| Use an item | PRESENT | `modules/game_interface/gameinterface.lua:729` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:256` → `apps/client/src/features/0011-world-interaction/input.rs:199` |
| Rotate furniture | PRESENT | `modules/game_interface/gameinterface.lua:736` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:401` → `apps/client/src/features/0445-house-furniture-rotation/input.rs:27` |
| Wrap or unwrap furniture | PRESENT | `modules/game_interface/gameinterface.lua:745` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:402` → `apps/client/src/features/0207-house-furniture-wrap/input.rs:27` |
| Browse a map field | PRESENT | `modules/game_interface/gameinterface.lua:752` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:296` → `apps/client/src/features/0230-browse-field/input.rs:4` |
| Offer a player an item in trade | PRESENT | `modules/game_interface/gameinterface.lua:765` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:282` → `apps/client/src/features/0060-player-trade/input.rs:44` |
| Open Prey | PRESENT | `modules/game_interface/gameinterface.lua:789` | `apps/client/src/features/0140-prey/input.rs:26` → `apps/client/src/features/0140-prey/input.rs:42` |
| Start an NPC conversation | PRESENT | `modules/game_interface/gameinterface.lua:831` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:288` → `apps/client/src/bevy_shell/gameplay_dialogs.rs:306` |
| Attack or stop attacking a creature | PRESENT | `modules/game_interface/gameinterface.lua:837` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:248` → `apps/client/src/bevy_shell/gameplay_dialogs.rs:323` |
| Follow or stop following a creature | PRESENT | `modules/game_interface/gameinterface.lua:847` | `apps/client/src/features/0064-target-move/input.rs:40` → `apps/client/src/features/0064-target-move/input.rs:83` |
| Open a private conversation | PRESENT | `modules/game_interface/gameinterface.lua:860` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:335` → `apps/client/src/features/0007-chat/console.rs:345` |
| Add a player to VIPs | PRESENT | `modules/game_interface/gameinterface.lua:872` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:338` → `apps/client/src/features/0043-vip/input.rs:12` |
| Ignore or unignore a player | PRESENT | `modules/game_interface/gameinterface.lua:882` | `apps/client/src/features/0007-chat/lists.rs:22` → `crates/client-ui/src/features/0007-chat/state.rs:395` |
| Invite a player to the party | PRESENT | `modules/game_interface/gameinterface.lua:896` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:376` → `apps/client/src/bevy_shell/game_ui/party.rs:72` |
| Accept a party invitation | PRESENT | `modules/game_interface/gameinterface.lua:893` | `apps/client/src/bevy_shell/game_ui/party.rs:215` → `apps/client/src/features/0042-party/input.rs:10` |
| Leave the party | PRESENT | `modules/game_interface/gameinterface.lua:818` | `apps/client/src/bevy_shell/game_ui/party.rs:216` → `apps/client/src/features/0042-party/input.rs:10` |
| Transfer party leadership | PRESENT | `modules/game_interface/gameinterface.lua:915` | `apps/client/src/bevy_shell/game_ui/party.rs:217` → `apps/client/src/features/0042-party/input.rs:10` |
| Toggle shared experience | PRESENT | `modules/game_interface/gameinterface.lua:813` | `apps/client/src/bevy_shell/game_ui/party.rs:219` → `apps/client/src/features/0042-party/input.rs:10` |
| Copy a player’s name | PRESENT | `modules/game_interface/gameinterface.lua:934` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:324` → `apps/client/src/bevy_shell/gameplay_dialogs.rs:517` |
| Report a player | PRESENT | `modules/game_interface/gameinterface.lua:928` | `apps/client/src/bevy_shell/gameplay_dialogs/projection.rs:321` → `apps/client/src/bevy_shell/gameplay_dialogs.rs:389` |
| Choose chase or stand | PRESENT | `modules/game_interface/gameinterface.lua:1543` | `apps/client/src/features/0087-pvp-expert/input.rs:42` → `apps/client/src/features/0087-pvp-expert/protocol.rs:17` |
| Choose the quantity to move | PRESENT | `modules/game_interface/styles/countwindow.otui:3` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:237` → `apps/client/src/features/0004-item-view-move/input.rs:45` |
| Invite a player to a private channel | PRESENT | `modules/game_interface/gameinterface.lua:864` | `apps/client/src/features/0080-chat-private-channel/view.rs:70` → `apps/client/src/features/0080-chat-private-channel/input.rs:39` |
| Exclude a player from a private channel | PRESENT | `modules/game_interface/gameinterface.lua:867` | `apps/client/src/features/0080-chat-private-channel/view.rs:71` → `apps/client/src/features/0080-chat-private-channel/input.rs:39` |
| Mount or dismount | PRESENT | `modules/game_interface/gameinterface.lua:796` | `apps/client/src/bevy_shell/game_ui/outfit.rs:177` → `apps/client/src/features/0502-actor-motion/protocol.rs:90` |
| Customise character appearance | PRESENT | `modules/game_interface/gameinterface.lua:785` | `apps/client/src/bevy_shell/game_ui/outfit.rs:109` → `apps/client/src/bevy_shell/game_ui/outfit.rs:175` |
| Manage loot destinations | PRESENT | `modules/game_interface/gameinterface.lua:964` | `apps/client/src/features/0098-quick-loot/input.rs:43` → `apps/client/src/features/0098-quick-loot/input.rs:54` |
| Add or remove an item from the loot filter | PRESENT | `modules/game_interface/gameinterface.lua:973` | `apps/client/src/features/0098-quick-loot/input.rs:69` → `apps/client/src/features/0098-quick-loot/input.rs:6` |
| Stop movement and combat | PRESENT | `modules/game_interface/gameinterface.lua:236` | `apps/client/src/features/0064-target-move/input.rs:22` → `apps/client/src/features/0064-target-move/input.rs:75` |
| Lock or unlock an action bar | PRESENT | `modules/game_interface/gameinterface.otui:105` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:23` → `apps/client/src/bevy_shell/game_ui/actions/menu.rs:176` |
| Quick-loot a selected corpse | PARTIAL | `modules/game_interface/gameinterface.lua:758` | Nearby loot-all is wired at `apps/client/src/features/0098-quick-loot/input.rs:10`. It does not bind the request to the clicked corpse. |
| Revoke a pending party invitation | MISSING | `modules/game_interface/gameinterface.lua:903` | Searched for revoke and rescind in party actions and invitation commands. |
| Stow all matching stacks | MISSING | `modules/game_interface/gameinterface.lua:988` | Searched for stow-by-type and inventory-wide aggregation in stash. |
| Stow container contents | MISSING | `modules/game_interface/gameinterface.lua:995` | Searched for stash-container and recursive deposit in stash and container actions. |
| Show or hide top menu | MISSING | `modules/game_interface/gameinterface.otui:66` | Searched for top-menu visibility control in shell settings. |
| Cycle game view mode | MISSING | `modules/game_interface/gameinterface.lua:267` | Searched for next view mode and viewport layout cycle. |
| Open a nested container in a separate window | PARTIAL | `modules/game_interface/gameinterface.lua:715` | The open request at `apps/client/src/features/0014-container-tree/input.rs:44` exposes no choice to reuse or retain the source window. |
| Move an item into its parent container | PARTIAL | `modules/game_interface/gameinterface.lua:773` | Container-to-container drag works. No move-to-parent shortcut resolves the parent of the selected item. |
| Copy a raw object ID | NOT-APPLICABLE | `modules/game_interface/gameinterface.lua:956` | Reference exposes an OTClient developer identifier in the world menu. |
| Exit the application | NOT-APPLICABLE | `modules/game_interface/gameinterface.lua:360` | Platform owns application termination. |
| Stow a chosen quantity of an item | PARTIAL | `modules/game_interface/gameinterface.lua:985` | The carried-row action stows a whole stack at `apps/client/src/bevy_shell/game_ui/stash/widgets.rs:149`. It has no stow quantity selector. |
| Force logout on a failing connection | MISSING | `modules/game_interface/gameinterface.lua:462` | Searched for forced disconnect and connection-failure override in logout controls and session actions. |
| Zoom the game map | PRESENT | `modules/game_interface/gameinterface.lua:229` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:31` → `apps/client/src/features/local-reference-preferences/graphics.rs:44` |
| Walk to a clicked tile | PRESENT | `modules/game_interface/gameinterface.lua:1008` | `apps/client/src/features/0011-world-interaction/input.rs:16` → `apps/client/src/features/0064-target-move/input.rs:98` |
| Drag an item between map and inventory locations | PRESENT | `modules/game_interface/widgets/uigamemap.lua:54` | `apps/client/src/bevy_shell/game_ui/inventory/world_drag.rs:378` → `apps/client/src/features/0004-item-view-move/input.rs:30` |
| Add or remove a sidebar | PRESENT | `modules/game_interface/gameinterface.otui:338` | `apps/client/src/bevy_shell/game_ui/hud_panes.rs:379` → `apps/client/src/bevy_shell/game_ui/hud_panes.rs:414` |
| Clear all floating map text and screen messages | PARTIAL | `modules/game_interface/gameinterface.lua:256` | The dismiss action clears the four screen slots at `apps/client/src/bevy_shell/text_messages.rs:147`. No corresponding purge of world speech text found. |
| Choose status-bar size and placement | MISSING | `modules/game_interface/widgets/statsbar.lua:915` | Searched for status-bar dimension and placement controls. |
| Choose which skills appear in the status bar | MISSING | `modules/game_interface/widgets/statsbar.lua:927` | Searched for skill visibility by name in top-bar settings and projection. |
| Hide the customisable status bar | MISSING | `modules/game_interface/widgets/statsbar.lua:941` | Searched for status bar visibility toggle in HUD options and region layout. |
| Push a creature onto another tile | PRESENT | `modules/game_interface/widgets/uigamemap.lua:91` | `apps/client/src/features/0082-creature-push/input.rs:76` → `apps/client/src/features/0082-creature-push/input.rs:136` |

## game_console

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Browse and join a channel | PRESENT | `modules/game_console/channelswindow.otui:10` | `apps/client/src/features/0007-chat/console.rs:37` → `apps/client/src/features/0007-chat/console.rs:334` |
| Open a private conversation | PRESENT | `modules/game_console/channelswindow.otui:43` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:238` → `apps/client/src/features/0007-chat/console.rs:345` |
| Send a private message | PRESENT | `modules/game_console/console.lua:1735` | `apps/client/src/bevy_shell/game_ui/chat/commands.rs:20` → `crates/client-ui/src/features/0007-chat/protocol.rs:79` |
| Speak in the selected channel | PRESENT | `modules/game_console/console.lua:793` | `apps/client/src/bevy_shell/game_ui/chat/commands.rs:20` → `crates/client-ui/src/features/0007-chat/protocol.rs:72` |
| Switch active chat tab | PRESENT | `modules/game_console/console.lua:265` | `apps/client/src/bevy_shell/game_ui/chat/commands.rs:25` → `apps/client/src/features/0007-chat/console.rs:170` |
| Leave or close the active channel | PRESENT | `modules/game_console/console.otui:340` | `apps/client/src/features/0007-chat/console.rs:74` → `apps/client/src/features/0007-chat/console.rs:305` |
| Mirror a channel read-only or close its mirror | PRESENT | `modules/game_console/console.lua:1474` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:259` → `apps/client/src/features/0007-chat/console.rs:112` |
| Save channel transcript | PRESENT | `modules/game_console/console.lua:1482` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:258` → `apps/client/src/features/0007-chat/console.rs:229` |
| Add or remove an ignored player | PRESENT | `modules/game_console/console.lua:1518` | `apps/client/src/features/0007-chat/lists.rs:22` → `crates/client-ui/src/features/0007-chat/state.rs:395` |
| Add or remove a whitelist exception | PRESENT | `modules/game_console/communicationwindow.otui:111` | `apps/client/src/features/0007-chat/lists.rs:22` → `crates/client-ui/src/features/0007-chat/state.rs:395` |
| Copy a message | PRESENT | `modules/game_console/console.lua:1542` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:235` → `apps/client/src/bevy_shell/game_ui/chat/widgets.rs:38` |
| Report a speaker | PRESENT | `modules/game_console/console.lua:1525` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:143` → `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:252` |
| Add a speaker to VIPs by name | PRESENT | `modules/game_console/console.lua:1500` | `apps/client/src/bevy_shell/gameplay_dialogs.rs:338` → `apps/client/src/features/0043-vip/input.rs:12` |
| Change Exiva permissions | PRESENT | `modules/game_console/exiva_option.lua:95` | `apps/client/src/features/0147-exiva-settings/input.rs:15` → `apps/client/src/features/0147-exiva-settings/input.rs:9` |
| Toggle keyboard chat mode | PRESENT | `modules/game_console/console.otui:484` | `apps/client/src/features/0007-chat/console.rs:127` → `apps/client/src/features/0007-chat/console.rs:179` |
| Create a private channel | PRESENT | `modules/game_console/console.lua:2003` | `apps/client/src/features/0080-chat-private-channel/view.rs:66` → `apps/client/src/features/0080-chat-private-channel/input.rs:39` |
| Invite a player to a private channel | PRESENT | `modules/game_console/console.lua:1453` | `apps/client/src/features/0080-chat-private-channel/view.rs:70` → `apps/client/src/features/0080-chat-private-channel/input.rs:39` |
| Exclude a player from a private channel | PRESENT | `modules/game_console/console.lua:1456` | `apps/client/src/features/0080-chat-private-channel/view.rs:71` → `apps/client/src/features/0080-chat-private-channel/input.rs:39` |
| Open Help channel | PRESENT | `modules/game_console/console.lua:307` | `apps/client/src/features/0007-chat/console.rs:67` → `apps/client/src/features/0007-chat/console.rs:84` |
| Clear the current transcript | MISSING | `modules/game_console/console.lua:1479` | Searched for clear-channel and transcript-clear actions. |
| Copy only the speaker’s name | MISSING | `modules/game_console/console.lua:1531` | Searched for speaker-name clipboard actions in the message menu. |
| Block all private messages | MISSING | `modules/game_console/communicationwindow.otui:88` | Searched for global private-message filter in communication lists and delivery. |
| Block all yells | MISSING | `modules/game_console/communicationwindow.otui:96` | Searched for yell rejection in incoming speech filters. |
| Bypass ignores for VIP speakers | MISSING | `modules/game_console/communicationwindow.otui:161` | Searched for VIP membership in chat delivery filters. |
| Suspend or enable the ignore list | MISSING | `modules/game_console/communicationwindow.otui:32` | Searched for ignore-list enable switch beside the membership editor. |
| Suspend or enable the whitelist | MISSING | `modules/game_console/communicationwindow.otui:104` | Searched for whitelist enable switch in communication preferences. |
| Lock or unlock chat dragging | MISSING | `modules/game_console/console.otui:294` | Searched for chat drag-lock control and transcript drag binding. |
| Select the channel transcript | PARTIAL | `modules/game_console/console.lua:1546` | Read-only text selection is per message row at `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:25`. No selection spans the transcript. |
| Copy selected text within a message | PRESENT | `modules/game_console/console.lua:1537` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:24` → `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:26` |
| Hide the console | PARTIAL | `modules/game_console/console.otui:318` | Composer mode changes at `apps/client/src/features/0007-chat/console.rs:127`. No matching toggle hides the transcript. |
| Recall a prior sent message | PRESENT | `modules/game_console/console.lua:249` | `apps/client/src/bevy_shell/game_ui/chat.rs:327` → `apps/client/src/bevy_shell/game_ui/chat.rs:92` |
| Choose speech volume | PRESENT | `modules/game_console/console.otui:450` | `apps/client/src/bevy_shell/game_ui/chat/widgets.rs:452` → `crates/client-ui/src/features/0007-chat/protocol.rs:31` |
| Continue or close a violation-report conversation | PARTIAL | `modules/game_console/console.lua:845` | Player reporting opens at `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:252`. No player-facing violation conversation or close-channel handshake found. |
| Read incoming speech in the active transcript | PRESENT | `modules/game_console/console.lua:190` | `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:29` → `apps/client/src/features/0007-chat/console.rs:360` |

## game_containers

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Close a container | PRESENT | `modules/game_containers/containers.lua:937` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:218` → `apps/client/src/features/0014-container-tree/input.rs:72` |
| Navigate to parent container | PRESENT | `modules/game_containers/containers.lua:951` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:216` → `apps/client/src/features/0014-container-tree/input.rs:57` |
| Toggle manual slot ordering | PRESENT | `modules/game_containers/container.otui:205` | `apps/client/src/bevy_shell/game_ui/install.rs:359` → `apps/client/src/features/0004-item-view-move/input.rs:81` |
| Sort by name, ascending | PRESENT | `modules/game_containers/container.otui:168` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by weight, ascending | PRESENT | `modules/game_containers/container.otui:174` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by expiry, ascending | PRESENT | `modules/game_containers/container.otui:180` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by stack size, ascending | PRESENT | `modules/game_containers/container.otui:186` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by name, descending | PRESENT | `modules/game_containers/container.otui:171` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by weight, descending | PRESENT | `modules/game_containers/container.otui:177` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by expiry, descending | PRESENT | `modules/game_containers/container.otui:183` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort by stack size, descending | PRESENT | `modules/game_containers/container.otui:189` | `apps/client/src/bevy_shell/game_ui/install/tools.rs:262` → `apps/client/src/bevy_shell/game_ui/install.rs:355` |
| Sort nested container contents | MISSING | `modules/game_containers/container.otui:201` | Searched for recursive sort flag and descendant traversal. |
| Choose whether bulk move includes nested containers | MISSING | `modules/game_containers/container.otui:217` | Searched for nested inclusion switch and nested exclusion in bulk request construction. |
| Page through ordinary container contents | MISSING | `modules/game_containers/containers.lua:905` | Searched for seek-in-container, page offset and ordinary container navigation controls. |
| Distribute contents to designated Obtain containers | PARTIAL | `modules/game_containers/container.otui:213` | The bulk mover targets one manually selected destination at `apps/client/src/features/0014-container-tree/input/bulk.rs:47`. No routing by loot category. The reference button itself returns immediately at `modules/game_containers/containers.lua:642`. |
| Sort containers before loose items | MISSING | `modules/game_containers/container.otui:197` | Searched for container-first ordering in the sort menu, command fields and key enumeration. |
| Minimise an individual open container | PARTIAL | `modules/game_containers/containers.lua:956` | Open views are rendered individually at `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:294`. No per-view minimise control was found. |
| Inspect the contents of an open container | PRESENT | `modules/game_containers/containers.lua:32` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:394` → `apps/client/src/features/0014-container-tree/protocol.rs:89` |

## game_inventory

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Choose offensive, balanced or defensive stance | PRESENT | `modules/game_inventory/inventory.lua:304` | `apps/client/src/features/0087-pvp-expert/input.rs:42` → `apps/client/src/features/0087-pvp-expert/protocol.rs:11` |
| Stand or chase the target | PRESENT | `modules/game_inventory/inventory.lua:256` | `apps/client/src/features/0087-pvp-expert/input.rs:42` → `apps/client/src/features/0087-pvp-expert/protocol.rs:17` |
| Toggle secure fighting | PRESENT | `modules/game_inventory/inventory.lua:306` | `apps/client/src/features/0087-pvp-expert/input.rs:42` → `apps/client/src/features/0087-pvp-expert/protocol.rs:22` |
| Choose expert PvP mode | PRESENT | `modules/game_inventory/inventory.lua:308` | `apps/client/src/features/0087-pvp-expert/input.rs:42` → `apps/client/src/features/0087-pvp-expert/protocol.rs:27` |
| Stop combat and movement | PRESENT | `modules/game_inventory/inventory.otui:240` | `apps/client/src/features/0064-target-move/input.rs:22` → `apps/client/src/features/0064-target-move/input.rs:75` |
| Collapse or expand equipment | PRESENT | `modules/game_inventory/inventory.otui:58` | `apps/client/src/bevy_shell/game_ui/inventory/view.rs:318` → `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:61` |
| Show blessings | PRESENT | `modules/game_inventory/inventory.otui:71` | `apps/client/src/bevy_shell/game_ui/inventory/view.rs:320` → `apps/client/src/bevy_shell/game_ui/hud/blessings.rs:242` |
| Open the store inbox from the purse | PRESENT | `modules/game_inventory/inventory.otui:78` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:158` → `apps/client/src/features/0199-cyclopedia-character/input.rs:67` |
| Check soul points | PRESENT | `modules/game_inventory/inventory.otui:347` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:61` → `apps/client/src/bevy_shell/game_ui/inventory/view.rs:103` |
| Check free carrying capacity | PRESENT | `modules/game_inventory/inventory.otui:376` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:16` → `apps/client/src/bevy_shell/game_ui/inventory/view.rs:131` |
| Show or hide expert PvP controls | MISSING | `modules/game_inventory/inventory.lua:453` | Searched for expert-control visibility setting in inventory and PvP forms. |
| Check proficiency for the equipped weapon | PARTIAL | `modules/game_inventory/inventory.lua:152` | Inspected weapons open proficiency at `apps/client/src/features/0132-inspection/input/related.rs:56`. No automatic proficiency refresh of the top bar on a left-hand equipment change found. |
| Equip or unequip an item by dragging it | PRESENT | `modules/game_inventory/inventory.otui:265` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:100` → `apps/client/src/features/0004-item-view-move/input.rs:30` |

## game_textmessage

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Read on-screen server feedback | PRESENT | `modules/game_textmessage/textmessage.lua:228` | `apps/client/src/bevy_shell/text_messages.rs:126` → `apps/client/src/bevy_shell/text_messages.rs:72` |
| Read private messages over the game view | PARTIAL | `modules/game_textmessage/textmessage.lua:288` | Private messages render in chat at `apps/client/src/bevy_shell/game_ui/chat/widgets/messages.rs:54`. The screen-message dispatcher lacks a private-message area. |
| Dismiss displayed screen messages | PRESENT | `modules/game_textmessage/textmessage.lua:328` | `apps/client/src/features/local-game-notifications/input.rs:31` → `apps/client/src/bevy_shell/text_messages.rs:147` |
| Read colour-coded loot messages | PARTIAL | `modules/game_textmessage/textmessage.lua:261` | The server log accepts text at `apps/client/src/features/0007-chat/console.rs:368`. No value-coloured item formatter found in the screen-message dispatcher. |

## game_textwindow

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Read or edit a house access list | PRESENT | `modules/game_textwindow/textwindow.lua:148` | `apps/client/src/bevy_shell/game_ui/houses.rs:541` → `apps/client/src/features/0049-house/input.rs:97` |
| Edit and submit an item inscription | PRESENT | `modules/game_textwindow/textwindow.lua:100` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:197` → `apps/client/src/features/0115-mail/input.rs:55` |
| Read an item inscription with authorship | PARTIAL | `modules/game_textwindow/textwindow.lua:30` | The inscription query returns text at `apps/client/src/features/0115-mail/input.rs:169`. No writer or writing date found in the displayed item record. |

## game_modaldialog

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Answer a server dialog with its selected choice and button | PRESENT | `modules/game_modaldialog/modaldialog.lua:35` | `apps/client/src/bevy_shell/game_ui/feature_dialogs.rs:201` → `apps/client/src/features/0450-server-modal/protocol/bridge.rs:123` |

## game_notifications

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Restrict capture to the game area | PRESENT | `modules/game_notifications/templates/screenshot.otui:27` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:183` → `apps/client/src/bevy_shell/screenshots/automatic.rs:93` |
| Include five seconds of preceding captures | PRESENT | `modules/game_notifications/templates/screenshot.otui:39` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:209` → `apps/client/src/features/local-reference-preferences/screenshot_backlog.rs:167` |
| Enable or disable all automatic captures | PARTIAL | `modules/game_notifications/templates/screenshot.otui:52` | Individual event settings exist at `apps/client/src/bevy_shell/game_ui/options/metadata.rs:189`. No master switch preserving event selections on that page. |
| Capture on level up | PRESENT | `modules/game_notifications/controllers/screenshot.lua:24` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:194` → `apps/client/src/features/local-reference-preferences/screenshot_backlog.rs:96` |
| Capture on skill up | PRESENT | `modules/game_notifications/controllers/screenshot.lua:25` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:199` → `apps/client/src/features/local-reference-preferences/screenshot_backlog.rs:96` |
| Capture on achievement | PRESENT | `modules/game_notifications/controllers/screenshot.lua:26` | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:204` → `apps/client/src/features/local-reference-preferences/screenshot_backlog.rs:96` |
| Capture on bestiary entry unlocked | MISSING | `modules/game_notifications/controllers/screenshot.lua:27` | Searched for bestiary entry unlocked in automatic capture policy, trigger dispatch and option controls. |
| Capture on bestiary entry completed | MISSING | `modules/game_notifications/controllers/screenshot.lua:28` | Searched for bestiary entry completed in automatic capture policy, trigger dispatch and option controls. |
| Capture on treasure found | MISSING | `modules/game_notifications/controllers/screenshot.lua:29` | Searched for treasure found in automatic capture policy, trigger dispatch and option controls. |
| Capture on valuable loot | MISSING | `modules/game_notifications/controllers/screenshot.lua:30` | Searched for valuable loot in automatic capture policy, trigger dispatch and option controls. |
| Capture on boss defeated | MISSING | `modules/game_notifications/controllers/screenshot.lua:31` | Searched for boss defeated in automatic capture policy, trigger dispatch and option controls. |
| Capture on death pve | PARTIAL | `modules/game_notifications/controllers/screenshot.lua:32` | One death preference covers both PvP and PvE at `apps/client/src/bevy_shell/game_ui/options/metadata.rs:189`. Independent selection is missing. |
| Capture on death pvp | PARTIAL | `modules/game_notifications/controllers/screenshot.lua:33` | One death preference covers both PvP and PvE at `apps/client/src/bevy_shell/game_ui/options/metadata.rs:189`. Independent selection is missing. |
| Capture on player kill | MISSING | `modules/game_notifications/controllers/screenshot.lua:34` | Searched for player kill in automatic capture policy, trigger dispatch and option controls. |
| Capture on player kill assist | MISSING | `modules/game_notifications/controllers/screenshot.lua:35` | Searched for player kill assist in automatic capture policy, trigger dispatch and option controls. |
| Capture on player attacking | MISSING | `modules/game_notifications/controllers/screenshot.lua:36` | Searched for player attacking in automatic capture policy, trigger dispatch and option controls. |
| Capture on highest damage dealt | MISSING | `modules/game_notifications/controllers/screenshot.lua:37` | Searched for highest damage dealt in automatic capture policy, trigger dispatch and option controls. |
| Capture on highest healing done | MISSING | `modules/game_notifications/controllers/screenshot.lua:38` | Searched for highest healing done in automatic capture policy, trigger dispatch and option controls. |
| Capture on low health | MISSING | `modules/game_notifications/controllers/screenshot.lua:39` | Searched for low health in automatic capture policy, trigger dispatch and option controls. |
| Capture on gift of life triggered | MISSING | `modules/game_notifications/controllers/screenshot.lua:40` | Searched for gift of life triggered in automatic capture policy, trigger dispatch and option controls. |
| Open the screenshots directory | MISSING | `modules/game_notifications/templates/screenshot.otui:87` | Searched for open-folder and reveal-directory actions in screenshot settings and capture code. |
| Reset screenshot preferences | PRESENT | `modules/game_notifications/templates/screenshot.otui:80` | `apps/client/src/bevy_shell/game_ui/options.rs:167` → `apps/client/src/bevy_shell/game_ui/options/metadata.rs:135` |
| Read event-specific notification banners | PARTIAL | `modules/game_notifications/controllers/infobanner.lua:217` | Achievement banners queue at `apps/client/src/bevy_shell/game_ui/feature_dialogs/banner_queue.rs:60`. No equivalent queue entries found for cosmetic unlocks, proficiency or quest progression. |

## game_inspect

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Read inspection details | PRESENT | `modules/game_inspect/inspect.lua:159` | `apps/client/src/bevy_shell/game_ui/item_details/widgets.rs:135` → `apps/client/src/features/0132-inspection/input.rs:7` |
| Inspect individual equipment slots | PRESENT | `modules/game_inspect/inspect.lua:477` | `apps/client/src/bevy_shell/game_ui/item_details/widgets.rs:135` → `apps/client/src/bevy_shell/game_ui/item_details.rs:243` |
| Open inspected item in Cyclopedia | PRESENT | `modules/game_inspect/inspect.lua:518` | `apps/client/src/bevy_shell/game_ui/item_details.rs:225` → `apps/client/src/features/0132-inspection/input/related.rs:72` |
| Open inspected weapon proficiency | PRESENT | `modules/game_inspect/inspect.lua:529` | `apps/client/src/bevy_shell/game_ui/item_details.rs:225` → `apps/client/src/features/0132-inspection/input/related.rs:56` |
| Copy inspection information | PRESENT | `modules/game_inspect/inspect.lua:540` | `apps/client/src/bevy_shell/game_ui/item_details/widgets.rs:286` → `apps/client/src/bevy_shell/game_ui/item_details.rs:235` |
| Switch between equipment and outfit preview | MISSING | `modules/game_inspect/inspect.lua:496` | Searched for outfit-preview mode in inspection controls and reply projection. |
| View inspected character’s Wheel of Destiny | MISSING | `modules/game_inspect/inspect.lua:556` | Searched for inspection-to-wheel action carrying the inspected creature ID. |

## game_walk

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Interrupt walking | PRESENT | `modules/game_walk/walk.lua:92` | `apps/client/src/features/0064-target-move/input.rs:22` → `apps/client/src/features/0064-target-move/input.rs:83` |
| Turn without stepping | PRESENT | `modules/game_walk/walk.lua:193` | `apps/client/src/features/0064-target-move/input.rs:44` → `apps/client/src/features/0064-target-move/input.rs:83` |
| Walk with directional keys | PRESENT | `modules/game_walk/walk.lua:120` | `apps/client/src/features/0013-paced-movement/input.rs:48` → `apps/client/src/features/0013-paced-movement/input.rs:56` |
| Walk diagonally with a chord or directional keypad | PRESENT | `modules/game_walk/walk.lua:161` | `apps/client/src/features/0013-paced-movement/protocol.rs:129` → `apps/client/src/features/0013-paced-movement/protocol.rs:144` |

## game_joystick

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Walk by holding or dragging the virtual joystick | MISSING | `modules/game_joystick/joystick.lua:103` | Searched touch_input callers, touch promotion, native event dispatch and Bevy touch controls. Only the test drives the joystick input adapter. The play tick consumes steps but no production pointer path supplies its touch state. |

## game_tutorial

| Operation | Status | OTClient file and line | Oteryn evidence or what is missing |
| --- | --- | --- | --- |
| Preview a vocation before choosing | PARTIAL | `modules/game_tutorial/template/html/tutorial.html:61` | The offer lists selectable vocations at `apps/client/src/bevy_shell/game_ui/tutorial/vocation.rs:71`. The choice UI lacks the reference cards’ spell icons. |
| Choose and confirm a tutorial vocation | PARTIAL | `modules/game_tutorial/controller/tutorial_controller.lua:334` | Selecting a button submits its ID at `apps/client/src/bevy_shell/game_ui/tutorial/vocation.rs:166` through `apps/client/src/features/0504-tutorial-vocation/protocol.rs:22`. No separate confirmation before committing the choice. |

## Gaps ranked by player impact

Navigation and irreversible choices rank first, followed by inventory workflows, communication, information display and capture preferences. Items within a group share its priority.

### 1. Movement, session and character choices

- Revoke a pending party invitation (MISSING, `game_interface`)
- Force logout on a failing connection (MISSING, `game_interface`)
- Walk by holding or dragging the virtual joystick (MISSING, `game_joystick`)
- Preview a vocation before choosing (PARTIAL, `game_tutorial`)
- Choose and confirm a tutorial vocation (PARTIAL, `game_tutorial`)

### 2. Inventory handling and communication permissions

- Quick-loot a selected corpse (PARTIAL, `game_interface`)
- Stow all matching stacks (MISSING, `game_interface`)
- Stow container contents (MISSING, `game_interface`)
- Open a nested container in a separate window (PARTIAL, `game_interface`)
- Move an item into its parent container (PARTIAL, `game_interface`)
- Stow a chosen quantity of an item (PARTIAL, `game_interface`)
- Block all private messages (MISSING, `game_console`)
- Block all yells (MISSING, `game_console`)
- Bypass ignores for VIP speakers (MISSING, `game_console`)
- Suspend or enable the ignore list (MISSING, `game_console`)
- Sort nested container contents (MISSING, `game_containers`)
- Choose whether bulk move includes nested containers (MISSING, `game_containers`)
- Page through ordinary container contents (MISSING, `game_containers`)
- Distribute contents to designated Obtain containers (PARTIAL, `game_containers`)
- Sort containers before loose items (MISSING, `game_containers`)
- Minimise an individual open container (PARTIAL, `game_containers`)
- Read private messages over the game view (PARTIAL, `game_textmessage`)

### 3. Communication and character information

- Clear the current transcript (MISSING, `game_console`)
- Copy only the speaker’s name (MISSING, `game_console`)
- Suspend or enable the whitelist (MISSING, `game_console`)
- Lock or unlock chat dragging (MISSING, `game_console`)
- Select the channel transcript (PARTIAL, `game_console`)
- Hide the console (PARTIAL, `game_console`)
- Continue or close a violation-report conversation (PARTIAL, `game_console`)
- Show or hide expert PvP controls (MISSING, `game_inventory`)
- Check proficiency for the equipped weapon (PARTIAL, `game_inventory`)
- Read colour-coded loot messages (PARTIAL, `game_textmessage`)
- Read an item inscription with authorship (PARTIAL, `game_textwindow`)
- Switch between equipment and outfit preview (MISSING, `game_inspect`)
- View inspected character’s Wheel of Destiny (MISSING, `game_inspect`)

### 4. Game-view controls

- Show or hide top menu (MISSING, `game_interface`)
- Cycle game view mode (MISSING, `game_interface`)
- Clear all floating map text and screen messages (PARTIAL, `game_interface`)
- Choose status-bar size and placement (MISSING, `game_interface`)
- Choose which skills appear in the status bar (MISSING, `game_interface`)
- Hide the customisable status bar (MISSING, `game_interface`)

### 5. Notifications and screenshot controls

- Enable or disable all automatic captures (PARTIAL, `game_notifications`)
- Capture on bestiary entry unlocked (MISSING, `game_notifications`)
- Capture on bestiary entry completed (MISSING, `game_notifications`)
- Capture on treasure found (MISSING, `game_notifications`)
- Capture on valuable loot (MISSING, `game_notifications`)
- Capture on boss defeated (MISSING, `game_notifications`)
- Capture on death pve (PARTIAL, `game_notifications`)
- Capture on death pvp (PARTIAL, `game_notifications`)
- Capture on player kill (MISSING, `game_notifications`)
- Capture on player kill assist (MISSING, `game_notifications`)
- Capture on player attacking (MISSING, `game_notifications`)
- Capture on highest damage dealt (MISSING, `game_notifications`)
- Capture on highest healing done (MISSING, `game_notifications`)
- Capture on low health (MISSING, `game_notifications`)
- Capture on gift of life triggered (MISSING, `game_notifications`)
- Open the screenshots directory (MISSING, `game_notifications`)
- Read event-specific notification banners (PARTIAL, `game_notifications`)
