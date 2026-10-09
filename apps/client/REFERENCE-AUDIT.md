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
| Cyclopedia / Items | Categories, search, level/vocation and hand filters | Detailed item and container management pending |
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
| Analytics Selector | Inspected selector; individual analyser windows pending |
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
maximum/all-time values. Remaining entries are being inspected separately.

Backpack context controls and remaining chat/navigation controls also need
inspection. Empty or account-restricted states must be reported as such.

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
this checkpoint does not claim every panel or behavior is complete.

## Oteryn implementation consequence

Build actual viewport/HUD/domain panels, inventory and action bars, chat, input
contexts, effect/audio consumers and persistent layout before exposing their
settings as active controls. The implementation status in SETTINGS.md describes
private work on `fix/native-client-login-20261007`; this documentation PR neither
publishes that source delta nor establishes full client readiness.
