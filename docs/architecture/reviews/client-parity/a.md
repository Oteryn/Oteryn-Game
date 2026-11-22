# Client parity audit — options, menus and combat controls

Oteryn `a4ad4164` and OTClient `4955a3fa22d5f5456ec6c658187fd2901545b750`. Counts describe player operations within each module. A selector with several values counts once. A visibility preference and the gameplay operation it controls are counted separately. Evidence is from static source tracing across the app client and client crates. Local options use the preference draft and apply path in `apps/client/src/bevy_shell/owner/options_draft.rs:79`.

| Module | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE | Operations |
|---|---:|---:|---:|---:|---:|
| client_options | 55 | 26 | 35 | 9 | 125 |
| client_bottommenu | 2 | 3 | 0 | 0 | 5 |
| client_topmenu | 0 | 3 | 2 | 3 | 8 |
| game_actionbar | 16 | 4 | 2 | 1 | 23 |
| game_hotkeys | 11 | 3 | 4 | 0 | 18 |
| game_shortcuts | 0 | 0 | 0 | 1 | 1 |
| game_cooldown | 2 | 0 | 0 | 0 | 2 |
| game_mainpanel | 5 | 0 | 1 | 0 | 6 |
| Total | 91 | 39 | 44 | 14 | 188 |

## client_options

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Choose mouse interaction scheme | PARTIAL | `modules/client_options/options.lua:201` | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:6` selects handedness. Classic and smart-click interaction modes are absent. |
| Choose loot gesture | PARTIAL | `modules/client_options/options.lua:205` | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:62` enables secondary-click loot. Shift-right and left-click presets are absent. |
| Switch basic or advanced settings | PRESENT | `modules/client_options/options.otui:254` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:184` → `apps/client/src/bevy_shell/game_ui/options.rs:244` |
| Apply settings | PRESENT | `modules/client_options/options.otui:271` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:192` → `apps/client/src/bevy_shell/owner/options_draft.rs:50` |
| Discard pending edits | PRESENT | `modules/client_options/options.otui:281` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:193` → `apps/client/src/bevy_shell/owner/options_draft.rs:45` |
| Display connection latency | PRESENT | `modules/client_options/styles/controls/general.otui:21` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:91` → `apps/client/src/features/local-reference-preferences/performance.rs:136` displays the observed RTT in milliseconds. |
| Override chase by walking | PRESENT | `modules/client_options/styles/controls/general.otui:76` | `apps/client/src/bevy_shell/game_ui/options/metadata/miscellaneous.rs:57` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Enable right-click NPC talk | MISSING | `modules/client_options/styles/controls/general.otui:87` | Searched NPC secondary-click dialogue preference and context-menu controls. |
| Toggle chat mode with Enter | PARTIAL | `modules/client_options/styles/controls/general.otui:98` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:65` sets distinct contexts. The profile selector does not configure the Enter gesture. |
| Move whole stacks directly | MISSING | `modules/client_options/styles/controls/general.otui:110` | Searched direct stack movement and skip quantity prompt setting. |
| Combine direction keys for diagonal walking | MISSING | `modules/client_options/styles/controls/general.otui:121` | Searched smart walking and diagonal key combination preference. |
| Maximize newly opened containers | MISSING | `modules/client_options/styles/controls/general.otui:133` | Searched container maximize-on-open preference. |
| Toggle on-screen text messages | PARTIAL | `modules/client_options/styles/controls/general.otui:144` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:143` filters spells, values and loot individually. No master text switch. |
| Set hotkey repeat delay | PARTIAL | `modules/client_options/styles/controls/general.otui:156` | `apps/client/src/bevy_shell/game_ui/options/metadata/controls.rs:64` edits keyboard delay without a distinct hotkey repeat setting. |
| Set walking delay after turning | MISSING | `modules/client_options/styles/controls/general.otui:174` | Searched post-turn movement delay preference. |
| Set walking delay after teleporting | MISSING | `modules/client_options/styles/controls/general.otui:192` | Searched teleport movement delay preference. |
| Set walking delay after changing floors | MISSING | `modules/client_options/styles/controls/general.otui:210` | Searched floor-change movement delay preference. |
| Open hotkey manager | PRESENT | `modules/client_options/styles/controls/general.otui:221` | `apps/client/src/bevy_shell/owner/options_draft.rs:20` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` |
| Create a binding preset | PRESENT | `modules/client_options/styles/controls/keybinds.otui:98` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:50` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:108` |
| Copy a binding preset | PRESENT | `modules/client_options/styles/controls/keybinds.otui:104` | The Add control in `apps/client/src/bevy_shell/game_ui/options/profiles.rs:50` captures and clones the selected binding set in `apps/client/src/features/local-reference-preferences/profile_contexts.rs:154`. |
| Rename a binding preset | PRESENT | `modules/client_options/styles/controls/keybinds.otui:110` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:51` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:109` |
| Remove a binding preset | PRESENT | `modules/client_options/styles/controls/keybinds.otui:116` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:52` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:110` |
| Automatically switch presets for a character | PARTIAL | `modules/client_options/styles/controls/keybinds.otui:128` | `apps/client/src/features/local-reference-preferences/character_scope.rs:17` automatically restores character preferences. No optional switch to a preset matching the character name. |
| Choose chat-on or chat-off binding set | PRESENT | `modules/client_options/styles/controls/keybinds.otui:154` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:65` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:111` |
| Search bindings by text | MISSING | `modules/client_options/styles/controls/keybinds.otui:187` | Searched binding filter and search field in keyboard editor. |
| Rebind primary or secondary shortcut | PRESENT | `modules/client_options/styles/controls/keybinds.otui:237` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:94` → `apps/client/src/bevy_shell/game_ui/options/keys.rs:128` |
| Define a custom hotkey action | PRESENT | `modules/client_options/styles/controls/keybinds.otui:275` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:374` |
| Restore default key assignments | PARTIAL | `modules/client_options/styles/controls/keybinds.otui:285` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:116` restores a selected action. The key list has no bulk reset. |
| Clear a shortcut | PARTIAL | `modules/client_options/styles/controls/key_edit.otui:61` | `apps/client/src/bevy_shell/game_ui/options/keys.rs:99` clears a secondary binding. Primary offers restore-default only. |
| Toggle world lighting | PRESENT | `modules/client_options/styles/graphics/effects.otui:14` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:101` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Adjust spell effect opacity | PRESENT | `modules/client_options/styles/graphics/effects.otui:143` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:105` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Draw effects after the entire floor | NOT-APPLICABLE | `modules/client_options/styles/graphics/effects.otui:77` | OTClient tile-renderer ordering override. |
| Limit visible dimension to aware range | NOT-APPLICABLE | `modules/client_options/styles/graphics/effects.otui:89` | OTClient map renderer range override. |
| Toggle vertical synchronization | PRESENT | `modules/client_options/styles/graphics/graphics.otui:41` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:20` → `apps/client/src/features/local-reference-preferences/graphics.rs:32` selects the presentation mode. |
| Show frame rate | PRESENT | `modules/client_options/styles/graphics/graphics.otui:47` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:83` → `apps/client/src/features/local-reference-preferences/performance.rs:126` renders measured FPS. |
| Cap foreground frame rate | PRESENT | `modules/client_options/styles/graphics/graphics.otui:53` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:63` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Enable OTClient adaptive FPS optimization | NOT-APPLICABLE | `modules/client_options/styles/graphics/graphics.otui:77` | OTClient rendering heuristic for frames below its threshold. |
| Force effect culling | NOT-APPLICABLE | `modules/client_options/styles/graphics/graphics.otui:89` | OTClient performance workaround for its effects renderer. |
| Switch texture loader to asynchronous mode | NOT-APPLICABLE | `modules/client_options/styles/graphics/graphics.otui:101` | OTClient asset loader implementation switch. |
| Show creature names | PRESENT | `modules/client_options/styles/interface/HUD.otui:23` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:35` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show health bars | PRESENT | `modules/client_options/styles/interface/HUD.otui:34` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:27` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show mana bar | PRESENT | `modules/client_options/styles/interface/HUD.otui:45` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:28` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show Harmony meter | PRESENT | `modules/client_options/styles/interface/HUD.otui:56` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:94` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Move touch joystick to right panel | MISSING | `modules/client_options/styles/interface/HUD.otui:73` | Searched joystick side and mobile movement panel placement. |
| Choose conditions shown in HUD or status bar | PRESENT | `modules/client_options/styles/interface/HUD.otui:159` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:27` → `apps/client/src/bevy_shell/game_ui/options/conditions.rs:94` |
| Show customizable character status bars | PARTIAL | `modules/client_options/styles/interface/HUD.otui:254` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:49` exposes individual arcs. No master customizable-bar visibility control. |
| Show character status bars | PARTIAL | `modules/client_options/styles/interface/HUD.otui:265` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:27` controls health and mana individually. No shared status-bar toggle. |
| Show selected action bars by side and row | PRESENT | `modules/client_options/styles/interface/actionbars.otui:28` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:233` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show assigned object counts | PRESENT | `modules/client_options/styles/interface/actionbars.otui:156` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:213` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show action tooltips | PRESENT | `modules/client_options/styles/interface/actionbars.otui:204` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:223` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show assigned hotkey on slot | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:144` | `apps/client/src/bevy_shell/game_ui/actions/projection.rs:255` formats the chord. No independent key legend switch in the action bar options. |
| Show spell parameters on buttons | MISSING | `modules/client_options/styles/interface/actionbars.otui:168` | Searched spell parameter visibility in action bar preferences and slot projection. |
| Toggle graphical cooldown overlay | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:180` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:218` has one cooldown preference. Graphic and seconds cannot be configured independently. |
| Toggle numeric cooldown seconds | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:192` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:218` has one shared cooldown preference. |
| Clear assignments by row or side | PARTIAL | `modules/client_options/styles/interface/actionbars.otui:241` | `apps/client/src/bevy_shell/game_ui/options/bar_clear.rs:40` clears a selected row. No single action clears an entire side. |
| Reset every action bar | MISSING | `modules/client_options/styles/interface/actionbars.otui:347` | Searched global reset in action bar settings and clear handlers. |
| Adjust minimum ambient light | PRESENT | `modules/client_options/styles/graphics/effects.otui:17` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:39` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Dim lower floors | PRESENT | `modules/client_options/styles/graphics/effects.otui:27` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:55` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Adjust floor fade duration | MISSING | `modules/client_options/styles/graphics/effects.otui:37` | Searched floor fade timing and floor transition duration preference. |
| Choose floor view mode | MISSING | `modules/client_options/styles/graphics/effects.otui:61` | Searched floor view selector and multifloor rendering modes. |
| Toggle floating effects | MISSING | `modules/client_options/styles/graphics/effects.otui:101` | Searched floating effect render switch. |
| Set missile opacity | MISSING | `modules/client_options/styles/graphics/effects.otui:128` | Searched missile or projectile opacity slider. |
| Set boss area effect opacity | PRESENT | `modules/client_options/styles/graphics/effects.otui:181` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:74` → `apps/client/src/bevy_shell/game_ui/options/commands.rs:21` |
| Toggle fullscreen | PRESENT | `modules/client_options/styles/graphics/graphics.otui:28` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:15` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Preserve integral sprite scaling | PRESENT | `modules/client_options/styles/graphics/graphics.otui:112` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:25` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Select antialiasing mode | MISSING | `modules/client_options/styles/graphics/graphics.otui:13` | Searched renderer filter and AA selectors. `apps/client/src/bevy_shell/ui.rs:432` explicitly reports it unavailable, and `apps/client/src/features/local-reference-preferences/graphics.rs:20` records the build setting. |
| Scale HUD | PRESENT | `modules/client_options/styles/interface/HUD.otui:85` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:12` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Scale creature information separately | MISSING | `modules/client_options/styles/interface/HUD.otui:98` | Searched creature information scale in HUD preferences. |
| Scale static map text | MISSING | `modules/client_options/styles/interface/HUD.otui:112` | Searched static map text size settings. |
| Scale animated map text | MISSING | `modules/client_options/styles/interface/HUD.otui:126` | Searched animated map text size settings. |
| Reorder condition indicators | PRESENT | `modules/client_options/styles/interface/HUD.otui:222` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:27` → `apps/client/src/bevy_shell/game_ui/options/conditions.rs:115` |
| Filter informational messages in console | PARTIAL | `modules/client_options/styles/interface/console.otui:12` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:124` has a shared feedback toggle. No independent informational filter. |
| Filter event messages in console | PARTIAL | `modules/client_options/styles/interface/console.otui:23` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:124` has a shared feedback toggle. No independent event filter. |
| Filter own status messages in console | PARTIAL | `modules/client_options/styles/interface/console.otui:34` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:124` has a shared feedback toggle. No independent own status filter. |
| Filter other players’ status messages in console | PARTIAL | `modules/client_options/styles/interface/console.otui:45` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:124` has a shared feedback toggle. No independent other players’ status filter. |
| Toggle chat timestamps | PARTIAL | `modules/client_options/styles/interface/console.otui:56` | The transcript formats timestamps in `apps/client/src/bevy_shell/game_ui/chat/widgets/transcript.rs:105`. The console options lack a timestamp switch. |
| Show speaker level in chat | PARTIAL | `modules/client_options/styles/interface/console.otui:67` | `apps/client/src/bevy_shell/game_ui/chat/widgets/transcript.rs:47` uses stored speaker metadata. The console options page lacks a levels switch. |
| Show private messages on the map | MISSING | `modules/client_options/styles/interface/console.otui:89` | Searched private speech overlay toggle in console and map settings. |
| Underline highlighted chat text | MISSING | `modules/client_options/styles/interface/console.otui:111` | Searched highlighted text underline setting in console rendering. |
| Show private messages in console | PRESENT | `modules/client_options/styles/interface/console.otui:78` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:102` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show loot messages on screen | PRESENT | `modules/client_options/styles/interface/console.otui:100` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:158` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Set chat scrollback length | PRESENT | `modules/client_options/styles/interface/console.otui:122` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:134` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Highlight hovered map target | MISSING | `modules/client_options/styles/interface/interface.otui:12` | Searched hover target highlight preference in map and cursor settings. |
| Select native cursor | NOT-APPLICABLE | `modules/client_options/styles/interface/interface.otui:24` | OTClient system-versus-software cursor backend switch. |
| Animate cursor | MISSING | `modules/client_options/styles/interface/interface.otui:36` | Searched animated cursor preference. |
| Show item icon while dragging | MISSING | `modules/client_options/styles/interface/interface.otui:47` | Searched drag icon visibility setting in item drag handlers. |
| Show left sidebar | MISSING | `modules/client_options/styles/interface/interface.otui:63` | Searched left sidebar visibility preference. |
| Show extra right sidebar | MISSING | `modules/client_options/styles/interface/interface.otui:77` | Searched extra right sidebar preference. |
| Choose crosshair style | PARTIAL | `modules/client_options/styles/interface/interface.otui:119` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:20` toggles crosshair visibility. No choice of styles. |
| Color loot by value | PARTIAL | `modules/client_options/styles/interface/interface.otui:143` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:169` enables loot highlight without the value-color modes. |
| Show expiry or charges on equipped items | MISSING | `modules/client_options/styles/interface/interface.otui:163` | Searched equipped item expiry overlay switch. |
| Show expiry or charges inside containers | MISSING | `modules/client_options/styles/interface/interface.otui:170` | Searched container item timer overlay switch. |
| Show expiry on unused items | MISSING | `modules/client_options/styles/interface/interface.otui:177` | Searched unused item timer visibility preference. |
| Show cooldown bar | PRESENT | `modules/client_options/styles/interface/interface.otui:88` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:203` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Show info banner | PRESENT | `modules/client_options/styles/interface/interface.otui:100` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:174` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Open OTClient project wiki | NOT-APPLICABLE | `modules/client_options/styles/misc/help.otui:17` | Both help links point to the OTClient GitHub wiki. |
| Clear OTClient settings cache and restart | NOT-APPLICABLE | `modules/client_options/styles/misc/help.otui:37` | Maintenance command for OTClient's settings store. |
| Change language | PRESENT | `modules/client_options/styles/misc/help.otui:49` | `apps/client/src/bevy_shell/game_ui/options/language.rs:9` → `apps/client/src/bevy_shell/game_ui/options/language.rs:39` |
| Set master volume | PRESENT | `modules/client_options/styles/sound/sound.otui:30` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:11` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Adjust music volume | PRESENT | `modules/client_options/styles/sound/sound.otui:49` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:27` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Adjust ambient volume | PRESENT | `modules/client_options/styles/sound/sound.otui:74` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:19` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Adjust item volume | PRESENT | `modules/client_options/styles/sound/sound.otui:95` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:35` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Adjust event volume | PRESENT | `modules/client_options/styles/sound/sound.otui:126` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:43` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Choose sound output device | PRESENT | `modules/client_options/styles/sound/sound.otui:14` | `apps/client/src/bevy_shell/game_ui/options/audio_device.rs:17` → `apps/client/src/bevy_shell/game_ui/options/audio_device.rs:44` |
| Toggle anthem | MISSING | `modules/client_options/styles/sound/sound.otui:62` | Searched anthem switch in music preferences. |
| Toggle eating and drinking sounds | MISSING | `modules/client_options/styles/sound/sound.otui:109` | Searched food and beverage sound filter. |
| Toggle moving item sounds | MISSING | `modules/client_options/styles/sound/sound.otui:114` | Searched item movement sound filter. |
| Filter attack, healing and support spells by caster | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:34` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:85` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Filter weapon sounds by caster | MISSING | `modules/client_options/styles/sound/battleSounds.otui:65` | Searched weapon-specific sound filters for own and other players. |
| Filter creature deaths | MISSING | `modules/client_options/styles/sound/battleSounds.otui:150` | Searched creature death audio switch. |
| Filter creature attack sounds | PARTIAL | `modules/client_options/styles/sound/battleSounds.otui:154` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:115` supplies attack, healing and support filters without a combined attacks-and-spells switch. |
| Adjust interface volume | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:22` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:51` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Toggle interaction sounds | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:42` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:132` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Toggle party membership sounds | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:53` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:137` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Toggle VIP presence sounds | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:57` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:138` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Toggle chat sounds | PRESENT | `modules/client_options/styles/sound/uiSounds.otui:69` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Choose audible notification channels | PARTIAL | `modules/client_options/styles/sound/uiSounds.otui:82` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:139` enables chat sounds globally. Party, guild, local private, private, NPC, global, team finder, raid and system switches are absent. |
| Adjust battle volume by source (self, others, creatures) | PRESENT | `modules/client_options/styles/sound/battleSounds.otui:21` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:61` edits the preference through `apps/client/src/bevy_shell/game_ui/options/commands.rs:21`. |
| Filter creature noises | MISSING | `modules/client_options/styles/sound/battleSounds.otui:144` | Searched creature noise channel switch distinct from attacks and deaths. |
| Adjust all effects opacity | PARTIAL | `modules/client_options/styles/graphics/effects.otui:114` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:105` sets opacity by caster. There is no shared effect alpha slider. |
| Switch OTClient user features profile | NOT-APPLICABLE | `modules/client_options/options.lua:249` | OTClient-specific numbered feature configuration slots. |
| Select active hotkey preset | PRESENT | `modules/client_options/options.lua:283` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:25` → `apps/client/src/bevy_shell/game_ui/options/profiles.rs:107` |
| Toggle FPS and latency indicators together | PRESENT | `modules/client_options/options.lua:478` | `apps/client/src/features/local-reference-preferences/utility_actions.rs:60` → `apps/client/src/features/local-reference-preferences/utility_actions.rs:90` |
| Toggle creature names and bars together | PARTIAL | `modules/client_options/options.lua:487` | `apps/client/src/features/local-reference-preferences/utility_actions.rs:85` toggles creature bars. The shortcut does not switch names with them. |
| Mute all sound | PRESENT | `modules/client_options/options.lua:495` | `apps/client/src/bevy_shell/game_ui/options/metadata/sound.rs:5` → `apps/client/src/features/0023-presentation/input.rs:6` |

