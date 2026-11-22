# Oteryn client parity audit · 22 November 2026

Oteryn base `888210d085970662027d94513046418191efd79f`. OTClient reference `mehah/otclient@4955a3fa22d5f5456ec6c658187fd2901545b750`.

The checkout contains 63 in-scope client and game modules. This inventory is incomplete. The evidence establishes individual working actions and gaps, but cannot establish a total missing-capability count. The reference inventory includes named bindings alongside player operations. Several bindings can implement one operation. Unverified rows cannot be included in a percentage.

| Unit | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE | Unverified |
|---|---:|---:|---:|---:|---:|
| Modules | 0 | 0 | 0 | 8 | 55 |
| Operations and reference bindings | 38 | 4 | 7 | 146 | 1880 |
The classified player-operation subset contains **38 PRESENT, 4 PARTIAL and 7 MISSING**. Eight module-level scope exclusions are supported. The larger table also counts individual binding records, including records within those excluded modules.

## Player impact

1. **Trades.** The main offer slot shows a question mark and quantity, so a player must inspect the offer to identify the item before accepting.
2. **Paid Prey rerolls.** A player must decide whether to proceed without knowing the gold charge. The confirmation explicitly warns about this.
3. **Combat target selection.** Four of the eight reference sort directions are missing from the battle list, including nearest first. Health ordering works in both directions.
4. **Party hunt settlement.** The party analyser exposes a refresh action. A group cannot use that panel to paste a hunt log and copy settlement instructions. See `game_lootsplitter`.
5. **Capture triggers.** The live screenshot page exposes four event switches. A player can capture a death, but cannot select separate PvP and PvE death policies there. Bestiary milestones, boss kills and tracked loot also lack individual controls. See the preference inventory.
6. **Update discovery.** The guide offers movement, combat, inventory and quest help. Its topic list omits historical major updates. The separate login bulletin displays operator news. See `game_compendium` and `apps/client/src/bevy_shell/login/bulletin.rs:31`.

## Modules

### client_bottommenu

PRESENT 1 | Unverified 14

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Browse calendar month | PRESENT | `modules/client_bottommenu/bottommenu.lua:42` | `apps/client/src/bevy_shell/login/bulletin.rs:137` → `apps/client/src/bevy_shell/login/bulletin.rs:32` |
| Random Hint | Unverified | `modules/client_bottommenu/bottommenu.otui:66` | — |
| Event Schedule | Unverified | `modules/client_bottommenu/bottommenu.otui:86` | — |
| Activate · onClickOnCalendar() | Unverified | `modules/client_bottommenu/bottommenu.otui:107` | — |
| Active | Unverified | `modules/client_bottommenu/bottommenu.otui:110` | — |
| Upcoming | Unverified | `modules/client_bottommenu/bottommenu.otui:124` | — |
| Boosted | Unverified | `modules/client_bottommenu/bottommenu.otui:148` | — |
| Creature | Unverified | `modules/client_bottommenu/bottommenu.otui:182` | — |
| Boss | Unverified | `modules/client_bottommenu/bottommenu.otui:211` | — |
| September 2023 | Unverified | `modules/client_bottommenu/calendar.otui:84` | — |
| Activate · modules.client_bottommenu.onClickOnPreviousCalendar() | Unverified | `modules/client_bottommenu/calendar.otui:94` | — |
| Activate · modules.client_bottommenu.onClickOnNextCalendar() | Unverified | `modules/client_bottommenu/calendar.otui:101` | — |
| * Event starts/ends at server save of this day. | Unverified | `modules/client_bottommenu/calendar.otui:115` | — |
| Close | Unverified | `modules/client_bottommenu/calendar.otui:130` | — |
| Activate · modules.client_bottommenu.onClickCloseCalendar() | Unverified | `modules/client_bottommenu/calendar.otui:135` | — |

### client_debug_info

NOT-APPLICABLE 15

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Showing and sending debug/stats informations | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otmod:1` | OTClient developer facility or OT server extension |
| Event · onCheckChange | NOT-APPLICABLE | `modules/client_debug_info/debug_info.lua:55` | OT extension |
| Debug Info | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:21` | OT extension |
| Proxies | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:54` | OT extension |
| Main | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:61` | OT extension |
| Render | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:68` | OT extension |
| Dispatcher | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:79` | OT extension |
| Lua | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:86` | OT extension |
| Lua by callback | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:93` | OT extension |
| Widgets & Objects | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:100` | OT extension |
| Enabled | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:104` | OT extension |
| Packets | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:116` | OT extension |
| Slow main functions | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:123` | OT extension |
| Slow render functions | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:130` | OT extension |
| Slow packets | NOT-APPLICABLE | `modules/client_debug_info/debug_info.otui:137` | OT extension |

### client_entergame

NOT-APPLICABLE 79

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Manages enter game and character list windows | NOT-APPLICABLE | `modules/client_entergame/entergame.otmod:1` | Platform admission and directory |
| Game API · safeLogout | NOT-APPLICABLE | `modules/client_entergame/characterlist.lua:379` | Platform |
| Game API · loginWorld | NOT-APPLICABLE | `modules/client_entergame/characterlist.lua:393` | Platform |
| Event · onUpdateNeeded | NOT-APPLICABLE | `modules/client_entergame/characterlist.lua:594` | Platform |
| Event · onGameStart | NOT-APPLICABLE | `modules/client_entergame/characterlist.lua:600` | Platform |
| Event · onGameEnd | NOT-APPLICABLE | `modules/client_entergame/characterlist.lua:606` | Platform |
| Event · onChildFocusChange | NOT-APPLICABLE | `modules/client_entergame/characterlist.lua:699` | Platform |
| Character List | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:187` | Platform |
| Character | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:259` | Platform |
| Activate · onSortButtonClick(self, 1) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:267` | Platform |
| Status | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:285` | Platform |
| Activate · onSortButtonClick(self, 2) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:293` | Platform |
| Level | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:311` | Platform |
| Activate · onSortButtonClick(self, 3) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:319` | Platform |
| Vocation | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:337` | Platform |
| Activate · onSortButtonClick(self, 4) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:345` | Platform |
| World | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:363` | Platform |
| Activate · onSortButtonClick(self, 5) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:371` | Platform |
| Account Status | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:433` | Platform |
| Free Account | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:458` | Platform |
| Get Premium | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:478` | Platform |
| Activate · onPremiumButtonClick(self) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:482` | Platform |
| Premium Benefits include: | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:520` | Platform |
| No level cap while training your skills offline | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:548` | Platform |
| Tame mounts to travel faster | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:576` | Platform |
| Imbue items with stronger effect | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:602` | Platform |
| ... and many more! | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:615` | Platform |
| Show Hidden Characters | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:660` | Platform |
| Show Outfit | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:693` | Platform |
| Ok | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:716` | Platform |
| Activate · CharacterList.doLogin() | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:721` | Platform |
| Cancel | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:725` | Platform |
| Activate · CharacterList.hide(true) | NOT-APPLICABLE | `modules/client_entergame/characterlist.otui:729` | Platform |
| Event · onFocusChange | NOT-APPLICABLE | `modules/client_entergame/createAccount.lua:378` | Platform |
| Event · onTextChange | NOT-APPLICABLE | `modules/client_entergame/createAccount.lua:383` | Platform |
| All | NOT-APPLICABLE | `modules/client_entergame/createAccount.lua:616` | Platform |
| Event · onOptionChange | NOT-APPLICABLE | `modules/client_entergame/createAccount.lua:629` | Platform |
| Email: | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:116` | Platform |
| Password: | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:129` | Platform |
| Repeat Password: | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:140` | Platform |
| Character Name: | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:193` | Platform |
| Activate · onClickSuggestName() | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:212` | Platform |
| male | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:230` | Platform |
| female | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:236` | Platform |
| Canary (America) | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:263` | Platform |
| Activate · toggleMainPanels(true) | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:276` | Platform |
| Activate · toggleCreateAccount(true) | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:291` | Platform |
| Activate · onClickStartPlaying() | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:303` | Platform |
| World Location: | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:563` | Platform |
| Activate · toggleMainPanels(false) | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:720` | Platform |
| Activate · onClickOkChangeWorld() | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:730` | Platform |
| Activate · onClickResetGameWorld() | NOT-APPLICABLE | `modules/client_entergame/createAccount.otui:740` | Platform |
| Event · onCheckChange | NOT-APPLICABLE | `modules/client_entergame/entergame.lua:251` | Platform |
| Game API · setClientVersion | NOT-APPLICABLE | `modules/client_entergame/entergame.lua:629` | Platform |
| Game API · setProtocolVersion | NOT-APPLICABLE | `modules/client_entergame/entergame.lua:630` | Platform |
| Game API · chooseRsa | NOT-APPLICABLE | `modules/client_entergame/entergame.lua:631` | Platform |
| Enter Game | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:2` | Platform |
| Acc Name: | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:34` | Platform |
| Remember password | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:64` | Platform |
| Remember account and password when starts client | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:65` | Platform |
| Forgot password and/or email | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:90` | Platform |
| Activate · g_platform.openUrl(Services.websites) | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:99` | Platform |
| Stay logged during session | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:110` | Platform |
| Server | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:130` | Platform |
| Server list | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:138` | Platform |
| Activate · EnterGame.showServerList() | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:142` | Platform |
| Make sure that your client uses\nthe correct game client version | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:146` | Platform |
| Client Version | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:155` | Platform |
| Port | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:176` | Platform |
| Auto login | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:193` | Platform |
| Open charlist automatically when starting client | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:194` | Platform |
| Enable HTTP login | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:203` | Platform |
| If login fails using HTTPS (encrypted) try with HTTP (unencrypted) | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:204` | Platform |
| Login | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:217` | Platform |
| Activate · EnterGame.doLogin() | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:222` | Platform |
| Create New Account | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:226` | Platform |
| Activate · \| | NOT-APPLICABLE | `modules/client_entergame/entergame.otui:231` | Platform |
| Waiting List | NOT-APPLICABLE | `modules/client_entergame/waitinglist.otui:3` | Platform |
| Activate · CharacterList.cancelWait() | NOT-APPLICABLE | `modules/client_entergame/waitinglist.otui:45` | Platform |

### client_options

Unverified 174

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onSelectionChange | Unverified | `modules/client_options/keybins.lua:711` | — |
| Event · onTextChange | Unverified | `modules/client_options/keybins.lua:725` | — |
| Event · onOptionChange | Unverified | `modules/client_options/options.lua:197` | — |
| Regular Controls | Unverified | `modules/client_options/options.lua:201` | — |
| Classic Controls | Unverified | `modules/client_options/options.lua:202` | — |
| Left Smart-Click | Unverified | `modules/client_options/options.lua:203` | — |
| Loot: Right | Unverified | `modules/client_options/options.lua:205` | — |
| Loot: SHIFT+Right | Unverified | `modules/client_options/options.lua:206` | — |
| Loot: Left | Unverified | `modules/client_options/options.lua:207` | — |
| (auto-select) | Unverified | `modules/client_options/options.lua:259` | — |
| General Stats | Unverified | `modules/client_options/options.otui:75` | — |
| Options | Unverified | `modules/client_options/options.otui:201` | — |
| Show Advanced Options | Unverified | `modules/client_options/options.otui:254` | — |
| Ok | Unverified | `modules/client_options/options.otui:260` | — |
| Activate · \| | Unverified | `modules/client_options/options.otui:266` | — |
| Apply | Unverified | `modules/client_options/options.otui:271` | — |
| Cancel | Unverified | `modules/client_options/options.otui:281` | — |
| Show connection ping | Unverified | `modules/client_options/styles/controls/general.otui:21` | — |
| Display connection speed to the server (milliseconds) | Unverified | `modules/client_options/styles/controls/general.otui:22` | — |
| Mouse Control: | Unverified | `modules/client_options/styles/controls/general.otui:43` | — |
| Allow auto chase override | Unverified | `modules/client_options/styles/controls/general.otui:76` | — |
| Right-click talk to NPC | Unverified | `modules/client_options/styles/controls/general.otui:87` | — |
| Enter to disable chat | Unverified | `modules/client_options/styles/controls/general.otui:98` | — |
| Enables disabling chat and enabling WASD when pressed Enter. | Unverified | `modules/client_options/styles/controls/general.otui:99` | — |
| Move stacks directly | Unverified | `modules/client_options/styles/controls/general.otui:110` | — |
| Enable smart walking | Unverified | `modules/client_options/styles/controls/general.otui:121` | — |
| Will detect when to use diagonal step based on the\nkeys you are pressing | Unverified | `modules/client_options/styles/controls/general.otui:122` | — |
| Open containers maximized | Unverified | `modules/client_options/styles/controls/general.otui:133` | — |
| Display text messages | Unverified | `modules/client_options/styles/controls/general.otui:144` | — |
| Hotkey delay: 30ms | Unverified | `modules/client_options/styles/controls/general.otui:156` | — |
| Walk delay after turn: 50ms | Unverified | `modules/client_options/styles/controls/general.otui:174` | — |
| Walk delay after teleport: 200ms | Unverified | `modules/client_options/styles/controls/general.otui:192` | — |
| Walk delay after floor change: 200ms | Unverified | `modules/client_options/styles/controls/general.otui:210` | — |
| Hotkeys Manager | Unverified | `modules/client_options/styles/controls/general.otui:221` | — |
| Activate · modules.game_hotkeys.show() | Unverified | `modules/client_options/styles/controls/general.otui:222` | — |
| This action must be bound to a single key. | Unverified | `modules/client_options/styles/controls/key_edit.otui:32` | — |
| This hotkey is already in use. | Unverified | `modules/client_options/styles/controls/key_edit.otui:40` | — |
| Clear | Unverified | `modules/client_options/styles/controls/key_edit.otui:61` | — |
| Add | Unverified | `modules/client_options/styles/controls/keybinds.otui:98` | — |
| Activate · addNewPreset() | Unverified | `modules/client_options/styles/controls/keybinds.otui:99` | — |
| Copy | Unverified | `modules/client_options/styles/controls/keybinds.otui:104` | — |
| Activate · copyPreset() | Unverified | `modules/client_options/styles/controls/keybinds.otui:105` | — |
| Rename | Unverified | `modules/client_options/styles/controls/keybinds.otui:110` | — |
| Remove | Unverified | `modules/client_options/styles/controls/keybinds.otui:116` | — |
| Auto-Switch Hotkey Preset | Unverified | `modules/client_options/styles/controls/keybinds.otui:128` | — |
| Chat Mode On | Unverified | `modules/client_options/styles/controls/keybinds.otui:154` | — |
| Chat Mode Off | Unverified | `modules/client_options/styles/controls/keybinds.otui:160` | — |
| Type to search for a hotkey: | Unverified | `modules/client_options/styles/controls/keybinds.otui:187` | — |
| Action | Unverified | `modules/client_options/styles/controls/keybinds.otui:231` | — |
| Primary Key | Unverified | `modules/client_options/styles/controls/keybinds.otui:237` | — |
| Secondary Key | Unverified | `modules/client_options/styles/controls/keybinds.otui:243` | — |
| New Action | Unverified | `modules/client_options/styles/controls/keybinds.otui:275` | — |
| Reset | Unverified | `modules/client_options/styles/controls/keybinds.otui:285` | — |
| Enable lights | Unverified | `modules/client_options/styles/graphics/effects.otui:14` | — |
| Draw Effect On Top | Unverified | `modules/client_options/styles/graphics/effects.otui:77` | — |
| Draw effect after drawing the entire floor. | Unverified | `modules/client_options/styles/graphics/effects.otui:78` | — |
| Limit Visible Dimension | Unverified | `modules/client_options/styles/graphics/effects.otui:89` | — |
| The limit is based on your maximum range. | Unverified | `modules/client_options/styles/graphics/effects.otui:90` | — |
| Opacity Of Spells Effects | Unverified | `modules/client_options/styles/graphics/effects.otui:143` | — |
| V-Sync | Unverified | `modules/client_options/styles/graphics/graphics.otui:41` | — |
| Limits your fps based on monitor refresh rate | Unverified | `modules/client_options/styles/graphics/graphics.otui:42` | — |
| Show frame rate | Unverified | `modules/client_options/styles/graphics/graphics.otui:47` | — |
| Game framerate limit: %s | Unverified | `modules/client_options/styles/graphics/graphics.otui:53` | — |
| Optimize FPS | Unverified | `modules/client_options/styles/graphics/graphics.otui:77` | — |
| Try to optimize when the frame rate is below 60. VISUAL PROBLEMS MAY OCCUR | Unverified | `modules/client_options/styles/graphics/graphics.otui:78` | — |
| Force Effect Optimization | Unverified | `modules/client_options/styles/graphics/graphics.otui:89` | — |
| Will avoid drawing effects on certain occasions. | Unverified | `modules/client_options/styles/graphics/graphics.otui:90` | — |
| Async texture loading | Unverified | `modules/client_options/styles/graphics/graphics.otui:101` | — |
| This option makes textures load asynchronously and uses less RAM. | Unverified | `modules/client_options/styles/graphics/graphics.otui:102` | — |
| Display creature names | Unverified | `modules/client_options/styles/interface/HUD.otui:23` | — |
| Display creature health bars | Unverified | `modules/client_options/styles/interface/HUD.otui:34` | — |
| Display player mana bar | Unverified | `modules/client_options/styles/interface/HUD.otui:45` | — |
| Display player Harmony bar | Unverified | `modules/client_options/styles/interface/HUD.otui:56` | — |
| Can be enabled only in version 15.00 or higher | Unverified | `modules/client_options/styles/interface/HUD.otui:57` | — |
| Joystick in RightPanel | Unverified | `modules/client_options/styles/interface/HUD.otui:73` | — |
| Special Conditions | Unverified | `modules/client_options/styles/interface/HUD.otui:145` | — |
| Show in HUD | Unverified | `modules/client_options/styles/interface/HUD.otui:159` | — |
| Show in Bar | Unverified | `modules/client_options/styles/interface/HUD.otui:182` | — |
| Show Customisable Status Bars | Unverified | `modules/client_options/styles/interface/HUD.otui:254` | — |
| Check this box to show the status bars for your character\ | Unverified | `modules/client_options/styles/interface/HUD.otui:255` | — |
| Show Status Bars | Unverified | `modules/client_options/styles/interface/HUD.otui:265` | — |
| Check this box to show your character\ | Unverified | `modules/client_options/styles/interface/HUD.otui:266` | — |
| Show Bottom Action Bars: | Unverified | `modules/client_options/styles/interface/actionbars.otui:20` | — |
| All | Unverified | `modules/client_options/styles/interface/actionbars.otui:28` | — |
| Bar 1 | Unverified | `modules/client_options/styles/interface/actionbars.otui:36` | — |
| Bar 2 | Unverified | `modules/client_options/styles/interface/actionbars.otui:45` | — |
| Bar 3 | Unverified | `modules/client_options/styles/interface/actionbars.otui:53` | — |
| Show Left Action Bars: | Unverified | `modules/client_options/styles/interface/actionbars.otui:61` | — |
| Show Right Action Bars: | Unverified | `modules/client_options/styles/interface/actionbars.otui:98` | — |
| Show Assigned Hotkey for Action Button | Unverified | `modules/client_options/styles/interface/actionbars.otui:144` | — |
| Display the hotkey assigned to each action button | Unverified | `modules/client_options/styles/interface/actionbars.otui:145` | — |
| Show Amount of Assigned Objects | Unverified | `modules/client_options/styles/interface/actionbars.otui:156` | — |
| Display the quantity of items assigned to action buttons | Unverified | `modules/client_options/styles/interface/actionbars.otui:157` | — |
| Show Spell Parameters | Unverified | `modules/client_options/styles/interface/actionbars.otui:168` | — |
| Display spell parameters on action buttons | Unverified | `modules/client_options/styles/interface/actionbars.otui:169` | — |
| Show Graphical Cooldown | Unverified | `modules/client_options/styles/interface/actionbars.otui:180` | — |
| Display visual cooldown animation on action buttons | Unverified | `modules/client_options/styles/interface/actionbars.otui:181` | — |
| Show Cooldown in Seconds | Unverified | `modules/client_options/styles/interface/actionbars.otui:192` | — |
| Display remaining cooldown time in seconds | Unverified | `modules/client_options/styles/interface/actionbars.otui:193` | — |
| Show Action Button Tooltip | Unverified | `modules/client_options/styles/interface/actionbars.otui:204` | — |
| Display tooltips when hovering over action buttons | Unverified | `modules/client_options/styles/interface/actionbars.otui:205` | — |
| Clear Bottom Action Bars: | Unverified | `modules/client_options/styles/interface/actionbars.otui:215` | — |
| Clear all assignments from bottom action bars | Unverified | `modules/client_options/styles/interface/actionbars.otui:230` | — |
| Activate · modules.game_actionbar.resetAction(1) | Unverified | `modules/client_options/styles/interface/actionbars.otui:241` | — |
| Activate · modules.game_actionbar.resetAction(2) | Unverified | `modules/client_options/styles/interface/actionbars.otui:251` | — |
| Activate · modules.game_actionbar.resetAction(3) | Unverified | `modules/client_options/styles/interface/actionbars.otui:261` | — |
| Clear Left Action Bars: | Unverified | `modules/client_options/styles/interface/actionbars.otui:263` | — |
| Activate · modules.game_actionbar.resetAction(4) | Unverified | `modules/client_options/styles/interface/actionbars.otui:279` | — |
| Activate · modules.game_actionbar.resetAction(5) | Unverified | `modules/client_options/styles/interface/actionbars.otui:289` | — |
| Activate · modules.game_actionbar.resetAction(6) | Unverified | `modules/client_options/styles/interface/actionbars.otui:299` | — |
| Clear Right Action Bars: | Unverified | `modules/client_options/styles/interface/actionbars.otui:301` | — |
| Activate · modules.game_actionbar.resetAction(7) | Unverified | `modules/client_options/styles/interface/actionbars.otui:317` | — |
| Activate · modules.game_actionbar.resetAction(8) | Unverified | `modules/client_options/styles/interface/actionbars.otui:327` | — |
| reset | Unverified | `modules/client_options/styles/interface/actionbars.otui:341` | — |
| Activate · modules.game_actionbar.resetActionBars() | Unverified | `modules/client_options/styles/interface/actionbars.otui:347` | — |
| Show info messages | Unverified | `modules/client_options/styles/interface/console.otui:12` | — |
| Show event messages | Unverified | `modules/client_options/styles/interface/console.otui:23` | — |
| Show status messages | Unverified | `modules/client_options/styles/interface/console.otui:34` | — |
| Show others status messages in console | Unverified | `modules/client_options/styles/interface/console.otui:45` | — |
| Show timestamps | Unverified | `modules/client_options/styles/interface/console.otui:56` | — |
| Show levels | Unverified | `modules/client_options/styles/interface/console.otui:67` | — |
| Show private messages | Unverified | `modules/client_options/styles/interface/console.otui:78` | — |
| Show private messages on screen | Unverified | `modules/client_options/styles/interface/console.otui:89` | — |
| Show loot messages on screen | Unverified | `modules/client_options/styles/interface/console.otui:100` | — |
| Show highlighted text underline | Unverified | `modules/client_options/styles/interface/console.otui:111` | — |
| Chat scrollback: 100 lines | Unverified | `modules/client_options/styles/interface/console.otui:122` | — |
| Highlight mouse target | Unverified | `modules/client_options/styles/interface/interface.otui:12` | — |
| Use Native Mouse Cursor | Unverified | `modules/client_options/styles/interface/interface.otui:24` | — |
| Show Animated Mouse Cursor | Unverified | `modules/client_options/styles/interface/interface.otui:36` | — |
| Show item icon while dragging | Unverified | `modules/client_options/styles/interface/interface.otui:47` | — |
| Show left panel | Unverified | `modules/client_options/styles/interface/interface.otui:63` | — |
| Show an extra right panel | Unverified | `modules/client_options/styles/interface/interface.otui:77` | — |
| Show spell group cooldowns | Unverified | `modules/client_options/styles/interface/interface.otui:88` | — |
| Can be enabled only in version 8.7x or higher | Unverified | `modules/client_options/styles/interface/interface.otui:89` | — |
| Show Info Banner | Unverified | `modules/client_options/styles/interface/interface.otui:100` | — |
| Tick this box to display an info banner for certain in-game events. | Unverified | `modules/client_options/styles/interface/interface.otui:101` | — |
| Show Expiry In Inventory | Unverified | `modules/client_options/styles/interface/interface.otui:163` | — |
| Check this box to see how much time or how many charges are left \non your equipped items | Unverified | `modules/client_options/styles/interface/interface.otui:164` | — |
| Show Expiry In Containers | Unverified | `modules/client_options/styles/interface/interface.otui:170` | — |
| Check this box to see how mmuch time or how many charges the \nitems in your open containers have left | Unverified | `modules/client_options/styles/interface/interface.otui:171` | — |
| Show Expiry On Unused Items | Unverified | `modules/client_options/styles/interface/interface.otui:177` | — |
| Check this box to see how much time or how many charges are left\n on items that have not been used yet | Unverified | `modules/client_options/styles/interface/interface.otui:178` | — |
| Wiki | Unverified | `modules/client_options/styles/misc/help.otui:11` | — |
| Activate · g_platform.openUrl("https://github.com/mehah/otclient/wiki") | Unverified | `modules/client_options/styles/misc/help.otui:17` | — |
| Info | Unverified | `modules/client_options/styles/misc/help.otui:20` | — |
| Clear Cache | Unverified | `modules/client_options/styles/misc/help.otui:37` | — |
| Change language | Unverified | `modules/client_options/styles/misc/help.otui:49` | — |
| Spells | Unverified | `modules/client_options/styles/sound/battleSounds.otui:34` | — |
| Attack | Unverified | `modules/client_options/styles/sound/battleSounds.otui:51` | — |
| Healing | Unverified | `modules/client_options/styles/sound/battleSounds.otui:55` | — |
| Support | Unverified | `modules/client_options/styles/sound/battleSounds.otui:59` | — |
| Weapons | Unverified | `modules/client_options/styles/sound/battleSounds.otui:65` | — |
| Creature Noises | Unverified | `modules/client_options/styles/sound/battleSounds.otui:144` | — |
| Death | Unverified | `modules/client_options/styles/sound/battleSounds.otui:150` | — |
| Attacks And Spells | Unverified | `modules/client_options/styles/sound/battleSounds.otui:154` | — |
| Anthem | Unverified | `modules/client_options/styles/sound/sound.otui:63` | — |
| Food and Beverages | Unverified | `modules/client_options/styles/sound/sound.otui:110` | — |
| Move Item | Unverified | `modules/client_options/styles/sound/sound.otui:115` | — |
| UI interactions | Unverified | `modules/client_options/styles/sound/uiSounds.otui:42` | — |
| Join/leave Party | Unverified | `modules/client_options/styles/sound/uiSounds.otui:53` | — |
| VIP login/logout | Unverified | `modules/client_options/styles/sound/uiSounds.otui:57` | — |
| Console Messages | Unverified | `modules/client_options/styles/sound/uiSounds.otui:69` | — |
| Party | Unverified | `modules/client_options/styles/sound/uiSounds.otui:82` | — |
| Guild | Unverified | `modules/client_options/styles/sound/uiSounds.otui:86` | — |
| Private Messages in Local Chat | Unverified | `modules/client_options/styles/sound/uiSounds.otui:90` | — |
| Private Messages | Unverified | `modules/client_options/styles/sound/uiSounds.otui:94` | — |
| Check this box to hear a sound when receiving a private message | Unverified | `modules/client_options/styles/sound/uiSounds.otui:95` | — |
| NPCs | Unverified | `modules/client_options/styles/sound/uiSounds.otui:106` | — |
| Global | Unverified | `modules/client_options/styles/sound/uiSounds.otui:110` | — |
| Check this box to hear a sound for global messages | Unverified | `modules/client_options/styles/sound/uiSounds.otui:111` | — |
| Team Finder | Unverified | `modules/client_options/styles/sound/uiSounds.otui:122` | — |
| Raid Announcements | Unverified | `modules/client_options/styles/sound/uiSounds.otui:126` | — |
| System Announcements | Unverified | `modules/client_options/styles/sound/uiSounds.otui:130` | — |
| Check this box to hear a sound for system announcements | Unverified | `modules/client_options/styles/sound/uiSounds.otui:131` | — |

