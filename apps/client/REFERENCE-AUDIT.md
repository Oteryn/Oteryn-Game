# Reference client audit — authoring checkpoint

Date: 2026-10-09. Source: official Tibia/CipSoft client running in the owner's
Synology container `otclient-track-a-kasmvnc`. The container name does not make
the running executable OTClient. This is a functional reference, not Oteryn code.

## Verified settings coverage

All 20 visible settings pages were inspected: Basic Options; Controls; General,
Action Bar and Custom Hotkeys; Interface; HUD, Console, Game Window, Action Bars
and Shortcuts; Graphics and Effects; Sound, Battle Sounds and UI Sounds;
Miscellaneous, Gameplay, Screenshots and Help. See [SETTINGS.md](SETTINGS.md)
for observed controls, intended Oteryn scope and owning subsystem dependencies.

[REFERENCE-PREFERENCE-KEYS.json](REFERENCE-PREFERENCE-KEYS.json) contains 213
preference field names from six sections,
without account values, cookies or credentials. Field discovery does not establish
runtime behavior. Captures were verified by visible content; stale or incorrectly
named captures are excluded from evidence.

## Verified sidebar coverage

| Window | Directly observed | Coverage limit |
| --- | --- | --- |
| Skills | Level, experience, gain, health/mana, capacity, speed, food, stamina; visibility/context menu | Reset experience counter not executed |
| Battle List | Filter strip; name and eight ascending/descending sort choices | Not every filter tooltip identified |
| Spell List | Search, details, formula, vocation, group, cooldown, mana and minimum level; filtering menu | No casting or assignment |
| VIP List | Empty list; add, sort, offline and grouping menu | No contact changes |
| Quest Log | Quest/mission details, search, sorting and tracker controls | Only character's available quest content |
| Compendium | Player guide, client features, useful information, updates and support tabs | Not every historical/documentation leaf |
| Cyclopedia / Map | Thais map, layers, filters and navigation | Not all map locations |
| Cyclopedia / Items | Categories/search/filters; item stats, trade offers, valuation and drop/loot switches | No preference or tracking assignment |
| Cyclopedia / Houses | House/guildhall categories and state/sort controls | Character owns no houses |
| Cyclopedia / Character | General information and section navigation | Character subpages pending |
| Cyclopedia / Bestiary | Creature categories, counts, search and tracking | Not all creature entries |
| Cyclopedia / Charms | Major/minor charms, cost, assignment and restrictions | No unlock or assignment |
| Cyclopedia / Bosstiary | Unknown boss grid, filters, progression tiers | No unlocked boss details |
| Cyclopedia / Boss Slots | Two locked slots, boosted boss and unlock conditions | Account restrictions retained |
| Cyclopedia / Magical Archive | Spell/rune list, combat and additional details | This is not Weapon Proficiency |
| Highscores | World, vocation, category and PvP filters; ranking and own rank | Not all filter combinations |
| Party List | Empty docked list and sorting menu | No invite or membership change |
| Wheel of Destiny | Official preview, wheel/perks and vessels | Character requires level 51, promotion and Premium for active use |
| Gem Atelier | Empty collection, affinity/quality filters and revelation costs | No revelation |
| Fragment Workshop | Modifier grid, search/filters, four grades and enhancement costs | No enhancement |
| Premium features | Expanded benefits list | No purchase; these are benefits, not extra panels |

## Follow-up with the mainland character

The owner signed in normally to a different, level-313 character on 2026-10-09.
Prey and Reward Wall now open their actual windows; the earlier beginner-character
restrictions remain historical observations, not the current access state.

Wheel of Destiny opens the actual character wheel with 263 available promotion
points, preset controls and perk summaries, rather than the earlier level-2
preview. On opening, the official client displayed a notice that its saved
presets had been adjusted. The auditor dismissed the notice and did not allocate
points or use Apply/Reset.

Forge Fusion shows item inputs, convergence, success probability, tier-loss
mitigation and resource requirements. Transfer shows source/destination and
consumed-resource requirements. Conversion shows dust/sliver/core conversion and
dust-limit increase. History is empty. No operation was submitted.

Weapon Proficiency is a separate window from Magical Archive. The selected weapon
shows inactive perks because the character does not meet its use requirements.
No weapon modification, perk assignment or reset was submitted.

Task Board shows difficulty, three bounty candidates, progress/rewards, preferred
list and reroll/claim controls, plus bounty-talisman upgrades. Weekly Tasks shows
0/6 kill and 0/6 delivery progress with a difficulty selection. Hunting Task Shop
shows outfit/addon/mount offers. No task selection, difficulty change, reroll,
claim, upgrade or purchase was performed. Social team access still requires
Premium; the filter form itself is visible.

