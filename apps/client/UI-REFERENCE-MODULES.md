# Reference interaction inventory: panels and modules

Continuation scope is governed by the consolidated
[coverage index](UI-REFERENCE-COVERAGE.md). This file records a subset of the
audit; its beginner-account restrictions and historical pending notes do not
invalidate the mainland-character observations in `REFERENCE-AUDIT.md`.

Audit resumed on 2026-10-09. This document records reference requirements, not
implemented Oteryn capabilities. The settings inventory is in
`UI-REFERENCE-SETTINGS.md`; earlier observations are in
`UI-REFERENCE-OBSERVATIONS.md`.

## Evidence boundary

The reference client was reached on Synology and a fresh capture confirmed the
world, docked panels and Store modal. Closing Store exposed the game viewport.
Private captures: `/tmp/oteryn-resume-current.png` and
`/tmp/oteryn-resume-panels.png`. No purchase was made.

The menu and Wheel/Prey captures below were retained from the preceding audit and visually
re-inspected during this continuation. They establish visible controls, not the
results of changing filters, sorting populated lists or persisting preferences.
Account values and selected filters must not become product defaults. Original
screenshots and artwork are not included in this repository.

## Spell List

Evidence: `/tmp/oteryn-spell-menu-current.png`.

- Docked panel with search, selectable spell rows, and selected-spell details.
- Details visible in this sample: formula, vocation, group, type, magic type,
  cooldown, mana/soul cost, minimum level, premium requirement, Show More.
- Filter checkboxes: Character Vocation, Character Level, Learnt Spells.
- Vocation choices: Druid, Knight, Paladin, Sorcerer, Monk, All Vocations.
- Group choices: Attack, Healing, Support, All Spell Groups.
- Account choices: Premium Account, Free Account.
- Spell types: Rune Spells, Instant Spells.

Still unverified: combination semantics, aggregate-checkbox behavior, full
Show More contents, empty results, selection retention and saved filter state.
Acceptance requires actual filtering of game-owned spell data and correct
details for different spells; a generic form with these labels is insufficient.

## VIP List

Evidence: `/tmp/oteryn-vip-context-current.png`.

- Context menu: Add new VIP; Sort by name; Sort by type; Sort by status;
  Hide offline VIPs; Show groups.
- The observed list was empty. This proves menu availability only.

Still unverified: add/edit dialogs, validation, group membership controls,
sort ordering/ties, online updates and persistence. Do not create contacts on the
reference account as an implicit part of inspection.

## Party List

Evidence: `/tmp/oteryn-party-context.png`.

- Context menu includes Edit Name.
- Eight sorting choices: ascending/descending Display Time, Distance,
  % Hit Points, and Name.
- The captured menu alone does not establish what Edit Name changes.

Still unverified: populated rows, distance units, missing/out-of-range data,
tie ordering, refresh behavior and the Edit Name dialog. Invite/leave/leadership
operations are not covered by this capture.

## Wheel: Gem Atelier and Fragment Workshop

Evidence: `/tmp/oteryn-wheel-gem-atelier.png` and
`/tmp/oteryn-wheel-fragment-workshop.png`, re-inspected during this continuation.
Both belong to a window explicitly titled Wheel of Destiny - Preview; preview
availability does not prove that the character can modify its actual wheel.

Gem Atelier has vessels at left, selected-gem modifiers above the main list,
search/clear, affinity and quality dropdowns, Locked only, and pagination.
The observed collection is empty. A separate revelation column presents three
gem tiers, inventory counts, Reveal buttons and costs. Dropdown memberships,
populated gem details, locking and successful revelation are still unverified.

Fragment Workshop has search/clear, a filter dropdown, paginated modifier tiles
and a separate selected-modifier grade ladder. The screenshot reports 69 mods
across three pages; only page 1 was inspected here. The selected Dodge modifier
shows grades I-IV and an Enhance control with two resource costs. Values in this
sample are reference data, not accepted Oteryn balance constants. No enhancement
was attempted; selection changes, other pages and mutation responses are pending.

## Prey restriction

Evidence: `/tmp/oteryn-prey-current.png`, re-inspected during this continuation.
The modal states that characters which have not reached the main continent
cannot select a prey, with an OK dismissal button. This establishes the restricted
state only. Slot selection, bonuses, rerolls and their costs remain unverified.

## Targeted gap checks: Quest Tracker, backpack and chat

Fresh captures from the restored beginner-character session, 2026-10-09:

- `/tmp/oteryn-quest-tracker-menu-verified.png`: Quest Tracker header menu has
  Remove all quests, Remove completed quests, Automatically track new quests,
  Automatically untrack completed quests. The last two are independent checkboxes.
  No removal or toggle was executed. Empty-list inspection does not establish
  automation effects, persistence or populated-entry context menus.