### client_serverlist

NOT-APPLICABLE 16

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Manages a server list of previously entered servers | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otmod:1` | Platform admission and directory |
| New Server | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:3` | Platform |
| Host | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:11` | Platform |
| Port | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:25` | Platform |
| Protocol | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:40` | Platform |
| Add | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:61` | Platform |
| Activate · AddServer.add() | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:66` | Platform |
| Cancel | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:70` | Platform |
| Activate · AddServer.hide() | NOT-APPLICABLE | `modules/client_serverlist/addserver.otui:75` | Platform |
| Activate · ServerList.remove(self:getParent()) | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:55` | Platform |
| Server List | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:59` | Platform |
| Add new server | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:91` | Platform |
| Activate · AddServer.show() | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:94` | Platform |
| Select | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:98` | Platform |
| Activate · ServerList.select() | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:103` | Platform |
| Activate · ServerList.hide() | NOT-APPLICABLE | `modules/client_serverlist/serverlist.otui:111` | Platform |

### client_terminal

NOT-APPLICABLE 5

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Terminal for executing lua functions | NOT-APPLICABLE | `modules/client_terminal/terminal.otmod:1` | OTClient developer facility or OT server extension |
| Game API · setPingDelay | NOT-APPLICABLE | `modules/client_terminal/commands.lua:36` | OT extension |
| Event · onTextChange | NOT-APPLICABLE | `modules/client_terminal/terminal.lua:165` | OT extension |
| Event · onScrollChange | NOT-APPLICABLE | `modules/client_terminal/terminal.lua:191` | OT extension |
| Terminal | NOT-APPLICABLE | `modules/client_terminal/terminal.otui:32` | OT extension |

### client_topmenu

Unverified 3

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/client_topmenu/topmenu.lua:89` | — |
| Event · onGameEnd | Unverified | `modules/client_topmenu/topmenu.lua:90` | — |
| Min:  | Unverified | `modules/client_topmenu/topmenu.lua:273` | — |

### game_actionbar

Unverified 25

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onManaChange | Unverified | `modules/game_actionbar/game_actionbar.lua:262` | — |
| Event · onSoulChange | Unverified | `modules/game_actionbar/game_actionbar.lua:263` | — |
| Event · onLevelChange | Unverified | `modules/game_actionbar/game_actionbar.lua:264` | — |
| Event · onSpellsChange | Unverified | `modules/game_actionbar/game_actionbar.lua:265` | — |
| Event · onSpellCooldown | Unverified | `modules/game_actionbar/game_actionbar.lua:270` | — |
| Event · onMultiUseCooldown | Unverified | `modules/game_actionbar/game_actionbar.lua:271` | — |
| Event · onSpellGroupCooldown | Unverified | `modules/game_actionbar/game_actionbar.lua:272` | — |
| Event · onSelectionChange | Unverified | `modules/game_actionbar/logics/ActionAssignmentWindows.lua:127` | — |
| Event · onCheckChange | Unverified | `modules/game_actionbar/logics/ActionAssignmentWindows.lua:486` | — |
| Clear Assigned Action | Unverified | `modules/game_actionbar/logics/ActionAssignmentWindows.lua:646` | — |
| Game API · useWith | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:298` | — |
| Game API · equipItemId | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:338` | — |
| Game API · closeContainerByItemId | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:343` | — |
| Game API · useInventoryItem | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:345` | — |
| Game API · useInventoryItemWith | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:350` | — |
| Game API · talk | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:378` | — |
| Edit Object | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:812` | — |
| Assign Object | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:816` | — |
| Close Multi-Action | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:846` | — |
| Clear Action | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:859` | — |
| ID:  | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:876` | — |
| Event · onGeometryChange | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:914` | — |
| Event · onVisibilityChange | Unverified | `modules/game_actionbar/logics/ActionButtonLogic.lua:915` | — |
| Activate · modules.game_actionbar.moveActionButtons(self) | Unverified | `modules/game_actionbar/otui/actionbar.otui:112` | — |
| Level: 0 | Unverified | `modules/game_actionbar/otui/style.otui:59` | — |

### game_analyser

Unverified 104

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_analyser/analyser.lua:167` | — |
| Event · onGameEnd | Unverified | `modules/game_analyser/analyser.lua:168` | — |
| Event · onSupplyTracker | Unverified | `modules/game_analyser/analyser.lua:169` | — |
| Event · onImpactTracker | Unverified | `modules/game_analyser/analyser.lua:171` | — |
| Event · onKillTracker | Unverified | `modules/game_analyser/analyser.lua:172` | — |
| Event · onBossCooldown | Unverified | `modules/game_analyser/analyser.lua:174` | — |
| Event · onUpdateExperience | Unverified | `modules/game_analyser/analyser.lua:175` | — |
| Event · onExperienceChange | Unverified | `modules/game_analyser/analyser.lua:179` | — |
| Event · onLevelChange | Unverified | `modules/game_analyser/analyser.lua:180` | — |
| Event · onPartyMembersChange | Unverified | `modules/game_analyser/analyser.lua:181` | — |
| Event · onShieldChange | Unverified | `modules/game_analyser/analyser.lua:185` | — |
| Analytics Selector | Unverified | `modules/game_analyser/analyser.otui:4` | — |
| Activate · toggleAnalysers('huntingButton') | Unverified | `modules/game_analyser/analyser.otui:53` | — |
| Activate · toggleAnalysers('lootButton') | Unverified | `modules/game_analyser/analyser.otui:69` | — |
| Activate · toggleAnalysers('supplyButton') | Unverified | `modules/game_analyser/analyser.otui:85` | — |
| Activate · toggleAnalysers('impactButton') | Unverified | `modules/game_analyser/analyser.otui:101` | — |
| Activate · toggleAnalysers('damageButton') | Unverified | `modules/game_analyser/analyser.otui:117` | — |
| Activate · toggleAnalysers('xpButton') | Unverified | `modules/game_analyser/analyser.otui:133` | — |
| Activate · toggleAnalysers('dropButton') | Unverified | `modules/game_analyser/analyser.otui:149` | — |
| Activate · toggleAnalysers('partyButton') | Unverified | `modules/game_analyser/analyser.otui:165` | — |
| Activate · toggleAnalysers('bossButton') | Unverified | `modules/game_analyser/analyser.otui:181` | — |
| Event · onFocusChange | Unverified | `modules/game_analyser/classes/BossCooldown.lua:143` | — |
| Unknown Boss | Unverified | `modules/game_analyser/classes/BossCooldown.lua:295` | — |
| Remove | Unverified | `modules/game_analyser/classes/DropTrackerAnalyser.lua:488` | — |
| Remove All | Unverified | `modules/game_analyser/classes/DropTrackerAnalyser.lua:492` | — |
| Start New Session | Unverified | `modules/game_analyser/classes/HuntingAnalyser.lua:199` | — |
| Copy to Clipboard | Unverified | `modules/game_analyser/classes/HuntingAnalyser.lua:203` | — |
| Save to File | Unverified | `modules/game_analyser/classes/HuntingAnalyser.lua:204` | — |
| Export to Json | Unverified | `modules/game_analyser/classes/HuntingAnalyser.lua:205` | — |
| Game API · sendPartyAnalyzerReset | Unverified | `modules/game_analyser/classes/HuntingAnalyser.lua:253` | — |
| Reset Data | Unverified | `modules/game_analyser/classes/ImpactAnalyser.lua:552` | — |
| Reset All-Time High | Unverified | `modules/game_analyser/classes/ImpactAnalyser.lua:553` | — |
| Set DPS target | Unverified | `modules/game_analyser/classes/ImpactAnalyser.lua:562` | — |
| Set HPS target | Unverified | `modules/game_analyser/classes/ImpactAnalyser.lua:574` | — |
| Set Loot Per Hour Target | Unverified | `modules/game_analyser/classes/LootAnalyser.lua:149` | — |
| Reset Data of Current Party Session | Unverified | `modules/game_analyser/classes/PartyHuntAnalyser.lua:482` | — |
| Use %s Prices | Unverified | `modules/game_analyser/classes/PartyHuntAnalyser.lua:487` | — |
| Game API · sendPartyAnalyzerPriceType | Unverified | `modules/game_analyser/classes/PartyHuntAnalyser.lua:488` | — |
| Copy to LootSplitter | Unverified | `modules/game_analyser/classes/PartyHuntAnalyser.lua:497` | — |
| Set Supply Per Hour Target | Unverified | `modules/game_analyser/classes/SupplyAnalyser.lua:240` | — |
| Set XP Per Hour Target | Unverified | `modules/game_analyser/classes/XPAanalyser.lua:446` | — |
| Test Boss Name | Unverified | `modules/game_analyser/styles/boss.otui:22` | — |
| No Cooldown | Unverified | `modules/game_analyser/styles/boss.otui:35` | — |
| Boss Cooldowns | Unverified | `modules/game_analyser/styles/boss.otui:54` | — |
| Activate · modules.game_analyser.clearSearch() | Unverified | `modules/game_analyser/styles/boss.otui:120` | — |
| Activate · modules.game_analyser.toggleBossCDFocus(true) | Unverified | `modules/game_analyser/styles/boss.otui:123` | — |
| Set DPS Target | Unverified | `modules/game_analyser/styles/dpshpsTarget.otui:46` | — |
| Cancel | Unverified | `modules/game_analyser/styles/dpshpsTarget.otui:75` | — |
| Ok | Unverified | `modules/game_analyser/styles/dpshpsTarget.otui:83` | — |
| Drop Tracker Configuration | Unverified | `modules/game_analyser/styles/dropTarget.otui:4` | — |
| Item Name | Unverified | `modules/game_analyser/styles/droptracker.otui:26` | — |
| Loot Drops: | Unverified | `modules/game_analyser/styles/droptracker.otui:37` | — |
| 0 | Unverified | `modules/game_analyser/styles/droptracker.otui:47` | — |
| Name | Unverified | `modules/game_analyser/styles/droptracker.otui:85` | — |
| (0) | Unverified | `modules/game_analyser/styles/droptracker.otui:94` | — |
| Drop Tracker | Unverified | `modules/game_analyser/styles/droptracker.otui:105` | — |
| Activate · modules.game_cyclopedia.show("items") | Unverified | `modules/game_analyser/styles/droptracker.otui:163` | — |
| Hunting Analyser | Unverified | `modules/game_analyser/styles/hunting.otui:4` | — |
| Session: | Unverified | `modules/game_analyser/styles/hunting.otui:40` | — |
| 00:00h | Unverified | `modules/game_analyser/styles/hunting.otui:49` | — |
| Raw XP Gain: | Unverified | `modules/game_analyser/styles/hunting.otui:58` | — |
| XP Gain: | Unverified | `modules/game_analyser/styles/hunting.otui:79` | — |
| Raw XP/h: | Unverified | `modules/game_analyser/styles/hunting.otui:99` | — |
| XP /h: | Unverified | `modules/game_analyser/styles/hunting.otui:120` | — |
| Loot: | Unverified | `modules/game_analyser/styles/hunting.otui:140` | — |
| Supplies: | Unverified | `modules/game_analyser/styles/hunting.otui:169` | — |
| Balance: | Unverified | `modules/game_analyser/styles/hunting.otui:198` | — |
| Damage: | Unverified | `modules/game_analyser/styles/hunting.otui:227` | — |
| Damage/h: | Unverified | `modules/game_analyser/styles/hunting.otui:247` | — |
| Healing: | Unverified | `modules/game_analyser/styles/hunting.otui:267` | — |
| Healing/h: | Unverified | `modules/game_analyser/styles/hunting.otui:287` | — |
| Killed Monsters: | Unverified | `modules/game_analyser/styles/hunting.otui:307` | — |
| None | Unverified | `modules/game_analyser/styles/hunting.otui:325` | — |
| Looted Items: | Unverified | `modules/game_analyser/styles/hunting.otui:334` | — |
| No Data | Unverified | `modules/game_analyser/styles/impact.otui:24` | — |
| Impact Analyser | Unverified | `modules/game_analyser/styles/impact.otui:32` | — |
| Damage | Unverified | `modules/game_analyser/styles/impact.otui:67` | — |
| Total: | Unverified | `modules/game_analyser/styles/impact.otui:75` | — |
| Max-DPS: | Unverified | `modules/game_analyser/styles/impact.otui:94` | — |
| All-Time High: | Unverified | `modules/game_analyser/styles/impact.otui:113` | — |
| Target-DPS: | Unverified | `modules/game_analyser/styles/impact.otui:139` | — |
| DPS: | Unverified | `modules/game_analyser/styles/impact.otui:177` | — |
| Damage Types | Unverified | `modules/game_analyser/styles/impact.otui:241` | — |
| Healing | Unverified | `modules/game_analyser/styles/impact.otui:268` | — |
| Max-HPS: | Unverified | `modules/game_analyser/styles/impact.otui:297` | — |
| Target-HPS: | Unverified | `modules/game_analyser/styles/impact.otui:342` | — |
| HPS: | Unverified | `modules/game_analyser/styles/impact.otui:380` | — |
| Input Analyser | Unverified | `modules/game_analyser/styles/input.otui:22` | — |
| Received Damage | Unverified | `modules/game_analyser/styles/input.otui:59` | — |
| Damage Sources | Unverified | `modules/game_analyser/styles/input.otui:178` | — |
| Loot Analyser | Unverified | `modules/game_analyser/styles/loot.otui:12` | — |
| Gold Value: | Unverified | `modules/game_analyser/styles/loot.otui:48` | — |
| Per Hour: | Unverified | `modules/game_analyser/styles/loot.otui:75` | — |
| Target: | Unverified | `modules/game_analyser/styles/loot.otui:129` | — |
| Set Loot Target | Unverified | `modules/game_analyser/styles/lootTarget.otui:4` | — |
| Balance | Unverified | `modules/game_analyser/styles/partyhunt.otui:30` | — |
| Party Hunt | Unverified | `modules/game_analyser/styles/partyhunt.otui:179` | — |
| Loot Type: | Unverified | `modules/game_analyser/styles/partyhunt.otui:234` | — |
| Leader | Unverified | `modules/game_analyser/styles/partyhunt.otui:245` | — |
| Supply Analyser | Unverified | `modules/game_analyser/styles/supply.otui:12` | — |
| XP Analyser | Unverified | `modules/game_analyser/styles/xp.otui:4` | — |
| XP/h: | Unverified | `modules/game_analyser/styles/xp.otui:102` | — |
| Next Level: | Unverified | `modules/game_analyser/styles/xp.otui:121` | — |
| - | Unverified | `modules/game_analyser/styles/xp.otui:131` | — |

### game_attachedeffects

NOT-APPLICABLE 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Attached Effects System, to create aura, wings... | NOT-APPLICABLE | `modules/game_attachedeffects/attachedeffects.otmod:1` | OTClient developer facility or OT server extension |
| Event · onOutfitChange | NOT-APPLICABLE | `modules/game_attachedeffects/attachedeffects.lua:43` | OT extension |

### game_battle

PRESENT 4 | MISSING 4 | PARTIAL 1 | Unverified 45

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Sort by Name (asc) | PRESENT | `modules/game_battle/battle.lua:35` | `apps/client/src/features/0017-attack/editor.rs:15` → `apps/client/src/features/0017-attack/preferences.rs:113` |
| Sort by HitPoints (asc) | PRESENT | `modules/game_battle/battle.lua:33` | `apps/client/src/features/0017-attack/editor.rs:16` → `apps/client/src/features/0017-attack/preferences.rs:114` |
| Sort by HitPoints (desc) | PRESENT | `modules/game_battle/battle.lua:34` | `apps/client/src/features/0017-attack/editor.rs:17` → `apps/client/src/features/0017-attack/preferences.rs:115` |
| Sort by DisplayTime (asc) | PRESENT | `modules/game_battle/battle.lua:29` | `apps/client/src/features/0017-attack/editor.rs:14` → `apps/client/src/features/0017-attack/preferences.rs:107` |
| Sort by DisplayTime (desc) | MISSING | `modules/game_battle/battle.lua:30` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:107` |
| Sort by Distance (asc) | MISSING | `modules/game_battle/battle.lua:31` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:112` |
| Sort by Distance (desc) | MISSING | `modules/game_battle/battle.lua:32` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:112` |
| Sort by Name (desc) | MISSING | `modules/game_battle/battle.lua:36` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:113` |
| Battle list ordering menu | PARTIAL | `modules/game_battle/battle.lua:32` | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:348` opens the list editor. `apps/client/src/features/0017-attack/preferences.rs:112` implements health in both directions, name ascending and appearance order. Distance and reverse name/display time are missing. |
| Event · onGeometryChange | Unverified | `modules/game_battle/battle.lua:306` | — |
| Edit Name | Unverified | `modules/game_battle/battle.lua:333` | — |
| Create New Battle List | Unverified | `modules/game_battle/battle.lua:334` | — |
| Clear Configurations | Unverified | `modules/game_battle/battle.lua:336` | — |
| Close Battle List | Unverified | `modules/game_battle/battle.lua:337` | — |
| Event · onCheckChange | Unverified | `modules/game_battle/battle.lua:774` | — |
| Event · onPositionChange | Unverified | `modules/game_battle/battle.lua:1359` | — |
| Event · onSkullChange | Unverified | `modules/game_battle/battle.lua:1361` | — |
| Event · onEmblemChange | Unverified | `modules/game_battle/battle.lua:1362` | — |
| Event · onOutfitChange | Unverified | `modules/game_battle/battle.lua:1363` | — |
| Event · onHealthPercentChange | Unverified | `modules/game_battle/battle.lua:1364` | — |
| Event · onZoomChange | Unverified | `modules/game_battle/battle.lua:1369` | — |
| Event · onHoverChange | Unverified | `modules/game_battle/battle.lua:1408` | — |
| Event · onAttackingCreatureChange | Unverified | `modules/game_battle/battle.lua:1532` | — |
| Event · onFollowingCreatureChange | Unverified | `modules/game_battle/battle.lua:1533` | — |
| Event · onGameEnd | Unverified | `modules/game_battle/battle.lua:1534` | — |
| Event · onGameStart | Unverified | `modules/game_battle/battle.lua:1535` | — |
| Game API · attack | Unverified | `modules/game_battle/battle.lua:2024` | — |
| Game API · look | Unverified | `modules/game_battle/battle.lua:2415` | — |
| Battle List | Unverified | `modules/game_battle/battle.otui:177` | — |
| Hide players | Unverified | `modules/game_battle/battle.otui:196` | — |
| Activate · modules.game_battle.onFilterButtonClick(self) | Unverified | `modules/game_battle/battle.otui:197` | — |
| Hide knights | Unverified | `modules/game_battle/battle.otui:201` | — |
| Hide paladins | Unverified | `modules/game_battle/battle.otui:206` | — |
| Hide druids | Unverified | `modules/game_battle/battle.otui:211` | — |
| Hide sorcerers | Unverified | `modules/game_battle/battle.otui:216` | — |
| Hide monks | Unverified | `modules/game_battle/battle.otui:221` | — |
| Hide summons | Unverified | `modules/game_battle/battle.otui:226` | — |
| Hide Npcs | Unverified | `modules/game_battle/battle.otui:231` | — |
| Hide monsters | Unverified | `modules/game_battle/battle.otui:236` | — |
| Hide non-skull players | Unverified | `modules/game_battle/battle.otui:241` | — |
| Hide party members | Unverified | `modules/game_battle/battle.otui:246` | — |
| Hide own guild members | Unverified | `modules/game_battle/battle.otui:251` | — |
| Open secondary battle list | Unverified | `modules/game_battle/battle.otui:344` | — |
| Sort Ascending by Display Time | Unverified | `modules/game_battle/battle.otui:352` | — |
| Sort Descending by Display Time | Unverified | `modules/game_battle/battle.otui:356` | — |
| Sort Ascending by Distance | Unverified | `modules/game_battle/battle.otui:360` | — |
| Sort Descending by Distance | Unverified | `modules/game_battle/battle.otui:364` | — |
| Sort Ascending by %% Hit Points | Unverified | `modules/game_battle/battle.otui:368` | — |
| Sort Descending by %% Hit Points | Unverified | `modules/game_battle/battle.otui:372` | — |
| Sort Ascending by Name | Unverified | `modules/game_battle/battle.otui:376` | — |
| Sort Descending by Name | Unverified | `modules/game_battle/battle.otui:380` | — |
| Enter a new name for this list. Leave empty to \nreset the name: | Unverified | `modules/game_battle/style/changeListName.otui:24` | — |
| Cancel | Unverified | `modules/game_battle/style/changeListName.otui:68` | — |
| Ok | Unverified | `modules/game_battle/style/changeListName.otui:81` | — |

### game_blessing

Unverified 7

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onBlessingsChange | Unverified | `modules/game_blessing/blessing.lua:29` | — |
| Event · onUpdateBlessDialog | Unverified | `modules/game_blessing/blessing.lua:40` | — |
| Game API · requestBless | Unverified | `modules/game_blessing/blessing.lua:88` | — |
| Game API · sendRequestStorePremiumBoost | Unverified | `modules/game_blessing/blessing.lua:152` | — |
| You are currently not protected by any blessing. | Unverified | `modules/game_blessing/blessing.lua:165` | — |
| 1 | Unverified | `modules/game_blessing/style.otui:9` | — |
| - | Unverified | `modules/game_blessing/style.otui:30` | — |

### game_bugreport

Unverified 6

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Game API · reportBug | Unverified | `modules/game_bugreport/bugreport.lua:30` | — |
| Report Bug | Unverified | `modules/game_bugreport/bugreport.otui:2` | — |
| Please use this dialog to only report bugs. Do not report rule violations here! | Unverified | `modules/game_bugreport/bugreport.otui:8` | — |
| Send | Unverified | `modules/game_bugreport/bugreport.otui:26` | — |
| Cancel | Unverified | `modules/game_bugreport/bugreport.otui:35` | — |
| Activate · self:getParent():hide() | Unverified | `modules/game_bugreport/bugreport.otui:39` | — |

### game_compendium

PRESENT 1 | MISSING 1 | Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Browse player guide | PRESENT | `modules/game_compendium/game_compendium.lua:15` | `apps/client/src/bevy_shell/game_ui/feature_dialogs/guide.rs:66` → `apps/client/src/bevy_shell/game_ui/feature_dialogs/guide.rs:49` |
| Browse major update articles in the compendium | MISSING | `modules/game_compendium/game_compendium.lua:14` | `apps/client/src/bevy_shell/game_ui/feature_dialogs/guide.rs:5` enumerates notices and four help topics. |
| Event · onSelectionChange | Unverified | `modules/game_compendium/game_compendium.lua:361` | — |
| Event · onGeometryChange | Unverified | `modules/game_compendium/game_compendium.lua:688` | — |

### game_console

Unverified 91

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Open Channel | Unverified | `modules/game_console/channelswindow.otui:10` | — |
| Select a channel: | Unverified | `modules/game_console/channelswindow.otui:19` | — |
| Open a private message channel: | Unverified | `modules/game_console/channelswindow.otui:43` | — |
| Cancel | Unverified | `modules/game_console/channelswindow.otui:65` | — |
| Activate · self:getParent():destroy() | Unverified | `modules/game_console/channelswindow.otui:70` | — |
| Open | Unverified | `modules/game_console/channelswindow.otui:78` | — |
| Activate · self:getParent():onEnter() | Unverified | `modules/game_console/channelswindow.otui:80` | — |
| Ignore List | Unverified | `modules/game_console/communicationwindow.otui:26` | — |
| Activate ignorelist | Unverified | `modules/game_console/communicationwindow.otui:32` | — |
| Ignored Players | Unverified | `modules/game_console/communicationwindow.otui:38` | — |
| Add | Unverified | `modules/game_console/communicationwindow.otui:64` | — |
| Remove | Unverified | `modules/game_console/communicationwindow.otui:73` | — |
| Global ignore settings | Unverified | `modules/game_console/communicationwindow.otui:81` | — |
| Ignore Private Messages | Unverified | `modules/game_console/communicationwindow.otui:88` | — |
| Ignore Yelling | Unverified | `modules/game_console/communicationwindow.otui:96` | — |
| Activate whitelist | Unverified | `modules/game_console/communicationwindow.otui:104` | — |
| Allowed Players | Unverified | `modules/game_console/communicationwindow.otui:111` | — |
| Global whitelist settings | Unverified | `modules/game_console/communicationwindow.otui:154` | — |
| Allow VIPs to message you | Unverified | `modules/game_console/communicationwindow.otui:161` | — |
| Save | Unverified | `modules/game_console/communicationwindow.otui:179` | — |
| Event · onTalk | Unverified | `modules/game_console/console.lua:190` | — |
| Event · onChannelList | Unverified | `modules/game_console/console.lua:191` | — |
| Event · onOpenChannel | Unverified | `modules/game_console/console.lua:192` | — |
| Event · onOpenPrivateChannel | Unverified | `modules/game_console/console.lua:193` | — |
| Event · onOpenOwnPrivateChannel | Unverified | `modules/game_console/console.lua:194` | — |
| Event · onCloseChannel | Unverified | `modules/game_console/console.lua:195` | — |
| Event · onRuleViolationChannel | Unverified | `modules/game_console/console.lua:196` | — |
| Event · onReceiveExivaOptions | Unverified | `modules/game_console/console.lua:200` | — |
| Event · onChannelEvent | Unverified | `modules/game_console/console.lua:201` | — |
| Event · onTabChange | Unverified | `modules/game_console/console.lua:265` | — |
| Disable chat mode, allow to walk using WASD | Unverified | `modules/game_console/console.lua:421` | — |
| Enable chat mode | Unverified | `modules/game_console/console.lua:426` | — |
| Game API · inviteToOwnChannel | Unverified | `modules/game_console/console.lua:748` | — |
| Game API · excludeFromOwnChannel | Unverified | `modules/game_console/console.lua:750` | — |
| Game API · joinChannel | Unverified | `modules/game_console/console.lua:779` | — |
| Game API · talkChannel | Unverified | `modules/game_console/console.lua:793` | — |
| Event · onHoverChange | Unverified | `modules/game_console/console.lua:818` | — |
| Game API · closeRuleViolation | Unverified | `modules/game_console/console.lua:845` | — |
| Game API · leaveChannel | Unverified | `modules/game_console/console.lua:859` | — |
| Game API · closeNpcChannel | Unverified | `modules/game_console/console.lua:862` | — |
| Event · onTextHoverChange | Unverified | `modules/game_console/console.lua:1285` | — |
| Invite player | Unverified | `modules/game_console/console.lua:1453` | — |
| Exclude player | Unverified | `modules/game_console/console.lua:1456` | — |
| Close | Unverified | `modules/game_console/console.lua:1463` | — |
| Close read-only | Unverified | `modules/game_console/console.lua:1469` | — |
| Open read-only | Unverified | `modules/game_console/console.lua:1474` | — |
| Clear Messages | Unverified | `modules/game_console/console.lua:1479` | — |
| Save Messages | Unverified | `modules/game_console/console.lua:1482` | — |
| Message to  | Unverified | `modules/game_console/console.lua:1496` | — |
| Game API · openPrivateChannel | Unverified | `modules/game_console/console.lua:1497` | — |
| Add to VIP list | Unverified | `modules/game_console/console.lua:1500` | — |
| Game API · addVip | Unverified | `modules/game_console/console.lua:1501` | — |
| Invite to private chat | Unverified | `modules/game_console/console.lua:1506` | — |
| Exclude from private chat | Unverified | `modules/game_console/console.lua:1509` | — |
| Unignore | Unverified | `modules/game_console/console.lua:1514` | — |
| Ignore | Unverified | `modules/game_console/console.lua:1518` | — |
| Rule Violation | Unverified | `modules/game_console/console.lua:1525` | — |
| Copy name | Unverified | `modules/game_console/console.lua:1531` | — |
| Copy | Unverified | `modules/game_console/console.lua:1537` | — |
| Copy message | Unverified | `modules/game_console/console.lua:1542` | — |
| Select all | Unverified | `modules/game_console/console.lua:1546` | — |
| Process | Unverified | `modules/game_console/console.lua:1551` | — |
| Game API · talkPrivate | Unverified | `modules/game_console/console.lua:1735` | — |
| Game API · openRuleViolation | Unverified | `modules/game_console/console.lua:1943` | — |
| Game API · openOwnChannel | Unverified | `modules/game_console/console.lua:2003` | — |
| Event · onChildFocusChange | Unverified | `modules/game_console/console.lua:2060` | — |
| Close Read-Only Tab | Unverified | `modules/game_console/console.lua:2627` | — |
| Show  | Unverified | `modules/game_console/console.lua:2634` | — |
| Invite player to selected chat channel | Unverified | `modules/game_console/console.otui:3` | — |
| Please Enter a character name: | Unverified | `modules/game_console/console.otui:16` | — |
| Ok | Unverified | `modules/game_console/console.otui:45` | — |
| Exclude player from selected chat channel | Unverified | `modules/game_console/console.otui:51` | — |
| Enable/ Disable draggable chat | Unverified | `modules/game_console/console.otui:294` | — |
| Activate · \| | Unverified | `modules/game_console/console.otui:305` | — |
| Hide Chat | Unverified | `modules/game_console/console.otui:318` | — |
| Close this channel | Unverified | `modules/game_console/console.otui:340` | — |
| Activate · removeCurrentTab() | Unverified | `modules/game_console/console.otui:348` | — |
| Open new channel | Unverified | `modules/game_console/console.otui:358` | — |
| Game API · requestChannels | Unverified | `modules/game_console/console.otui:366` | — |
| Activate · g_game.requestChannels() | Unverified | `modules/game_console/console.otui:366` | — |
| Ignore players | Unverified | `modules/game_console/console.otui:376` | — |
| Activate · onClickIgnoreButton() | Unverified | `modules/game_console/console.otui:384` | — |
| Exiva Options are only available on Optional Pvp game worlds | Unverified | `modules/game_console/console.otui:405` | — |
| Set talk type | Unverified | `modules/game_console/console.otui:450` | — |
| Activate · sayModeChange() | Unverified | `modules/game_console/console.otui:452` | — |
| Chat Off | Unverified | `modules/game_console/console.otui:484` | — |
| Activate · toggleChat() | Unverified | `modules/game_console/console.otui:486` | — |
| Game API · sendExivaOptions | Unverified | `modules/game_console/exiva_option.lua:95` | — |
| Event · onSelectionChange | Unverified | `modules/game_console/exiva_option.lua:126` | — |
| Report Rule Violation | Unverified | `modules/game_console/violationwindow.otui:3` | — |
| Activate · self:getParent():onEscape() | Unverified | `modules/game_console/violationwindow.otui:49` | — |

### game_containers

PRESENT 2 | Unverified 20

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Close container | PRESENT | `modules/game_containers/containers.lua:937` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:218` → `apps/client/src/features/0014-container-tree/input.rs:72` |
| Open parent container | PRESENT | `modules/game_containers/containers.lua:951` | `apps/client/src/bevy_shell/game_ui/inventory/actions.rs:216` → `apps/client/src/features/0014-container-tree/input.rs:57` |
| Sort Ascending by Name | Unverified | `modules/game_containers/container.otui:168` | — |
| Sort Descending by Name | Unverified | `modules/game_containers/container.otui:171` | — |
| Sort Ascending by Weight | Unverified | `modules/game_containers/container.otui:174` | — |
| Sort Descending by Weight | Unverified | `modules/game_containers/container.otui:177` | — |
| Sort Ascending by Expiry | Unverified | `modules/game_containers/container.otui:180` | — |
| Sort Descending by Expiry | Unverified | `modules/game_containers/container.otui:183` | — |
| Sort Ascending by Stack Size | Unverified | `modules/game_containers/container.otui:186` | — |
| Sort Descending by Stack Size | Unverified | `modules/game_containers/container.otui:189` | — |
| Sort Containers First | Unverified | `modules/game_containers/container.otui:197` | — |
| Sort Nested Containers | Unverified | `modules/game_containers/container.otui:201` | — |
| Use Manual Sort Mode | Unverified | `modules/game_containers/container.otui:205` | — |
| Move Contents to \ | Unverified | `modules/game_containers/container.otui:213` | — |
| Move Nested Containers | Unverified | `modules/game_containers/container.otui:217` | — |
| Event · onSizeChange | Unverified | `modules/game_containers/containers.lua:31` | — |
| Event · onUpdateItem | Unverified | `modules/game_containers/containers.lua:32` | — |
| Event · onGameEnd | Unverified | `modules/game_containers/containers.lua:35` | — |
| Event · onCheckChange | Unverified | `modules/game_containers/containers.lua:129` | — |
| Game API · seekInContainer | Unverified | `modules/game_containers/containers.lua:905` | — |
| Game API · close | Unverified | `modules/game_containers/containers.lua:937` | — |
| Game API · openParent | Unverified | `modules/game_containers/containers.lua:951` | — |