## Complete extra-shortcut inventory

The original layout contained 11 buttons: Skills, Battle List, Spell List, VIP
List, help, Quest Log, Compendium, Cyclopedia, Highscores, Player Guide and Manage
Shortcuts. The owner authorized displaying every remaining available button.
All **17 extra shortcuts** were added; the available list became empty:

| Extra shortcut | Inspection at this checkpoint |
| --- | --- |
| Party List | Inspected |
| Wheel of Destiny | Inspected in normal preview, including both supporting tabs |
| Quest Tracker | Visible; dedicated controls still pending |
| Unjustified Points | Inspected: zero open points and three skull-threshold bars |
| Prey Dialog | Mainland character: two creature-choice slots, reroll controls/costs, locked third slot |
| Kill Tracker | Inspected: prey creature tracking, inactive state |
| Reward Wall | Mainland character: seven-day cycle, resting bonuses, expired streak, jokers and empty history |
| Analytics Selector | All nine entries opened; visible panels/menus and Premium/empty-state restrictions recorded below |
| Bosstiary | Inspected through Cyclopedia; shortcut route pending |
| Boss Slots | Inspected through Cyclopedia; shortcut route pending |
| Bosstiary Tracker | Inspected: empty docked tracker with header controls |
| Bestiary Tracker | Inspected: tracked creature, numeric progress and progress bar |
| Imbuement Tracker | Inspected: equipment entry and imbuement slots |
| Weapon Proficiency | Inspected: weapon catalogue/search/filters, XP, perk tree and requirement warning |
| Exaltation Forge | Inspected all four tabs: Fusion, Transfer, Conversion and History |
| Social | Team finder visible; seeing/joining/assembling teams requires Premium |
| Task Board | Inspected Bounty Tasks, Weekly Tasks and Hunting Task Shop |

Analytics Selector shows nine entries, including Boss Cooldowns. Hunting opens
the Premium Store on this account. Loot and Supply open docked value/per-hour
panels. Supply's header menu exposes Reset Data and per-hour gauge/graph controls,
with those graphical controls labelled Premium. Impact shows damage totals, DPS,
maximum DPS and all-time high, damage-type breakdown, healing totals, HPS and
maximum/all-time values. Damage Input opens received-damage totals, maximum DPS, a minutes graph and
source/type breakdown. Its menu includes session values, graph/type/source
visibility, reset and clipboard copy. XP shows gain, XP/hour and next-level
progress; its menu has raw-XP display and Premium gauge/graph controls. Drop
Tracker opens Cyclopedia Items to choose a tracked item; the item page contains
the Track drops checkbox. The audit left it unchanged. Party Hunt shows session duration and
loot-price mode, with last-session reset and clipboard copy. Boss Cooldowns
opens an empty-state panel explaining that only bosses killed at least once
are displayed. All nine selector entries have now been opened; empty-state and
Premium restrictions do not establish behavior with populated hunt data.

Backpack context controls and remaining chat/navigation controls also need
inspection. Empty or account-restricted states must be reported as such.

### Analyser menu details

- Impact: reset data/all-time high, session values, damage types; DPS/HPS gauges
  and graphs carry Premium labels.
- Damage Input: reset data, session values, damage graph/types/sources and copy.
- XP: reset data, raw XP, Premium XP/hour gauge and graph.
- Party Hunt: reset data of last session and copy.
- Boss Cooldowns: sort by cooldown or name.
- Drop Tracker: no additional header menu appeared; Add Tracked Drop navigates
  to the item catalogue instead of immediately enabling tracking.

## Item and container controls

An armor detail was opened directly: armor, imbuement slots, weight, market
eligibility, body position and classification; NPC sell/buy offers; average
market price and optional own loot value; NPC-versus-market valuation; track
item drops and skip-when-quick-looting switches. No valuation, drop tracking or
skip preference was changed.

Manage Containers exposes a per-category Loot/Obtain assignment matrix with
clear controls, use-main-container fallback, skipped/accepted loot filters,
search, clear/add list controls and Premium availability explanations. Categories
were scrolled to inspect the bounded list. No container assignment or list
mutation was submitted.

## Character stat subpages