## client_bottommenu

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Read featured login hint | PARTIAL | `modules/client_bottommenu/bottommenu.lua:112` | `apps/client/src/bevy_shell/login/bulletin.rs:29` displays operator news. No illustrated random hint presentation. |
| See active and upcoming event summary | PARTIAL | `modules/client_bottommenu/bottommenu.lua:326` | `apps/client/src/bulletin.rs:46` renders a month. No dedicated today and tomorrow summary. |
| Open event calendar | PRESENT | `modules/client_bottommenu/bottommenu.lua:135` | `apps/client/src/bevy_shell/login/bulletin.rs:154` → `apps/client/src/bulletin.rs:46` |
| Browse calendar months | PRESENT | `modules/client_bottommenu/bottommenu.lua:472` | `apps/client/src/bevy_shell/login/bulletin.rs:154` → `apps/client/src/bulletin.rs:49` |
| Identify boosted creature and boss | PARTIAL | `modules/client_bottommenu/bottommenu.lua:540` | `crates/client-ui/src/features/0151-daily-boost/view.rs:93` displays both in play. The admission bulletin does not show them. |

## client_topmenu

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Toggle top menu | MISSING | `modules/client_topmenu/topmenu.lua:117` | Searched top menu visibility action and toolbar toggle binding. |
| Adjust HUD zoom on touch device | PARTIAL | `modules/client_topmenu/topmenu.lua:130` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:12` changes interface scale. No mobile zoom buttons. |
| Read frame rate statistics | PARTIAL | `modules/client_topmenu/topmenu.lua:246` | `apps/client/src/bevy_shell/game_ui/options/metadata/graphics.rs:83` enables frame rate display. The min, midpoint and maximum tooltip is absent. |
| Read connection ping | PARTIAL | `modules/client_topmenu/topmenu.lua:289` | `apps/client/src/features/local-reference-preferences/performance.rs:136` shows command RTT. The low, medium and high lag classifications of the reference are absent. |
| Read online population | MISSING | `modules/client_topmenu/topmenu.lua:348` | Searched online population counters in admission and toolbar UI. |
| Read community streaming statistics | NOT-APPLICABLE | `modules/client_topmenu/topmenu.lua:351` | OTClient server-configured Discord and YouTube promotion counters. |
| Open community stream links | NOT-APPLICABLE | `modules/client_topmenu/topmenu.lua:362` | OTClient server-configured Discord and YouTube destinations. |
| Manage web account | NOT-APPLICABLE | `modules/client_topmenu/topmenu.lua:491` | Account management belongs to Platform. |

## game_actionbar

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Assign or edit spell | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:808` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:8` → `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:130` |
| Assign or edit carried object | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:816` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:12` → `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:100` |
| Assign text to slot | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:826` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:15` → `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:203` |
| Assign passive ability | PARTIAL | `modules/game_actionbar/logics/ActionButtonLogic.lua:829` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:210` stores Instinct, Harmony or Serene. The reference offers Gift of Life. |
| Assign a special client action | MISSING | `modules/game_actionbar/logics/ActionButtonLogic.lua:833` | Searched special client action assignment in slot editor and BarAction variants. |
| Bind key to action slot | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:836` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:17` → `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:153` |
| Build and edit a multi-action slot | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:850` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:54` → `apps/client/src/features/local-spell-action-bar/assignments.rs:246` |
| Clear slot assignment | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:859` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:20` → `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:198` |
| Copy item definition ID | NOT-APPLICABLE | `modules/game_actionbar/logics/ActionButtonLogic.lua:876` | OTClient item-ID debugging convenience. |
| Equip assigned item | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:338` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:100` → `apps/client/src/features/local-spell-action-bar/assignments.rs:405` |
| Toggle an assigned container | MISSING | `modules/game_actionbar/logics/ActionButtonLogic.lua:343` | Searched close-by-item-ID and open-container toggle in slot dispatch. |
| Use assigned object | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:345` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:123` → `apps/client/src/features/local-spell-action-bar/item_actions.rs:25` |
| Use object on self, target or cursor | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:350` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:120` → `apps/client/src/features/local-spell-action-bar/item_actions.rs:25` |
| Send or compose assigned text | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:378` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:203` → `apps/client/src/features/local-spell-action-bar/assignments.rs:421` |
| Monitor slot readiness and remaining cooldown | PRESENT | `modules/game_actionbar/game_actionbar.lua:454` | `apps/client/src/bevy_shell/game_ui/actions/projection.rs:71` → `apps/client/src/features/local-spell-action-bar/view.rs:224` |
| Cast assigned spell | PRESENT | `modules/game_actionbar/game_actionbar.lua:613` | `apps/client/src/bevy_shell/game_ui/actions.rs:445` activates the slot. `apps/client/src/features/local-spell-action-bar/assignments.rs:355` resolves the live catalogue entry and issues the cast. |
| Scroll through action buttons | PARTIAL | `modules/game_actionbar/otui/actionbar.otui:112` | `apps/client/src/bevy_shell/game_ui/actions.rs:449` → `apps/client/src/features/local-spell-action-bar/layout.rs:218` moves one slot per press. No jump to first or last visible page. |
| Lock action bars against edits | PARTIAL | `modules/game_actionbar/logics/ActionBarLayout.lua:281` | `apps/client/src/bevy_shell/game_ui/actions/menu.rs:177` locks the chosen row. The reference locks all three bars on a side at once. |
| Rearrange slots by dragging | PRESENT | `modules/game_actionbar/logics/ActionButtonLogic.lua:1361` | `apps/client/src/bevy_shell/game_ui/actions/dragdrop.rs:55` → `apps/client/src/features/local-spell-action-bar/layout.rs:224` |
| Find a spell by name for assignment | PRESENT | `modules/game_actionbar/logics/ActionAssignmentWindows.lua:8` | `apps/client/src/bevy_shell/game_ui/actions/editor/catalog.rs:24` → `apps/client/src/bevy_shell/game_ui/actions/editor/catalog.rs:56` |
| Supply a spell parameter | PARTIAL | `modules/game_actionbar/logics/ActionAssignmentWindows.lua:155` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:203` allows manually composing words as text. No parameter editor attached to a selected spell. |
| Execute first ready action in a multi-action slot | PRESENT | `modules/game_actionbar/logics/MultiActionLogic.lua:139` | `apps/client/src/bevy_shell/game_ui/actions.rs:445` → `apps/client/src/features/local-spell-action-bar/assignments.rs:339` selects the first ready entry. |
| Bind an exact item upgrade tier | PRESENT | `modules/game_actionbar/logics/ActionAssignmentWindows.lua:529` | `apps/client/src/bevy_shell/game_ui/actions/editor/assign.rs:229` → `apps/client/src/features/local-spell-action-bar/assignments.rs:405` |

## game_hotkeys

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Browse and edit current bindings | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:21` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:112` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:56` |
| Reset all custom hotkeys | MISSING | `modules/game_hotkeys/hotkeys_manager.otui:46` | Searched bulk reset in custom hotkey controls. |
| Capture and add a shortcut | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:54` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:312` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:374` |
| Remove a shortcut | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:63` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:363` → `apps/client/src/bevy_shell/game_ui/hotkeys.rs:366` |
| Compose predefined chat text | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:91` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:40` → `apps/client/src/features/local-reference-preferences/custom_dispatch.rs:192` |
| Send predefined text immediately | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:109` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:362` → `apps/client/src/features/local-reference-preferences/custom_dispatch.rs:192` |
| Choose an object for a shortcut | PARTIAL | `modules/game_hotkeys/hotkeys_manager.otui:126` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:329` accepts a carried object. No map pick or artwork in the selector. |
| Clear object while retaining shortcut | MISSING | `modules/game_hotkeys/hotkeys_manager.otui:136` | Searched clear object action in custom hotkey editor. |
| Use hotkey item on self | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:147` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:49` → `apps/client/src/features/local-reference-preferences/custom_dispatch.rs:309` |
| Use hotkey item on target | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:158` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:50` → `apps/client/src/features/local-reference-preferences/custom_dispatch.rs:284` |
| Choose item target with crosshair | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:169` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:51` → `apps/client/src/features/local-reference-preferences/custom_dispatch.rs:297` |
| Use item immediately at pointer | MISSING | `modules/game_hotkeys/hotkeys_manager.otui:180` | Searched immediate cursor-position mode in custom hotkey editor and custom dispatch. |
| Save hotkey edits | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:198` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:374` → `apps/client/src/bevy_shell/owner/options_draft.rs:50` |
| Discard hotkey edits | PRESENT | `modules/game_hotkeys/hotkeys_manager.otui:207` | `apps/client/src/bevy_shell/game_ui/options/widgets.rs:193` → `apps/client/src/bevy_shell/owner/options_draft.rs:45` |
| Use bound item directly | PRESENT | `modules/game_hotkeys/hotkeys_manager.lua:598` | `apps/client/src/bevy_shell/game_ui/hotkeys.rs:52` → `apps/client/src/features/local-reference-preferences/custom_dispatch.rs:296` |
| Switch WASD/chat mode by shortcut | PARTIAL | `modules/game_hotkeys/hotkeys_manager.lua:523` | `apps/client/src/bevy_shell/game_ui/options/profiles.rs:65` and `apps/client/src/features/local-reference-preferences/chat_context.rs:54` switch chat context. The custom editor lacks the reference toggle action. |
| Select next or previous battle target by shortcut | MISSING | `modules/game_hotkeys/hotkeys_manager.lua:526` | Searched attack-next, attack-previous and target cycling actions in custom dispatch and battle input. |
| Toggle chasing by shortcut | PARTIAL | `modules/game_hotkeys/hotkeys_manager.lua:529` | `apps/client/src/bevy_shell/game_ui/hud.rs:199` sends chase mode from the HUD. No chase toggle in the custom key action selector. |