### game_cooldown

Unverified 14

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameEnd | Unverified | `modules/game_cooldown/cooldown.lua:18` | — |
| Event · onGameStart | Unverified | `modules/game_cooldown/cooldown.lua:19` | — |
| Event · onSpellGroupCooldown | Unverified | `modules/game_cooldown/cooldown.lua:20` | — |
| Event · onSpellCooldown | Unverified | `modules/game_cooldown/cooldown.lua:21` | — |
| Attack | Unverified | `modules/game_cooldown/cooldown.otui:50` | — |
| Healing | Unverified | `modules/game_cooldown/cooldown.otui:62` | — |
| Support | Unverified | `modules/game_cooldown/cooldown.otui:74` | — |
| Special | Unverified | `modules/game_cooldown/cooldown.otui:86` | — |
| Crippling | Unverified | `modules/game_cooldown/cooldown.otui:98` | — |
| Focus | Unverified | `modules/game_cooldown/cooldown.otui:110` | — |
| Ultimate Strike | Unverified | `modules/game_cooldown/cooldown.otui:122` | — |
| Great Beams | Unverified | `modules/game_cooldown/cooldown.otui:134` | — |
| Bursts of Nature | Unverified | `modules/game_cooldown/cooldown.otui:146` | — |
| Virtue | Unverified | `modules/game_cooldown/cooldown.otui:158` | — |

### game_creatureinformation

Unverified 9

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onOutfitChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:157` | — |
| Event · onHealthPercentChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:159` | — |
| Event · onChangeName | Unverified | `modules/game_creatureinformation/creatureinformation.lua:160` | — |
| Event · onTypeChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:161` | — |
| Event · onIconChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:162` | — |
| Event · onSkullChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:163` | — |
| Event · onShieldChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:164` | — |
| Event · onEmblemChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:165` | — |
| Event · onManaChange | Unverified | `modules/game_creatureinformation/creatureinformation.lua:185` | — |

### game_cyclopedia

PARTIAL 1 | Unverified 286

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Browse the character’s familiar collection | PARTIAL | `modules/game_cyclopedia/tab/character/character.lua:226` | `apps/client/src/bevy_shell/game_ui/cyclopedia/widgets.rs:277` explains the gallery shows content catalogue entries. `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:139` filters the page by familiar kind. Character ownership is unavailable. |
| Set bid limit: | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:8` | — |
| Size: 17sqm | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:57` | — |
| Status: auctioned (no bid yet) | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:63` | — |
| Max. Beds: 1 | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:72` | — |
| Rent:    | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:78` | — |
| 0 | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:144` | — |
| Boots | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:169` | — |
| 1 | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:185` | — |
| Physical | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:299` | — |
| ? | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:346` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.bossSlotSelectBoss(self) | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:349` | — |
| Unlocks if you reach the\nProwess level for any boss | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:378` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.SelectBossSearchText("", true, self) | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:435` | — |
| Select | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:443` | — |
| Total Kills | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:480` | — |
| Remove | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:515` | — |
| Item Name | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:646` | — |
| Name | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:664` | — |
| Title | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:676` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.selectCharm(self, self:isChecked()) | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:684` | — |
| General Stats | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:818` | — |
| Known:  | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:860` | — |
| Total:  | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:867` | — |
| Track | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:973` | — |
| Common | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1096` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.selectCreatureCharm(self, self:isChecked()) | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1101` | — |
| sort by name | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1331` | — |
| Short by completion percentage | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1335` | — |
| sort by remaining kills | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1339` | — |
| sort by ascending | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1345` | — |
| sort by descending | Unverified | `modules/game_cyclopedia/cyclopedia_widgets.otui:1349` | — |
| Event · onParseCyclopediaTracker | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:41` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:93` | — |
| Event · onUpdateBestiaryMonsterData | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:97` | — |
| Event · onParseCyclopediaCharacterGeneralStats | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:103` | — |
| Event · onParseCyclopediaCharacterCombatStats | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:104` | — |
| Event · onParseCyclopediaCharacterBadges | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:105` | — |
| Event · onCyclopediaCharacterRecentDeaths | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:106` | — |
| Event · onCyclopediaCharacterRecentKills | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:107` | — |
| Event · onUpdateCyclopediaCharacterItemSummary | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:108` | — |
| Event · onParseCyclopediaCharacterAppearances | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:109` | — |
| Event · onParseCyclopediaStoreSummary | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:110` | — |
| Event · onCyclopediaCharacterOffenceStats | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:112` | — |
| Event · onCyclopediaCharacterDefenceStats | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:113` | — |
| Event · onCyclopediaCharacterMiscStats | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:114` | — |
| Event · onUpdateBestiaryCharmsData | Unverified | `modules/game_cyclopedia/game_cyclopedia.lua:118` | — |
| Cyclopedia | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:4` | — |
| Activate · SelectWindow("items") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:29` | — |
| Activate · SelectWindow("bestiary") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:51` | — |
| Activate · SelectWindow("charms") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:73` | — |
| Activate · SelectWindow("map") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:95` | — |
| Activate · SelectWindow("houses") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:117` | — |
| Activate · SelectWindow("character") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:139` | — |
| Activate · SelectWindow("bosstiary") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:161` | — |
| Activate · SelectWindow("bossSlot") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:183` | — |
| Activate · SelectWindow("magicalArchives") | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:205` | — |
| Close | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:236` | — |
| Activate · toggle() | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:240` | — |
| Back | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:247` | — |
| Activate · toggleBack() | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:252` | — |
| Minor Charm echoes\nEarned by unlocking or upgrading major charm and promotion your character | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:312` | — |
| 0 / 0 | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:324` | — |
| Bestiary Tracker | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:333` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.toggleBestiaryTracker() | Unverified | `modules/game_cyclopedia/game_cyclopedia.otui:335` | — |
| Event · onCheckChange | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:21` | — |
| Game API · sendStatusTrackerBestiary | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:82` | — |
| Game API · requestBestiary | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:150` | — |
| The Animus Mastery for this creature is unlocked.\nIt yields  | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:431` | — |
| Game API · requestBestiarySearch | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:451` | — |
| Game API · requestBestiaryOverview | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:461` | — |
| Game API · requestBosstiaryInfo | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:863` | — |
| stop Tracking  | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:1312` | — |
| Add to Loot List | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:1335` | — |
| Remove from Loot List | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.lua:1340` | — |
| Search | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:12` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.BestiarySearch(self:getText()) | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:16` | — |
| 1 / 1 | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:37` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.changeBestiaryPage(false, true) | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:45` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.changeBestiaryPage(true, false) | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:56` | — |
| Creature Name | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:109` | — |
| Track Kills | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:134` | — |
| Charm Selection: | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:322` | — |
| Location(s) | Unverified | `modules/game_cyclopedia/tab/bestiary/bestiary.otui:457` | — |
| Game API · requestBossSlootInfo | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.lua:7` | — |
| Game API · requestBossSlotAction | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.lua:236` | — |
| Progress | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.otui:17` | — |
| Boss Points | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.otui:24` | — |
| Slot 1: Locked | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.otui:63` | — |
| Slot 2: Locked | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.otui:74` | — |
| Boosted Boss: The Source Of Co... | Unverified | `modules/game_cyclopedia/tab/boss_slots/boss_slots.otui:80` | — |
| Once you have reached the Prowess level, you can assign the boss\nto a boss slot. | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.lua:16` | — |
| Prowess | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:74` | — |
| Expertise | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:82` | — |
| Mastery | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:90` | — |
| Filter | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:102` | — |
| Bane | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:112` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.changeBosstiaryFilter(self, self:isChecked()) | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:116` | — |
| Archfoe | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:128` | — |
| Nemesis | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:144` | — |
| No Kills | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:161` | — |
| Few Kills | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:170` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.BosstiarySearchText("", true) | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:242` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.changeBosstiaryPage(false, true) | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:260` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.changeBosstiaryPage(true, false) | Unverified | `modules/game_cyclopedia/tab/bosstiary/bosstiary.otui:271` | — |
| Alphabetically | Unverified | `modules/game_cyclopedia/tab/character/character.lua:377` | — |
| By Grade | Unverified | `modules/game_cyclopedia/tab/character/character.lua:378` | — |
| By Unlock Date | Unverified | `modules/game_cyclopedia/tab/character/character.lua:379` | — |
| Game API · requestCharacterInfo | Unverified | `modules/game_cyclopedia/tab/character/character.lua:1090` | — |
| %s: %s | Unverified | `modules/game_cyclopedia/tab/character/character.lua:1787` | — |
| Player Name | Unverified | `modules/game_cyclopedia/tab/character/character.otui:65` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.selectCharacterPage() | Unverified | `modules/game_cyclopedia/tab/character/character.otui:68` | — |
| Level: 1\nKnight | Unverified | `modules/game_cyclopedia/tab/character/character.otui:74` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.characterButton(self) | Unverified | `modules/game_cyclopedia/tab/character/character.otui:174` | — |
| Activate · modules.game_wheel.toggle() | Unverified | `modules/game_cyclopedia/tab/character/character.otui:182` | — |
| Status: Loading, please wait... | Unverified | `modules/game_cyclopedia/tab/character/character.otui:209` | — |
| You are inspecting | Unverified | `modules/game_cyclopedia/tab/character/character.otui:224` | — |
| Level | Unverified | `modules/game_cyclopedia/tab/character/character.otui:247` | — |
| Experience | Unverified | `modules/game_cyclopedia/tab/character/character.otui:260` | — |
| XP Gain Rate | Unverified | `modules/game_cyclopedia/tab/character/character.otui:271` | — |
| Activate · modules.game_shop.toggle() | Unverified | `modules/game_cyclopedia/tab/character/character.otui:280` | — |
| Hit Points | Unverified | `modules/game_cyclopedia/tab/character/character.otui:304` | — |
| Mana | Unverified | `modules/game_cyclopedia/tab/character/character.otui:315` | — |
| Soul Points | Unverified | `modules/game_cyclopedia/tab/character/character.otui:326` | — |
| Capacity | Unverified | `modules/game_cyclopedia/tab/character/character.otui:337` | — |
| Speed | Unverified | `modules/game_cyclopedia/tab/character/character.otui:348` | — |
| Food | Unverified | `modules/game_cyclopedia/tab/character/character.otui:359` | — |
| Stamina | Unverified | `modules/game_cyclopedia/tab/character/character.otui:370` | — |
| Offline Training | Unverified | `modules/game_cyclopedia/tab/character/character.otui:383` | — |
| Account Status | Unverified | `modules/game_cyclopedia/tab/character/character.otui:405` | — |
| 33 | Unverified | `modules/game_cyclopedia/tab/character/character.otui:408` | — |
| Account Premium | Unverified | `modules/game_cyclopedia/tab/character/character.otui:417` | — |
| Loyalty Title | Unverified | `modules/game_cyclopedia/tab/character/character.otui:429` | — |
| Skills | Unverified | `modules/game_cyclopedia/tab/character/character.otui:437` | — |
| Magic | Unverified | `modules/game_cyclopedia/tab/character/character.otui:449` | — |
| Fist | Unverified | `modules/game_cyclopedia/tab/character/character.otui:467` | — |
| Club | Unverified | `modules/game_cyclopedia/tab/character/character.otui:485` | — |
| Sword | Unverified | `modules/game_cyclopedia/tab/character/character.otui:503` | — |
| Axe | Unverified | `modules/game_cyclopedia/tab/character/character.otui:521` | — |
| Distance | Unverified | `modules/game_cyclopedia/tab/character/character.otui:539` | — |
| Shielding | Unverified | `modules/game_cyclopedia/tab/character/character.otui:557` | — |
| Fishing | Unverified | `modules/game_cyclopedia/tab/character/character.otui:575` | — |
| Account Badges | Unverified | `modules/game_cyclopedia/tab/character/character.otui:587` | — |
| Attack Value: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:648` | — |
| Converted Damage: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:665` | — |
| None | Unverified | `modules/game_cyclopedia/tab/character/character.otui:670` | — |
| Defence Value: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:688` | — |
| Armor Value: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:700` | — |
| Mitigation: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:712` | — |
| Blessings: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:724` | — |
| Activate · modules.game_blessing.toggle() | Unverified | `modules/game_cyclopedia/tab/character/character.otui:738` | — |
| Damage Reduction: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:755` | — |
| Concoctions: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:781` | — |
| Critical Hit: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:801` | — |
| Chance | Unverified | `modules/game_cyclopedia/tab/character/character.otui:811` | — |
| Extra Damage | Unverified | `modules/game_cyclopedia/tab/character/character.otui:824` | — |
| Life Leech: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:837` | — |
| Mana Leech: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:850` | — |
| Data and Time | Unverified | `modules/game_cyclopedia/tab/character/character.otui:880` | — |
| Cause of Death | Unverified | `modules/game_cyclopedia/tab/character/character.otui:894` | — |
| Previous | Unverified | `modules/game_cyclopedia/tab/character/character.otui:925` | — |
| Next | Unverified | `modules/game_cyclopedia/tab/character/character.otui:936` | — |
| Page 1 / 1 | Unverified | `modules/game_cyclopedia/tab/character/character.otui:947` | — |
| Description | Unverified | `modules/game_cyclopedia/tab/character/character.otui:990` | — |
| Status | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1005` | — |
| Achievements Points:  | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1079` | — |
| Grade 1:  | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1086` | — |
| Grade 2:  | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1100` | — |
| Grade 3:  | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1115` | — |
| Grade 4:  | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1130` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.achievementFilter(self) | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1162` | — |
| Amount | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1337` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.characterItemListFilter(self) | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1454` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.characterAppearancesFilter(self) | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1490` | — |
| Show All | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1516` | — |
| Show Standard Outfits | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1517` | — |
| Show Quest Outfits | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1518` | — |
| Show Store Outfits | Unverified | `modules/game_cyclopedia/tab/character/character.otui:1519` | — |
| Active Concoctions: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:2035` | — |
| Active Foods: | Unverified | `modules/game_cyclopedia/tab/character/character.otui:2050` | — |
| Event · onSelectionChange | Unverified | `modules/game_cyclopedia/tab/charms/charms.lua:282` | — |
| Game API · BuyCharmRune | Unverified | `modules/game_cyclopedia/tab/charms/charms.lua:789` | — |
| Information | Unverified | `modules/game_cyclopedia/tab/charms/charms.otui:13` | — |
| Unlock | Unverified | `modules/game_cyclopedia/tab/charms/charms.otui:46` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.actionCharmButton(self) | Unverified | `modules/game_cyclopedia/tab/charms/charms.otui:51` | — |
| Select Creature | Unverified | `modules/game_cyclopedia/tab/charms/charms.otui:169` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.clearCharm() | Unverified | `modules/game_cyclopedia/tab/charms/charms1410.otui:94` | — |
| Activate · self:getParent():getChildById('SearchEdit'):clearText() | Unverified | `modules/game_cyclopedia/tab/charms/charms1410.otui:148` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.actionSelectCharmButton(self) | Unverified | `modules/game_cyclopedia/tab/charms/charms1410.otui:202` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.resetAllCharms() | Unverified | `modules/game_cyclopedia/tab/charms/charms1410.otui:232` | — |
| Major Charms | Unverified | `modules/game_cyclopedia/tab/charms/charms1410.otui:275` | — |
| Minor Charms | Unverified | `modules/game_cyclopedia/tab/charms/charms1410.otui:295` | — |
| Event · onOptionChange | Unverified | `modules/game_cyclopedia/tab/house/house.lua:36` | — |
| Game API · requestRejectHouseTransfer | Unverified | `modules/game_cyclopedia/tab/house/house.lua:312` | — |
| Game API · requestShowHouses | Unverified | `modules/game_cyclopedia/tab/house/house.lua:317` | — |
| Game API · requestAcceptHouseTransfer | Unverified | `modules/game_cyclopedia/tab/house/house.lua:387` | — |
| Game API · requestCancelHouseTransfer | Unverified | `modules/game_cyclopedia/tab/house/house.lua:455` | — |
| Game API · requestTransferHouse | Unverified | `modules/game_cyclopedia/tab/house/house.lua:566` | — |
| Event · onTextChange | Unverified | `modules/game_cyclopedia/tab/house/house.lua:642` | — |
| Game API · requestMoveOutHouse | Unverified | `modules/game_cyclopedia/tab/house/house.lua:708` | — |
| Game API · requestBidHouse | Unverified | `modules/game_cyclopedia/tab/house/house.lua:972` | — |
| This house is a shop. | Unverified | `modules/game_cyclopedia/tab/house/house.lua:1148` | — |
| Guildhalls | Unverified | `modules/game_cyclopedia/tab/house/house.otui:46` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.houseFilter(self) | Unverified | `modules/game_cyclopedia/tab/house/house.otui:51` | — |
| Houses and Flats | Unverified | `modules/game_cyclopedia/tab/house/house.otui:59` | — |
| No house selected | Unverified | `modules/game_cyclopedia/tab/house/house.otui:138` | — |
| Auction | Unverified | `modules/game_cyclopedia/tab/house/house.otui:180` | — |
| There is no bid so far.\nBe the first to bid on this house. | Unverified | `modules/game_cyclopedia/tab/house/house.otui:189` | — |
| Pending Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:196` | — |
| Your Limit: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:227` | — |
| Price: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:250` | — |
| Place Your Bid | Unverified | `modules/game_cyclopedia/tab/house/house.otui:291` | — |
| Cancel | Unverified | `modules/game_cyclopedia/tab/house/house.otui:306` | — |
| Bid | Unverified | `modules/game_cyclopedia/tab/house/house.otui:316` | — |
| House Information | Unverified | `modules/game_cyclopedia/tab/house/house.otui:325` | — |
| Name: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:331` | — |
| Size: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:346` | — |
| Max. Beds: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:361` | — |
| Rent: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:376` | — |
| Auction Information | Unverified | `modules/game_cyclopedia/tab/house/house.otui:396` | — |
| Select Move Date | Unverified | `modules/game_cyclopedia/tab/house/house.otui:403` | — |
| Move Out | Unverified | `modules/game_cyclopedia/tab/house/house.otui:428` | — |
| Move Information | Unverified | `modules/game_cyclopedia/tab/house/house.otui:508` | — |
| Paid until: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:514` | — |
| Move Date: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:529` | — |
| - | Unverified | `modules/game_cyclopedia/tab/house/house.otui:546` | — |
| Select a date in the future! | Unverified | `modules/game_cyclopedia/tab/house/house.otui:577` | — |
| You will move out at the daily server save of the selected day. | Unverified | `modules/game_cyclopedia/tab/house/house.otui:584` | — |
| All items you have left in the house will be automatically\nsent to your inbox. | Unverified | `modules/game_cyclopedia/tab/house/house.otui:589` | — |
| Configure House Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:597` | — |
| Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:622` | — |
| Transfer Information | Unverified | `modules/game_cyclopedia/tab/house/house.otui:702` | — |
| Transfer Date: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:723` | — |
| New Owner: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:783` | — |
| Field must be completed. | Unverified | `modules/game_cyclopedia/tab/house/house.otui:801` | — |
| Note that the new owner has to accept the transfer. | Unverified | `modules/game_cyclopedia/tab/house/house.otui:808` | — |
| Transfer Price: | Unverified | `modules/game_cyclopedia/tab/house/house.otui:813` | — |
| Cancel House Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:843` | — |
| Cancel Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:868` | — |
| If you want to keep this house. click on  | Unverified | `modules/game_cyclopedia/tab/house/house.otui:1020` | — |
| Accept House Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:1028` | — |
| Accept Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:1053` | — |
| Reject House Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:1213` | — |
| Reject Transfer | Unverified | `modules/game_cyclopedia/tab/house/house.otui:1238` | — |
| Reject the transfer if you do not want the house\ntransferred to you. | Unverified | `modules/game_cyclopedia/tab/house/house.otui:1390` | — |
| Event · onChildFocusChange | Unverified | `modules/game_cyclopedia/tab/items/items.lua:695` | — |
| Game API · inspectionObject | Unverified | `modules/game_cyclopedia/tab/items/items.lua:872` | — |
| All | Unverified | `modules/game_cyclopedia/tab/items/items.lua:1062` | — |
| Class  | Unverified | `modules/game_cyclopedia/tab/items/items.lua:1066` | — |
| Game API · sendPartyLootPrice | Unverified | `modules/game_cyclopedia/tab/items/items.lua:1229` | — |
| Categories | Unverified | `modules/game_cyclopedia/tab/items/items.otui:11` | — |
| Voc. | Unverified | `modules/game_cyclopedia/tab/items/items.otui:69` | — |
| 1H | Unverified | `modules/game_cyclopedia/tab/items/items.otui:91` | — |
| 2H | Unverified | `modules/game_cyclopedia/tab/items/items.otui:114` | — |
| Items | Unverified | `modules/game_cyclopedia/tab/items/items.otui:147` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.ItemSearch("", true) | Unverified | `modules/game_cyclopedia/tab/items/items.otui:222` | — |
| Loot Value Source | Unverified | `modules/game_cyclopedia/tab/items/items.otui:238` | — |
| NPC Buy Value | Unverified | `modules/game_cyclopedia/tab/items/items.otui:253` | — |
| Market Average Value | Unverified | `modules/game_cyclopedia/tab/items/items.otui:290` | — |
| Please select an item from the list to see its details here. | Unverified | `modules/game_cyclopedia/tab/items/items.otui:320` | — |
| Basic Item Details | Unverified | `modules/game_cyclopedia/tab/items/items.otui:336` | — |
| Sell To | Unverified | `modules/game_cyclopedia/tab/items/items.otui:407` | — |
| Buy From | Unverified | `modules/game_cyclopedia/tab/items/items.otui:445` | — |
| Track drops of this item | Unverified | `modules/game_cyclopedia/tab/items/items.otui:532` | — |
| Skip when Quick Selling | Unverified | `modules/game_cyclopedia/tab/items/items.otui:576` | — |
| Manage Containers | Unverified | `modules/game_cyclopedia/tab/items/items.otui:654` | — |
| Activate · \| | Unverified | `modules/game_cyclopedia/tab/items/items.otui:658` | — |
| All Vocations | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.lua:157` | — |
| Sorcerer | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.lua:158` | — |
| Druid | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.lua:159` | — |
| Paladin | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.lua:160` | — |
| Knight | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.lua:161` | — |
| Monk | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.lua:162` | — |
| Select Spell | Unverified | `modules/game_cyclopedia/tab/magicalArchives/magicalArchives.otui:55` | — |
| Event · onPositionChange | Unverified | `modules/game_cyclopedia/tab/map/map.lua:83` | — |
| Event · onValueChange | Unverified | `modules/game_cyclopedia/tab/map/map.lua:177` | — |
| Display | Unverified | `modules/game_cyclopedia/tab/map/map.otui:33` | — |
| Filter Marks | Unverified | `modules/game_cyclopedia/tab/map/map.otui:176` | — |
| Views | Unverified | `modules/game_cyclopedia/tab/map/map.otui:184` | — |
| Surface View | Unverified | `modules/game_cyclopedia/tab/map/map.otui:203` | — |
| Map View | Unverified | `modules/game_cyclopedia/tab/map/map.otui:210` | — |
| Level separator | Unverified | `modules/game_cyclopedia/tab/map/map.otui:217` | — |
| Navigation | Unverified | `modules/game_cyclopedia/tab/map/map.otui:239` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.setZoom(false) | Unverified | `modules/game_cyclopedia/tab/map/map.otui:418` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.setZoom(true) | Unverified | `modules/game_cyclopedia/tab/map/map.otui:429` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.upLayer() | Unverified | `modules/game_cyclopedia/tab/map/map.otui:465` | — |
| Activate · modules.game_cyclopedia.Cyclopedia.downLayer() | Unverified | `modules/game_cyclopedia/tab/map/map.otui:473` | — |

