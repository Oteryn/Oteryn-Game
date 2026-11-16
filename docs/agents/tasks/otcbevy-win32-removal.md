# OTCBEVY — drop the native Win32 client, ship only the Bevy client

```yaml
task_id: otcbevy-win32-removal
mode: IMPLEMENT
status: authoring
pr: null
branch: claude/happy-davinci-ijz3wy-otcbevy
repository: Oteryn/Oteryn-Game
owner_decision: 2026-11-16 drop the Win32 client; Windows CI builds/tests the Bevy exe (workflow change consented)
```

## Summary

`oteryn-client.exe` on Windows becomes the Bevy shell. Every player-visible window
that only the Win32 shell presented is ported to the Bevy shell first; then the Win32
UI (`windows_shell*`, `miniwindow*`, `window_layout`, the per-feature Win32 windows
and their `cfg(windows)` blocks) is removed. The client-ui view models and the
`PlayLink` / `PlayView` logic stay, because Bevy reads them.

Authoring container constraint: no `cargo build/check/test/clippy` was run here (2 GB
disk). The code is written against the signatures it calls; compilation and tests run
in the follow-up pass on the PC. Every test below is **written, not run**.

## Inventory method

- Win32 side: every `fn open_window` (and the shared Win32 `Form` / status-window
  hosts it calls) under `apps/client/src/features/**`, plus the non-feature Win32 UI
  files that match `windows_sys::Win32::UI|CreateWindowExW|SendMessageW|GetDlgItem`.
- Bevy side: a non-test file under `apps/client/src/bevy_shell/**` that calls the
  feature's own `*_panel()` accessor (feature-prefixed name only). The line shown is
  the first such call. The 54 `*_input` routes the Win32 shell calls and Bevy does
  not are no evidence by themselves: Bevy opens most windows through its own controls.
- Scope: Tibia Global parity (`docs/architecture/reviews/TIBIA-CLIENT-PARITY-2026-11-08.md`).
  OTClient-only extras stay out.

## Feature windows (Win32 opener -> Bevy presenter)

