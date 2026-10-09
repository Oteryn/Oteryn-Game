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

A private catalogue contains 213 preference field names from six sections,
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
| Unjustified Points | Pending |
| Prey Dialog | Pending |
| Kill Tracker | Pending |
| Reward Wall | Pending |
| Analytics Selector | Pending |
| Bosstiary | Inspected through Cyclopedia; shortcut route pending |
| Boss Slots | Inspected through Cyclopedia; shortcut route pending |
| Bosstiary Tracker | Pending |
| Bestiary Tracker | Pending |
| Imbuement Tracker | Pending |
| Weapon Proficiency | Pending; do not confuse with Magical Archive |
| Exaltation Forge | Pending |
| Social | Pending |
| Task Board | Pending |

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