### game_forge

Unverified 6

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onBrowseForgeHistory | Unverified | `modules/game_forge/game_forge.lua:44` | — |
| Event · onOpenForge | Unverified | `modules/game_forge/game_forge.lua:46` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_forge/game_forge.lua:48` | — |
| Game API · openPortableForgeRequest | Unverified | `modules/game_forge/game_forge.lua:125` | — |
| Game API · forgeRequest | Unverified | `modules/game_forge/game_forge.lua:369` | — |
| Game API · sendForgeBrowseHistoryRequest | Unverified | `modules/game_forge/game_forge.lua:1211` | — |

### game_healthcircle

Unverified 46

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_healthcircle/game_healthcircle.lua:101` | — |
| Event · onHealthChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:155` | — |
| Event · onManaChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:156` | — |
| Event · onSkillChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:157` | — |
| Event · onManaShieldChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:158` | — |
| Event · onMagicLevelChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:159` | — |
| Event · onLevelChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:160` | — |
| Event · onGeometryChange | Unverified | `modules/game_healthcircle/game_healthcircle.lua:187` | — |
| Magic Level | Unverified | `modules/game_healthcircle/game_healthcircle.lua:735` | — |
| Fist Fighting | Unverified | `modules/game_healthcircle/game_healthcircle.lua:736` | — |
| Club Fighting | Unverified | `modules/game_healthcircle/game_healthcircle.lua:737` | — |
| Sword Fighting | Unverified | `modules/game_healthcircle/game_healthcircle.lua:738` | — |
| Axe Fighting | Unverified | `modules/game_healthcircle/game_healthcircle.lua:739` | — |
| Distance Fighting | Unverified | `modules/game_healthcircle/game_healthcircle.lua:740` | — |
| Shielding | Unverified | `modules/game_healthcircle/game_healthcircle.lua:741` | — |
| Fishing | Unverified | `modules/game_healthcircle/game_healthcircle.lua:742` | — |
| Top | Unverified | `modules/game_healthcircle/game_healthcircle.lua:744` | — |
| Bottom | Unverified | `modules/game_healthcircle/game_healthcircle.lua:745` | — |
| Hide | Unverified | `modules/game_healthcircle/game_healthcircle.lua:747` | — |
| Compact | Unverified | `modules/game_healthcircle/game_healthcircle.lua:748` | — |
| Default | Unverified | `modules/game_healthcircle/game_healthcircle.lua:749` | — |
| Large | Unverified | `modules/game_healthcircle/game_healthcircle.lua:750` | — |
| Parallel | Unverified | `modules/game_healthcircle/game_healthcircle.lua:751` | — |
| Health circle | Unverified | `modules/game_healthcircle/option_healthcircle.otui:12` | — |
| Show health red circle. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:13` | — |
| Mana circle | Unverified | `modules/game_healthcircle/option_healthcircle.otui:30` | — |
| Show mana blue circle. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:31` | — |
| Experience circle | Unverified | `modules/game_healthcircle/option_healthcircle.otui:48` | — |
| Show experience yellow circle. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:49` | — |
| Skill circle | Unverified | `modules/game_healthcircle/option_healthcircle.otui:66` | — |
| Show skill purple circle. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:67` | — |
| Skill: | Unverified | `modules/game_healthcircle/option_healthcircle.otui:83` | — |
| Choose skill to show in skill circle. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:99` | — |
| Distance from center | Unverified | `modules/game_healthcircle/option_healthcircle.otui:115` | — |
| Set distance from center. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:116` | — |
| Opacity | Unverified | `modules/game_healthcircle/option_healthcircle.otui:130` | — |
| Set circle opacity. | Unverified | `modules/game_healthcircle/option_healthcircle.otui:131` | — |
| Bar Style: | Unverified | `modules/game_healthcircle/option_healthcircle.otui:144` | — |
| Event · onCheckChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:332` | — |
| Event · onFocusChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:386` | — |
| Event · onChildFocusChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:417` | — |
| Event · onStatesChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:806` | — |
| Event · onSkullChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:807` | — |
| Event · onEmblemChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:808` | — |
| Event · onRegenerationChange | Unverified | `modules/game_healthcircle/statusiconbar.lua:809` | — |
| Event · onGameEnd | Unverified | `modules/game_healthcircle/statusiconbar.lua:814` | — |

### game_healthinfo

Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onHealthChange | Unverified | `modules/game_healthinfo/healthinfo.lua:32` | — |
| Event · onManaChange | Unverified | `modules/game_healthinfo/healthinfo.lua:33` | — |

### game_highscore

PRESENT 5 | Unverified 28

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Select ranking category | PRESENT | `modules/game_highscore/game_highscore.lua:116` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:7` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Select vocation | PRESENT | `modules/game_highscore/game_highscore.lua:115` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:18` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Next ranking page | PRESENT | `modules/game_highscore/game_highscore.lua:241` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:32` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Previous ranking page | PRESENT | `modules/game_highscore/game_highscore.lua:243` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:26` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Show own rank | PRESENT | `modules/game_highscore/game_highscore.lua:257` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:43` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Game API · requestHighscore | Unverified | `modules/game_highscore/game_highscore.lua:268` | — |
| 1 | Unverified | `modules/game_highscore/game_highscore.otui:39` | — |
| - | Unverified | `modules/game_highscore/game_highscore.otui:58` | — |
| Highscores | Unverified | `modules/game_highscore/game_highscore.otui:136` | — |
| Highscores Filter | Unverified | `modules/game_highscore/game_highscore.otui:148` | — |
| Game World: | Unverified | `modules/game_highscore/game_highscore.otui:160` | — |
| BattlEye: | Unverified | `modules/game_highscore/game_highscore.otui:178` | — |
| Vocation: | Unverified | `modules/game_highscore/game_highscore.otui:199` | — |
| Category: | Unverified | `modules/game_highscore/game_highscore.otui:219` | — |
| World Type: | Unverified | `modules/game_highscore/game_highscore.otui:238` | — |
| Submit | Unverified | `modules/game_highscore/game_highscore.otui:267` | — |
| Activate · modules.game_highscore.submit() | Unverified | `modules/game_highscore/game_highscore.otui:268` | — |
| Rank | Unverified | `modules/game_highscore/game_highscore.otui:293` | — |
| Character | Unverified | `modules/game_highscore/game_highscore.otui:311` | — |
| Vocation | Unverified | `modules/game_highscore/game_highscore.otui:323` | — |
| Game World | Unverified | `modules/game_highscore/game_highscore.otui:337` | — |
| Level | Unverified | `modules/game_highscore/game_highscore.otui:351` | — |
| Points | Unverified | `modules/game_highscore/game_highscore.otui:375` | — |
| 1 / 50 | Unverified | `modules/game_highscore/game_highscore.otui:383` | — |
| Activate · modules.game_highscore.nextPage() | Unverified | `modules/game_highscore/game_highscore.otui:393` | — |
| Activate · modules.game_highscore.nextEndPage() | Unverified | `modules/game_highscore/game_highscore.otui:404` | — |
| Last Update: 9999999 Second Ago | Unverified | `modules/game_highscore/game_highscore.otui:414` | — |
| Activate · modules.game_highscore.prevPage() | Unverified | `modules/game_highscore/game_highscore.otui:423` | — |
| Activate · modules.game_highscore.prevEndPage() | Unverified | `modules/game_highscore/game_highscore.otui:434` | — |
| Show Own Rank | Unverified | `modules/game_highscore/game_highscore.otui:443` | — |
| Activate · modules.game_highscore.showOwnRank() | Unverified | `modules/game_highscore/game_highscore.otui:445` | — |
| Close | Unverified | `modules/game_highscore/game_highscore.otui:458` | — |
| Activate · modules.game_highscore.hide() | Unverified | `modules/game_highscore/game_highscore.otui:459` | — |

### game_hotkeys

Unverified 40

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onSelectionChange | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:100` | — |
| None | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:106` | — |
| Event · onOptionChange | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:111` | — |
| Event · onChildFocusChange | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:118` | — |
| Event · onGameStart | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:129` | — |
| Event · onGameEnd | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:130` | — |
| Game API · setChaseMode | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:554` | — |
| Game API · findPlayerItem | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:596` | — |
| Game API · use | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:598` | — |
| Game API · useInventoryItem | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:601` | — |
| Game API · useWith | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:607` | — |
| Game API · useInventoryItemWith | Unverified | `modules/game_hotkeys/hotkeys_manager.lua:610` | — |
| Hotkeys | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:13` | — |
| Current hotkeys | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:21` | — |
| Reset All | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:46` | — |
| Activate · modules.game_hotkeys.reset() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:49` | — |
| Add | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:54` | — |
| Activate · modules.game_hotkeys.addHotkey() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:59` | — |
| Remove | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:63` | — |
| Activate · modules.game_hotkeys.removeHotkey() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:69` | — |
| Hotkey Action | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:73` | — |
| Edit hotkey text | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:91` | — |
| Send automatically | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:109` | — |
| Select object | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:126` | — |
| Activate · modules.game_hotkeys.startChooseItem() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:132` | — |
| Clear object | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:136` | — |
| Activate · modules.game_hotkeys.clearObject() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:143` | — |
| Use on yourself | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:147` | — |
| Use on target | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:158` | — |
| With crosshair | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:169` | — |
| Use at cursor position | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:180` | — |
| Ok | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:198` | — |
| Activate · modules.game_hotkeys.ok() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:202` | — |
| Cancel | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:207` | — |
| Activate · modules.game_hotkeys.cancel() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:211` | — |
| Button Assign | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:215` | — |
| Please, press the key you wish to add onto your hotkeys manager | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:220` | — |
| Current hotkey to add: %s | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:229` | — |
| Activate · modules.game_hotkeys.hotkeyCaptureOk(self:getParent(), self:getParent():getChildById('comboPreview').keyCombo) | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:249` | — |
| Activate · self:getParent():destroy() | Unverified | `modules/game_hotkeys/hotkeys_manager.otui:257` | — |

### game_imbuementtracker

PRESENT 1 | Unverified 10

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Refresh duration tracker | PRESENT | `modules/game_imbuementtracker/imbuementtracker.lua:45` | `apps/client/src/bevy_shell/game_ui/imbuement_tracker.rs:208` → `apps/client/src/features/0068-imbue/input/tracker.rs:94` |
| Game API · imbuementDurations | Unverified | `modules/game_imbuementtracker/imbuementtracker.lua:45` | — |
| Event · onGameStart | Unverified | `modules/game_imbuementtracker/imbuementtracker.lua:51` | — |
| Event · onGameEnd | Unverified | `modules/game_imbuementtracker/imbuementtracker.lua:52` | — |
| Event · onUpdateImbuementTracker | Unverified | `modules/game_imbuementtracker/imbuementtracker.lua:53` | — |
| Event · onCheckChange | Unverified | `modules/game_imbuementtracker/imbuementtracker.lua:106` | — |
| Show imbuements that last less than 1h | Unverified | `modules/game_imbuementtracker/imbuementtracker.otui:21` | — |
| Show imbuements that last between 1h and 3h | Unverified | `modules/game_imbuementtracker/imbuementtracker.otui:25` | — |
| Show imbuements that last more than 3h | Unverified | `modules/game_imbuementtracker/imbuementtracker.otui:29` | — |
| Show items with no active imbuement | Unverified | `modules/game_imbuementtracker/imbuementtracker.otui:33` | — |
| Imbuements | Unverified | `modules/game_imbuementtracker/imbuementtracker.otui:104` | — |

### game_imbuing

PRESENT 3 | Unverified 45

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Apply imbuement | PRESENT | `modules/game_imbuing/new_design/classes/imbuementitem.lua:417` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:63` → `apps/client/src/features/0068-imbue/input.rs:127` |
| Clear imbuement | PRESENT | `modules/game_imbuing/new_design/classes/imbuementitem.lua:192` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:67` → `apps/client/src/features/0068-imbue/input.rs:127` |
| Select item | PRESENT | `modules/game_imbuing/new_design/classes/imbuementselection.lua:90` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:282` → `apps/client/src/features/0068-imbue/input.rs:127` |
| Event · onGameStart | Unverified | `modules/game_imbuing/imbuing.lua:130` | — |
| Event · onGameEnd | Unverified | `modules/game_imbuing/imbuing.lua:131` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_imbuing/imbuing.lua:135` | — |
| Event · onMessageDialog | Unverified | `modules/game_imbuing/imbuing.lua:137` | — |
| Event · onHoverChange | Unverified | `modules/game_imbuing/new_design/classes/imbuementitem.lua:154` | — |
| Game API · clearImbuement | Unverified | `modules/game_imbuing/new_design/classes/imbuementitem.lua:192` | — |
| Game API · applyImbuement | Unverified | `modules/game_imbuing/new_design/classes/imbuementitem.lua:417` | — |
| Game API · selectImbuementScroll | Unverified | `modules/game_imbuing/new_design/classes/imbuementselection.lua:88` | — |
| Game API · selectImbuementItem | Unverified | `modules/game_imbuing/new_design/classes/imbuementselection.lua:90` | — |
| Game API · closeImbuingWindow | Unverified | `modules/game_imbuing/new_design/t_imbui.lua:121` | — |
| Activate · modules.game_imbuing.onSelectItem() | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:8` | — |
| Activate · modules.game_imbuing.onSelectScroll() | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:22` | — |
| Pick Item or Imbue a Blank Scroll | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:88` | — |
| Activate · modules.game_imbuing.onItemSlot(self) | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:147` | — |
| Item Information | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:176` | — |
| Select Quality and Imbuement | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:197` | — |
| Activate · modules.game_imbuing.onItemBaseType(self:getId()) | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:220` | — |
| Raises your walking speed by 30. Lasts for 20h 0min while equipped. | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:277` | — |
| Imbue Empty Slot with  | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:296` | — |
| Selected Imbuement | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:476` | — |
| Clear Imbuement  | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:522` | — |
| Blank Imbuement Scroll | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:643` | — |
| Select an Imbuement | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:664` | — |
| Activate · modules.game_imbuing.onScrollBaseType(self:getId()) | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:687` | — |
| Imbue Blank Scroll with  | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:745` | — |
| Imbuement Shrine | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:836` | — |
| Close | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:872` | — |
| Activate · modules.game_imbuing.close() | Unverified | `modules/game_imbuing/new_design/t_imbui.otui:876` | — |
| Event · onOptionChange | Unverified | `modules/game_imbuing/old_design/imbuing.lua:75` | — |
| The imbuement requires  | Unverified | `modules/game_imbuing/old_design/imbuing.lua:117` | — |
| Items can have up to three imbuements slots. This slot is not available for this item. | Unverified | `modules/game_imbuing/old_design/imbuing.otui:7` | — |
| Slot 1 | Unverified | `modules/game_imbuing/old_design/imbuing.otui:73` | — |
| Slot 2 | Unverified | `modules/game_imbuing/old_design/imbuing.otui:86` | — |
| Slot 3 | Unverified | `modules/game_imbuing/old_design/imbuing.otui:99` | — |
| Select Slot: | Unverified | `modules/game_imbuing/old_design/imbuing.otui:116` | — |
| Imbue Empty Slot | Unverified | `modules/game_imbuing/old_design/imbuing.otui:130` | — |
| Requires the following astral sources: | Unverified | `modules/game_imbuing/old_design/imbuing.otui:160` | — |
| Success Rate: | Unverified | `modules/game_imbuing/old_design/imbuing.otui:175` | — |
| Click here to carry out the selected imbuement. This will consume the required astral sources and gold. | Unverified | `modules/game_imbuing/old_design/imbuing.otui:239` | — |
| Clear Imbuement | Unverified | `modules/game_imbuing/old_design/imbuing.otui:265` | — |
| Time remaining: | Unverified | `modules/game_imbuing/old_design/imbuing.otui:296` | — |
| Clear imbuement: | Unverified | `modules/game_imbuing/old_design/imbuing.otui:302` | — |
| Your needs have changed? Click here to clear the imbuement from your item for a fee. | Unverified | `modules/game_imbuing/old_design/imbuing.otui:345` | — |
| Imbue Item | Unverified | `modules/game_imbuing/old_design/imbuing.otui:378` | — |
| Activate · modules.game_imbuing.hide() | Unverified | `modules/game_imbuing/old_design/imbuing.otui:415` | — |

### game_inspect

Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGeometryChange | Unverified | `modules/game_inspect/inspect.lua:416` | — |
| Game API · openWheel | Unverified | `modules/game_inspect/inspect.lua:561` | — |

### game_interface