| feature | Win32 window (opener) | Bevy presenter |
|---|---|---|
| 0003-npc-service | apps/client/src/features/0003-npc-service/native.rs:73 | `apps/client/src/bevy_shell/game_ui/npc_shop/conversation.rs:56` |
| 0004-item-view-move | apps/client/src/features/0004-item-view-move/native.rs:36 | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:426` |
| 0007-chat | apps/client/src/features/0007-chat/native.rs:51 | `apps/client/src/bevy_shell/game_ui/chat.rs:269` |
| 0008-achievement-notices | apps/client/src/features/0008-achievement-notices/native.rs:20 | MISSING -> ported (form) |
| 0009-highscores | apps/client/src/features/0009-highscores/native.rs:47 | `apps/client/src/bevy_shell/game_ui/highscores.rs:95` |
| 0010-analyser | apps/client/src/features/0010-analyser/view.rs:32 | `apps/client/src/bevy_shell/game_ui/analyser.rs:41` |
| 0014-container-tree | apps/client/src/features/0014-container-tree/native.rs:44 | `apps/client/src/bevy_shell/controls/container.rs:25` |
| 0016-quest-log | apps/client/src/features/0016-quest-log/native.rs:77 | `apps/client/src/bevy_shell/game_ui/journal/submission.rs:52` |
| 0017-attack | apps/client/src/features/0017-attack/native.rs:71 | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:177` |
| 0025-appearance | apps/client/src/features/0025-appearance/view.rs:8 | `apps/client/src/bevy_shell/game_ui/outfit/animation.rs:81` |
| 0031-tutorial-notices | apps/client/src/features/0031-tutorial-notices/native.rs:17 | `apps/client/src/bevy_shell/game_ui/tutorial/vocation.rs:53` |
| 0037-depot | apps/client/src/features/0037-depot/native.rs:38 | `apps/client/src/bevy_shell/game_ui/inventory.rs:269` |
| 0039-timed-item | apps/client/src/features/0039-timed-item/view.rs:8 | MISSING -> ported (form) |
| 0040-guild | apps/client/src/features/0040-guild/native.rs:29 | MISSING -> ported (form) |
| 0042-party | apps/client/src/features/0042-party/native.rs:71 | `apps/client/src/bevy_shell/game_ui/party.rs:59` |
| 0043-vip | apps/client/src/features/0043-vip/native.rs:38 | `apps/client/src/bevy_shell/game_ui/hud/vip.rs:155` |
| 0049-house | apps/client/src/features/0049-house/view.rs:11 | `apps/client/src/bevy_shell/game_ui/houses.rs:100` |
| 0051-staff | apps/client/src/features/0051-staff/view.rs:8 | MISSING -> ported (form, b6305c1c41) |
| 0053-world-event | apps/client/src/features/0053-world-event/native.rs:29 | MISSING -> ported (form) |
| 0054-training | apps/client/src/features/0054-training/view.rs:8 | MISSING -> ported (form) |
| 0055-guild-bank | apps/client/src/features/0055-guild-bank/view.rs:8 | MISSING -> ported (form) |
| 0057-cyclopedia-map | apps/client/src/features/0057-cyclopedia-map/view.rs:33, apps/client/src/features/0057-cyclopedia-map/view/atlas.rs:15 | `apps/client/src/bevy_shell/game_ui/cyclopedia_map.rs:105` |
| 0059-house-move-out | apps/client/src/features/0059-house-move-out/view.rs:8 | MISSING -> ported (form) |
| 0060-player-trade | apps/client/src/features/0060-player-trade/native.rs:50 | `apps/client/src/bevy_shell/game_ui/trade.rs:77` |
| 0062-stash | apps/client/src/features/0062-stash/native.rs:45 | `apps/client/src/bevy_shell/game_ui/stash.rs:73` |
| 0063-market | apps/client/src/features/0063-market/native.rs:62 | `apps/client/src/bevy_shell/game_ui/market/purchase.rs:96` |
| 0067-bestiary-charm-progression | apps/client/src/features/0067-bestiary-charm-progression/view.rs:14 | `apps/client/src/bevy_shell/game_ui/bestiary.rs:54` |
| 0068-imbue | apps/client/src/features/0068-imbue/native.rs:52 | `apps/client/src/bevy_shell/game_ui/imbuement_tracker.rs:53` |
| 0072-task-board | apps/client/src/features/0072-task-board/native.rs:51 | `apps/client/src/bevy_shell/game_ui/task_board.rs:150` |
| 0076-player-report | apps/client/src/features/0076-player-report/view.rs:8, apps/client/src/features/0076-player-report/bug.rs:152 | `apps/client/src/bevy_shell/game_ui/rule_violation.rs:34` |
| 0081-npc-shop | apps/client/src/features/0081-npc-shop/native.rs:41 | `apps/client/src/bevy_shell/game_ui/npc_shop.rs:79` |
| 0086-house-transfer | apps/client/src/features/0086-house-transfer/view.rs:8 | MISSING -> ported (form) |
| 0087-pvp-expert | apps/client/src/features/0087-pvp-expert/view.rs:8 | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:178` |
| 0097-character-info | apps/client/src/features/0097-character-info/view.rs:8 | MISSING -> ported (form) |
| 0098-quick-loot | apps/client/src/features/0098-quick-loot/native.rs:54 | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:484` |
| 0099-guildhall | apps/client/src/features/0099-guildhall/view.rs:8 | MISSING -> ported (form) |
| 0103-guild-war | apps/client/src/features/0103-guild-war/view.rs:8 | MISSING -> ported (form) |
| 0115-mail | apps/client/src/features/0115-mail/view.rs:8 | MISSING -> ported (form) |
| 0117-boss-encounter | apps/client/src/features/0117-boss-encounter/native.rs:3 | `apps/client/src/bevy_shell/game_ui/bosstiary.rs:35` |
| 0132-inspection | apps/client/src/features/0132-inspection/view.rs:49 | `apps/client/src/bevy_shell/game_ui/cyclopedia_items.rs:93` |
| 0134-party-skull | apps/client/src/features/0134-party-skull/view.rs:8 | `apps/client/src/bevy_shell/game_ui/unjustified_points.rs:44` |
| 0140-prey | apps/client/src/features/0140-prey/native.rs:50 | `apps/client/src/bevy_shell/game_ui/prey.rs:171` |
| 0146-container-sort | apps/client/src/features/0146-container-sort/view.rs:8 | `apps/client/src/bevy_shell/game_ui/inventory.rs:272` |
| 0147-exiva-settings | apps/client/src/features/0147-exiva-settings/native.rs:34 | MISSING -> ported (form) |
| 0148-wheel-allocation | apps/client/src/features/0148-wheel-allocation/presets.rs:135, apps/client/src/features/0148-wheel-allocation/native.rs:54 | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:92` |
| 0149-raid-schedule | apps/client/src/features/0149-raid-schedule/view.rs:8 | MISSING -> ported (form) |
| 0150-house-podium | apps/client/src/features/0150-house-podium/view.rs:17 | MISSING -> ported (form) |
| 0151-daily-boost | apps/client/src/features/0151-daily-boost/native.rs:35 | MISSING -> ported (form) |
| 0152-item-forge | apps/client/src/features/0152-item-forge/window.rs:53 | `apps/client/src/bevy_shell/game_ui/forge.rs:96` |
| 0153-depot-search | apps/client/src/features/0153-depot-search/native_windows.rs:26, apps/client/src/features/0153-depot-search/native/windows.rs:28 | `apps/client/src/bevy_shell/game_ui/inventory.rs:270` |
| 0154-store | apps/client/src/features/0154-store/native/browser.rs:38, apps/client/src/features/0154-store/native/classic.rs:57 | `apps/client/src/bevy_shell/game_ui/store.rs:72` |
| 0165-hireling | apps/client/src/features/0165-hireling/native.rs:17 | MISSING -> ported (form) |
| 0168-animus-mastery | apps/client/src/features/0168-animus-mastery/native.rs:39 | MISSING -> ported (form) |
| 0170-arena | apps/client/src/features/0170-arena/native.rs:51 | MISSING -> ported (form) |
| 0179-hotkey-equip | apps/client/src/features/0179-hotkey-equip/native.rs:78 | MISSING -> ported (form) |
| 0182-daily-reward | apps/client/src/features/0182-daily-reward/native.rs:42 | `apps/client/src/bevy_shell/game_ui/reward_wall.rs:83` |
| 0195-weapon-proficiency | apps/client/src/features/0195-weapon-proficiency/native_window.rs:41, apps/client/src/features/0195-weapon-proficiency/native_localized_1.rs:39, apps/client/src/features/0195-weapon-proficiency/native/window_host.rs:41 | `apps/client/src/bevy_shell/game_ui/proficiency.rs:71` |
| 0197-cyclopedia-tracker | apps/client/src/features/0197-cyclopedia-tracker/native.rs:19 | MISSING -> ported (form) |
| 0199-cyclopedia-character | apps/client/src/features/0199-cyclopedia-character/view/character_window.rs:38, apps/client/src/features/0199-cyclopedia-character/view/sanctions.rs:23, apps/client/src/features/0199-cyclopedia-character/view/blessings.rs:22 | `apps/client/src/bevy_shell/game_ui/hud/blessings.rs:167` |
| 0207-house-furniture-wrap | apps/client/src/features/0207-house-furniture-wrap/view.rs:8 | MISSING -> ported (form) |
| 0224-boss-rewards | apps/client/src/features/0224-boss-rewards/native.rs:65 | MISSING -> ported (form) |
| 0225-boss-difficulty | apps/client/src/features/0225-boss-difficulty/native.rs:115 | MISSING -> ported (form) |
| 0230-browse-field | apps/client/src/features/0230-browse-field/native_platform.rs:39, apps/client/src/features/0230-browse-field/native/platform.rs:41 | `apps/client/src/bevy_shell/game_ui/inventory.rs:268` |
| 0233-player-typing | apps/client/src/features/0233-player-typing/native.rs:34 | `apps/client/src/bevy_shell/scene/typing.rs:41` |
| 0240-chat-party-room | apps/client/src/features/0240-chat-party-room/native.rs:32 | MISSING -> ported (form) |
| 0250-hireling-management | apps/client/src/features/0250-hireling-management/native.rs:33 | MISSING -> ported (form) |
| 0258-vip-presence | apps/client/src/features/0258-vip-presence/native.rs:17 | `apps/client/src/bevy_shell/game_ui/hud/vip/presentation.rs:213` |
| 0259-chat-public-channels | apps/client/src/features/0259-chat-public-channels/native.rs:56 | MISSING -> ported (form) |
| 0274-loyalty-skills | apps/client/src/features/0274-loyalty-skills/native.rs:82 | `apps/client/src/bevy_shell/game_ui/hud.rs:338` |
| 0287-actor-spell-catalog | apps/client/src/features/0287-actor-spell-catalog/book_native.rs:74, apps/client/src/features/0287-actor-spell-catalog/view.rs:78 | `apps/client/src/bevy_shell/game_ui/actions/editor/catalog.rs:51` |
| 0290-cast-control | apps/client/src/features/0290-cast-control/view.rs:8 | MISSING -> ported (form, b6305c1c41) |
| 0291-cast-chat | apps/client/src/features/0291-cast-chat/view.rs:8 | MISSING -> ported (form, b6305c1c41) |
| 0445-house-furniture-rotation | apps/client/src/features/0445-house-furniture-rotation/view.rs:11 | MISSING -> ported (form) |
| local-account-achievements | apps/client/src/features/local-account-achievements/native.rs:50 | MISSING -> ported (form) |
| local-spell-action-bar | apps/client/src/features/local-spell-action-bar/view.rs:100 | `apps/client/src/bevy_shell/game_ui/actions/dragdrop.rs:36` |

### Sub-windows of features that Bevy otherwise covers

| window | Win32 file:line | Bevy |
|---|---|---|
| Coin market (Ctrl+Alt+F8) | apps/client/src/features/0063-market/input/coins.rs:14 | MISSING -> ported (form) |
| Boss cooldown tracker | apps/client/src/features/0117-boss-encounter/native.rs:179 | MISSING -> ported (form) |
| Welcome-back reward | apps/client/src/features/0182-daily-reward/returner_native.rs:4 | MISSING -> ported (form) |
| Outfit Memorial | apps/client/src/features/0025-appearance/view/memorial.rs:32 | MISSING -> ported (form) |
| Rule violation record | apps/client/src/features/0199-cyclopedia-character/view/sanctions.rs:23 | MISSING -> ported (form) |
| In-game Premium overview | apps/client/src/features/local-premium/game_native.rs:3 | MISSING in game (pre-login: `apps/client/src/bevy_shell/account_services.rs:77`) -> ported (form) |
| Account Friends (Ctrl+Alt+F) | apps/client/src/features/0043-vip/account.rs:11 | MISSING -> ported (form) |
| Private chat channels (create/invite/exclude/leave) | apps/client/src/features/0080-chat-private-channel/native.rs:20 (controls inside the Win32 chat window) | MISSING -> ported (form) |
| Party effects status (Ctrl+Alt+Shift+P) | apps/client/src/play/status_window.rs:137 via apps/client/src/features/0280-party-effects/protocol.rs:29 | MISSING -> ported (form) |
| Potion durations status | apps/client/src/play/status_window.rs:137 via apps/client/src/features/0381-potion-durations/protocol.rs:30 | MISSING -> ported (form) |
| Condition / rest / battle instinct / skill progress / mechanics status | apps/client/src/play/status_window.rs:137 | `apps/client/src/bevy_shell/game_ui/hud/status_icons.rs:60`, `apps/client/src/bevy_shell/game_ui/hud/skills.rs` |
| Wheel builds (presets) | apps/client/src/features/0148-wheel-allocation/presets.rs:135 | `apps/client/src/bevy_shell/game_ui/wheel/presets.rs` |
| In-game coin gift | apps/client/src/features/0154-store/view.rs:40 | `apps/client/src/bevy_shell/game_ui/store.rs:72` (`coin_gift_control`) |

### Non-feature Win32 UI

| piece | Win32 file | Bevy |
|---|---|---|
| Game window, HUD, input routing, renderer host | apps/client/src/windows_shell.rs, windows_shell_methods_1.rs, windows_shell/{application_handler,input,graphics}.rs | `apps/client/src/bevy_shell/mod.rs`, `controls.rs`, `scene.rs`, `game_ui/hud.rs` |
| Login window | apps/client/src/features/local-native-login/shell.rs | `apps/client/src/bevy_shell/login.rs` |
| Character selection | apps/client/src/features/local-character-selection/native.rs | `apps/client/src/bevy_shell/selection.rs` |
| Settings / options | apps/client/src/features/local-reference-preferences/native*.rs | `apps/client/src/bevy_shell/game_ui/options.rs` |
| Spell / action bar HUD | apps/client/src/features/local-spell-action-bar/native.rs | `apps/client/src/bevy_shell/game_ui/actions.rs` |
| Screenshot capture | apps/client/src/windows_shell/capture.rs | `apps/client/src/bevy_shell/screenshots.rs` |
| Miniwindow docking and saved window bounds | apps/client/src/miniwindow.rs, miniwindow/*.rs, window_layout.rs | `apps/client/src/bevy_shell/game_ui/layout.rs`, `window_drag.rs` |
| Game notifications popup | apps/client/src/features/local-game-notifications/state.rs | `apps/client/src/bevy_shell/game_ui/feature_dialogs/notices.rs` |
| Minimap window | apps/client/src/features/local-minimap/native*.rs | `apps/client/src/bevy_shell/game_ui/minimap.rs`, `satellite_map.rs` |
| Cast watch HUD (`--watch`) | apps/client/src/cast_watch/native_hud.rs | KEPT: `--watch` still runs the native winit/Win32 spectator (owner question) |

## Non-UI Windows platform pieces that stay

| piece | today | after the change |
|---|---|---|
| Single-instance mutex `Global\OterynClient-<SID>`, install transaction mutex, `show_error` | apps/client/src/win_mutex.rs (`#[cfg(windows)]`, the only `unsafe` allow) | unchanged; called from `apps/client/src/main.rs` before the Bevy shell starts |
| Windows Credential Manager vault | apps/client/src/device_login/vault_windows.rs via vault.rs | unchanged |
| Device login confirmation (MessageBox) | apps/client/src/device_login/confirmation.rs | unchanged (platform dialog, not game UI) |
| Updater: maintenance mutex handshake and `--updates` maintenance dialog | apps/client/src/updater.rs, updater/window.rs | unchanged; the Bevy shell's in-game updater stays at `apps/client/src/bevy_shell/updater.rs` |
| Package integrity repair prompt | apps/client/src/package_integrity.rs:224 | unchanged; called from `main.rs` |
| Crash report consent / survey (MessageBox) | apps/client/src/crash_report.rs:146, apps/client/src/about.rs:56 | unchanged |
| Windows 8+ / Common Controls 6 manifest | apps/client/resources/windows.manifest, embedded by apps/client/build.rs | unchanged (every bin, so `oteryn-client.exe` keeps it) |
| Audio | rodio (Windows dependency), `AudioOutput` in features/0023-presentation/output.rs | Bevy path `apps/client/src/bevy_shell/audio.rs` over the same `AudioOutput`; the Win32-only `NativeAudio` host goes with the Win32 shell |