## game_shortcuts

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Select or clear touch targeting mode (attack, look, follow or use) | NOT-APPLICABLE | `modules/game_shortcuts/shortcuts.lua:48` | OTClient mobile-only modifier palette. The official desktop client has no mobile touch palette. |

## game_cooldown

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Track individual spell cooldown | PRESENT | `modules/game_cooldown/cooldown.lua:238` | `apps/client/src/bevy_shell/game_ui/actions/cooldown.rs:30` → `apps/client/src/features/local-spell-action-bar/view.rs:224` |
| Track spell group cooldown | PRESENT | `modules/game_cooldown/cooldown.lua:273` | `apps/client/src/bevy_shell/game_ui/actions/cooldown.rs:106` → `apps/client/src/features/local-spell-action-bar/view.rs:229` |

## game_mainpanel

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Open store from control panel | PRESENT | `modules/game_mainpanel/mainpanel.lua:183` | `apps/client/src/bevy_shell/ui/toolbar/shortcuts.rs:84` → `apps/client/src/features/0154-store/input.rs:6` |
| Collapse or expand control panel | MISSING | `modules/game_mainpanel/mainpanel.lua:244` | Searched toolbar shrink and compact mode in toolbar manager. |
| Choose visible control buttons | PRESENT | `modules/game_mainpanel/mainpanel.lua:427` | `apps/client/src/bevy_shell/ui/toolbar/manager.rs:93` → `apps/client/src/bevy_shell/ui/toolbar/manager.rs:63` |
| Reorder control buttons | PRESENT | `modules/game_mainpanel/mainpanel.lua:475` | `apps/client/src/bevy_shell/ui/toolbar/manager.rs:93` → `apps/client/src/bevy_shell/ui/toolbar/manager.rs:70` |
| Restore default controls | PRESENT | `modules/game_mainpanel/mainpanel.lua:551` | `apps/client/src/bevy_shell/ui/toolbar/manager.rs:93` → `apps/client/src/bevy_shell/ui/toolbar/manager.rs:85` |
| Open control-button manager | PRESENT | `modules/game_mainpanel/mainpanel.lua:235` | `apps/client/src/bevy_shell/ui/toolbar/grid.rs:262` → `apps/client/src/bevy_shell/ui/toolbar/manager.rs:93` |