Unverified 111

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_interface/gameinterface.lua:122` | — |
| Event · onGameEnd | Unverified | `modules/game_interface/gameinterface.lua:123` | — |
| Event · onUpdateFinished | Unverified | `modules/game_interface/gameinterface.lua:133` | — |
| Event · onGeometryChange | Unverified | `modules/game_interface/gameinterface.lua:148` | — |
| Event · onCheckChange | Unverified | `modules/game_interface/gameinterface.lua:197` | — |
| Logout | Unverified | `modules/game_interface/gameinterface.lua:328` | — |
| Exit | Unverified | `modules/game_interface/gameinterface.lua:360` | — |
| Game API · safeLogout | Unverified | `modules/game_interface/gameinterface.lua:409` | — |
| Game API · forceLogout | Unverified | `modules/game_interface/gameinterface.lua:462` | — |
| Game API · useWith | Unverified | `modules/game_interface/gameinterface.lua:551` | — |
| Game API · requestTrade | Unverified | `modules/game_interface/gameinterface.lua:570` | — |
| Look | Unverified | `modules/game_interface/gameinterface.lua:673` | — |
| Game API · look | Unverified | `modules/game_interface/gameinterface.lua:674` | — |
| Inspect | Unverified | `modules/game_interface/gameinterface.lua:679` | — |
| Game API · inspectCharacter | Unverified | `modules/game_interface/gameinterface.lua:681` | — |
| Game API · inspectionNormalObject | Unverified | `modules/game_interface/gameinterface.lua:685` | — |
| Game API · inspectionObject | Unverified | `modules/game_interface/gameinterface.lua:687` | — |
| Cyclopedia | Unverified | `modules/game_interface/gameinterface.lua:693` | — |
| Weapon Proficiency | Unverified | `modules/game_interface/gameinterface.lua:698` | — |
| Open | Unverified | `modules/game_interface/gameinterface.lua:712` | — |
| Game API · open | Unverified | `modules/game_interface/gameinterface.lua:713` | — |
| Open in new window | Unverified | `modules/game_interface/gameinterface.lua:715` | — |
| Use with ... | Unverified | `modules/game_interface/gameinterface.lua:725` | — |
| Use | Unverified | `modules/game_interface/gameinterface.lua:729` | — |
| Game API · use | Unverified | `modules/game_interface/gameinterface.lua:730` | — |
| Rotate | Unverified | `modules/game_interface/gameinterface.lua:736` | — |
| Game API · rotate | Unverified | `modules/game_interface/gameinterface.lua:737` | — |
| Game API · wrap | Unverified | `modules/game_interface/gameinterface.lua:742` | — |
| Wrap | Unverified | `modules/game_interface/gameinterface.lua:745` | — |
| Unwrap | Unverified | `modules/game_interface/gameinterface.lua:748` | — |
| Browse Field | Unverified | `modules/game_interface/gameinterface.lua:752` | — |
| Game API · browseField | Unverified | `modules/game_interface/gameinterface.lua:753` | — |
| Game API · sendQuickLoot | Unverified | `modules/game_interface/gameinterface.lua:758` | — |
| Trade with ... | Unverified | `modules/game_interface/gameinterface.lua:765` | — |
| Move up | Unverified | `modules/game_interface/gameinterface.lua:773` | — |
| Game API · moveToParentContainer | Unverified | `modules/game_interface/gameinterface.lua:774` | — |
| Game API · requestOutfit | Unverified | `modules/game_interface/gameinterface.lua:785` | — |
| Prey Dialog | Unverified | `modules/game_interface/gameinterface.lua:789` | — |
| Mount | Unverified | `modules/game_interface/gameinterface.lua:796` | — |
| Dismount | Unverified | `modules/game_interface/gameinterface.lua:800` | — |
| Disable Shared Experience | Unverified | `modules/game_interface/gameinterface.lua:809` | — |
| Game API · partyShareExperience | Unverified | `modules/game_interface/gameinterface.lua:810` | — |
| Enable Shared Experience | Unverified | `modules/game_interface/gameinterface.lua:813` | — |
| Leave Party | Unverified | `modules/game_interface/gameinterface.lua:818` | — |
| Game API · partyLeave | Unverified | `modules/game_interface/gameinterface.lua:819` | — |
| Talk | Unverified | `modules/game_interface/gameinterface.lua:831` | — |
| Game API · talk | Unverified | `modules/game_interface/gameinterface.lua:832` | — |
| Attack | Unverified | `modules/game_interface/gameinterface.lua:837` | — |
| Game API · attack | Unverified | `modules/game_interface/gameinterface.lua:838` | — |
| Stop Attack | Unverified | `modules/game_interface/gameinterface.lua:841` | — |
| Follow | Unverified | `modules/game_interface/gameinterface.lua:847` | — |
| Game API · follow | Unverified | `modules/game_interface/gameinterface.lua:848` | — |
| Stop Follow | Unverified | `modules/game_interface/gameinterface.lua:851` | — |
| Message to %s | Unverified | `modules/game_interface/gameinterface.lua:860` | — |
| Game API · openPrivateChannel | Unverified | `modules/game_interface/gameinterface.lua:861` | — |
| Invite to private chat | Unverified | `modules/game_interface/gameinterface.lua:864` | — |
| Game API · inviteToOwnChannel | Unverified | `modules/game_interface/gameinterface.lua:865` | — |
| Exclude from private chat | Unverified | `modules/game_interface/gameinterface.lua:867` | — |
| Game API · excludeFromOwnChannel | Unverified | `modules/game_interface/gameinterface.lua:868` | — |
| Add to VIP list | Unverified | `modules/game_interface/gameinterface.lua:872` | — |
| Game API · addVip | Unverified | `modules/game_interface/gameinterface.lua:873` | — |
| Unignore | Unverified | `modules/game_interface/gameinterface.lua:878` | — |
| Ignore | Unverified | `modules/game_interface/gameinterface.lua:882` | — |
| Join %s\ | Unverified | `modules/game_interface/gameinterface.lua:892` | — |
| Game API · partyJoin | Unverified | `modules/game_interface/gameinterface.lua:893` | — |
| Invite to Party | Unverified | `modules/game_interface/gameinterface.lua:896` | — |
| Game API · partyInvite | Unverified | `modules/game_interface/gameinterface.lua:897` | — |
| Revoke %s\ | Unverified | `modules/game_interface/gameinterface.lua:902` | — |
| Game API · partyRevokeInvitation | Unverified | `modules/game_interface/gameinterface.lua:903` | — |
| Pass Leadership to %s | Unverified | `modules/game_interface/gameinterface.lua:914` | — |
| Game API · partyPassLeadership | Unverified | `modules/game_interface/gameinterface.lua:915` | — |
| Rule Violation | Unverified | `modules/game_interface/gameinterface.lua:928` | — |
| Copy Name | Unverified | `modules/game_interface/gameinterface.lua:934` | — |
| ID:  | Unverified | `modules/game_interface/gameinterface.lua:956` | — |
| Stow | Unverified | `modules/game_interface/gameinterface.lua:985` | — |
| Stow all items of this type | Unverified | `modules/game_interface/gameinterface.lua:988` | — |
| Game API · stashStowItem | Unverified | `modules/game_interface/gameinterface.lua:989` | — |
| Stow container\ | Unverified | `modules/game_interface/gameinterface.lua:995` | — |
| Game API · move | Unverified | `modules/game_interface/gameinterface.lua:1171` | — |
| Game API · setChaseMode | Unverified | `modules/game_interface/gameinterface.lua:1543` | — |
| Event · onValueChange | Unverified | `modules/game_interface/gameinterface.lua:1581` | — |
| Show Top Menu | Unverified | `modules/game_interface/gameinterface.otui:66` | — |
| Activate · modules.game_actionbar.changeLockStatus(self, "Left") | Unverified | `modules/game_interface/gameinterface.otui:105` | — |
| Activate · modules.game_actionbar.changeLockStatus(self, "Right") | Unverified | `modules/game_interface/gameinterface.otui:145` | — |
| Activate · modules.game_actionbar.changeLockStatus(self, "Bottom") | Unverified | `modules/game_interface/gameinterface.otui:181` | — |
| Activate · onIncreaseLeftPanels() | Unverified | `modules/game_interface/gameinterface.otui:338` | — |
| Activate · onDecreaseLeftPanels() | Unverified | `modules/game_interface/gameinterface.otui:352` | — |
| Activate · onIncreaseRightPanels() | Unverified | `modules/game_interface/gameinterface.otui:367` | — |
| Activate · onDecreaseRightPanels() | Unverified | `modules/game_interface/gameinterface.otui:382` | — |
| Stow Items | Unverified | `modules/game_interface/styles/countStashWindow.otui:3` | — |
| Cancel | Unverified | `modules/game_interface/styles/countStashWindow.otui:36` | — |
| Ok | Unverified | `modules/game_interface/styles/countStashWindow.otui:46` | — |
| Move Stackable Item | Unverified | `modules/game_interface/styles/countwindow.otui:3` | — |
| Mana Shield | Unverified | `modules/game_interface/widgets/statsbar.lua:308` | — |
| Event · onStatsTextUpdate | Unverified | `modules/game_interface/widgets/statsbar.lua:878` | — |
| Show | Unverified | `modules/game_interface/widgets/statsbar.lua:927` | — |
| Hide | Unverified | `modules/game_interface/widgets/statsbar.lua:932` | — |
| Hide Customisable Status Bars | Unverified | `modules/game_interface/widgets/statsbar.lua:941` | — |
| Event · onExperienceChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1180` | — |
| Event · onLevelChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1181` | — |
| Event · onHealthChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1182` | — |
| Event · onManaChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1183` | — |
| Event · onManaShieldChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1184` | — |
| Event · onMagicLevelChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1185` | — |
| Event · onBaseMagicLevelChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1186` | — |
| Event · onSkillChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1187` | — |
| Event · onBaseSkillChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1188` | — |
| Event · onStatesChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1189` | — |
| Event · onHarmonyChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1190` | — |
| Event · onSereneChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1191` | — |
| Event · onVocationChange | Unverified | `modules/game_interface/widgets/statsbar.lua:1192` | — |

### game_inventory

Unverified 23

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Game API · sendWeaponProficiencyAction | Unverified | `modules/game_inventory/inventory.lua:152` | — |
| Game API · setChaseMode | Unverified | `modules/game_inventory/inventory.lua:256` | — |
| Event · onCheckChange | Unverified | `modules/game_inventory/inventory.lua:267` | — |
| Event · onSelectionChange | Unverified | `modules/game_inventory/inventory.lua:281` | — |
| Event · onInventoryChange | Unverified | `modules/game_inventory/inventory.lua:291` | — |
| Event · onSoulChange | Unverified | `modules/game_inventory/inventory.lua:292` | — |
| Event · onFreeCapacityChange | Unverified | `modules/game_inventory/inventory.lua:293` | — |
| Game API · setFightMode | Unverified | `modules/game_inventory/inventory.lua:304` | — |
| Game API · setSafeFight | Unverified | `modules/game_inventory/inventory.lua:306` | — |
| Game API · setPVPMode | Unverified | `modules/game_inventory/inventory.lua:308` | — |
| Event · onFightModeChange | Unverified | `modules/game_inventory/inventory.lua:317` | — |
| Event · onChaseModeChange | Unverified | `modules/game_inventory/inventory.lua:318` | — |
| Event · onSafeFightChange | Unverified | `modules/game_inventory/inventory.lua:319` | — |
| Event · onPVPModeChange | Unverified | `modules/game_inventory/inventory.lua:320` | — |
| Activate · changeInventorySize() | Unverified | `modules/game_inventory/inventory.otui:58` | — |
| Activate · modules.game_blessing.toggle() | Unverified | `modules/game_inventory/inventory.otui:71` | — |
| Open purse | Unverified | `modules/game_inventory/inventory.otui:78` | — |
| Activate · \| | Unverified | `modules/game_inventory/inventory.otui:84` | — |
| Game API · use | Unverified | `modules/game_inventory/inventory.otui:87` | — |
| Stop | Unverified | `modules/game_inventory/inventory.otui:240` | — |
| Game API · stop | Unverified | `modules/game_inventory/inventory.otui:248` | — |
| Soul | Unverified | `modules/game_inventory/inventory.otui:347` | — |
| Cap | Unverified | `modules/game_inventory/inventory.otui:376` | — |

### game_joystick

Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_joystick/joystick.lua:21` | — |
| Event · onGameEnd | Unverified | `modules/game_joystick/joystick.lua:22` | — |

### game_lootsplitter

MISSING 2 | Unverified 17

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Generate a settlement from a pasted hunt session log | MISSING | `modules/game_lootsplitter/lootsplitter.lua:207` | `apps/client/src/bevy_shell/game_ui/analyser/widgets.rs:182` offers a party refresh. No paste-and-settle action found. |
| Copy generated settlement | MISSING | `modules/game_lootsplitter/lootsplitter.lua:201` | `apps/client/src/bevy_shell/game_ui/analyser/widgets.rs:182` has no settlement output to copy. |
| PlayerName | Unverified | `modules/game_lootsplitter/advanced.otui:11` | — |
| Remove Player | Unverified | `modules/game_lootsplitter/advanced.otui:27` | — |
| Loot Splitter - Advanced Options | Unverified | `modules/game_lootsplitter/advanced.otui:37` | — |
| Close | Unverified | `modules/game_lootsplitter/advanced.otui:68` | — |
| Activate · advancedOptions:hide() lootsplitter:show() | Unverified | `modules/game_lootsplitter/advanced.otui:70` | — |
| Apply | Unverified | `modules/game_lootsplitter/advanced.otui:77` | — |
| Activate · applyChanges() | Unverified | `modules/game_lootsplitter/advanced.otui:78` | — |
| Event · onTextChange | Unverified | `modules/game_lootsplitter/lootsplitter.lua:257` | — |
| Event · onCheckChange | Unverified | `modules/game_lootsplitter/lootsplitter.lua:275` | — |
| Hunt Session Log | Unverified | `modules/game_lootsplitter/lootsplitter.otui:23` | — |
| Activate · onGenerateButtonClick() | Unverified | `modules/game_lootsplitter/lootsplitter.otui:65` | — |
| Hunt Session Result | Unverified | `modules/game_lootsplitter/lootsplitter.otui:73` | — |
| Activate · hide() | Unverified | `modules/game_lootsplitter/lootsplitter.otui:112` | — |
| Copy | Unverified | `modules/game_lootsplitter/lootsplitter.otui:119` | — |
| Activate · copyResultText() | Unverified | `modules/game_lootsplitter/lootsplitter.otui:120` | — |
| Advanced Options | Unverified | `modules/game_lootsplitter/lootsplitter.otui:127` | — |
| Activate · showAdvancedOptions() | Unverified | `modules/game_lootsplitter/lootsplitter.otui:128` | — |

### game_mainpanel

Unverified 8

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Activate · changeOptionsSize() | Unverified | `modules/game_mainpanel/mainoptionspanel.otui:20` | — |
| Displayed Buttons | Unverified | `modules/game_mainpanel/option_control_buttons.otui:30` | — |
| Activate · moveToAvailable() | Unverified | `modules/game_mainpanel/option_control_buttons.otui:64` | — |
| Activate · moveButtonUp() | Unverified | `modules/game_mainpanel/option_control_buttons.otui:77` | — |
| Activate · moveButtonDown() | Unverified | `modules/game_mainpanel/option_control_buttons.otui:90` | — |
| Available Buttons | Unverified | `modules/game_mainpanel/option_control_buttons.otui:111` | — |
| Activate · moveToDisplayed() | Unverified | `modules/game_mainpanel/option_control_buttons.otui:143` | — |
| Activate · reset() | Unverified | `modules/game_mainpanel/option_control_buttons.otui:158` | — |

### game_market

PRESENT 3 | Unverified 80

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Create offer | PRESENT | `modules/game_market/market_helpers.lua:20` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:241` → `apps/client/src/features/0063-market/input.rs:45` |
| Cancel own offer | PRESENT | `modules/game_market/market_helpers.lua:24` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:251` → `apps/client/src/features/0063-market/input.rs:45` |
| Accept offer | PRESENT | `modules/game_market/market_helpers.lua:16` | `apps/client/src/bevy_shell/game_ui/market/offers.rs:275` → `apps/client/src/features/0063-market/input.rs:45` |
| Event · onTopListValueChange | Unverified | `modules/game_market/classes/t_history.lua:4` | — |
| Event · onBottomListValueChange | Unverified | `modules/game_market/classes/t_history.lua:5` | — |
| Event · onValueChange | Unverified | `modules/game_market/classes/t_history.lua:186` | — |
| Event · onChildFocusChange | Unverified | `modules/game_market/classes/t_history.lua:240` | — |
| Game API · browseMarket | Unverified | `modules/game_market/market_helpers.lua:3` | — |
| Game API · leaveMarket | Unverified | `modules/game_market/market_helpers.lua:12` | — |
| Game API · acceptMarketOffer | Unverified | `modules/game_market/market_helpers.lua:16` | — |
| Game API · createMarketOffer | Unverified | `modules/game_market/market_helpers.lua:20` | — |
| Event · onCheckChange | Unverified | `modules/game_market/t_market.lua:82` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_market/t_market.lua:89` | — |
| Event · onGameEnd | Unverified | `modules/game_market/t_market.lua:90` | — |
| Event · onGameStart | Unverified | `modules/game_market/t_market.lua:91` | — |
| Event · onMarketEnter | Unverified | `modules/game_market/t_market.lua:92` | — |
| Event · onMarketBrowse | Unverified | `modules/game_market/t_market.lua:93` | — |
| Event · onMarketDetail | Unverified | `modules/game_market/t_market.lua:94` | — |
| Event · onMarketReadOffer | Unverified | `modules/game_market/t_market.lua:95` | — |
| Event · onParseStoreGetCoin | Unverified | `modules/game_market/t_market.lua:96` | — |
| Event · onMarketLeave | Unverified | `modules/game_market/t_market.lua:97` | — |
| %s%s%s%s | Unverified | `modules/game_market/t_market.lua:1064` | — |
| All | Unverified | `modules/game_market/t_market.lua:1146` | — |
| None | Unverified | `modules/game_market/t_market.lua:1147` | — |
| Class  | Unverified | `modules/game_market/t_market.lua:1149` | — |
| Tier  | Unverified | `modules/game_market/t_market.lua:1154` | — |
| Number of Transactions: 0 | Unverified | `modules/game_market/t_market.otui:33` | — |
| Highest Price: 0 gold | Unverified | `modules/game_market/t_market.otui:44` | — |
| Average Price: 0 gold | Unverified | `modules/game_market/t_market.otui:55` | — |
| Lowest Price: 0 gold | Unverified | `modules/game_market/t_market.otui:66` | — |
| Unknow | Unverified | `modules/game_market/t_market.otui:419` | — |
| Level | Unverified | `modules/game_market/t_market.otui:480` | — |
| Activate · modules.game_market.onSortMarketFields(self, self:isChecked()) | Unverified | `modules/game_market/t_market.otui:483` | — |
| Voc. | Unverified | `modules/game_market/t_market.otui:491` | — |
| 1H | Unverified | `modules/game_market/t_market.otui:502` | — |
| 2H | Unverified | `modules/game_market/t_market.otui:514` | — |
| Tier 0 | Unverified | `modules/game_market/t_market.otui:532` | — |
| Show Locker only | Unverified | `modules/game_market/t_market.otui:541` | — |
| Items: | Unverified | `modules/game_market/t_market.otui:560` | — |
| Search: | Unverified | `modules/game_market/t_market.otui:620` | — |
| Activate · modules.game_market.onClearSearch() | Unverified | `modules/game_market/t_market.otui:647` | — |
| Details: | Unverified | `modules/game_market/t_market.otui:664` | — |
| Statistics: | Unverified | `modules/game_market/t_market.otui:700` | — |
| Sell Offers: | Unverified | `modules/game_market/t_market.otui:746` | — |
| Amount: | Unverified | `modules/game_market/t_market.otui:756` | — |
| 0 | Unverified | `modules/game_market/t_market.otui:767` | — |
| Total: | Unverified | `modules/game_market/t_market.otui:792` | — |
| Accept | Unverified | `modules/game_market/t_market.otui:813` | — |
| Activate · modules.game_market.onAcceptSellOffer() | Unverified | `modules/game_market/t_market.otui:817` | — |
| Buy Offers: | Unverified | `modules/game_market/t_market.otui:998` | — |
| Activate · modules.game_market.onAcceptBuyOffer() | Unverified | `modules/game_market/t_market.otui:1069` | — |
| Create Offer: | Unverified | `modules/game_market/t_market.otui:1248` | — |
| Sell | Unverified | `modules/game_market/t_market.otui:1256` | — |
| Activate · modules.game_market.changeOfferType(self, true) | Unverified | `modules/game_market/t_market.otui:1267` | — |
| Buy | Unverified | `modules/game_market/t_market.otui:1275` | — |
| Activate · modules.game_market.changeOfferType(self, false) | Unverified | `modules/game_market/t_market.otui:1286` | — |
| Amount: 0 | Unverified | `modules/game_market/t_market.otui:1296` | — |
| Piece Price: | Unverified | `modules/game_market/t_market.otui:1320` | — |
| Gross Profit: | Unverified | `modules/game_market/t_market.otui:1346` | — |
| Fee: | Unverified | `modules/game_market/t_market.otui:1373` | — |
| Total Profit: | Unverified | `modules/game_market/t_market.otui:1409` | — |
| Anonymous | Unverified | `modules/game_market/t_market.otui:1434` | — |
| Create | Unverified | `modules/game_market/t_market.otui:1456` | — |
| Activate · modules.game_market.createMarketOffer() | Unverified | `modules/game_market/t_market.otui:1459` | — |
| Activate · modules.game_market.openStorePotions() | Unverified | `modules/game_market/t_market.otui:1469` | — |
| Activate · modules.game_market.buyPoints() | Unverified | `modules/game_market/t_market.otui:1545` | — |
| Market | Unverified | `modules/game_market/t_market.otui:1552` | — |
| Activate · offersButton() | Unverified | `modules/game_market/t_market.otui:1554` | — |
| Close | Unverified | `modules/game_market/t_market.otui:1561` | — |
| Activate · closeMarket() | Unverified | `modules/game_market/t_market.otui:1565` | — |
| My Offers | Unverified | `modules/game_market/t_market.otui:1571` | — |
| Activate · myOffersButton(self) | Unverified | `modules/game_market/t_market.otui:1576` | — |
| Details | Unverified | `modules/game_market/t_market.otui:1582` | — |
| Activate · detailsButton() | Unverified | `modules/game_market/t_market.otui:1587` | — |
| Offers | Unverified | `modules/game_market/t_market.otui:1593` | — |
| Cancel Offer | Unverified | `modules/game_market/t_market.otui:1616` | — |
| Activate · modules.game_market.MarketOwnOffers.cancelMarketOffer(true) | Unverified | `modules/game_market/t_market.otui:1619` | — |
| Sell Offers (0): | Unverified | `modules/game_market/t_market.otui:1627` | — |
| Buy Offers (0): | Unverified | `modules/game_market/t_market.otui:1836` | — |
| Activate · modules.game_market.MarketOwnOffers.cancelMarketOffer(false) | Unverified | `modules/game_market/t_market.otui:1848` | — |
| Offer History | Unverified | `modules/game_market/t_market.otui:2127` | — |
| Current Offers | Unverified | `modules/game_market/t_market.otui:2136` | — |
| Activate · openPlataform(self) | Unverified | `modules/game_market/t_market.otui:2576` | — |

### game_minimap

Unverified 9

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onChangeWorldTime | Unverified | `modules/game_minimap/minimap.lua:108` | — |
| Event · onPositionChange | Unverified | `modules/game_minimap/minimap.lua:112` | — |
| Activate · upLayer() | Unverified | `modules/game_minimap/minimap.otui:60` | — |
| Activate · downLayer() | Unverified | `modules/game_minimap/minimap.otui:68` | — |
| Activate · openCyclopediaMap() | Unverified | `modules/game_minimap/minimap.otui:78` | — |
| Activate · zoomIn() | Unverified | `modules/game_minimap/minimap.otui:94` | — |
| Activate · zoomOut() | Unverified | `modules/game_minimap/minimap.otui:110` | — |
| Activate · resetMap() | Unverified | `modules/game_minimap/minimap.otui:148` | — |
| Activate · \| | Unverified | `modules/game_minimap/minimap.otui:164` | — |

### game_modaldialog

Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onModalDialog | Unverified | `modules/game_modaldialog/modaldialog.lua:20` | — |
| Game API · answerModalDialog | Unverified | `modules/game_modaldialog/modaldialog.lua:35` | — |

### game_notifications

Unverified 8

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Only Capture Game Window | Unverified | `modules/game_notifications/templates/screenshot.otui:27` | — |
| Keep Blacklog of the Screenshots of the Last 5 Seconds | Unverified | `modules/game_notifications/templates/screenshot.otui:39` | — |
| Enable Auto Screenshots | Unverified | `modules/game_notifications/templates/screenshot.otui:52` | — |
| Select all events that sould trigger auto Screenshots: | Unverified | `modules/game_notifications/templates/screenshot.otui:57` | — |
| Reset | Unverified | `modules/game_notifications/templates/screenshot.otui:80` | — |
| Activate · modules.game_notifications.resetValues() | Unverified | `modules/game_notifications/templates/screenshot.otui:84` | — |
| Open Screenshots Folder | Unverified | `modules/game_notifications/templates/screenshot.otui:87` | — |
| Activate · modules.game_notifications.OpenFolder() | Unverified | `modules/game_notifications/templates/screenshot.otui:91` | — |

### game_npctrade

PRESENT 3 | Unverified 38

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Buy item | PRESENT | `modules/game_npctrade/controllers/npc_legacy_ui.lua:160` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:141` → `apps/client/src/features/0081-npc-shop/input.rs:16` |
| Sell item | PRESENT | `modules/game_npctrade/controllers/npc_legacy_ui.lua:163` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:141` → `apps/client/src/features/0081-npc-shop/input.rs:16` |
| Inspect shop item | PRESENT | `modules/game_npctrade/controllers/npc_legacy_ui.lua:181` | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:143` → `apps/client/src/features/0081-npc-shop/view.rs:18` |
| Event · onSelectionChange | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:74` | — |
| Event · onFreeCapacityChange | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:83` | — |
| Event · onInventoryChange | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:84` | — |
| Game API · buyItem | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:160` | — |
| Game API · sellItem | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:163` | — |
| Look | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:180` | — |
| Game API · inspectNpcTrade | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:181` | — |
| Game API · closeNpcTrade | Unverified | `modules/game_npctrade/controllers/npc_legacy_ui.lua:435` | — |
| Sort by Name | Unverified | `modules/game_npctrade/controllers/npc_options.lua:4` | — |
| Sort by Price | Unverified | `modules/game_npctrade/controllers/npc_options.lua:7` | — |
| Sort by Weight | Unverified | `modules/game_npctrade/controllers/npc_options.lua:10` | — |
| Event · onScrollChange | Unverified | `modules/game_npctrade/controllers/npc_trader.lua:175` | — |
| Inspect | Unverified | `modules/game_npctrade/controllers/npc_trader.lua:230` | — |
| Game API · inspectionObject | Unverified | `modules/game_npctrade/controllers/npc_trader.lua:231` | — |
| Game API · imbuementDurations | Unverified | `modules/game_npctrade/controllers/npc_trader.lua:387` | — |
| Event · onOpenNpcTrade | Unverified | `modules/game_npctrade/game_npctrader.lua:28` | — |
| Event · onUpdateImbuementTracker | Unverified | `modules/game_npctrade/game_npctrader.lua:43` | — |
| Event · onCloseNpcTrade | Unverified | `modules/game_npctrade/game_npctrader.lua:54` | — |
| Event · onTalk | Unverified | `modules/game_npctrade/game_npctrader.lua:71` | — |
| NPC Trade | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:35` | — |
| List of items that you | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:41` | — |
| Buy | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:42` | — |
| Sell | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:54` | — |
| Name | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:104` | — |
| Price | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:115` | — |
| Your Money | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:126` | — |
| Weight | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:138` | — |
| Your Capacity | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:150` | — |
| Search | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:185` | — |
| Buy with backpack | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:204` | — |
| Ignore capacity | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:214` | — |
| Ignore equipped | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:224` | — |
| Show all items | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:236` | — |
| Sell All | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:248` | — |
| Activate · modules.game_npctrade.sellAll() | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:254` | — |
| Activate · modules.game_npctrade.onTradeClick() | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:263` | — |
| Close | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:266` | — |
| Activate · modules.game_npctrade.closeNpcTrade() | Unverified | `modules/game_npctrade/templates/npctrade_legacy.otui:270` | — |

### game_outfit

PRESENT 1 | Unverified 6

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Submit appearance change | PRESENT | `modules/game_outfit/outfit.lua:1877` | `apps/client/src/bevy_shell/game_ui/outfit.rs:185` → `apps/client/src/features/0025-appearance/input.rs:45` |
| Event · onChildFocusChange | Unverified | `modules/game_outfit/outfit.lua:96` | — |
| Event · onOpenOutfitWindow | Unverified | `modules/game_outfit/outfit.lua:155` | — |
| Event · onGameEnd | Unverified | `modules/game_outfit/outfit.lua:156` | — |
| Event · onCheckChange | Unverified | `modules/game_outfit/outfit.lua:398` | — |
| Event · onSelectionChange | Unverified | `modules/game_outfit/outfit.lua:461` | — |
| Game API · changeOutfit | Unverified | `modules/game_outfit/outfit.lua:1877` | — |

### game_paperdolls

NOT-APPLICABLE 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Attached Paperdolls System | NOT-APPLICABLE | `modules/game_paperdolls/paperdolls.otmod:1` | OTClient developer facility or OT server extension |
| Event · onOutfitChange | NOT-APPLICABLE | `modules/game_paperdolls/setup.lua:46` | OT extension |

### game_playerdeath

Unverified 4

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| You are dead | Unverified | `modules/game_playerdeath/deathwindow.otui:3` | — |
| Ok | Unverified | `modules/game_playerdeath/deathwindow.otui:18` | — |
| Cancel | Unverified | `modules/game_playerdeath/deathwindow.otui:26` | — |
| Game API · safeLogout | Unverified | `modules/game_playerdeath/playerdeath.lua:85` | — |

### game_playermount

PRESENT 1 | Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Mount or dismount | PRESENT | `modules/game_playermount/playermount.lua:40` | `apps/client/src/bevy_shell/game_ui/outfit.rs:177` → `apps/client/src/features/0502-actor-motion/protocol.rs:90` |
| Event · onGameStart | Unverified | `modules/game_playermount/playermount.lua:3` | — |
| Event · onGameEnd | Unverified | `modules/game_playermount/playermount.lua:4` | — |

### game_playertrade

PRESENT 3 | PARTIAL 1 | Unverified 12

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Accept trade | PRESENT | `modules/game_playertrade/tradewindow.otui:98` | `apps/client/src/bevy_shell/game_ui/trade.rs:36` → `apps/client/src/features/0060-player-trade/input.rs:6` |
| Reject trade | PRESENT | `modules/game_playertrade/playertrade.lua:30` | `apps/client/src/bevy_shell/game_ui/trade.rs:37` → `apps/client/src/features/0060-player-trade/input.rs:6` |
| Identify an offered item visually | PARTIAL | `modules/game_playertrade/playertrade.lua:59` | `apps/client/src/bevy_shell/game_ui/trade/widgets.rs:346` renders a question mark and stack count. The offered item has no usable appearance in this slot. |
| Inspect the selected entry on either side | PRESENT | `modules/game_playertrade/playertrade.lua:64` | `apps/client/src/bevy_shell/game_ui/trade.rs:38` → `apps/client/src/features/0060-player-trade/presentation.rs:146` |
| Event · onOwnTrade | Unverified | `modules/game_playertrade/playertrade.lua:7` | — |
| Event · onCounterTrade | Unverified | `modules/game_playertrade/playertrade.lua:8` | — |
| Event · onCloseTrade | Unverified | `modules/game_playertrade/playertrade.lua:9` | — |
| Event · onGameEnd | Unverified | `modules/game_playertrade/playertrade.lua:10` | — |
| Game API · rejectTrade | Unverified | `modules/game_playertrade/playertrade.lua:30` | — |
| Game API · inspectTrade | Unverified | `modules/game_playertrade/playertrade.lua:64` | — |
| Trade | Unverified | `modules/game_playertrade/tradewindow.otui:2` | — |
| Accept | Unverified | `modules/game_playertrade/tradewindow.otui:91` | — |
| Game API · acceptTrade | Unverified | `modules/game_playertrade/tradewindow.otui:98` | — |
| Activate · g_game.acceptTrade(); self:disable() | Unverified | `modules/game_playertrade/tradewindow.otui:98` | — |
| Reject | Unverified | `modules/game_playertrade/tradewindow.otui:101` | — |
| Activate · g_game.rejectTrade() | Unverified | `modules/game_playertrade/tradewindow.otui:107` | — |

### game_prey

PARTIAL 1 | Unverified 27

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Display the gold price before paid list reroll | PARTIAL | `modules/game_prey/prey.lua:78` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:224` warns of a possible charge but cannot quote it. `apps/client/src/features/0140-prey/input.rs:42` sends the request. |
| Event · onGameStart | Unverified | `modules/game_prey/prey.lua:73` | — |
| Event · onGameEnd | Unverified | `modules/game_prey/prey.lua:74` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_prey/prey.lua:75` | — |
| Event · onPreyFreeRerolls | Unverified | `modules/game_prey/prey.lua:76` | — |
| Event · onPreyTimeLeft | Unverified | `modules/game_prey/prey.lua:77` | — |
| Event · onPreyRerollPrice | Unverified | `modules/game_prey/prey.lua:78` | — |
| Event · onPreyLocked | Unverified | `modules/game_prey/prey.lua:79` | — |
| Event · onPreyInactive | Unverified | `modules/game_prey/prey.lua:80` | — |
| Event · onPreyActive | Unverified | `modules/game_prey/prey.lua:81` | — |
| Event · onPreySelection | Unverified | `modules/game_prey/prey.lua:82` | — |
| Event · onPreySelectionChangeMonster | Unverified | `modules/game_prey/prey.lua:83` | — |
| Event · onPreyListSelection | Unverified | `modules/game_prey/prey.lua:84` | — |
| Event · onPreyWildcardSelection | Unverified | `modules/game_prey/prey.lua:85` | — |
| Game API · preyRequest | Unverified | `modules/game_prey/prey.lua:422` | — |
| Inactive Prey. \n\nClick in this window to open the prey dialog. | Unverified | `modules/game_prey/prey.lua:572` | — |
| Event · onHoverChange | Unverified | `modules/game_prey/prey.lua:611` | — |
| Game API · preyAction | Unverified | `modules/game_prey/prey.lua:814` | — |
| Event · onCheckChange | Unverified | `modules/game_prey/prey.lua:1450` | — |
| Activate · modules.game_mainpanel.toggleStore() | Unverified | `modules/game_prey/prey.otui:17` | — |
| No creature selected | Unverified | `modules/game_prey/prey.otui:369` | — |
| Activate · modules.game_prey.onRaceSearchClearClicked(self) | Unverified | `modules/game_prey/prey.otui:428` | — |
| Activate · modules.game_prey.onItemBoxChecked(self) | Unverified | `modules/game_prey/prey.otui:577` | — |
| Activate · modules.game_prey.onPreyRaceListItemClicked(self) | Unverified | `modules/game_prey/prey.otui:609` | — |
| Preys | Unverified | `modules/game_prey/prey.otui:656` | — |
| Activate · modules.game_prey.hide() | Unverified | `modules/game_prey/prey.otui:704` | — |
| Prey | Unverified | `modules/game_prey/prey.otui:786` | — |
| Prey Creatures | Unverified | `modules/game_prey/prey.otui:830` | — |

### game_proficiency

Unverified 30

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_proficiency/proficiency.lua:213` | — |
| Event · onGameEnd | Unverified | `modules/game_proficiency/proficiency.lua:214` | — |
| Game API · sendWeaponProficiencyAction | Unverified | `modules/game_proficiency/proficiency.lua:307` | — |
| Weapons: All | Unverified | `modules/game_proficiency/proficiency.lua:960` | — |
| Weapons: Swords | Unverified | `modules/game_proficiency/proficiency.lua:961` | — |
| Weapons: Axes | Unverified | `modules/game_proficiency/proficiency.lua:962` | — |
| Weapons: Clubs | Unverified | `modules/game_proficiency/proficiency.lua:963` | — |
| Weapons: Distance | Unverified | `modules/game_proficiency/proficiency.lua:964` | — |
| Weapons: Wands | Unverified | `modules/game_proficiency/proficiency.lua:965` | — |
| Weapons: Fist | Unverified | `modules/game_proficiency/proficiency.lua:966` | — |
| Event · onOptionChange | Unverified | `modules/game_proficiency/proficiency.lua:967` | — |
| Event · onTextChange | Unverified | `modules/game_proficiency/proficiency.lua:975` | — |
| Game API · sendWeaponProficiencyApply | Unverified | `modules/game_proficiency/proficiency.lua:2345` | — |
| Weapon Proficiency | Unverified | `modules/game_proficiency/proficiency.otui:223` | — |
| Unknown Item | Unverified | `modules/game_proficiency/proficiency.otui:248` | — |
| 0 / 0 | Unverified | `modules/game_proficiency/proficiency.otui:291` | — |
| 0 XP for next level | Unverified | `modules/game_proficiency/proficiency.otui:303` | — |
| Level | Unverified | `modules/game_proficiency/proficiency.otui:315` | — |
| Activate · modules.game_proficiency.WeaponProficiency:toggleFilterOption(self) | Unverified | `modules/game_proficiency/proficiency.otui:318` | — |
| Voc. | Unverified | `modules/game_proficiency/proficiency.otui:326` | — |
| 1H | Unverified | `modules/game_proficiency/proficiency.otui:337` | — |
| 2H | Unverified | `modules/game_proficiency/proficiency.otui:348` | — |
| Close | Unverified | `modules/game_proficiency/proficiency.otui:502` | — |
| Activate · modules.game_proficiency.WeaponProficiency:onCloseWindow() | Unverified | `modules/game_proficiency/proficiency.otui:506` | — |
| Reset | Unverified | `modules/game_proficiency/proficiency.otui:512` | — |
| Activate · modules.game_proficiency.WeaponProficiency:onResetClick() | Unverified | `modules/game_proficiency/proficiency.otui:518` | — |
| Apply | Unverified | `modules/game_proficiency/proficiency.otui:524` | — |
| Activate · modules.game_proficiency.WeaponProficiency:onApplyClick() | Unverified | `modules/game_proficiency/proficiency.otui:530` | — |
| Ok | Unverified | `modules/game_proficiency/proficiency.otui:536` | — |
| Activate · modules.game_proficiency.WeaponProficiency:onOkClick() | Unverified | `modules/game_proficiency/proficiency.otui:542` | — |

