# Win32 client UI vs `crates/client-ui`: switch-over audit

Read-only audit of `origin/main`. Scope: files under `apps/client/src/` that contain `windows_sys::Win32::UI`, `CreateWindowExW`, `SendMessageW` or `GetDlgItem`. That is 190 files and 39,834 lines: 181 files (38,500 lines) in 65 feature directories and 9 files (1,334 lines) outside them.

## Read this first: what `crates/client-ui` is today

`crates/client-ui` is a platform-neutral **view-model layer**, not a widget toolkit. A typical feature directory holds `state.rs` (panel struct), `protocol.rs` (`request`/`apply_result`), `input.rs` (open key over `oteryn_input_actions`), `view.rs` (`panel_text`-style functions that return a `String`) and `registrations/*.json`. Example: `crates/client-ui/src/features/0009-highscores/view.rs::panel_text` returns the text that `apps/client/src/features/0009-highscores/native.rs` puts into a read-only multiline `EDIT`. That native file creates the `COMBOBOX`, `BUTTON` and `EDIT` controls itself and maps their IDs onto `oteryn_client_ui::highscores::Action`.

`crates/client-ui/Cargo.toml` depends only on client-domain, client-runtime, input-actions, renderer and serde_json. It has no layout, widgets, focus or text editing. Only 4 places use `oteryn_renderer`: world-interaction, local-minimap (geometry only) and local-spell-action-bar (quads). The crate README itself says "There is no gameplay behavior or public UI API yet" (`crates/client-ui/README.md`).

Classes below therefore mean:

- **COMPLETE (C):** client-ui holds the state, every intent or action the Win32 window raises, the protocol mapping and the text the window shows. The Win32 file is a thin host. Switching over needs only the shared widget kit plus binding.
- **PARTIAL (P):** some of what the window shows or does is not in client-ui. It lives in the Win32 file (own label formatting, GDI/pixel painting, ListView/TreeView) or in apps/client non-Win32 modules. The missing parts are named.
- **SKELETON (K):** client-ui has only a stub, a key binding or geometry.
- **NONE (N):** no client-ui view.

Sizes cover per-feature binding work only, **assuming the shared widget kit already exists**: S under 1 day, M 1-3 days, L over 3 days. The shared kit is sized separately below.

Evidence method: for every feature, the Win32 control inventory (counts of window classes and styles), which `oteryn_client_ui::*` items the Win32 files call, the `format!` and `tr!` calls inside the Win32 files, the imports from apps/client helpers, and the client-ui file list, registrations and `#[test]` count. I also spot-read 0003, 0009, 0014, 0016, 0037, 0040, 0042, 0043, 0060, 0062, 0068, 0140, 0147, 0151, 0179, 0195 and local-account-achievements, and listed the headers of 0025, 0057, 0097, 0098, 0115, 0117, 0150, 0153, 0154, 0274, 0287 and 0450. For features with plain symmetric signals I did not read every line: their classification rests on the signals (Win32 calls the client-ui `panel_text`/`request`/`Intent`, no own formatting or painting).

## Feature table