Character Stats shows progression, skills, resource/capacity/speed, stamina,
offline training, account status and badges. Offence Stats breaks down flat
healing/damage, attack contributions and critical chance/extra damage. Defence
Stats shows equipment/skill defence contributions, armor, mitigation, magic-shield
capacity and reductions for physical/fire/earth/energy/ice/holy/death. Misc. Stats
shows blessings and their detail entry. The general-stat group has four child
pages; navigation names are checked against actual contents, not capture names.
Recent Deaths and Recent PvP Kills were both opened, including pagination and
empty record tables. Achievements shows grade counts, accomplishment filters,
sorting, dates, descriptions and a scrollable achievement list. Item Summary
shows inventory/depot/inbox/stash/store-inbox selectors, search and list/grid
views. Appearances opens separate Outfit, Mount and Familiar collections.
Store Summary shows account-benefit categories, including XP boosts, blessings,
Prey, Task Board, daily rewards and charms; its lower scroll region remains
unverified. Character Titles exposes current title, permanent/temporary and
locked/unlocked filters, search and a title table. No title was activated.
Familiar collection filtering offers all, standard and quest familiars.

## Static client-file sweep

The installed reference is the official Tibia distribution, despite the
`otclient-track-a-kasmvnc` container name. Its `bin/client.en.qm` is a Qt
translation catalogue, not application source. A read-only parser consumed its
702,294-byte messages block completely and found 4,622 distinct source keys.
No account/configuration values, proprietary graphics or translated text corpus
are published here. Qt framework QML files are not evidence of product UI source.

The catalogue independently identifies 358 `optionsmenu` keys and exactly 28
shortcut definitions, matching the live shortcut inventory. Key counts include
captions, help and warnings; they are not counts of independent settings.
Static discovery establishes interface vocabulary, not reachability or behavior.

Additional controls identified in files, still requiring targeted runtime checks:

- Social: friend list, invitations, friend configuration, account search and
  badges, alongside join/assemble team pages already encountered.
- Containers: name, stack size, weight and expiry sorting in both directions;
  backpacks first, manual sorting, recursive sorting and filtering.
- Quest Tracker: automatic addition/removal, remove tracked/all/completed,
  track new quests and navigation to the Quest Log.
- Loot analyser: reset, per-hour view, graph, gauge and gauge target.
- Battle List: vocation filters including Monk; players, monsters, NPCs,
  player summons, own guild, party and non-skulled players; secondary lists
  and primary-list selection, subject to account restrictions.
- Chat: channel opening/closing, participants, secondary chat, server and
  join/leave message visibility, mute/unmute, copy and save channel.
- Hotkeys: add/copy/rename/remove presets, automatic preset switching,
  search, new action and distinct chat-on/chat-off bindings.
- Graphics vocabulary includes three antialiasing choices and twelve renderer
  identifiers. Platform-specific identifiers do not establish Linux/Windows
  availability or define the renderer choices Oteryn should expose.
- Sound includes automatic output-device selection, separate volume groups,
  creature attacks/deaths/noises, weapons, spells, eating/item movement and
  UI/chat/system/private/guild/party/raid/team-finder/VIP notifications.

This sweep substantially expands the inventory without treating unavailable
menus or destructive actions as tested. Remaining dropdown, context-menu and
account-dependent behavior checks retain their pending status.

## Evidence and preservation

Private screenshots remain under `/workspace/artifacts/oteryn-reference`, with
sidebar evidence under its `sidebar` directory and ongoing runtime captures on
the owner's NAS. Reference screenshots and proprietary artwork are deliberately
not committed as Oteryn assets. No credentials or personal account data belong
in this document. Temporary shortcut additions must be restored to the original
layout after the audit and checked against the scoped preference snapshot.

No purchase, reset, profile deletion, spell assignment, enhancement, party invite
or gameplay action was required for this audit. Shortcut visibility was saved
through the normal UI with the owner's authorization. Inspection is ongoing;
this checkpoint does not claim every panel or behavior is complete. Closing
Manage Containers closed the shared Cyclopedia modal; subsequent stale
navigation clicks landed in the game viewport. That capture is excluded and
modal navigation now verifies the actual title before each click; fixed-position
checks compare against a previously verified capture, not an unverified live
frame. Input modifier/button state was released before resuming navigation. No gameplay
action was intended as an audit step.

## Oteryn implementation consequence

Build actual viewport/HUD/domain panels, inventory and action bars, chat, input
contexts, effect/audio consumers and persistent layout before exposing their
settings as active controls. The implementation status in SETTINGS.md describes
private work on `fix/native-client-login-20261007`; this documentation PR neither
publishes that source delta nor establishes full client readiness.