### game_questlog

PRESENT 4 | Unverified 13

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Request quest list | PRESENT | `modules/game_questlog/game_questlog.lua:548` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:178` → `apps/client/src/features/0016-quest-log/input.rs:18` |
| Search quest list | PRESENT | `modules/game_questlog/game_questlog.lua:602` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:189` → `apps/client/src/features/0016-quest-log/input.rs:8` |
| Open quests and request mission details | PRESENT | `modules/game_questlog/game_questlog.lua:438` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:66` → `apps/client/src/features/0016-quest-log/input.rs:18` |
| Change quest tracking | PRESENT | `modules/game_questlog/game_questlog.lua:267` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:89` → `apps/client/src/features/0016-quest-log/input.rs:18` |
| Game API · sendRequestTrackerQuestLog | Unverified | `modules/game_questlog/game_questlog.lua:267` | — |
| Game API · requestQuestLine | Unverified | `modules/game_questlog/game_questlog.lua:438` | — |
| Game API · requestQuestLog | Unverified | `modules/game_questlog/game_questlog.lua:548` | — |
| Remove All quest | Unverified | `modules/game_questlog/game_questlog.lua:693` | — |
| Remove completed quests | Unverified | `modules/game_questlog/game_questlog.lua:732` | — |
| remove | Unverified | `modules/game_questlog/game_questlog.lua:1142` | — |
| Event · onQuestLog | Unverified | `modules/game_questlog/game_questlog.lua:1288` | — |
| Event · onQuestLine | Unverified | `modules/game_questlog/game_questlog.lua:1289` | — |
| Event · onQuestTracker | Unverified | `modules/game_questlog/game_questlog.lua:1290` | — |
| Event · onUpdateQuestTracker | Unverified | `modules/game_questlog/game_questlog.lua:1291` | — |
| Activate · self:getParent():getChildById('SearchEdit'):clearText() | Unverified | `modules/game_questlog/styles/game_questlog.otui:84` | — |
| Quest Tracker | Unverified | `modules/game_questlog/styles/game_questlog.otui:121` | — |
| Activate · modules.game_questlog.show() | Unverified | `modules/game_questlog/styles/game_questlog.otui:203` | — |

### game_quickloot

Unverified 27

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onQuickLootContainers | Unverified | `modules/game_quickloot/quickloot.lua:28` | — |
| Game API · sendQuickLoot | Unverified | `modules/game_quickloot/quickloot.lua:34` | — |
| Game API · requestQuickLootBlackWhiteList | Unverified | `modules/game_quickloot/quickloot.lua:68` | — |
| Game API · openContainerQuickLoot | Unverified | `modules/game_quickloot/quickloot.lua:333` | — |
| bag | Unverified | `modules/game_quickloot/quickloot.otui:19` | — |
| Activate · modules.game_quickloot.QuickLoot.clearFilterItem(self) | Unverified | `modules/game_quickloot/quickloot.otui:30` | — |
| Unassigned Loot | Unverified | `modules/game_quickloot/quickloot.otui:44` | — |
| Activate · modules.game_quickloot.QuickLoot.chooseItem(self) | Unverified | `modules/game_quickloot/quickloot.otui:58` | — |
| Activate · modules.game_quickloot.QuickLoot.clearItem(self) | Unverified | `modules/game_quickloot/quickloot.otui:67` | — |
| Activate · modules.game_quickloot.QuickLoot.openContainer(self) | Unverified | `modules/game_quickloot/quickloot.otui:77` | — |
| Manage Containers | Unverified | `modules/game_quickloot/quickloot.otui:113` | — |
| Use main container as fallback | Unverified | `modules/game_quickloot/quickloot.otui:131` | — |
| Tick this check box to loot into your main container if you cannot \nloot into the container for the particular loot category and \nunassigned loot. Note that  | Unverified | `modules/game_quickloot/quickloot.otui:132` | — |
| Loot | Unverified | `modules/game_quickloot/quickloot.otui:136` | — |
| Obtain | Unverified | `modules/game_quickloot/quickloot.otui:144` | — |
| Quick Loot Filter: | Unverified | `modules/game_quickloot/quickloot.otui:189` | — |
| Activate · QuickLoot.filter(self, self:isChecked()) | Unverified | `modules/game_quickloot/quickloot.otui:203` | — |
| Clear Skipped Loot List | Unverified | `modules/game_quickloot/quickloot.otui:217` | — |
| Activate · QuickLoot.clearFilterItems() | Unverified | `modules/game_quickloot/quickloot.otui:223` | — |
| Add to Skipped Loot List | Unverified | `modules/game_quickloot/quickloot.otui:227` | — |
| Activate · \| | Unverified | `modules/game_quickloot/quickloot.otui:232` | — |
| Select containers for your loot!\n\nRefine your Quick Looting by setting objects on your skipped loot or accepted loot list! | Unverified | `modules/game_quickloot/quickloot.otui:244` | — |
| Activate · modules.game_mainpanel.toggleStore() | Unverified | `modules/game_quickloot/quickloot.otui:265` | — |
| Premium | Unverified | `modules/game_quickloot/quickloot.otui:276` | — |
| Activate · QuickLoot.clearSearch() | Unverified | `modules/game_quickloot/quickloot.otui:295` | — |
| Close | Unverified | `modules/game_quickloot/quickloot.otui:327` | — |
| Activate · QuickLoot.toggle() | Unverified | `modules/game_quickloot/quickloot.otui:331` | — |

### game_rewardwall

PRESENT 1 | Unverified 28

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Claim available reward | PRESENT | `modules/game_rewardwall/game_rewardwall.lua:119` | `apps/client/src/bevy_shell/game_ui/reward_wall.rs:164` → `apps/client/src/features/0182-daily-reward/input.rs:14` |
| Game API · sendOpenRewardWall | Unverified | `modules/game_rewardwall/game_rewardwall.lua:36` | — |
| Game API · requestGetRewardDaily | Unverified | `modules/game_rewardwall/game_rewardwall.lua:119` | — |
| Event · onOpenRewardWall | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1242` | — |
| Event · onCloseRewardWall | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1243` | — |
| Event · onDailyReward | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1244` | — |
| Event · onRewardHistory | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1245` | — |
| Event · onDailyRewardCollectionState | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1247` | — |
| Event · onParseStoreGetPurchaseStatus | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1248` | — |
| Game API · requestOpenRewardHistory | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1307` | — |
| Game API · sendRequestStorePremiumBoost | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1317` | — |
| Game API · sendRequestUsefulThings | Unverified | `modules/game_rewardwall/game_rewardwall.lua:1322` | — |
| Reward Wall | Unverified | `modules/game_rewardwall/game_rewardwall.otui:3` | — |
| History | Unverified | `modules/game_rewardwall/game_rewardwall.otui:18` | — |
| Resting Area Bonuses | Unverified | `modules/game_rewardwall/game_rewardwall.otui:46` | — |
| Daily Rewards | Unverified | `modules/game_rewardwall/game_rewardwall.otui:156` | — |
| Activate · modules.game_rewardwall.rewardWallController:onClickSendStoreRewardWall() | Unverified | `modules/game_rewardwall/game_rewardwall.otui:428` | — |
| Activate · modules.game_rewardwall.rewardWallController:onClickSendStoreJoker() | Unverified | `modules/game_rewardwall/game_rewardwall.otui:561` | — |
| Activate · modules.game_rewardwall.rewardWallController:onClickshowHistory() | Unverified | `modules/game_rewardwall/game_rewardwall.otui:588` | — |
| Close | Unverified | `modules/game_rewardwall/game_rewardwall.otui:592` | — |
| Activate · modules.game_rewardwall.rewardWallController:onClickToggle() | Unverified | `modules/game_rewardwall/game_rewardwall.otui:598` | — |
| Pick Reward | Unverified | `modules/game_rewardwall/styles/pickreward.otui:2` | — |
| Activate · modules.game_rewardwall.destroyPickReward(true) | Unverified | `modules/game_rewardwall/styles/pickreward.otui:88` | — |
| Ok | Unverified | `modules/game_rewardwall/styles/pickreward.otui:92` | — |
| Activate · modules.game_rewardwall.onClickBtnOk() | Unverified | `modules/game_rewardwall/styles/pickreward.otui:99` | — |
| Activate · modules.game_rewardwall.onPickRewardQtyClick(self, 'min') | Unverified | `modules/game_rewardwall/styles/style.otui:135` | — |
| Activate · modules.game_rewardwall.onPickRewardQtyClick(self, 'dec') | Unverified | `modules/game_rewardwall/styles/style.otui:142` | — |
| Activate · modules.game_rewardwall.onPickRewardQtyClick(self, 'inc') | Unverified | `modules/game_rewardwall/styles/style.otui:173` | — |
| Activate · modules.game_rewardwall.onPickRewardQtyClick(self, 'max') | Unverified | `modules/game_rewardwall/styles/style.otui:180` | — |

### game_ruleviolation

Unverified 12

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onFocusChange | Unverified | `modules/game_ruleviolation/ruleviolation.lua:77` | — |
| Game API · reportRuleViolation | Unverified | `modules/game_ruleviolation/ruleviolation.lua:149` | — |
| Name | Unverified | `modules/game_ruleviolation/ruleviolation.otui:39` | — |
| Statement | Unverified | `modules/game_ruleviolation/ruleviolation.otui:45` | — |
| Reason | Unverified | `modules/game_ruleviolation/ruleviolation.otui:52` | — |
| Action | Unverified | `modules/game_ruleviolation/ruleviolation.otui:73` | — |
| IP Address Banishment | Unverified | `modules/game_ruleviolation/ruleviolation.otui:95` | — |
| Comment | Unverified | `modules/game_ruleviolation/ruleviolation.otui:102` | — |
| Cancel | Unverified | `modules/game_ruleviolation/ruleviolation.otui:108` | — |
| Activate · hide() | Unverified | `modules/game_ruleviolation/ruleviolation.otui:112` | — |
| Ok | Unverified | `modules/game_ruleviolation/ruleviolation.otui:115` | — |
| Activate · report() | Unverified | `modules/game_ruleviolation/ruleviolation.otui:120` | — |

### game_shop

NOT-APPLICABLE 22

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| In-game shop | NOT-APPLICABLE | `modules/game_shop/game_shop.otmod:1` | OTClient developer facility or OT server extension |
| Activate · modules.game_shop.cancelChangeName() | NOT-APPLICABLE | `modules/game_shop/changename.otui:12` | OT extension |
| Activate · modules.game_shop.confirmChangeName() | NOT-APPLICABLE | `modules/game_shop/changename.otui:21` | OT extension |
| Event · onGameStart | NOT-APPLICABLE | `modules/game_shop/game_shop.lua:32` | OT extension |
| Event · onGameEnd | NOT-APPLICABLE | `modules/game_shop/game_shop.lua:33` | OT extension |
| Activate · modules.game_shop.select(self) | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:15` | OT extension |
| Activate · modules.game_shop.onOfferBuy(self) | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:162` | OT extension |
| Activate · modules.game_shop.selectOffer(self) | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:248` | OT extension |
| Activate · modules.game_shop.onSearch() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:373` | OT extension |
| Activate · modules.game_shop.hide() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:393` | OT extension |
| Activate · modules.game_shop.showHistory() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:404` | OT extension |
| Activate · modules.game_shop.buyPoints() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:433` | OT extension |
| Activate · modules.game_shop.toggleGiftCoins() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:448` | OT extension |
| Activate · modules.game_shop.prevPage() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:511` | OT extension |
| Activate · modules.game_shop.nextPage() | NOT-APPLICABLE | `modules/game_shop/game_shop.otui:520` | OT extension |
| Activate · modules.game_shop.confirmGift() | NOT-APPLICABLE | `modules/game_shop/gift.otui:12` | OT extension |
| Activate · modules.game_shop.cancelGift() | NOT-APPLICABLE | `modules/game_shop/gift.otui:20` | OT extension |
| Gift Tibia Coins | NOT-APPLICABLE | `modules/game_shop/giftcoins.otui:6` | OT extension |
| Cancel | NOT-APPLICABLE | `modules/game_shop/giftcoins.otui:131` | OT extension |
| Activate · modules.game_shop.cancelGiftCoins() | NOT-APPLICABLE | `modules/game_shop/giftcoins.otui:138` | OT extension |
| Gift | NOT-APPLICABLE | `modules/game_shop/giftcoins.otui:142` | OT extension |
| Activate · modules.game_shop.confirmGiftCoins() | NOT-APPLICABLE | `modules/game_shop/giftcoins.otui:148` | OT extension |

### game_shortcuts

Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_shortcuts/shortcuts.lua:13` | — |
| Event · onGameEnd | Unverified | `modules/game_shortcuts/shortcuts.lua:14` | — |

### game_skills

Unverified 100

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onMagicLevelChange | Unverified | `modules/game_skills/offlinetraining1524.lua:85` | — |
| Event · onBaseMagicLevelChange | Unverified | `modules/game_skills/offlinetraining1524.lua:86` | — |
| Event · onSkillChange | Unverified | `modules/game_skills/offlinetraining1524.lua:87` | — |
| Event · onBaseSkillChange | Unverified | `modules/game_skills/offlinetraining1524.lua:88` | — |
| You have %s percent to go | Unverified | `modules/game_skills/offlinetraining1524.lua:173` | — |
| Game API · sendStartOfflineTraining | Unverified | `modules/game_skills/offlinetraining1524.lua:230` | — |
| Event · onExperienceChange | Unverified | `modules/game_skills/skills.lua:75` | — |
| Event · onLevelChange | Unverified | `modules/game_skills/skills.lua:76` | — |
| Event · onHealthChange | Unverified | `modules/game_skills/skills.lua:77` | — |
| Event · onManaChange | Unverified | `modules/game_skills/skills.lua:78` | — |
| Event · onSoulChange | Unverified | `modules/game_skills/skills.lua:79` | — |
| Event · onFreeCapacityChange | Unverified | `modules/game_skills/skills.lua:80` | — |
| Event · onTotalCapacityChange | Unverified | `modules/game_skills/skills.lua:81` | — |
| Event · onBaseCapacityChange | Unverified | `modules/game_skills/skills.lua:82` | — |
| Event · onStaminaChange | Unverified | `modules/game_skills/skills.lua:83` | — |
| Event · onOfflineTrainingChange | Unverified | `modules/game_skills/skills.lua:84` | — |
| Event · onRegenerationChange | Unverified | `modules/game_skills/skills.lua:85` | — |
| Event · onSpeedChange | Unverified | `modules/game_skills/skills.lua:86` | — |
| Event · onBaseSpeedChange | Unverified | `modules/game_skills/skills.lua:87` | — |
| Event · onFlatDamageHealingChange | Unverified | `modules/game_skills/skills.lua:93` | — |
| Event · onAttackInfoChange | Unverified | `modules/game_skills/skills.lua:94` | — |
| Event · onConvertedDamageChange | Unverified | `modules/game_skills/skills.lua:95` | — |
| Event · onImbuementsChange | Unverified | `modules/game_skills/skills.lua:96` | — |
| Event · onDefenseInfoChange | Unverified | `modules/game_skills/skills.lua:97` | — |
| Event · onCombatAbsorbValuesChange | Unverified | `modules/game_skills/skills.lua:98` | — |
| Event · onForgeBonusesChange | Unverified | `modules/game_skills/skills.lua:99` | — |
| Event · onExperienceRateChange | Unverified | `modules/game_skills/skills.lua:100` | — |
| Event · onMultiOfflineTrainingDialog | Unverified | `modules/game_skills/skills.lua:102` | — |
| Event · onCheckChange | Unverified | `modules/game_skills/skills.lua:318` | — |
| You have %s of %s capacity left | Unverified | `modules/game_skills/skills.lua:646` | — |
| Activate · \| | Unverified | `modules/game_skills/skills.otui:82` | — |
| Game API · sendRequestStorePremiumBoost | Unverified | `modules/game_skills/skills.otui:84` | — |
| Skills | Unverified | `modules/game_skills/skills.otui:115` | — |
| Level | Unverified | `modules/game_skills/skills.otui:127` | — |
| Experience | Unverified | `modules/game_skills/skills.otui:135` | — |
| XP Gain Rate | Unverified | `modules/game_skills/skills.otui:142` | — |
| Hit Points | Unverified | `modules/game_skills/skills.otui:153` | — |
| Mana | Unverified | `modules/game_skills/skills.otui:160` | — |
| Soul Points | Unverified | `modules/game_skills/skills.otui:167` | — |
| Capacity | Unverified | `modules/game_skills/skills.otui:174` | — |
| Speed | Unverified | `modules/game_skills/skills.otui:181` | — |
| Regeneration Time | Unverified | `modules/game_skills/skills.otui:188` | — |
| Stamina | Unverified | `modules/game_skills/skills.otui:194` | — |
| Offline Training | Unverified | `modules/game_skills/skills.otui:201` | — |
| Magic Level | Unverified | `modules/game_skills/skills.otui:212` | — |
| Fist Fighting | Unverified | `modules/game_skills/skills.otui:223` | — |
| Club Fighting | Unverified | `modules/game_skills/skills.otui:233` | — |
| Sword Fighting | Unverified | `modules/game_skills/skills.otui:243` | — |
| Axe Fighting | Unverified | `modules/game_skills/skills.otui:253` | — |
| Distance Fighting | Unverified | `modules/game_skills/skills.otui:263` | — |
| Shielding | Unverified | `modules/game_skills/skills.otui:273` | — |
| Fishing | Unverified | `modules/game_skills/skills.otui:283` | — |
| Critical Hit Chance | Unverified | `modules/game_skills/skills.otui:298` | — |
| Critical Hit Damage | Unverified | `modules/game_skills/skills.otui:304` | — |
| Life Leech Chance | Unverified | `modules/game_skills/skills.otui:310` | — |
| Life Leech Amount | Unverified | `modules/game_skills/skills.otui:316` | — |
| Mana Leech Chance | Unverified | `modules/game_skills/skills.otui:322` | — |
| Mana Leech Amount | Unverified | `modules/game_skills/skills.otui:328` | — |
| Fatal | Unverified | `modules/game_skills/skills.otui:334` | — |
| Dodge | Unverified | `modules/game_skills/skills.otui:340` | — |
| Momentum | Unverified | `modules/game_skills/skills.otui:346` | — |
| Transcendence | Unverified | `modules/game_skills/skills.otui:352` | — |
| Damage/Healing | Unverified | `modules/game_skills/skills.otui:360` | — |
| Attack Value | Unverified | `modules/game_skills/skills.otui:367` | — |
| Converted Damage | Unverified | `modules/game_skills/skills.otui:376` | — |
| Converted Element | Unverified | `modules/game_skills/skills.otui:383` | — |
| Life Leech | Unverified | `modules/game_skills/skills.otui:391` | — |
| Mana Leech | Unverified | `modules/game_skills/skills.otui:397` | — |
| Critical Hit: | Unverified | `modules/game_skills/skills.otui:403` | — |
| Chance | Unverified | `modules/game_skills/skills.otui:409` | — |
| Extra Damage | Unverified | `modules/game_skills/skills.otui:416` | — |
| Onslaught | Unverified | `modules/game_skills/skills.otui:423` | — |
| Physical | Unverified | `modules/game_skills/skills.otui:436` | — |
| Fire | Unverified | `modules/game_skills/skills.otui:445` | — |
| Earth | Unverified | `modules/game_skills/skills.otui:455` | — |
| Energy | Unverified | `modules/game_skills/skills.otui:464` | — |
| Ice | Unverified | `modules/game_skills/skills.otui:473` | — |
| Holy | Unverified | `modules/game_skills/skills.otui:482` | — |
| Death | Unverified | `modules/game_skills/skills.otui:492` | — |
| Healing | Unverified | `modules/game_skills/skills.otui:501` | — |
| Drown | Unverified | `modules/game_skills/skills.otui:510` | — |
| Lifedrain | Unverified | `modules/game_skills/skills.otui:519` | — |
| Manadrain | Unverified | `modules/game_skills/skills.otui:528` | — |
| Defence Value | Unverified | `modules/game_skills/skills.otui:537` | — |
| Armor Value | Unverified | `modules/game_skills/skills.otui:544` | — |
| Mantra Value | Unverified | `modules/game_skills/skills.otui:551` | — |
| Mitigation | Unverified | `modules/game_skills/skills.otui:558` | — |
| Damage Reflection | Unverified | `modules/game_skills/skills.otui:570` | — |
| Amplification | Unverified | `modules/game_skills/skills.otui:595` | — |
| Reset Experience Counter | Unverified | `modules/game_skills/skills.otui:647` | — |
| Magic | Unverified | `modules/game_skills/skills.otui:667` | — |
| Fist | Unverified | `modules/game_skills/skills.otui:671` | — |
| Club | Unverified | `modules/game_skills/skills.otui:675` | — |
| Sword | Unverified | `modules/game_skills/skills.otui:679` | — |
| Axe | Unverified | `modules/game_skills/skills.otui:683` | — |
| Distance | Unverified | `modules/game_skills/skills.otui:687` | — |
| Offence Stats | Unverified | `modules/game_skills/skills.otui:704` | — |
| Defence Stats | Unverified | `modules/game_skills/skills.otui:708` | — |
| Misc. Stats | Unverified | `modules/game_skills/skills.otui:712` | — |
| Show all Skill Bars | Unverified | `modules/game_skills/skills.otui:721` | — |