| feature | win32 files | win32 lines | client-ui dir | class | missing parts | size | shared controls needed |
|---|---|---|---|---|---|---|---|
| 0003-npc-service | 1 | 304 | `crates/client-ui/src/features/0003-npc-service` (6 files, 315 lines, 7 tests) | COMPLETE | - | S | edit (text input), read-only text, context menu (Talk/Trade), buttons, modal |
| 0004-item-view-move | 3 | 696 | `crates/client-ui/src/features/0004-item-view-move` (3 files, 302 lines, 6 tests) | PARTIAL | client-ui holds only drag `Movement` state; its registration is `unsupported` (CLIENT-STRUCT-1 placeholder); count dialog (view/count.rs), drag view and protocol stay in apps/client | M | text input (count), list, modal, focus |
| 0007-chat | 8 | 1163 | `crates/client-ui/src/features/0007-chat` (7 files, 770 lines, 12 tests) | PARTIAL | console presentation/filters live in apps/client console.rs (200) + native_console/native_mirror; console font; 8 Win32 files | L | text input IME/Polish, clipboard, scrolling list, focus/Tab, modal (chat_dialog), checkbox |
| 0008-achievement-notices | 1 | 118 | `crates/client-ui/src/features/0008-achievement-notices` (7 files, 260 lines, 4 tests) | COMPLETE | - | S | read-only text |
| 0009-highscores | 1 | 322 | `crates/client-ui/src/features/0009-highscores` (5 files, 282 lines, 0 tests) | COMPLETE | - | S | combo x2, edit, buttons, read-only scrolling text |
| 0012-item-equip-drop | 1 | 111 | `crates/client-ui/src/features/0012-item-equip-drop` (4 files, 232 lines, 6 tests) | COMPLETE | - | S | list |
| 0014-container-tree | 1 | 390 | `crates/client-ui/src/features/0014-container-tree` (6 files, 277 lines, 0 tests) | COMPLETE | minor: one destination/debug label formatted in Win32 (native.rs:338) | S | list, edit, buttons, focus |
| 0016-quest-log | 4 | 1217 | `crates/client-ui/src/features/0016-quest-log` (10 files, 875 lines, 13 tests) | PARTIAL | quest/mission/tracker row labels formatted in Win32 (native.rs:282, native/run.rs:254-293) | M | lists, combo, edits, read-only text, focus/Tab |
| 0017-attack | 5 | 1172 | `crates/client-ui/src/features/0017-attack` (8 files, 425 lines, 6 tests) | PARTIAL | battle-list preferences, sort, `row_label`, list validation in apps/client preferences.rs (193); metadata dialog; 33 buttons, 8 combos, 6 checkboxes | L | combo, checkbox, list, edit, modal |
| 0025-appearance | 2 | 486 | `crates/client-ui/src/features/0025-appearance` (9 files, 642 lines, 5 tests) | PARTIAL | outfit preview pixels (apps/client view/preview*.rs) drawn with StretchDIBits; no image view in client-ui | M | image surface, edit, modal |
| 0031-tutorial-notices | 1 | 160 | `crates/client-ui/src/features/0031-tutorial-notices` (6 files, 144 lines, 3 tests) | COMPLETE | - | S | read-only text, radio |
| 0037-depot | 1 | 338 | `crates/client-ui/src/features/0037-depot` (6 files, 350 lines, 5 tests) | COMPLETE | - | S | buttons, modal |
| 0040-guild | 1 | 240 | `crates/client-ui/src/features/0040-guild` (6 files, 260 lines, 3 tests) | COMPLETE | - | S | list, text input, read-only text |
| 0042-party | 1 | 459 | `crates/client-ui/src/features/0042-party` (6 files, 736 lines, 11 tests) | PARTIAL | owner-drawn list rows (portrait, HP/mana bars, skull) in apps/client list_draw.rs + portraits.rs (not matched by grep); context menu | M | owner-drawn list, context menu, checkbox, edit |
| 0043-vip | 3 | 680 | `crates/client-ui/src/features/0043-vip` (5 files, 291 lines, 1 tests) | COMPLETE | minor: status string join in Win32; control construction in native_controls.rs (not matched by grep, 137 lines) | M | lists, checkbox, edit, focus/Tab |
| 0049-house | 3 | 509 | `crates/client-ui/src/features/0049-house` (8 files, 741 lines, 2 tests) | PARTIAL | ownership input/state in apps/client input/ownership.rs + state/ownership.rs; map window | M | combo, edit, modal (MessageBox), read-only text |
| 0053-world-event | 1 | 142 | `crates/client-ui/src/features/0053-world-event` (6 files, 196 lines, 3 tests) | COMPLETE | - | S | read-only text |
| 0057-cyclopedia-map | 2 | 474 | `crates/client-ui/src/features/0057-cyclopedia-map` (10 files, 879 lines, 6 tests) | PARTIAL | atlas painted with GDI (atlas_window.rs FillRect, atlas_native/paint.rs); app view.rs (346) and input/atlas.rs (282) not in client-ui | L | map/image surface, mouse, read-only text |
| 0060-player-trade | 1 | 372 | `crates/client-ui/src/features/0060-player-trade` (6 files, 467 lines, 6 tests) | COMPLETE | - | S | lists, context menu, buttons, modal |
| 0062-stash | 1 | 405 | `crates/client-ui/src/features/0062-stash` (6 files, 410 lines, 7 tests) | COMPLETE | - | S | lists, edits, buttons, modal |
| 0063-market | 3 | 922 | `crates/client-ui/src/features/0063-market` (9 files, 1225 lines, 10 tests) | PARTIAL | SysListView32 table (apps/client view/table.rs) and catalogue window; coin input in app input/coins.rs | M | multi-column list, combo, checkbox, edits |
| 0067-bestiary-charm-progression | 4 | 687 | `crates/client-ui/src/features/0067-bestiary-charm-progression` (11 files, 1020 lines, 4 tests) | PARTIAL | catalogue pixels (apps/client catalogue_pixels.rs), catalogue/selector input in app; label formatting in Win32 | M | combo, list, image surface, modal |
| 0068-imbue | 1 | 342 | `crates/client-ui/src/features/0068-imbue` (7 files, 707 lines, 4 tests) | COMPLETE | - | S | lists, buttons, modal |
| 0072-task-board | 4 | 927 | `crates/client-ui/src/features/0072-task-board` (10 files, 1025 lines, 4 tests) | PARTIAL | SysTreeView32 tree (native/tree.rs); two extra windows (soulseals, weekly results); label formatting | M | tree view, combo, checkbox, modal |
| 0080-chat-private-channel | 1 | 210 | `crates/client-ui/src/features/0080-chat-private-channel` (6 files, 470 lines, 6 tests) | COMPLETE | - | S | list, text input, read-only text |
| 0081-npc-shop | 1 | 387 | `crates/client-ui/src/features/0081-npc-shop` (8 files, 710 lines, 10 tests) | COMPLETE | - | S | list, checkbox, edits, modal |
| 0097-character-info | 1 | 323 | `crates/client-ui/src/features/0097-character-info` (7 files, 296 lines, 3 tests) | PARTIAL | title row labels formatted in Win32 (view/titles.rs:218) | S | combo, list, edit |
| 0098-quick-loot | 4 | 699 | `crates/client-ui/src/features/0098-quick-loot` (5 files, 217 lines, 0 tests) | PARTIAL | 125 tr! strings and owner paint in Win32 (native/paint.rs); app native/projection.rs; extra entries in world context menu; client-ui dir has 0 unit tests | M | list, edit, context menu |
| 0115-mail | 2 | 563 | `crates/client-ui/src/features/0115-mail` (7 files, 207 lines, 3 tests) | PARTIAL | shared Form + editor/confirmation dialogs in Win32; protocol/methods/context in apps/client (316+205+167); reused by 0182 | M | multiline text input IME/Polish, modal, checkbox |
| 0117-boss-encounter | 1 | 356 | `crates/client-ui/src/features/0117-boss-encounter` (13 files, 1638 lines, 15 tests) | PARTIAL | GDI portrait gallery (StretchDIBits, FillRect); app native.rs (203) + view/portraits.rs | M | image surface, buttons |
| 0140-prey | 3 | 794 | `crates/client-ui/src/features/0140-prey` (7 files, 795 lines, 12 tests) | COMPLETE | native/run.rs (197) and native_tests.rs are Win32 too; only star labels formatted in Win32 | M | buttons, checkbox, radio, modal |
| 0147-exiva-settings | 1 | 267 | `crates/client-ui/src/features/0147-exiva-settings` (5 files, 99 lines, 1 tests) | COMPLETE | - | S | list, checkbox, edit |
| 0148-wheel-allocation | 10 | 1462 | `crates/client-ui/src/features/0148-wheel-allocation` (23 files, 2860 lines, 31 tests) | PARTIAL | gem/wheel presentation, surface and presets in apps/client (gem_presentation.rs 131, wheel_presentation.rs 80, presets.rs 160); three near-duplicate localized window files | L | combo, lists, edits, focus/Tab, modal |
| 0150-house-podium | 1 | 174 | `crates/client-ui/src/features/0150-house-podium` (6 files, 162 lines, 2 tests) | PARTIAL | artwork preview decode + StretchDIBits; preview assets in apps/client | S | image surface |
| 0151-daily-boost | 1 | 100 | `crates/client-ui/src/features/0151-daily-boost` (5 files, 252 lines, 3 tests) | COMPLETE | - | S | read-only text |
| 0152-item-forge | 4 | 864 | `crates/client-ui/src/features/0152-item-forge` (12 files, 1330 lines, 7 tests) | PARTIAL | apps/client controller.rs (277), icons.rs, lifecycle.rs not in client-ui | M | combo, checkbox, list, modal |
| 0153-depot-search | 2 | 531 | `crates/client-ui/src/features/0153-depot-search` (5 files, 171 lines, 1 tests) | PARTIAL | presentation()/submit in apps/client native.rs (181) instead of client-ui view; two near-duplicate Win32 files | M | list, edits, modal |
| 0154-store | 5 | 976 | `crates/client-ui/src/features/0154-store` (9 files, 1292 lines, 5 tests) | PARTIAL | browser layout, coin/token text, product images (StretchDIBits), assets in app native/assets.rs; client-ui state is used via super::store | L | list, combo, edits (search, gift), image surface, modal |
| 0165-hireling | 1 | 143 | `crates/client-ui/src/features/0165-hireling` (5 files, 164 lines, 3 tests) | COMPLETE | - | S | list |
| 0168-animus-mastery | 1 | 210 | `crates/client-ui/src/features/0168-animus-mastery` (6 files, 278 lines, 4 tests) | COMPLETE | - | S | buttons, read-only text |
| 0170-arena | 1 | 294 | `crates/client-ui/src/features/0170-arena` (6 files, 619 lines, 7 tests) | COMPLETE | - | S | combo, buttons |
| 0179-hotkey-equip | 1 | 379 | `crates/client-ui/src/features/0179-hotkey-equip` (5 files, 144 lines, 1 tests) | PARTIAL | key list and status labels formatted in Win32 (native.rs:306, 334); app preferences.rs | S | lists, checkbox |
| 0182-daily-reward | 2 | 260 | `crates/client-ui/src/features/0182-daily-reward` (11 files, 855 lines, 7 tests) | COMPLETE | reuses 0115-mail Win32 Form | S | buttons, modal |
| 0195-weapon-proficiency | 4 | 1466 | `crates/client-ui/src/features/0195-weapon-proficiency` (6 files, 806 lines, 0 tests) | PARTIAL | display/choose/submit in apps/client native.rs (217); row labels formatted in Win32 (native/window_host.rs:240-278); three near-duplicate window files; HUD buttons; client-ui feature dir has 0 unit tests | L | combo, lists, checkbox, edits, focus/Tab |
| 0197-cyclopedia-tracker | 2 | 514 | `crates/client-ui/src/features/0197-cyclopedia-tracker` (6 files, 335 lines, 4 tests) | COMPLETE | - | M | list, checkbox, combo, edits |
| 0199-cyclopedia-character | 9 | 1593 | `crates/client-ui/src/features/0199-cyclopedia-character` (28 files, 3364 lines, 25 tests) | PARTIAL | display/submit in apps/client native.rs; view helpers (blessings, gallery_pixels, misc_layout, target) in app; 9 Win32 files | L | combo, lists, edits, image surface, focus/Tab |
| 0224-boss-rewards | 1 | 243 | `crates/client-ui/src/features/0224-boss-rewards` (6 files, 264 lines, 2 tests) | COMPLETE | - | S | edits, read-only text |
| 0225-boss-difficulty | 1 | 362 | `crates/client-ui/src/features/0225-boss-difficulty` (6 files, 211 lines, 1 tests) | PARTIAL | client-ui handler registration `unsupported`; protocol in apps/client protocol.rs | S | combo, edit |
| 0230-browse-field | 2 | 515 | `crates/client-ui/src/features/0230-browse-field` (5 files, 151 lines, 1 tests) | PARTIAL | result_text/submit/item_command in apps/client native.rs (175); two near-duplicate Win32 files | M | edits, read-only text |
| 0233-player-typing | 1 | 139 | `crates/client-ui/src/features/0233-player-typing` (6 files, 305 lines, 5 tests) | COMPLETE | - | S | edit (focus/text polling) |
| 0240-chat-party-room | 1 | 174 | `crates/client-ui/src/features/0240-chat-party-room` (6 files, 139 lines, 1 tests) | COMPLETE | - | S | text input, read-only text |
| 0250-hireling-management | 1 | 324 | `crates/client-ui/src/features/0250-hireling-management` (6 files, 365 lines, 3 tests) | COMPLETE | - | S | combo x3, edits |
| 0258-vip-presence | 1 | 106 | `crates/client-ui/src/features/0258-vip-presence` (6 files, 84 lines, 1 tests) | COMPLETE | - | S | read-only text |
| 0259-chat-public-channels | 1 | 253 | `crates/client-ui/src/features/0259-chat-public-channels` (6 files, 293 lines, 4 tests) | COMPLETE | - | S | list, read-only text |
| 0274-loyalty-skills | 2 | 451 | `crates/client-ui/src/features/0274-loyalty-skills` (5 files, 146 lines, 2 tests) | PARTIAL | configurable SkillsLayout/FAMILIES projection in apps/client layout.rs (169) | M | combo, checkbox, buttons |
| 0287-actor-spell-catalog | 2 | 533 | `crates/client-ui/src/features/0287-actor-spell-catalog` (7 files, 364 lines, 2 tests) | PARTIAL | SpellbookState, filters, GROUPS/VOCATIONS in apps/client book.rs (311) + archive.rs | M | combo x4, edits, list |
| 0450-server-modal | 1 | 199 | `crates/client-ui/src/features/0450-server-modal` (5 files, 436 lines, 5 tests) | PARTIAL | Dialog/Submission + image in apps/client view.rs/view/image.rs; client-ui has state/protocol/view and components/modal_dialogs.rs | S | modal, list, edit |
| local-account-achievements | 4 | 765 | `crates/client-ui/src/features/local-account-achievements` (7 files, 471 lines, 6 tests) | PARTIAL | detail/row/header labels formatted in Win32 (native/run.rs:131,209,245); app view/backend.rs | M | combo, lists, edits |
| local-account-services | 1 | 287 | none | NONE | no client-ui dir; controller in apps/client local-account-services/controller.rs + mod.rs | M | list, combo, edits, modal |
| local-character-selection | 1 | 281 | none | NONE | no client-ui dir; CharacterRow/SelectionAction in apps/client mod.rs | M | list, combo, buttons, focus/Tab |
| local-game-notifications | 1 | 113 | none (model: `crates/client-ui/src/components/notifications.rs`) | COMPLETE | no feature dir, but model is client-ui components/notifications.rs (NotificationCenter), used by apps/client state.rs | S | read-only text |
| local-minimap | 1 | 333 | `crates/client-ui/src/features/local-minimap` (3 files, 97 lines, 2 tests) | SKELETON | client-ui has only geometry/click mapping (view.rs, 97 lines); state (349), scene, transfer, view (176) in apps/client | M | map surface, combo, list, edit |
| local-native-login | 2 | 323 | none | NONE | no client-ui dir; LoginForm/LoginSettings in apps/client mod.rs (271) | M | text input (password, IME), focus/Tab, modal |
| local-reference-preferences | 40 | 6808 | `crates/client-ui/src/features/local-reference-preferences` (1 files, 10 lines, 0 tests) | SKELETON | client-ui has only input.rs (open key, 10 lines); all ~25 settings pages, profiles, shortcuts, custom editor in apps/client (~70 non-Win32 files) | L | checkbox x33, combo x19, edits x44, lists, trackbar, context menu, modal, focus/Tab |
| local-spell-action-bar | 5 | 1090 | `crates/client-ui/src/features/local-spell-action-bar` (8 files, 596 lines, 8 tests) | PARTIAL | assignments (461), hotkeys, layout, view (343) in apps/client; GDI paint and 4 context-menu sites in Win32; client-ui has quad view + cooldown | L | context menu, owner paint, list, modal |