## Cross-check against the independent inventory

An independent inventory against `origin/main` 641a191c92 listed 173 player-visible rows:
78 present in Bevy and 95 MISSING. `apps/client/src` is the same at that commit and at
this branch's base. Status of the 95 MISSING rows on this branch:

- P: ported on this branch (42)
- S: Bevy already presented it; the row was stale (24)
- B: pre-game account screens moved to a separate lane (4)
- K: kept as a Windows-native path (2)
- N: not ported (23): by design 16, 138, 152; in scope but not ported 12, 35, 50, 52,
  73, 119, 140, 141, 144, 147, 149, 150, 155, 158, 162-166, 173

| row | status | where / why |
|---|---|---|
| 5 | P | 069a9f4959 picker Mark/Train marked |
| 6 | B | lane (b) |
| 7 | B | lane (b) |
| 8 | B | lane (b) |
| 9 | B | entry overview lane (b); in-game premium form ported 0593fe242b |
| 11 | S | bevy_shell/login/bulletin.rs (News) |
| 12 | N | toggles not in Bevy options; Sign out device turns remember-device off |
| 13 | P | 2e80440985, 70a75a0d36 login Sign out device (configured Platform sign-out) |
| 14 | P | 70a75a0d36 login Support and Cookies links |
| 16 | N | no pre-login offline world in Bevy (scene.rs:1, by design) |
| 19 | S | bevy_shell/controls.rs:287 |
| 22 | S | bevy_shell/controls.rs:310 (shortcut click -> modifier_click) |
| 25 | P | f7eec7087c NPC Talk/Trade context entries |
| 29 | S | game_ui/inventory.rs:272 |
| 30 | S | game_ui/depot.rs |
| 31 | S | game_ui/inventory.rs:270 |
| 32 | S | game_ui/inventory.rs:268, context Browse Field |
| 33 | P | form 0593fe242b |
| 35 | N | battle-list editor (Bevy selects saved lists only) |
| 38 | P | coin market form 0593fe242b |
| 40 | S | gameplay_dialogs.rs Trade with |
| 50 | N | Bevy uses its fixed console font |
| 51 | S | chat message menu Save messages (appends chat.log; no path/overwrite choice) |
| 52 | N | channel mirror |
| 53 | P | f7eec7087c Communication lists form |
| 58 | P | form |
| 59 | P | form |
| 60 | P | form |
| 61 | P | form |
| 62 | S | game_ui/hud/battle.rs (pvp_expert_edit) |
| 63 | S | game_ui/unjustified_points.rs |
| 65 | P | form |
| 73 | N | multiaction editor |
| 75 | P | form |
| 80 | P | form |
| 81 | P | form |
| 83 | P | form |
| 85 | P | form |
| 89 | S | game_ui/wheel/atelier.rs |
| 91 | S | game_ui/forge/widgets.rs (resource_dialog) |
| 95 | P | form |
| 96 | P | form |
| 97 | P | form |
| 100 | P | form |
| 101 | S | game_ui/cyclopedia_map.rs |
| 102 | S | game_ui/cyclopedia_map.rs |
| 103 | S | game_ui/cyclopedia_items.rs |
| 104 | S | game_ui/bestiary.rs |
| 105 | S | game_ui/bestiary.rs |
| 106 | S | game_ui/bosstiary.rs; cooldown form ported |
| 107 | P | form |
| 108 | P | form |
| 109 | P | form |
| 110 | P | form |
| 111 | P | form |
| 112 | P | form |
| 113 | P | form |
| 114 | P | form |
| 115 | P | form |
| 116 | P | form |
| 117 | S | game_ui/houses.rs (ownership and access lists) |
| 119 | N | house map window |
| 120 | P | form |
| 121 | P | form |
| 122 | P | form |
| 123 | P | form |
| 124 | P | form |
| 125 | P | form |
| 126 | P | form b6305c1c41 |
| 135 | P | form b6305c1c41 |
| 136 | P | form b6305c1c41 |
| 137 | K | --watch keeps cast_watch/native.rs (owner question) |
| 138 | N | touch controls |
| 140 | N | window geometry persistence |
| 141 | N | editable HUD-pane layout |
| 143 | S | game_ui/minimap/markers.rs; create-mark key wired 70a75a0d36 |
| 144 | N | minimap import/export (backend kept, no Bevy caller) |
| 147 | N | options parity |
| 149 | N | options parity |
| 150 | N | options parity |
| 152 | N | options parity (Bevy renderer is wgpu) |
| 155 | N | options parity |
| 158 | N | options parity |
| 160 | S | bevy_shell/screenshots.rs |
| 161 | S | bevy_shell/screenshots/automatic |
| 162 | N | options parity |
| 163 | N | options parity |
| 164 | N | options parity |
| 165 | N | options parity |
| 166 | N | options parity |
| 167 | P | a255b39a78 login About |
| 168 | P | a255b39a78 login Survey (Windows) |
| 169 | S | desktop_startup.rs:16 (Windows) |
| 171 | K | Windows --updates maintenance dialog kept; bevy_shell/updater.rs in game |
| 173 | N | combined status line |