### game_soundDebug

NOT-APPLICABLE 5

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Live visual debug tools for the sound system | NOT-APPLICABLE | `modules/game_soundDebug/sounddebug.otmod:1` | OTClient developer facility or OT server extension |
| Event · onHoverChange | NOT-APPLICABLE | `modules/game_soundDebug/sounddebug.lua:169` | OT extension |
| Event · onGameEnd | NOT-APPLICABLE | `modules/game_soundDebug/sounddebug.lua:500` | OT extension |
| Event · onGameStart | NOT-APPLICABLE | `modules/game_soundDebug/sounddebug.lua:501` | OT extension |
| Sound Debug | NOT-APPLICABLE | `modules/game_soundDebug/sounddebug.otui:4` | OT extension |

### game_spelllist

Unverified 32

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_spelllist/spelllist.lua:103` | — |
| Event · onGameEnd | Unverified | `modules/game_spelllist/spelllist.lua:104` | — |
| Event · onSelectionChange | Unverified | `modules/game_spelllist/spelllist.lua:160` | — |
| Event · onChildFocusChange | Unverified | `modules/game_spelllist/spelllist.lua:236` | — |
| Activate · toggleFilter(self) | Unverified | `modules/game_spelllist/spelllist.otui:33` | — |
| Spell List | Unverified | `modules/game_spelllist/spelllist.otui:41` | — |
| Close | Unverified | `modules/game_spelllist/spelllist.otui:58` | — |
| Activate · toggle() | Unverified | `modules/game_spelllist/spelllist.otui:62` | — |
| Name | Unverified | `modules/game_spelllist/spelllist.otui:76` | — |
| Filters | Unverified | `modules/game_spelllist/spelllist.otui:81` | — |
| Level | Unverified | `modules/game_spelllist/spelllist.otui:86` | — |
| Hide spells for higher exp. levels | Unverified | `modules/game_spelllist/spelllist.otui:87` | — |
| Vocation | Unverified | `modules/game_spelllist/spelllist.otui:91` | — |
| Hide spells for other vocations | Unverified | `modules/game_spelllist/spelllist.otui:92` | — |
| Formula | Unverified | `modules/game_spelllist/spelllist.otui:98` | — |
| Group | Unverified | `modules/game_spelllist/spelllist.otui:111` | — |
| Type | Unverified | `modules/game_spelllist/spelllist.otui:117` | — |
| Cooldown | Unverified | `modules/game_spelllist/spelllist.otui:123` | — |
| Mana | Unverified | `modules/game_spelllist/spelllist.otui:135` | — |
| Premium | Unverified | `modules/game_spelllist/spelllist.otui:141` | — |
| Description | Unverified | `modules/game_spelllist/spelllist.otui:147` | — |
| Any | Unverified | `modules/game_spelllist/spelllist.otui:215` | — |
| Sorcerer | Unverified | `modules/game_spelllist/spelllist.otui:223` | — |
| Druid | Unverified | `modules/game_spelllist/spelllist.otui:231` | — |
| Paladin | Unverified | `modules/game_spelllist/spelllist.otui:239` | — |
| Knight | Unverified | `modules/game_spelllist/spelllist.otui:247` | — |
| Monk | Unverified | `modules/game_spelllist/spelllist.otui:255` | — |
| Attack | Unverified | `modules/game_spelllist/spelllist.otui:282` | — |
| Healing | Unverified | `modules/game_spelllist/spelllist.otui:290` | — |
| Support | Unverified | `modules/game_spelllist/spelllist.otui:298` | — |
| No | Unverified | `modules/game_spelllist/spelllist.otui:325` | — |
| Yes | Unverified | `modules/game_spelllist/spelllist.otui:333` | — |

### game_stash

PRESENT 1 | Unverified 19

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Withdraw chosen amount | PRESENT | `modules/game_stash/game_stash.lua:474` | `apps/client/src/bevy_shell/game_ui/stash.rs:119` → `apps/client/src/features/0062-stash/input.rs:104` |
| Retrieve | Unverified | `modules/game_stash/game_stash.lua:265` | — |
| Cyclopedia | Unverified | `modules/game_stash/game_stash.lua:269` | — |
| Show in Market | Unverified | `modules/game_stash/game_stash.lua:288` | — |
| Add to Loot List | Unverified | `modules/game_stash/game_stash.lua:303` | — |
| Remove from Loot List | Unverified | `modules/game_stash/game_stash.lua:307` | — |
| Event · onValueChange | Unverified | `modules/game_stash/game_stash.lua:452` | — |
| Game API · stashWithdraw | Unverified | `modules/game_stash/game_stash.lua:474` | — |
| Event · onScrollChange | Unverified | `modules/game_stash/game_stash.lua:552` | — |
| Show All | Unverified | `modules/game_stash/game_stash.lua:555` | — |
| Show  | Unverified | `modules/game_stash/game_stash.lua:609` | — |
| Supply Stash | Unverified | `modules/game_stash/game_stash.otui:3` | — |
| Activate · self:getParent():clearText() | Unverified | `modules/game_stash/game_stash.otui:35` | — |
| Manage Containers | Unverified | `modules/game_stash/game_stash.otui:117` | — |
| Activate · openManageContainers() | Unverified | `modules/game_stash/game_stash.otui:122` | — |
| Close | Unverified | `modules/game_stash/game_stash.otui:126` | — |
| Activate · onSupplyStashClose() | Unverified | `modules/game_stash/game_stash.otui:131` | — |
| Stash Withdraw | Unverified | `modules/game_stash/game_stash.otui:162` | — |
| Cancel | Unverified | `modules/game_stash/game_stash.otui:185` | — |
| Ok | Unverified | `modules/game_stash/game_stash.otui:195` | — |

### game_store

Unverified 49

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Game API · sendRequestStoreHome | Unverified | `modules/game_store/game_store.lua:465` | — |
| Game API · requestStoreOffers | Unverified | `modules/game_store/game_store.lua:467` | — |
| Event · onChildFocusChange | Unverified | `modules/game_store/game_store.lua:498` | — |
| Event · onParseStoreGetCoin | Unverified | `modules/game_store/game_store.lua:505` | — |
| Event · onParseStoreGetCategories | Unverified | `modules/game_store/game_store.lua:506` | — |
| Event · onParseStoreCreateHome | Unverified | `modules/game_store/game_store.lua:507` | — |
| Event · onParseStoreCreateProducts | Unverified | `modules/game_store/game_store.lua:508` | — |
| Event · onParseStoreGetHistory | Unverified | `modules/game_store/game_store.lua:509` | — |
| Event · onParseStoreGetPurchaseStatus | Unverified | `modules/game_store/game_store.lua:510` | — |
| Event · onParseStoreOfferDescriptions | Unverified | `modules/game_store/game_store.lua:511` | — |
| Event · onParseStoreError | Unverified | `modules/game_store/game_store.lua:512` | — |
| Event · onStoreInit | Unverified | `modules/game_store/game_store.lua:513` | — |
| Game API · sendRequestStorePremiumBoost | Unverified | `modules/game_store/game_store.lua:585` | — |
| Disable | Unverified | `modules/game_store/game_store.lua:593` | — |
| Game API · openStore | Unverified | `modules/game_store/game_store.lua:923` | — |
| Game API · requestTransactionHistory | Unverified | `modules/game_store/game_store.lua:958` | — |
| Game API · buyStoreOffer | Unverified | `modules/game_store/game_store.lua:1081` | — |
| Game API · sendRequestStoreOfferById | Unverified | `modules/game_store/game_store.lua:1151` | — |
| Event · onValueChange | Unverified | `modules/game_store/game_store.lua:1244` | — |
| Game API · transferCoins | Unverified | `modules/game_store/game_store.lua:1270` | — |
| Game API · sendRequestStoreSearch | Unverified | `modules/game_store/game_store.lua:1286` | — |
| Store | Unverified | `modules/game_store/game_store.otui:97` | — |
| Close | Unverified | `modules/game_store/game_store.otui:107` | — |
| Activate · modules.game_store.toggle() | Unverified | `modules/game_store/game_store.otui:109` | — |
| History | Unverified | `modules/game_store/game_store.otui:117` | — |
| Activate · modules.game_store.toggleTransferHistory() | Unverified | `modules/game_store/game_store.otui:119` | — |
| ) | Unverified | `modules/game_store/game_store.otui:173` | — |
| Get Tibia Coins | Unverified | `modules/game_store/game_store.otui:186` | — |
| Get | Unverified | `modules/game_store/game_store.otui:192` | — |
| Activate · modules.game_store.getCoinsWebsite() | Unverified | `modules/game_store/game_store.otui:194` | — |
| Transfer Coins | Unverified | `modules/game_store/game_store.otui:203` | — |
| Setup up an auction to sell you currents characters. | Unverified | `modules/game_store/game_store.otui:212` | — |
| Activate · modules.game_store.search() | Unverified | `modules/game_store/game_store.otui:258` | — |
| Next Page | Unverified | `modules/game_store/game_store.otui:406` | — |
| Activate · modules.game_store.requestTransactionHistory(self) | Unverified | `modules/game_store/game_store.otui:407` | — |
| Page 1 / 4 | Unverified | `modules/game_store/game_store.otui:413` | — |
| Prev Page | Unverified | `modules/game_store/game_store.otui:420` | — |
| Activate · modules.game_store.changeImagenHome(self:getId()) | Unverified | `modules/game_store/game_store.otui:446` | — |
| Enter New Character Name | Unverified | `modules/game_store/style/changename.otui:10` | — |
| Please enter the new name for your character: | Unverified | `modules/game_store/style/changename.otui:20` | — |
| Cancel | Unverified | `modules/game_store/style/changename.otui:59` | — |
| Ok | Unverified | `modules/game_store/style/changename.otui:70` | — |
| Gift Tibia Coins | Unverified | `modules/game_store/style/transferpoints.otui:10` | — |
| Please select the amount of Tibia\nCoins you like to gift and enter the\nname of the character that should\nreceive the Tibia Coins. | Unverified | `modules/game_store/style/transferpoints.otui:20` | — |
| Recipient: | Unverified | `modules/game_store/style/transferpoints.otui:30` | — |
| Transferable Tibia Coins: | Unverified | `modules/game_store/style/transferpoints.otui:50` | — |
| Gift | Unverified | `modules/game_store/style/transferpoints.otui:134` | — |
| Buy | Unverified | `modules/game_store/style/ui.otui:74` | — |
| General Stats | Unverified | `modules/game_store/style/ui.otui:224` | — |

### game_taskboard

Unverified 28

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Game API · bountyPreferredAction | Unverified | `modules/game_taskboard/controller/modal_preferred.lua:79` | — |
| Event · onScrollChange | Unverified | `modules/game_taskboard/controller/modal_preferred.lua:244` | — |
| Game API · soulsealFightAction | Unverified | `modules/game_taskboard/controller/modal_soulseal.lua:648` | — |
| Game API · bountyTaskAction | Unverified | `modules/game_taskboard/controller/tab_bounty.lua:140` | — |
| Game API · bountyTalismanUpgrade | Unverified | `modules/game_taskboard/controller/tab_bounty.lua:226` | — |
| Game API · taskHuntingShopPurchase | Unverified | `modules/game_taskboard/controller/tab_shop.lua:57` | — |
| Game API · weeklyTaskAction | Unverified | `modules/game_taskboard/controller/tab_weekly.lua:221` | — |
| Cyclopedia | Unverified | `modules/game_taskboard/controller/tab_weekly.lua:303` | — |
| Show in Market | Unverified | `modules/game_taskboard/controller/tab_weekly.lua:319` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_taskboard/tasks.lua:169` | — |
| Event · onTaskHuntingShopData | Unverified | `modules/game_taskboard/tasks.lua:170` | — |
| Event · onTaskHuntingShopResult | Unverified | `modules/game_taskboard/tasks.lua:171` | — |
| Event · onWeeklyTaskData | Unverified | `modules/game_taskboard/tasks.lua:172` | — |
| Event · onBountyTaskData | Unverified | `modules/game_taskboard/tasks.lua:173` | — |
| Event · onBountyKillUpdate | Unverified | `modules/game_taskboard/tasks.lua:174` | — |
| Event · onWeeklyKillUpdate | Unverified | `modules/game_taskboard/tasks.lua:175` | — |
| Game API · taskHuntingShopRequest | Unverified | `modules/game_taskboard/tasks.lua:278` | — |
| Game API · sendRequestUsefulThings | Unverified | `modules/game_taskboard/tasks.lua:306` | — |
| Creature: %s\nDuration: %s\n\nClick in this window to open the prey dialog. | Unverified | `modules/game_taskboard/trackers/miniwindows_tracker.lua:222` | — |
| No active Bounty Task.\n\nClick to open the Bounty Task panel. | Unverified | `modules/game_taskboard/trackers/miniwindows_tracker.lua:276` | — |
| Bounty Task: %s\nProgress: %d/%d kills\n\nClick to open the Bounty Task panel. | Unverified | `modules/game_taskboard/trackers/miniwindows_tracker.lua:294` | — |
| Bounty Task: %s\nProgress: %d/%d kills\nCompleted! Click to claim your reward. | Unverified | `modules/game_taskboard/trackers/miniwindows_tracker.lua:312` | — |
| Weekly Task: %s\nProgress: %d/%d kills | Unverified | `modules/game_taskboard/trackers/miniwindows_tracker.lua:385` | — |
| Weekly Task: %s\nProgress: %d/%d kills\nCompleted! | Unverified | `modules/game_taskboard/trackers/miniwindows_tracker.lua:412` | — |
| Kill Tracker | Unverified | `modules/game_taskboard/trackers/styles/kill_tracker.otui:194` | — |
| Prey Creatures | Unverified | `modules/game_taskboard/trackers/styles/kill_tracker.otui:227` | — |
| Bounty Task | Unverified | `modules/game_taskboard/trackers/styles/kill_tracker.otui:253` | — |
| Weekly Tasks | Unverified | `modules/game_taskboard/trackers/styles/kill_tracker.otui:268` | — |

### game_textmessage

No named control extracted.

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|

### game_textwindow

Unverified 5

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameEnd | Unverified | `modules/game_textwindow/textwindow.lua:9` | — |
| Game API · editText | Unverified | `modules/game_textwindow/textwindow.lua:100` | — |
| Game API · editList | Unverified | `modules/game_textwindow/textwindow.lua:148` | — |
| Ok | Unverified | `modules/game_textwindow/textwindow.otui:53` | — |
| Cancel | Unverified | `modules/game_textwindow/textwindow.otui:63` | — |

### game_tutorial

Unverified 2

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onHoverChange | Unverified | `modules/game_tutorial/controller/tutorial_controller.lua:139` | — |
| Game API · sendTutorialChangeVocation | Unverified | `modules/game_tutorial/controller/tutorial_controller.lua:334` | — |

### game_unjustifiedpoints

Unverified 10

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onGameStart | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:23` | — |
| Event · onGameEnd | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:24` | — |
| Event · onUnjustifiedPointsChange | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:25` | — |
| Event · onOpenPvpSituationsChange | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:26` | — |
| Event · onAttackingCreatureChange | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:27` | — |
| Event · onSkullChange | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:30` | — |
| Remaining skull time | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:189` | — |
| You have no skull | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:192` | — |
| No Skull time active | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.lua:281` | — |
| Open PvP Situations | Unverified | `modules/game_unjustifiedpoints/unjustifiedpoints.otui:92` | — |

### game_viplist

PRESENT 4 | Unverified 45

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Add contact | PRESENT | `modules/game_viplist/viplist.lua:233` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:75` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Edit contact | PRESENT | `modules/game_viplist/viplist.lua:329` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:80` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Remove contact | PRESENT | `modules/game_viplist/viplist.lua:408` | `apps/client/src/bevy_shell/game_ui/hud/vip/input.rs:148` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Remove group | PRESENT | `modules/game_viplist/viplist.lua:792` | `apps/client/src/bevy_shell/game_ui/hud/vip/groups.rs:200` → `apps/client/src/features/0043-vip/protocol.rs:42` |
| Add VIP Group | Unverified | `modules/game_viplist/addgroup.otui:14` | — |
| Please enter a group name: | Unverified | `modules/game_viplist/addgroup.otui:26` | — |
| Cancel | Unverified | `modules/game_viplist/addgroup.otui:70` | — |
| Activate · modules.game_viplist.destroyAddGroupWindow() | Unverified | `modules/game_viplist/addgroup.otui:74` | — |
| Ok | Unverified | `modules/game_viplist/addgroup.otui:83` | — |
| Activate · modules.game_viplist.addGroup() | Unverified | `modules/game_viplist/addgroup.otui:87` | — |
| Add to VIP list | Unverified | `modules/game_viplist/addvip.otui:14` | — |
| Please enter a character name | Unverified | `modules/game_viplist/addvip.otui:26` | — |
| Activate · modules.game_viplist.destroyAddWindow() | Unverified | `modules/game_viplist/addvip.otui:73` | — |
| Activate · modules.game_viplist.addVip() | Unverified | `modules/game_viplist/addvip.otui:86` | — |
| Empty | Unverified | `modules/game_viplist/editvip.otui:2` | — |
| Edit VIP | Unverified | `modules/game_viplist/editvip.otui:37` | — |
| Description | Unverified | `modules/game_viplist/editvip.otui:52` | — |
| Notify-Login | Unverified | `modules/game_viplist/editvip.otui:75` | — |
| Notify on login | Unverified | `modules/game_viplist/editvip.otui:185` | — |
| Member of the following groups: | Unverified | `modules/game_viplist/editvip.otui:202` | — |
| Event · onAddVip | Unverified | `modules/game_viplist/viplist.lua:34` | — |
| Event · onVipStateChange | Unverified | `modules/game_viplist/viplist.lua:35` | — |
| Event · onVipGroupChange | Unverified | `modules/game_viplist/viplist.lua:36` | — |
| Game API · addVip | Unverified | `modules/game_viplist/viplist.lua:233` | — |
| Game API · editVip | Unverified | `modules/game_viplist/viplist.lua:329` | — |
| Game API · removeVip | Unverified | `modules/game_viplist/viplist.lua:408` | — |
| Game API · openPrivateChannel | Unverified | `modules/game_viplist/viplist.lua:583` | — |
| Add new VIP | Unverified | `modules/game_viplist/viplist.lua:658` | — |
| Add new group | Unverified | `modules/game_viplist/viplist.lua:665` | — |
| Sort by name | Unverified | `modules/game_viplist/viplist.lua:672` | — |
| Sort by type | Unverified | `modules/game_viplist/viplist.lua:678` | — |
| Sort by status | Unverified | `modules/game_viplist/viplist.lua:684` | — |
| Hide offline VIPs | Unverified | `modules/game_viplist/viplist.lua:690` | — |
| Show offline VIPs | Unverified | `modules/game_viplist/viplist.lua:694` | — |
| Show groups | Unverified | `modules/game_viplist/viplist.lua:701` | — |
| Hide groups | Unverified | `modules/game_viplist/viplist.lua:706` | — |
| Edit %s | Unverified | `modules/game_viplist/viplist.lua:753` | — |
| Remove %s | Unverified | `modules/game_viplist/viplist.lua:758` | — |
| Message to %s | Unverified | `modules/game_viplist/viplist.lua:765` | — |
| Invite to private chat | Unverified | `modules/game_viplist/viplist.lua:776` | — |
| Game API · inviteToOwnChannel | Unverified | `modules/game_viplist/viplist.lua:777` | — |
| Exclude from private chat | Unverified | `modules/game_viplist/viplist.lua:779` | — |
| Game API · excludeFromOwnChannel | Unverified | `modules/game_viplist/viplist.lua:780` | — |
| Edit group %s | Unverified | `modules/game_viplist/viplist.lua:788` | — |
| Remove group %s | Unverified | `modules/game_viplist/viplist.lua:791` | — |
| Game API · editVipGroups | Unverified | `modules/game_viplist/viplist.lua:792` | — |
| Copy Name | Unverified | `modules/game_viplist/viplist.lua:838` | — |
| No Group | Unverified | `modules/game_viplist/viplist.otui:23` | — |
| VIP | Unverified | `modules/game_viplist/viplist.otui:69` | — |

### game_walk

Unverified 7

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Game API · stop | Unverified | `modules/game_walk/walk.lua:92` | — |
| Game API · walk | Unverified | `modules/game_walk/walk.lua:120` | — |
| Game API · turn | Unverified | `modules/game_walk/walk.lua:193` | — |
| Event · onGameStart | Unverified | `modules/game_walk/walk.lua:260` | — |
| Event · onFocusChange | Unverified | `modules/game_walk/walk.lua:271` | — |
| Game API · enableFeature | Unverified | `modules/game_walk/walk.lua:275` | — |
| Game API · disableFeature | Unverified | `modules/game_walk/walk.lua:277` | — |

### game_wheel

Unverified 143

| Capability or reference binding | Disposition | OTClient | Oteryn evidence or scope |
|---|---|---|---|
| Event · onChildFocusChange | Unverified | `modules/game_wheel/classes/gematelier.lua:154` | — |
| %s%sSwitch the gem\ | Unverified | `modules/game_wheel/classes/gematelier.lua:718` | — |
| Game API · gemAction | Unverified | `modules/game_wheel/classes/gematelier.lua:940` | — |
| Increase your mitigation multiplicatively. | Unverified | `modules/game_wheel/classes/wheelclass.lua:1489` | — |
| Game API · sendApplyWheelPoints | Unverified | `modules/game_wheel/classes/wheelclass.lua:2014` | — |
| x 1 | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:33` | — |
| Enhance Mod Grade | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:79` | — |
| There is no mod that matches your filters. | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:85` | — |
| Activate · modules.game_wheel.Workshop.onUpgradeModification(self) | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:111` | — |
| 0 | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:144` | — |
| Grade IV | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:427` | — |
| +x%% Some Bonus | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:436` | — |
| Grade III | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:447` | — |
| Grade II | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:467` | — |
| Grade I | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:487` | — |
| Activate · self:getParent():clearText() | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:533` | — |
| All | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:543` | — |
| Basic Mods | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:544` | — |
| Supreme Mods | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:545` | — |
| In-Vessel Mods | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:546` | — |
| Activate · modules.game_wheel.Workshop.showFragmentList(false, true) | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:564` | — |
| Page 0 / 0 (0 Mods) | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:576` | — |
| Activate · modules.game_wheel.Workshop.showFragmentList(false, false) | Unverified | `modules/game_wheel/styles/fragmentMenu.otui:587` | — |
| Activate · modules.game_wheel.GemAtelier.onModRedirect(self) | Unverified | `modules/game_wheel/styles/gemMenu.otui:13` | — |
| +3%% Fire Resistence | Unverified | `modules/game_wheel/styles/gemMenu.otui:49` | — |
| Vessels | Unverified | `modules/game_wheel/styles/gemMenu.otui:414` | — |
| Activate · modules.game_wheel.GemAtelier.onClickVessel(self, 0) | Unverified | `modules/game_wheel/styles/gemMenu.otui:448` | — |
| Activate · modules.game_wheel.GemAtelier.onClickVessel(self, 1) | Unverified | `modules/game_wheel/styles/gemMenu.otui:498` | — |
| Activate · modules.game_wheel.GemAtelier.onClickVessel(self, 2) | Unverified | `modules/game_wheel/styles/gemMenu.otui:548` | — |
| Activate · modules.game_wheel.GemAtelier.onClickVessel(self, 3) | Unverified | `modules/game_wheel/styles/gemMenu.otui:598` | — |
| Gem Revelation | Unverified | `modules/game_wheel/styles/gemMenu.otui:648` | — |
| Lesser Gem (x 0) | Unverified | `modules/game_wheel/styles/gemMenu.otui:691` | — |
| Activate · modules.game_wheel.GemAtelier.onRevealGem(self, 0) | Unverified | `modules/game_wheel/styles/gemMenu.otui:702` | — |
| Activate · modules.game_wheel.GemAtelier.onRevealGem(self, 1) | Unverified | `modules/game_wheel/styles/gemMenu.otui:783` | — |
| Activate · modules.game_wheel.GemAtelier.onRevealGem(self, 2) | Unverified | `modules/game_wheel/styles/gemMenu.otui:864` | — |
| Click on a gem to see its mods | Unverified | `modules/game_wheel/styles/gemMenu.otui:915` | — |
| There is no gem available. | Unverified | `modules/game_wheel/styles/gemMenu.otui:934` | — |
| Lesser Guardian Gem | Unverified | `modules/game_wheel/styles/gemMenu.otui:964` | — |
| Domain Affinity | Unverified | `modules/game_wheel/styles/gemMenu.otui:981` | — |
| Activate · modules.game_wheel.GemAtelier.manageVessel(false) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1016` | — |
| Activate · modules.game_wheel.GemAtelier.manageVessel(true) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1030` | — |
| Activate · modules.game_wheel.GemAtelier.onSwitchDomain(self) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1046` | — |
| Activate · modules.game_wheel.GemAtelier.onDestroyGem(self) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1088` | — |
| Activate · self:getParent():setText("") | Unverified | `modules/game_wheel/styles/gemMenu.otui:1119` | — |
| All Affinities | Unverified | `modules/game_wheel/styles/gemMenu.otui:1129` | — |
| Top Left | Unverified | `modules/game_wheel/styles/gemMenu.otui:1130` | — |
| Top Right | Unverified | `modules/game_wheel/styles/gemMenu.otui:1131` | — |
| Bottom Left | Unverified | `modules/game_wheel/styles/gemMenu.otui:1132` | — |
| Bottom Right | Unverified | `modules/game_wheel/styles/gemMenu.otui:1133` | — |
| All Qualities | Unverified | `modules/game_wheel/styles/gemMenu.otui:1144` | — |
| Lesser | Unverified | `modules/game_wheel/styles/gemMenu.otui:1145` | — |
| Regular | Unverified | `modules/game_wheel/styles/gemMenu.otui:1146` | — |
| Greater | Unverified | `modules/game_wheel/styles/gemMenu.otui:1147` | — |
| Locked only | Unverified | `modules/game_wheel/styles/gemMenu.otui:1153` | — |
| Activate · modules.game_wheel.GemAtelier.managePage(self, true) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1181` | — |
| Page 0 / 0 (0 Gems) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1196` | — |
| Activate · modules.game_wheel.GemAtelier.managePage(self, false) | Unverified | `modules/game_wheel/styles/gemMenu.otui:1207` | — |
| New Preset | Unverified | `modules/game_wheel/styles/newPreset.otui:3` | — |
| Preset name: | Unverified | `modules/game_wheel/styles/newPreset.otui:15` | — |
| Preset names must be unique. Also, this field must not be empty. | Unverified | `modules/game_wheel/styles/newPreset.otui:32` | — |
| Use empty Wheel of Destiny | Unverified | `modules/game_wheel/styles/newPreset.otui:50` | — |
| Copy preset  | Unverified | `modules/game_wheel/styles/newPreset.otui:65` | — |
| Import | Unverified | `modules/game_wheel/styles/newPreset.otui:80` | — |
| Export code does not match a valid Wheel of Destiny. | Unverified | `modules/game_wheel/styles/newPreset.otui:99` | — |
| Cancel | Unverified | `modules/game_wheel/styles/newPreset.otui:124` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onCancelConfig() | Unverified | `modules/game_wheel/styles/newPreset.otui:126` | — |
| Ok | Unverified | `modules/game_wheel/styles/newPreset.otui:132` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onConfirmCreatePreset() | Unverified | `modules/game_wheel/styles/newPreset.otui:136` | — |
| Rename Preset | Unverified | `modules/game_wheel/styles/renamePreset.otui:3` | — |
| Preset Name: | Unverified | `modules/game_wheel/styles/renamePreset.otui:15` | — |
| Presets names need to be unique. Also, this field must not be empty. | Unverified | `modules/game_wheel/styles/renamePreset.otui:30` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onConfirmRenamePreset(true) | Unverified | `modules/game_wheel/styles/renamePreset.otui:58` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onConfirmRenamePreset(false) | Unverified | `modules/game_wheel/styles/renamePreset.otui:67` | — |
| Healing Link | Unverified | `modules/game_wheel/styles/wheelMenu.otui:5` | — |
| 0%% | Unverified | `modules/game_wheel/styles/wheelMenu.otui:15` | — |
| Selection | Unverified | `modules/game_wheel/styles/wheelMenu.otui:82` | — |
| Vessel contains no gem | Unverified | `modules/game_wheel/styles/wheelMenu.otui:103` | — |
| Sealed Vessel (VR 0) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:160` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onChangeGemButton(0) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:192` | — |
| 0 / 0 | Unverified | `modules/game_wheel/styles/wheelMenu.otui:216` | — |
| Dedication Perk | Unverified | `modules/game_wheel/styles/wheelMenu.otui:229` | — |
| toDo | Unverified | `modules/game_wheel/styles/wheelMenu.otui:243` | — |
| Conviction Perk | Unverified | `modules/game_wheel/styles/wheelMenu.otui:259` | — |
| + Max | Unverified | `modules/game_wheel/styles/wheelMenu.otui:290` | — |
| Activate · modules.game_wheel.onAddMax() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:294` | — |
| +1 | Unverified | `modules/game_wheel/styles/wheelMenu.otui:300` | — |
| - Max | Unverified | `modules/game_wheel/styles/wheelMenu.otui:310` | — |
| Activate · modules.game_wheel.onRmvMax() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:314` | — |
| -1 | Unverified | `modules/game_wheel/styles/wheelMenu.otui:320` | — |
| Available Promotion Points | Unverified | `modules/game_wheel/styles/wheelMenu.otui:350` | — |
| Activate · toggleTabBarButtons('informationButton') | Unverified | `modules/game_wheel/styles/wheelMenu.otui:418` | — |
| Activate · toggleTabBarButtons('managePresetsButton') | Unverified | `modules/game_wheel/styles/wheelMenu.otui:438` | — |
| Fill or empty slice with a\nright-click.\n\nHold Alt, Shift, Ctrl, or\nCtrl+Shift to modify the step\nsize. | Unverified | `modules/game_wheel/styles/wheelMenu.otui:462` | — |
| Fill or empty a slice with a\nright-click. | Unverified | `modules/game_wheel/styles/wheelMenu.otui:545` | — |
| Activate · modules.game_wheel.WheelOfDestiny.showNewPreset(true) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:562` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onRenamePreset() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:576` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onDeletePreset() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:590` | — |
| Activate · modules.game_wheel.onWheelOfDestinyApply(false) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:605` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onImportPreset() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:617` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onExportPreset() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:629` | — |
| Name | Unverified | `modules/game_wheel/styles/wheelMenu.otui:648` | — |
| Points | Unverified | `modules/game_wheel/styles/wheelMenu.otui:665` | — |
| Activate · toggleSummary() | Unverified | `modules/game_wheel/styles/wheelMenu.otui:707` | — |
| Summary | Unverified | `modules/game_wheel/styles/wheelMenu.otui:711` | — |
| Dedication Perks | Unverified | `modules/game_wheel/styles/wheelMenu.otui:767` | — |
| Hit Points | Unverified | `modules/game_wheel/styles/wheelMenu.otui:787` | — |
| Mana | Unverified | `modules/game_wheel/styles/wheelMenu.otui:811` | — |
| Capacity | Unverified | `modules/game_wheel/styles/wheelMenu.otui:835` | — |
| Mitigation Mult. | Unverified | `modules/game_wheel/styles/wheelMenu.otui:859` | — |
| Conviction Perks | Unverified | `modules/game_wheel/styles/wheelMenu.otui:897` | — |
| Revelation Perks | Unverified | `modules/game_wheel/styles/wheelMenu.otui:982` | — |
| Damage and Healing | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1003` | — |
| +0 | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1012` | — |
| Avatar Of Nature | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1037` | — |
| Locked | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1046` | — |
| Transforms you into a\npowerful form that reduces\ndamage taken and increases\ndamage dealt. | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1061` | — |
| Blessing of the... | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1072` | — |
| Gift Of Life | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1107` | — |
| Twin Bursts | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1142` | — |
| Activate · modules.game_wheel.onWheelClick(g_window:getMousePosition()) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:1175` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onGemVesselClick(1) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:2567` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onGemVesselClick(3) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:2676` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onGemVesselClick(2) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:2785` | — |
| Activate · modules.game_wheel.WheelOfDestiny.onGemVesselClick(0) | Unverified | `modules/game_wheel/styles/wheelMenu.otui:3036` | — |
| Event · onOptionChange | Unverified | `modules/game_wheel/wheel.lua:47` | — |
| Event · onSelectionChange | Unverified | `modules/game_wheel/wheel.lua:78` | — |
| Event · onGameEnd | Unverified | `modules/game_wheel/wheel.lua:95` | — |
| Event · onGameStart | Unverified | `modules/game_wheel/wheel.lua:96` | — |
| Event · onResourcesBalanceChange | Unverified | `modules/game_wheel/wheel.lua:99` | — |
| Game API · openWheel | Unverified | `modules/game_wheel/wheel.lua:146` | — |
| Wheel Of Destiny | Unverified | `modules/game_wheel/wheel.otui:3` | — |
| Activate · loadMenu('wheelMenu') | Unverified | `modules/game_wheel/wheel.otui:36` | — |
| Activate · loadMenu('gemMenu') | Unverified | `modules/game_wheel/wheel.otui:53` | — |
| Activate · loadMenu('fragmentMenu') | Unverified | `modules/game_wheel/wheel.otui:70` | — |
| Activate · print("toDo Preset") | Unverified | `modules/game_wheel/wheel.otui:189` | — |
| Current Preset: Default-Preset | Unverified | `modules/game_wheel/wheel.otui:197` | — |
| Close | Unverified | `modules/game_wheel/wheel.otui:206` | — |
| Activate · hide() | Unverified | `modules/game_wheel/wheel.otui:210` | — |
| Reset | Unverified | `modules/game_wheel/wheel.otui:216` | — |
| Activate · resetWheel() | Unverified | `modules/game_wheel/wheel.otui:220` | — |
| Apply | Unverified | `modules/game_wheel/wheel.otui:226` | — |
| Activate · onWheelOfDestinyApply(false) | Unverified | `modules/game_wheel/wheel.otui:230` | — |
| Activate · onWheelOfDestinyApply(true) | Unverified | `modules/game_wheel/wheel.otui:240` | — |