Win32 files are counted only by the four fixed strings. **31 more files (3,328 lines)** use Win32 through `use super::*` or other imports and are not in the counts. Examples: `features/0042-party/list_draw.rs` (owner-drawn rows), `features/0043-vip/native_controls.rs`, `features/0057-cyclopedia-map/view/atlas_native/paint.rs`, `features/0140-prey/native/run.rs`, `features/0011-world-interaction/native.rs` (world context popup), `miniwindow/native.rs` + `miniwindow/events.rs` (the window hooks that make native windows game-window children), 11 files in `local-reference-preferences`, and `device_login/vault_windows.rs`. Those files are part of the switch-over too.

Duplicated Win32 windows seen while reading, which should be collapsed rather than ported:

- 0195: `native_window.rs`, `native_localized_1.rs` and `native/window_host.rs`
- 0153: `native/windows.rs` and `native_windows.rs`
- 0230: `native/platform.rs` and `native_platform.rs`
- 0148: `native_localized_2.rs`, `native_localized_10.rs` and `native_localized_11.rs`

## Shared controls

The Win32 code gets these from the OS: EDIT gives IME, clipboard, caret and selection; LISTBOX and EDIT give scrolling; `WS_TABSTOP` and `IsDialogMessage` give focus order; plus combo, menus and MessageBox. client-ui has none of them as widgets.