## Counts

- Feature windows: 75 Win32 openers (72 features), 43 already presented by Bevy, 32
  MISSING ported through the generic Bevy form window (0051 staff and 0290/0291 cast
  included after review).
- Sub-windows and status panels: 13 listed, 3 already in Bevy, 10 MISSING ported.
- Later ports outside the form window: character-picker batch training, NPC Talk/Trade
  context entries, the communication-lists form, device sign-out, About, the survey, the
  Support and Cookies links and the minimap create-mark key.
- Independent inventory, 95 MISSING rows: 42 ported, 24 stale, 4 other lane, 2 kept, 23 not
  ported (3 by design, 20 in scope).

## Assumptions

- The Win32 windows of the ported features were already generic text forms (labels,
  edit fields, buttons, a read-only summary; `features/0115-mail/native.rs`). The Bevy
  presenter shows the same title, labels, fields, buttons and summary, so a combo box
  of the Win32 window becomes a text field with the same accepted values.
- The rows left as N are options-page parity (147-166), layout persistence (140-141),
  touch (138), the house map window (119), the battle-list editor (35), the multiaction
  editor (73), the chat font, mirror and log path (50-52), minimap import/export (144),
  remembered-login toggles (12), and the combined status line (173). Offline world (16),
  touch (138) and renderer selection (152) are by design.
  Their client-ui and `PlayLink` back ends stay for a later port.
- Workflows, installer and packaging, the exe manifest, `[[bin]]`/`[features]` wiring and
  the build and shipping docs (`apps/client/BEVY.md` build section) moved to a separate
  lane, as did the pre-game account screens (bazaar, transfer, coin packages, premium
  overview, admission queue).

## Closeout

Partial. This lane ported 42 of the 95 independent MISSING rows; 20 in-scope rows stay
unported (listed above) and need an owner decision or a follow-up task. The record closes
when the batch PR with the workflow and account-screen lanes merges. Not compiled in the authoring container (no
`cargo build/check/test` there); the PC compile pass reports errors back.
