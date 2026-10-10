# OTClient source evidence mapped to audit gaps

Read-only upstream inspection on 2026-10-09, repository
`opentibiabr/otclient`, pinned revision
`53c3878a1c5119c78e148adb32def53fea3c53f2` (resolved from main).
No upstream code was executed or copied into the Oteryn runtime. These findings
describe that upstream revision, not proven official-client behavior, nor an
accepted Oteryn protocol contract. Original-client live evidence remains in the
[coverage index](UI-REFERENCE-COVERAGE.md).

## Source map and targeted consequences

| Audit lane | Source establishes | Remaining original-client question |
| --- | --- | --- |
| Quest Tracker | Menu callbacks, local JSON persistence, server tracker send/update routes | Completion semantics and persistence parity |
| Containers | Complete menu layout, manual-sort dependencies, action dispatch, explicit unfinished handlers | Official nested/manual semantics and confirmation dialogs |
| Console | Conditional tab and speaker menus; handlers for read-only, copy, ignore, private channels and save | Exact original-client conditions/names and participants UI |
| Store Summary | Lower sections for hirelings and purchased house items; partial data renderer | Official lower-region layout and populated fields |
| Stash | Search/category/seller/sort layout, item context routes, amount selector and withdraw call | Official filter memberships, placement/error behavior |
| Options | Value/action/widget/settings wiring; character-name preset auto-switch; loot-mode choices | Original-client auto-switch rules, transactional save/cancel behavior |
| Market | Browse/details/own offers, acceptance/composer/search/filter entry points | Full validation/confirmation rules and original-client parity |

## Quest Tracker: behavior visible in source

[game_questlog.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_questlog/game_questlog.lua#L781)
lines 781-789 set independent auto-track and auto-untrack booleans, save settings,
and call `sendQuestTracker`. Lines 91-103 load JSON; 145-175 serialize/write with
error handling and a player-name guard. Lines 878-960 rebuild/update displayed
missions from received tracker data and schedule a save. The record includes
quest and mission identities, not just label text.

This supplies an implementation reference for the controls already seen live.
It does not prove the official client's storage format or Oteryn server support.
No need to reopen the empty tracker merely to rediscover those menu labels.

## Containers: visible controls include unfinished handlers

[container.otui](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_containers/container.otui#L163)
defines the same eight sort choices and five additional controls recorded live.
[containers.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_containers/containers.lua#L74)
lines 74-145 wire the menu; manual mode disables sorting and the containers-first /
nested-sort toggles. Lines 670-676 clear the current sort and those toggles when
manual mode is enabled. These upstream semantics need parity checks before reuse.

Explicit limits at this revision:

- Lines 427-432: both weight-sort branches contain TODO and return.
- Lines 641-643: Move Contents to Obtain Containers contains TODO and returns.
- Lines 718-725: nested-container movement is still described as TODO.

These are source-level gaps, not working behavior to port blindly. Menu presence
must remain separate from implementation completeness.

## Console: conditions explain why one menu is insufficient

[console.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_console/console.lua#L1443)
lines 1443-1487 distinguish owned private channels, default/server tabs, active
tabs and the active read-only tab. Invite/exclude appears for an owned private
channel; Close excludes default/server tabs; read-only toggles change with state;
clear/save are added for the current tab.

Lines 1490-1547 build a separate speaker/message menu: private message, conditional
VIP addition, private-channel invite/exclude, Ignore/Unignore, permission-dependent
rule violation, copy name/selection/message, and select all. These source labels
are not claimed as an exact transcription of the original client's menu.
Saving console settings uses `game_console` (lines 534-543); this is separate
from exporting channel messages. No message, invitation or export was sent/run.

## Store Summary: lower region is no longer an unknown search area

[character.otui](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_cyclopedia/tab/character/character.otui#L1774)
lines 1774-1842 declare hireling count, jobs, outfits and Purchased House Items
inside the scrollable summary.
[character.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_cyclopedia/tab/character/character.lua#L1256)
lines 1256-1302 render boost time, blessings, prey slots/wildcards, task expansion,
instant rewards, charm expansion, hireling count and house-item rows. The function
accepts `hirelingSkills` but does not consume it in this body. Therefore layout
declarations do not establish complete population of hireling subfields.
Only a targeted comparison of these lower sections remains necessary.

## Stash: amount-selection flow

[game_stash.otui](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_stash/game_stash.otui)
declares search/clear, category, seller and sort selectors, item grid, Manage
Containers and Close; a separate Stash Withdraw modal has item/count and OK/Cancel.
[game_stash.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_stash/game_stash.lua#L441)
lines 441-485 bound count to 1..available, start at the available amount, use
Up/Down for +/-10, Left/Right for single steps, PageUp/PageDown for max/min,
Enter/OK to withdraw and Escape/Cancel to dismiss. Confirmation calls
`g_game.stashWithdraw`; that call is not an Oteryn endpoint specification.
Lines 250-311 also expose Retrieve, Cyclopedia, conditional Market and loot-list
routes. The next live check should compare these specific routes, not rediscover
the entire layout by clicking each item.

## Options and Market

[options.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/client_options/options.lua#L524)
lines 524-535 attempt selection of a preset named after the character when
auto-switch is enabled, then synchronize the action-bar set. Lines 550-595 guard
unknown/unchanged options, invoke their action, synchronize a matching widget and
write settings. Lines 205-207 enumerate Loot: Right, SHIFT+Right and Left.
Lines 671-673 leave Show Advanced Options as TODO. This source does not establish
original-client OK/Apply/Cancel rollback semantics.

[t_market.lua](https://github.com/opentibiabr/otclient/blob/53c3878a1c5119c78e148adb32def53fea3c53f2/modules/game_market/t_market.lua)
provides entry points for own offers (246), incoming browse data (672), accepting
sell/buy offers (1573/1588), price editing (1628), composing an offer (1802),
search (1886) and details (2300). This pass located these functions; their complete
validation bodies have NOT yet been reviewed. Do not mark Market audited from
this map alone.

## Continuation method

Read the mapped layout and handler first. Mark each item as upstream layout,
upstream handler, explicit stub, or original-client observation. Use the installed
original's translation/resource inventory to locate unmatched features, then
perform a live probe only for a named unresolved difference. Keep current source
revision and original-client account restrictions distinct. Source inspection
does not close end-to-end acceptance or justify copying balance/protocol values.