## Gaps ranked by player impact

Combat execution and movement rank first, followed by information that changes a player’s decisions, configuration tasks, and presentation preferences. Within each group, the table identifies the exact missing behavior.

1. Choose loot gesture — PARTIAL (`client_options`).
2. Choose mouse interaction scheme — PARTIAL (`client_options`).
3. Combine direction keys for diagonal walking — MISSING (`client_options`).
4. Enable right-click NPC talk — MISSING (`client_options`).
5. Set walking delay after changing floors — MISSING (`client_options`).
6. Set walking delay after teleporting — MISSING (`client_options`).
7. Set walking delay after turning — MISSING (`client_options`).
8. Assign a special client action — MISSING (`game_actionbar`).
9. Assign passive ability — PARTIAL (`game_actionbar`).
10. Lock action bars against edits — PARTIAL (`game_actionbar`).
11. Scroll through action buttons — PARTIAL (`game_actionbar`).
12. Supply a spell parameter — PARTIAL (`game_actionbar`).
13. Toggle an assigned container — MISSING (`game_actionbar`).
14. Choose an object for a shortcut — PARTIAL (`game_hotkeys`).
15. Clear object while retaining shortcut — MISSING (`game_hotkeys`).
16. Reset all custom hotkeys — MISSING (`game_hotkeys`).
17. Select next or previous battle target by shortcut — MISSING (`game_hotkeys`).
18. Switch WASD/chat mode by shortcut — PARTIAL (`game_hotkeys`).
19. Toggle chasing by shortcut — PARTIAL (`game_hotkeys`).
20. Use item immediately at pointer — MISSING (`game_hotkeys`).
21. Identify boosted creature and boss — PARTIAL (`client_bottommenu`).
22. See active and upcoming event summary — PARTIAL (`client_bottommenu`).
23. Filter event messages in console — PARTIAL (`client_options`).
24. Filter informational messages in console — PARTIAL (`client_options`).
25. Filter other players’ status messages in console — PARTIAL (`client_options`).
26. Filter own status messages in console — PARTIAL (`client_options`).
27. Show character status bars — PARTIAL (`client_options`).
28. Show customizable character status bars — PARTIAL (`client_options`).
29. Show expiry on unused items — MISSING (`client_options`).
30. Show expiry or charges inside containers — MISSING (`client_options`).
31. Show expiry or charges on equipped items — MISSING (`client_options`).
32. Show private messages on the map — MISSING (`client_options`).
33. Show speaker level in chat — PARTIAL (`client_options`).
34. Toggle graphical cooldown overlay — PARTIAL (`client_options`).
35. Toggle numeric cooldown seconds — PARTIAL (`client_options`).
36. Toggle on-screen text messages — PARTIAL (`client_options`).
37. Read connection ping — PARTIAL (`client_topmenu`).
38. Automatically switch presets for a character — PARTIAL (`client_options`).
39. Clear a shortcut — PARTIAL (`client_options`).
40. Maximize newly opened containers — MISSING (`client_options`).
41. Reset every action bar — MISSING (`client_options`).
42. Search bindings by text — MISSING (`client_options`).
43. Show extra right sidebar — MISSING (`client_options`).
44. Show left sidebar — MISSING (`client_options`).
45. Show spell parameters on buttons — MISSING (`client_options`).
46. Collapse or expand control panel — MISSING (`game_mainpanel`).
47. Read featured login hint — PARTIAL (`client_bottommenu`).
48. Adjust all effects opacity — PARTIAL (`client_options`).
49. Adjust floor fade duration — MISSING (`client_options`).
50. Animate cursor — MISSING (`client_options`).
51. Choose audible notification channels — PARTIAL (`client_options`).
52. Choose crosshair style — PARTIAL (`client_options`).
53. Choose floor view mode — MISSING (`client_options`).
54. Clear assignments by row or side — PARTIAL (`client_options`).
55. Color loot by value — PARTIAL (`client_options`).
56. Filter creature attack sounds — PARTIAL (`client_options`).
57. Filter creature deaths — MISSING (`client_options`).
58. Filter creature noises — MISSING (`client_options`).
59. Filter weapon sounds by caster — MISSING (`client_options`).
60. Highlight hovered map target — MISSING (`client_options`).
61. Move touch joystick to right panel — MISSING (`client_options`).
62. Move whole stacks directly — MISSING (`client_options`).
63. Restore default key assignments — PARTIAL (`client_options`).
64. Scale animated map text — MISSING (`client_options`).
65. Scale creature information separately — MISSING (`client_options`).
66. Scale static map text — MISSING (`client_options`).
67. Select antialiasing mode — MISSING (`client_options`).
68. Set hotkey repeat delay — PARTIAL (`client_options`).
69. Set missile opacity — MISSING (`client_options`).
70. Show assigned hotkey on slot — PARTIAL (`client_options`).
71. Show item icon while dragging — MISSING (`client_options`).
72. Toggle anthem — MISSING (`client_options`).
73. Toggle chat mode with Enter — PARTIAL (`client_options`).
74. Toggle chat timestamps — PARTIAL (`client_options`).
75. Toggle creature names and bars together — PARTIAL (`client_options`).
76. Toggle eating and drinking sounds — MISSING (`client_options`).
77. Toggle floating effects — MISSING (`client_options`).
78. Toggle moving item sounds — MISSING (`client_options`).
79. Underline highlighted chat text — MISSING (`client_options`).
80. Adjust HUD zoom on touch device — PARTIAL (`client_topmenu`).
81. Read frame rate statistics — PARTIAL (`client_topmenu`).
82. Read online population — MISSING (`client_topmenu`).
83. Toggle top menu — MISSING (`client_topmenu`).