| control | status | evidence |
|---|---|---|
| Text input with IME / Polish characters | **missing** (plumbing only) | `crates/input-platform/src/winit_adapter.rs:70-75` maps winit `Ime::Commit` to `TextCommit`, and `crates/input-actions/src/text.rs` bounds it at 4,096 bytes. No client-ui or apps/client code consumes `NormalizedInputEvent::TextCommitted` (only `0011-world-interaction/input.rs:108` skips it). `components/modal_dialogs.rs::TextEditor` is a `String` plus a byte-budget check, with no caret, selection or preedit. Text rendering is `crates/renderer/src/name_text.rs::glyph_rgba`, a 5x7 ASCII bitmap where "Unsupported Unicode uses a visible question mark", so Polish ą/ł/ż would render as `?`. |
| Clipboard | **missing** | No `clipboard` in client-ui, input-actions, input-platform or renderer. Win32 gets it from EDIT controls (the features never call clipboard APIs directly). |
| Scrolling list | **missing** | No list or scroll widget. `NormalizedInputEvent::Wheel` exists (`crates/input-actions/src/physical.rs`). `components/console_display.rs` only trims the log. Also missing: a multi-column list (0063 uses `SysListView32`), a tree view (0072 uses `SysTreeView32`) and owner-drawn rows (0042, 0098). |
| Focus / Tab order | **missing** | Only window-level `FocusChanged` (`crates/input-actions/src/router.rs`). 0010 and 0287 `input.rs` say "text focus is handled by the shell". |
| Combo box | **missing** | Only index-to-ID helpers (`crates/client-ui/src/features/0009-highscores/mod.rs` `category_for_selection`/`category_selection`; `0170-arena/input.rs:27`). Combos are used by 24 features (Win32 `"COMBOBOX"` count > 0). |
| Context menu | **model only** | `crates/client-ui/src/features/0011-world-interaction/state.rs` (`Menu`, `ContextAction`, `context_entries`) models the world popup. Rendering is Win32 (`apps/client/src/features/0011-world-interaction/native.rs`). `TrackPopupMenu`/`CreatePopupMenu` are also used by 0003, 0042, 0060, local-reference-preferences and local-spell-action-bar. |
| Modal dialog | **model only** | `crates/client-ui/src/components/modal_dialogs.rs` (`ServerDialog`, `NoticeDialog`, `DialogAnswer`, `ServerDialogHandler`, `DismissedNotices`, `TextEditor`) and `features/0450-server-modal`. No modal surface or input capture. Win32 uses `EnableWindow` (owner disabling and control enabling) in 64 feature files and `MessageBoxW` in 10 features plus 6 non-feature files. |
| Also missing | **missing** | Buttons, checkboxes (16 features), radio buttons, trackbar (preferences), labels, window/panel frame, layout and DPI scaling (apps/client `local-reference-preferences/native_ui_scale.rs` is Win32). Image surfaces exist only as renderer quads (`AtlasImage`/`QuadInstance` in `features/local-spell-action-bar/view.rs`). The Win32 pixel previews (0025, 0117, 0150, 0154) use `StretchDIBits`. |