- `/tmp/oteryn-backpack-menu-verified.png`: ascending/descending sort choices for
  Name, Weight, Expiry and Stack Size; Sort Containers First; Sort Nested Containers;
  Use Manual Sort Mode; Move contents to Obtain Container; Move Nested Containers.
  The last four prefixed with Sort/Use/Move must retain their distinct semantics:
  nested sorting and nested moving are separate controls. No sorting or item
  movement was executed, and recursive confirmation dialogs remain unverified.
  Container filtering is not shown by this menu and remains an independent gap.
- `/tmp/oteryn-chat-tab-menu-verified.png`: Server Log tab context menu exposes
  Show in Read-Only Tab, Save Window and Clear Window. No save or clear occurred.
  These observations do not establish the menu of a player channel or individual
  speaker. Local Chat probes did not yield a captured menu; no absence claim is
  made. Participants, mute and secondary-chat controls remain pending.

## Comparison checkpoint (implementation)

Compared against Oteryn client source at
`ea3a33101a285df26d73cee96b7b7fbeb79a32e2`.
`src/panel_catalog.rs` declares Spell List details, VIP name/status/group and
Party member/role/status. Those field catalogues do not enumerate the complete
menus above and are not acceptance evidence for filtering or sorting behavior.
Runtime consumers and interaction tests still require a separate implementation
pass. No panel is marked complete by this document.

## Cyclopedia: Items

Re-inspected evidence: `/tmp/oteryn-cyclopedia-item-details.png`.
The observed layout contains independently scrollable categories, item results,
basic details, Sell To and Buy From lists; search/clear; Level, Voc., 1H and 2H
filter controls and a dropdown whose full membership remains unverified.

The selected armour exposes armour value, imbuement slots, weight, market
tradeability, body position and classification. These are item-dependent fields,
not a complete schema for every category. Vendor entries include price and
residence. Valuation has NPC Buy Value / Market Average Value alternatives,
average market price, Prefer Own Loot Value input and resulting value. Independent
controls track item drops and exclude the item from quick looting. Manage
Containers is a separate route. No custom price, tracking or exclusion was saved.

Acceptance still needs other item categories, full dropdown lists, missing-market
data handling, input validation, persistence and the actual loot consumers.

## Cyclopedia: Bosstiary and Boss Slots

Re-inspected evidence: `/tmp/oteryn-bosstiary-current.png` and
`/tmp/oteryn-boss-slots-current.png`.

Bosstiary filters include Bane, Archfoe, Nemesis, No Kills, Few Kills, Prowess,
Expertise and Mastery. An eight-card page displays silhouettes for unknown bosses,
total kills and three progress tiers. Search/clear and previous/next pagination
are separate controls. The capture reports page 1/40; the remaining pages were
not inspected in this continuation.

Boss Slots separates boss-point progress, current/next equipment-loot bonus,
two selectable-slot regions and a central boosted boss with kills, tiers,
equipment-loot bonus and kill multiplier. Both personal slots are locked in the
sample: the first requires Prowess for any boss; the second displays 1500 Boss
Points. These are observed reference conditions, not approved Oteryn balancing.
Unlocked selection, change costs, confirmation and server updates remain pending.

## Cyclopedia: Magical Archive

Re-inspected evidence: `/tmp/oteryn-magical-archive-filter.png`.
The same visible filter families as Spell List accompany a selected spell's
name, words and restriction indicator. Combat Stats and Additional are separate
tabs. Combat Stats displays mana, spell group, base power, scaling, cooldown,
group cooldown, magic type and range. Unavailable values appear as dashes.
Assign Spell to Action Bar is a distinct action; its complete flow is pending.
The Additional-tab capture did not render during this pass, so its contents are
not inferred from its filename.

## Wheel: main preview

Re-inspected evidence: `/tmp/oteryn-wheel-preview.png`.
The main view is a radial node graph with a left selection/details region and
available promotion points. Right-side groups are Dedication Perks (hit points,
mana, capacity, mitigation multiplier), Conviction Perks, Vessels and Revelation
Perks. The preview explicitly says free accounts can see selected perks/gems
but do not receive their effect. This is separate from point availability.
Node dependencies, hover details, allocation, reset and save remain unverified.

## Analytics Selector and analyser controls

`/tmp/oteryn-hunting-analyser.png` and `/tmp/oteryn-analytics-complete.png`
show the selector, not completed analyser contents. It exposes Hunting, Loot,
Supply, Impact, Damage Input, XP, Drop Tracker, Party Hunt and Boss Cooldowns.

Fresh evidence `/tmp/oteryn-audit-loot-live.png` confirms that selecting Loot
opens a docked Loot Analyser with Gold Value and Per Hour. The fresh context-menu
capture `/tmp/oteryn-audit-loot-context.png` shows Reset Data, Loot Per Hour Gauge
and Loot Per Hour Graph; the latter two are marked premium features. No reset
or preference toggle was executed. The sample is zero-valued and establishes
neither live accumulation nor valuation correctness.