## Official window observations

The official client was observed separately from OTClient. These entries preserve the observed scope and identify matching module tables. An empty list, account gate or unopened action limits what the observation proves.

| Official window | Observed controls | Module or Oteryn surface |
|---|---|---|
| Skills | Level, experience, gain, health/mana, capacity, speed, food, stamina; visibility/context menu | `game_skills` |
| Battle List | Filter strip; name and eight ascending/descending sort choices | `game_battle` |
| Spell List | Search, details, formula, vocation, group, cooldown, mana and minimum level; filtering menu | `game_spelllist` |
| VIP List | Empty list; add, sort, offline and grouping menu | `game_viplist` |
| Quest Log | Quest/mission details, search, sorting and tracker controls | `game_questlog` |
| Compendium | Player guide, client features, useful information, updates and support tabs | `game_compendium` |
| Cyclopedia / Map | Thais map, layers, filters and navigation | `game_cyclopedia` |
| Cyclopedia / Items | Categories/search/filters; item stats, trade offers, valuation and drop/loot switches | `game_cyclopedia` |
| Cyclopedia / Houses | House/guildhall categories and state/sort controls | `game_cyclopedia` |
| Cyclopedia / Character | General information and section navigation | `game_cyclopedia` |
| Cyclopedia / Bestiary | Creature categories, counts, search and tracking | `game_cyclopedia` |
| Cyclopedia / Charms | Major/minor charms, cost, assignment and restrictions | `game_cyclopedia` |
| Cyclopedia / Bosstiary | Unknown boss grid, filters, progression tiers | `game_cyclopedia` |
| Cyclopedia / Boss Slots | Slot grid, boosted boss and unlock conditions | `game_cyclopedia` |
| Cyclopedia / Magical Archive | Spell/rune list, combat and additional details | `game_cyclopedia` |
| Highscores | World, vocation, category and PvP filters; ranking and own rank | `game_highscore` |
| Party List | Empty docked list and sorting menu | `party` |
| Wheel of Destiny | Official preview, wheel/perks and vessels | `game_wheel` |
| Gem Atelier | Empty collection, affinity/quality filters and revelation costs | `game_wheel` |
| Fragment Workshop | Modifier grid, search/filters, four grades and enhancement costs | `game_wheel` |
| Premium features | Expanded benefits list | `local-premium` |

## Official settings

The official inventory in `apps/client/REFERENCE-AUDIT.md:11` lists 20 pages. The Bevy navigation declares Basic and 16 advanced destinations. General Hotkeys hosts profile and binding editors, Controls opens the hotkey editor, and Misc includes guide and file transfer controls. The 23 labels in the retained settings model do not establish 23 reachable Bevy pages.

| Official page | Live Bevy destination | Source |
|---|---|---|
| Basic Options | Basic | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:164` |
| Controls | Controls | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:165` |
| General Hotkeys | GeneralHotkeys | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:167` |
| Action Bar Hotkeys | Controls opens the hotkey editor | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:340` |
| Custom Hotkeys | General Hotkeys hosts profiles and key bindings | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:336` |
| Interface | Interface | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:168` |
| HUD | Hud | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:169` |
| Console | Console | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:170` |
| Game Window | GameWindow | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:171` |
| Action Bars | ActionBars | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:172` |
| Shortcuts | No dedicated Shortcuts destination | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:27` |
| Graphics | Graphics | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:173` |
| Effects | Effects | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:174` |
| Sound | Sound | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:175` |
| Battle Sounds | BattleSounds | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:176` |
| UI Sounds | UiSounds | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:177` |
| Miscellaneous | Misc | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:178` |
| Gameplay | Gameplay | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:179` |
| Screenshots | Screenshots | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| Help | Guide under Misc | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:327` |

### Preference fields

Each row maps a field to an editable Bevy Options control where the semantics could be traced. A broader control is identified explicitly. An unmatched field remains unverified across contextual menus and alternate input paths. The shared widget builder reads the draft and creates the corresponding checkbox, slider or choice at `apps/client/src/bevy_shell/game_ui/options/widgets.rs:343`. The typed adapter updates that draft at `apps/client/src/bevy_shell/game_ui/options/metadata.rs:120`. These are control-exposure findings only.

| Reference key | Exposed control or gap | Definition |
|---|---|---|
| `ChatEnabled` | Related control · Show Local Speech | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:101` |
| `GlobalMessagesEnabled` | Related control · Show Public Channel Messages | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:109` |
| `GuildMessagesEnabled` | Show Guild Messages | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:107` |
| `NpcMessageseEnabled` | No equivalent editor verified | — |
| `PartyMessagesEnabled` | Show Party Messages | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:113` |
| `PrivateMessagesEnabled` | Related control · Show Private Messages and Channels | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:103` |
| `PrivateMessagesWithoutTabEnabled` | No equivalent editor verified | — |
| `RaidMessagesEnabled` | No equivalent editor verified | — |
| `SystemMessagesEnabled` | No equivalent editor verified | — |
| `TeamFinderMessagesEnabled` | No equivalent editor verified | — |
| `actionBarBottomLocked` | No equivalent editor verified | — |
| `actionBarLeftLocked` | No equivalent editor verified | — |
| `actionBarRightLocked` | No equivalent editor verified | — |
| `actionBarShowBottom1` | Show Bottom Action Bar 1 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:241` |
| `actionBarShowBottom2` | Show Bottom Action Bar 2 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:246` |
| `actionBarShowBottom3` | Show Bottom Action Bar 3 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:251` |
| `actionBarShowLeft1` | Show Left Action Bar 1 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:257` |
| `actionBarShowLeft2` | Show Left Action Bar 2 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:262` |
| `actionBarShowLeft3` | Show Left Action Bar 3 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:267` |
| `actionBarShowRight1` | Show Right Action Bar 1 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:279` |
| `actionBarShowRight2` | Show Right Action Bar 2 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:284` |
| `actionBarShowRight3` | Show Right Action Bar 3 | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:289` |
| `actionBarsShowBottom` | Show All Bottom Action Bars | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:234` |
| `actionBarsShowLeft` | Show All Left Action Bars | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:255` |
| `actionBarsShowRight` | Show All Right Action Bars | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:272` |
| `actionButtonAllowTooltip` | Show Action Tooltips | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:224` |
| `actionButtonAutoInsertSpells` | Automatically Insert New Actions | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:229` |
| `actionButtonShowAmount` | Show Item Amounts | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:214` |
| `actionButtonShowCooldownNumbers` | No equivalent editor verified | — |
| `actionButtonShowGraphicalCooldown` | No equivalent editor verified | — |
| `actionButtonShowHotkey` | No equivalent editor verified | — |
| `actionButtonShowSpellParameters` | No equivalent editor verified | — |
| `alwaysTurnTowardsMoveDirection` | No equivalent editor verified | — |
| `antialiasingMode` | No equivalent editor verified | — |
| `autoChaseEnabled` | No equivalent editor verified | — |
| `charTradePaymentRemindAgain` | No equivalent editor verified | — |
| `characterSelectionShowHidden` | No equivalent editor verified | — |
| `characterSelectionShowOutfits` | No equivalent editor verified | — |
| `characterSelectionSortAscendingOrder` | No equivalent editor verified | — |
| `characterSelectionSortColumn` | No equivalent editor verified | — |
| `colorizeLootStyle` | No equivalent editor verified | — |
| `combatShowFrames` | No equivalent editor verified | — |
| `combatShowPvpFrames` | Show PvP Frames | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:168` |
| `connectionStabilityOptimizationEnabled` | Optimise Connection Stability | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:45` |
| `consoleShowEventMessages` | No equivalent editor verified | — |
| `consoleShowInfoMessages` | No equivalent editor verified | — |
| `consoleShowJoinLeaveMessages` | No equivalent editor verified | — |
| `consoleShowLevels` | No equivalent editor verified | — |
| `consoleShowStatusMessagesOfOthers` | No equivalent editor verified | — |
| `consoleShowStatusMessagesOwn` | No equivalent editor verified | — |
| `consoleShowTimestamps` | No equivalent editor verified | — |
| `consoleShowTimestampsSeconds` | No equivalent editor verified | — |
| `containerMoveToManagedContainerRecursive` | No equivalent editor verified | — |
| `containerMoveToManagedContainerRecursiveShowWarningAgain` | No equivalent editor verified | — |
| `containerMoveToManagedContainerRecursiveWarning` | No equivalent editor verified | — |
| `containerSortBackpacksFirst` | No equivalent editor verified | — |
| `containerSortRecursive` | No equivalent editor verified | — |
| `containerSortRecursiveShowWarningAgain` | No equivalent editor verified | — |
| `controlSchemeIndex` | Related control · Mouse Preset | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:6` |
| `cooldownBarEnabled` | Show Cooldown Bar | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:204` |
| `creatueShowNpcIcon` | Show NPC Icons | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:42` |
| `creatureHudEnabled` | No equivalent editor verified | — |
| `creatureShowHealth` | No equivalent editor verified | — |
| `creatureShowMarks` | Show Other Creatures' Marks | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:38` |
| `creatureShowName` | Show Creature Names | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:35` |
| `dragAndDropDefaultActionIsMoveAll` | No equivalent editor verified | — |
| `emergencyDialogVersion` | No equivalent editor verified | — |
| `frameRateLimit` | Frame Rate Limit (0 = unlimited) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:63` |
| `frameRateLimitEnabled` | Related control · Frame Rate Limit (0 = unlimited) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:63` |
| `gameWindowLowerPaneHeight` | No equivalent editor verified | — |
| `gameWindowResizingMode` | No equivalent editor verified | — |
| `gameWindowScaleOnlyByEvenMultiples` | Related control · Scale Using Only Integral Multiples | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:26` |
| `gameWindowShowAttackAnimation` | No equivalent editor verified | — |
| `gameWindowShowBoostedCreatureMessages` | No equivalent editor verified | — |
| `gameWindowShowHotkeyUsageMessages` | No equivalent editor verified | — |
| `gameWindowShowInfoBanner` | Show Banner | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:174` |
| `gameWindowShowLootHighlighting` | Highlight Loot | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:170` |
| `gameWindowShowLootMessages` | Show Loot Messages | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:159` |
| `gameWindowShowMessages` | No equivalent editor verified | — |
| `gameWindowShowOfflineTrainingMessages` | No equivalent editor verified | — |
| `gameWindowShowOthersSpells` | Related control · Show Other Creatures' Spells | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:149` |
| `gameWindowShowOwnSpells` | Show Own Spells | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:144` |
| `gameWindowShowPotionMessages` | No equivalent editor verified | — |
| `gameWindowShowPrivateMessages` | No equivalent editor verified | — |
| `gameWindowShowStoreMessages` | No equivalent editor verified | — |
| `gameWindowShowTargetHighlight` | No equivalent editor verified | — |
| `gameWindowShowTextualEffects` | Show Value Text | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:154` |
| `gameWindowShowTragetFrame` | Show Target Frame | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:164` |
| `gameWindowUpperPaneHeight` | No equivalent editor verified | — |
| `inspectPlayerAllowAllEnabled` | No equivalent editor verified | — |
| `keyboardDelayMs` | Keyboard Delay (ms) | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:64` |
| `keyboardDelayUseDefault` | No equivalent editor verified | — |
| `lightAmbientLevel` | Minimum Ambient Brightness (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:39` |
| `lightAttenuationCloudsIndoor` | No equivalent editor verified | — |
| `lightEffectsEnabled` | Show Lights | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:101` |
| `lightLevelSeparatorLevel` | Lower-Floor Dimming (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:55` |
| `linkCopyWarningEnabled` | No equivalent editor verified | — |
| `loginEmailAddress` | Platform account identity | NOT-APPLICABLE |
| `lootSchemeIndex` | No equivalent editor verified | — |
| `marketShowLockerOnly` | No equivalent editor verified | — |
| `monsterBossAreaEffectsOpacity` | Boss Area Opacity (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:74` |
| `monsterEffectsOpacity` | Creature Effects Opacity (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:121` |
| `mouseAnimatedCursor` | No equivalent editor verified | — |
| `mouseBigCursor` | No equivalent editor verified | — |
| `mouseHighlightTarget` | No equivalent editor verified | — |
| `mouseSystemCursor` | No equivalent editor verified | — |
| `openNewPrivateMessagesInNewTab` | No equivalent editor verified | — |
| `optionsShowAdvanced` | No equivalent editor verified | — |
| `otherPlayersEffectsOpacity` | Other Players' Effects Opacity (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:113` |
| `ownEffectsOpacity` | Own Effects Opacity (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:105` |
| `playerHudArcDistance` | Arc Distance | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:57` |
| `playerHudArcHeight` | Related control · Arc Size | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:64` |
| `playerHudArcOpacity` | Arc Opacity (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:66` |
| `playerHudArcWidth` | Related control · Arc Size | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:64` |
| `playerHudEnabled` | No equivalent editor verified | — |
| `playerHudShowArcs` | No equivalent editor verified | — |
| `playerHudShowBars` | No equivalent editor verified | — |
| `playerHudShowHarmonyLeft` | No equivalent editor verified | — |
| `playerShowHarmonySerene` | Show Serene Effects | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:103` |
| `playerShowHealth` | Show Health | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:27` |
| `playerShowMana` | Show Mana | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:28` |
| `playerShowMarks` | Show Own Marks | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:36` |
| `playerShowName` | Show Own Name | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:29` |
| `quickLogin` | Quick login: use the last available character | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:30` |
| `quickLootAllCorpsesInAreaEnabled` | No equivalent editor verified | — |
| `rememberLoginEmail` | Platform account identity | NOT-APPLICABLE |
| `rendererIndex` | No equivalent editor verified | — |
| `rotateWithAlt` | Rotation Modifier | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:39` |
| `rotateWithCtrl` | Rotation Modifier | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:39` |
| `rotateWithShift` | Rotation Modifier | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:39` |
| `screenshotsAchievementEnabled` | Capture on achievement | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:203` |
| `screenshotsAutoScreenshotsEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsBestiaryFullEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsBestiaryPartlyEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsBossKillEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsDeathPvEEnabled` | Related control · Capture on death | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:188` |
| `screenshotsDeathPvPEnabled` | Related control · Capture on death | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:188` |
| `screenshotsGiftOfLifeEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsLevelUpEnabled` | Capture on level up | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:193` |
| `screenshotsLowHitPointsEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsNewMaxDamageEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsNewMaxHealingEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsPlayerKillAssistEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsPlayerKillFullEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsPvPAttackEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsRewardChestEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsSkillUpEnabled` | Capture on skill up | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:198` |
| `screenshotsTrackedLootEnabled` | No individual switch on Screenshots page | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:181` |
| `screenshotsUseBacklog` | Screenshot backlog (seconds, 0 disables) | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:208` |
| `screnshotsOnlyGameWindow` | Capture game area only | `apps/client/src/bevy_shell/game_ui/options/metadata.rs:183` |
| `sessionRemainLoggedIn` | misc.remember_device | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:4` |
| `showEMailAsPlainText` | Platform account identity | NOT-APPLICABLE |
| `showExpireInContainers` | No equivalent editor verified | — |
| `showExpireInInventory` | No equivalent editor verified | — |
| `showExpireWhenUnused` | No equivalent editor verified | — |
| `showFpsLagIndicator` | Related control · Show Frame Rate | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:83` |
| `soundAmbienceVolume` | Ambient Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:20` |
| `soundAnthemEnabled` | No equivalent editor verified | — |
| `soundBattleCreaturesAttacksEnabled` | Creature Attack Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:116` |
| `soundBattleCreaturesDeathEnabled` | No equivalent editor verified | — |
| `soundBattleCreaturesNoisesEnabled` | No equivalent editor verified | — |
| `soundBattleCreaturesVolume` | Creature Sounds Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:78` |
| `soundBattleOthersAttackSpellsEnabled` | Other Players' Attack Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:101` |
| `soundBattleOthersHealingSpellsEnabled` | Other Players' Healing Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:106` |
| `soundBattleOthersSpellsEnabled` | No equivalent editor verified | — |
| `soundBattleOthersSupportSpellsEnabled` | Other Players' Support Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:111` |
| `soundBattleOthersVolume` | Other Players' Sounds Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:70` |
| `soundBattleOthersWeaponsEnabled` | No equivalent editor verified | — |
| `soundBattleOwnAttackSpellsEnabled` | Own Attack Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:86` |
| `soundBattleOwnHealingSpellsEnabled` | Own Healing Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:91` |
| `soundBattleOwnSpellsEnabled` | No equivalent editor verified | — |
| `soundBattleOwnSupportSpellsEnabled` | Own Support Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:96` |
| `soundBattleOwnVolume` | Own Sounds Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:62` |
| `soundBattleOwnWeaponsEnabled` | No equivalent editor verified | — |
| `soundEatingEnabled` | No equivalent editor verified | — |
| `soundEventsVolume` | Event Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:44` |
| `soundItemsVolume` | Item Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:36` |
| `soundMasterVolume` | Master Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:12` |
| `soundMasterVolumeOld` | No equivalent editor verified | — |
| `soundMoveItemEnabled` | No equivalent editor verified | — |
| `soundMusicVolume` | Music Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:28` |
| `soundPartyEnabled` | Party Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:137` |
| `soundUiInteractionsEnabled` | Interaction Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:133` |
| `soundUiVolume` | UI Volume (%) | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:52` |
| `soundVipEnabled` | VIP Sounds | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:138` |
| `stashAskBeforeStowContainerContent` | No equivalent editor verified | — |
| `statusBarEnabled` | No equivalent editor verified | — |
| `statusPanelEnabled` | Show Status Panel | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:44` |
| `storeAskBeforeBuyingProducts` | No equivalent editor verified | — |
| `useFreetypeRenderer` | No equivalent editor verified | — |
| `usedSoundDevice` | Audio device selector | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:317` |
| `vSyncEnabled` | Vertical Synchronisation (V-Sync) | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:21` |
| `clientWindowHeight` | No equivalent editor verified | — |
| `clientWindowIsBorderless` | No equivalent editor verified | — |
| `clientWindowIsMaximized` | No equivalent editor verified | — |
| `clientWindowPositionX` | No equivalent editor verified | — |
| `clientWindowPositionY` | No equivalent editor verified | — |
| `clientWindowScreenName` | No equivalent editor verified | — |
| `clientWindowWidth` | No equivalent editor verified | — |
| `chatModeOn` | No equivalent editor verified | — |
| `lootChannelOpen` | No equivalent editor verified | — |
| `openChannels` | No equivalent editor verified | — |
| `disabledButtons` | No equivalent editor verified | — |
| `enabledButtons` | No equivalent editor verified | — |
| `height` | No equivalent editor verified | — |
| `width` | No equivalent editor verified | — |
| `x` | No equivalent editor verified | — |
| `y` | No equivalent editor verified | — |
| `showInBar` | Condition columns and ordering | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:323` |
| `showInHUD` | Condition columns and ordering | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:323` |
| `specialConditionsOrder` | Condition columns and ordering | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:323` |
| `visibleInBar` | Condition columns and ordering | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:323` |
| `visibleInHUD` | Condition columns and ordering | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:323` |