Rough shared-kit size, my estimate from the list above and not a measured figure: Unicode font and text layout L; text input (caret, selection, IME preedit, clipboard) L; list, scroll, table and tree L; button, checkbox, radio, combo and trackbar M; focus/Tab, modal, context menu, panel frame and layout L. That comes to **about 22 person-days** (4L + 1M), separate from the feature totals.

## Non-feature files

| file | lines | what it does | client-ui equivalent | size |
|---|---|---|---|---|
| `apps/client/src/about.rs` | 93 | About text, an optional experience survey saved with consent, and `show_information` via `MessageBoxW` | none; the text and survey logic are already platform-neutral in this file | S |
| `apps/client/src/cast_watch/native_hud.rs` | 58 | `STATIC` text child on the cast Watch winit window | none (client-ui `0289-cast-view` holds cast state, not this HUD) | S |
| `apps/client/src/crash_report.rs` | 209 | Crash evidence capture; a pending report is reviewed via a Yes/No `MessageBoxW` (line 172) | none; needs a modal confirm | S |
| `apps/client/src/device_login/confirmation.rs` | 24 | `confirm_action` Yes/No `MessageBoxW` for close/logout | none; needs a modal confirm | S |
| `apps/client/src/play/status_window.rs` | 357 | One shared Win32 window type (STATIC/EDIT/BUTTON, 1 s refresh) for the condition, mechanics, rest, battle-instinct and skill-progress panels | **yes**: every panel's text and request come from client-ui (`oteryn_client_ui::$module::panel_text`/`request`, features 0276/0277/0278/0279/0384) | S |
| `apps/client/src/updater/window.rs` | 59 | Updater maintenance prompts (`MessageBoxW` Yes/No/Cancel) and the mutex handshake | none; runs before the game UI exists, so it may stay native | S |
| `apps/client/src/win_mutex.rs` | 179 | Per-user install mutex, token SID, launcher error `MessageBoxW` | n/a: an OS service, not UI; only the error box is UI | 0 |
| `apps/client/src/windows_shell/application_handler.rs` | 263 | winit `ApplicationHandler` for the game window; uses `GetMessageExtraInfo` and a Yes/No `MessageBoxW` confirm (line 172) | n/a (shell); needs a modal confirm | S |
| `apps/client/src/windows_shell/capture.rs` | 92 | Screenshot capture input/redraw; Yes/No `MessageBoxW` `ask` | none; needs a modal confirm | S |

Non-feature sum: 8 x S = **4 person-days**. Most of it goes away once the shared modal-confirm widget exists.

## Totals

- Per class (65 features): COMPLETE 30, PARTIAL 30, SKELETON 2 (local-minimap, local-reference-preferences), NONE 3 (local-account-services, local-character-selection, local-native-login).
- Per size: S 32, M 24, L 9.
- Feature binding work: 32 x 0.5 + 24 x 2 + 9 x 5 = **109 person-days**.
- Non-feature files: **4 person-days**.
- Shared widget kit (estimate, not in the 109): **about 22 person-days**.
- **Overall: about 135 person-days.**

Caveats:

- `local-reference-preferences` (40 Win32 files, 6,808 lines; about 25 settings pages) is counted as one L (5 days). Realistically it is closer to 3 x L, which would add about 10 days.
- The 31 Win32 files the grep missed (3,328 lines) are folded into their features' sizes, except `miniwindow/*` and `0011-world-interaction/native.rs`. Those two fall under the shared kit (window parenting, context menu).