Retained composite `/tmp/oteryn-analyser-menus.png` was visually re-inspected:

- Impact: Reset Data, Reset All-Time High, Show Session Values; DPS gauge/graph,
  Damage Types, HPS gauge/graph. Gauges and graphs are marked premium. The visible
  healing section contains total, HPS, maximum HPS and all-time high.
- Damage Input: Reset Data, Show Session Values, Show Damage Graph, Show Damage
  Types, Show Damage Sources, Copy to Clipboard.
- XP: Reset Data, Show Raw XP, XP Per Hour Gauge, XP Per Hour Graph; the latter
  two are marked premium.
- Drop Tracker: Add Tracked Drop is visible. Its successful add flow remains
  unverified; `/tmp/oteryn-drop-form.png` shows Cyclopedia Items, not a standalone
  completed tracking form.

`/tmp/oteryn-supply-menu.png` confirms Supply's Gold Value and Per Hour display,
but contains no open context menu. `/tmp/oteryn-party-analyser-menu.png` shows
Reset Data of Last Session and Copy to Clipboard. Population, calculations,
session boundaries, reset effects and clipboard contents remain pending.

Evidence correction: `/tmp/oteryn-loot-modal.png`,
`/tmp/oteryn-supply-modal.png` and `/tmp/oteryn-party-hunt-modal.png` contain world
views without the named modals. They must not count as module coverage.

## Remaining audit

### Live continuation after owner restored login

Fresh `/tmp/oteryn-audit-return.png` confirms the restored world session.
The older disconnection checkpoint below is historical, not the current blocker.

- Hunting Analyser selection was followed by a Premium Time Store page
  (`/tmp/oteryn-supply-live-return.png`; filename reflects the next attempted
  navigation, not the actual page). The initial screenshot was taken before the
  transition completed. No purchase occurred; Hunting's interior remains unknown.
- Supply Analyser opens a separate docked panel with Gold Value and Per Hour.
  Its header menu offers Reset Data, Supply Per Hour Gauge and Supply Per Hour
  Graph; both display options carry premium labels. Evidence:
  `/tmp/oteryn-supply-live-verified.png` and
  `/tmp/oteryn-supply-header-verified.png`. No reset/toggle was applied.
- Drop Tracker's Add Tracked Drop opens Cyclopedia Items with no selected item.
  This route is now directly observed, resolving the ambiguity of the earlier
  `drop-form` capture. Evidence: `/tmp/oteryn-drop-live-return.png` and
  `/tmp/oteryn-drop-route-verified.png`. Actual tracking was not enabled.
- Magical Archive's Additional tab shows a Source row and a separately scrollable
  description. Avatar of Balance reports Wheel of Destiny as its source.
  Evidence: `/tmp/oteryn-archive-additional-verified.png`.
- Selecting Destroy Field Rune retains Additional and changes its source to a
  spell-learning level. It also exposes a third tab, Rune Spell, absent for the
  previous spell. The rune header has a magic-level restriction, distinct from
  the creation spell's level requirement. Evidence:
  `/tmp/oteryn-archive-rune-additional.png`.
- Rune Spell displays Mana / SP, Spell group, Restriction, Amount, Cooldown,
  Group Cooldown and Vocations. Evidence: `/tmp/oteryn-archive-rune-spell.png`.
  These creation-spell fields must not be conflated with using the rune.
  Observed numeric values are examples, not approved Oteryn balance constants.
- Assign Spell to Action Bar dismisses the archive in this sample. The full
  capture `/tmp/oteryn-archive-assign-full.png` shows the world and action bars,
  without an assignment modal. Escape was used before selecting any slot.
  Destination selection, persistence and spell execution remain unverified;
  closing the archive alone is not successful-assignment evidence.

The selection-dependent third tab demonstrates why a single spell screenshot
cannot establish complete archive coverage. Other spell types and restrictions
still need samples; this continuation does not claim all conditional fields.

Complete remaining Cyclopedia tabs, full analyser contents and their menus,
and Wheel interactions beyond the preview. Prey's interior requires an
eligible reference character; the restriction is not a completed Prey audit.
Inspect the remaining shortcut panels and their context menus. The 28-shortcut
inventory is not an exhaustive inventory of all dialogs or all interactions.

Live-session checkpoint: the final capture `/tmp/oteryn-audit-restored.png`
shows Connection lost / remote host closed the connection. An earlier fresh
capture displayed the inactivity warning. Restoration of the pre-audit panel
layout could therefore not be confirmed. Further live world-panel inspection
requires the owner to restore the reference login; retained evidence remains
available for read-only inspection.
