# Live reference observations — 2026-10-09

Status: incomplete audit; not implementation or visual acceptance. The owner
rejected incremental reskinning of incomplete forms. Inspect the original first,
then design the replacement from the complete interaction inventory.

Source: running proprietary reference client on the authorized Synology session.
Only independently written observations are published. Screenshots remain private.
Do not infer factory defaults from the logged-in character's current selections.
No Apply, Reset, profile mutation, purchases or character actions were performed.

## Critical correction: HUD condition list

The six initially visible rows are a viewport, NOT the complete condition list.
Scrolling to the bottom with overlapping captures establishes these 35 entries
in the observed order (which is not asserted to be the factory order):

1. Poisoned
2. Burning
3. Electrified
4. Bleeding
5. Agony
6. Powerless
7. Rooted
8. Feared
9. Drunk
10. Magic Shield
11. Monk’s Virtue bonus
12. Slowed
13. Haste
14. Logout Block
15. Drowning
16. Freezing
17. Dazzled
18. Cursed
19. Strengthened
20. Protection Zone Block
21. In Protection Zone
22. Resting Area
23. Lesser Hex
24. Intense Hex
25. Greater Hex
26. Goshnar’s Taint
27. Bakragore’s Taint
28. Yellow Skull
29. Party Mode
30. White Skull
31. Red Skull
32. Black Skull
33. Orange Skull
34. In Guild War
35. Hungry

Every visible row has an icon and independent Show in HUD / Show in Bar fields.
Headers contain aggregate checkboxes. Up/down ordering controls are beside the
scrollbar; reordering semantics have not been exercised. Preserve original account
preferences during further inspection. The Oteryn `CONDITIONS` array and catalogue
currently cover only entries 1–6; its aggregate action also edits only those six.
The previous claim of a complete HUD page is withdrawn. Larger text and a reachable
footer establish neither complete coverage nor accepted visual fidelity.

Private evidence on the NAS: `/tmp/oteryn-hud-current.png`,
`/tmp/oteryn-hud-conditions-gap.png`, and
`/tmp/oteryn-hud-conditions-02.png` through `-10.png`.
The final two captures repeat the bottom, confirming the end of this list.

## Controls and profile-based hotkeys

Mouse Preset dropdown inspected: Classic Controls, Regular Controls,
Left Smart-Click. The current selection is not a factory-default claim.
Reference hotkey pages share profile selection, Add/Copy/Rename/Remove,
Auto-Switch Hotkey Preset, Chat Mode On/Off, search with clear button, and a table
with Action / First Key / Second Key. Search persists across these pages and
filters after a delay; an immediate capture can still show the previous results.

Oteryn has catalogue placeholders for profiles and bindings, but no auto-switch
entry was found in its source. A pair of generic binding fields is not equivalent
to two bindings for every action under each profile and chat mode.

Observed general-action families: individual/group action-bar visibility;
first/next/previous battle target; channel and chat mode/text operations; dialogs;
loot; minimap; session/miscellaneous; movement; PvP modes; display; docked windows.
Specific dialog routes include Bugreport, Compendium, Cyclopedia Bestiary,
Character, Charms, Items, Magical Archive, Map, Exaltation Forge, Exiva Options,
Ignore List, Manage Containers, Options, Custom Hotkeys, Prey, Questlog,
Reward Wall, Social Assemble Team/Badges/Friend List/Friends Config/Friends
Invitations/Friends Search/Join Team, Task Board, Weapon Proficiency,
and Wheel of Destiny. Presence of a route does not establish its inner screens.

Docked-window actions observed include secondary battle list, Kill Tracker, VIP,
XP analyser, analytics selector, battle list, bestiary tracker, boss cooldowns,
bosstiary tracker, drop tracker, hunting analyser, imbuement tracker, impact
analyser, input analyser, loot analyser, party hunt analyser, party list,
player guide, quest tracker, skills, spell list and supply analyser.

Private evidence: `/tmp/oteryn-general-hotkeys-01.png` through `-10.png`,
`/tmp/oteryn-hotkeys-dialogs.png`, `/tmp/oteryn-hotkeys-chat.png` and
`/tmp/oteryn-controls-presets-20261009.png`. There is an unverified interval
between general captures 02 and 03; do not call the full action inventory complete.

## Action-bar capacity

Filtered the original table to `Bottom Action Bar: Action Button 1.` and scrolled
to its end: it ends at **1.50**, not 1.12. The first page shows 1.01 onward.
Oteryn currently has nine rows with 12 slots per row. This is a known mismatch;
do not extrapolate the capacity of other original rows before checking them.
Private evidence: `/tmp/oteryn-action-hotkeys-current.png` and
`/tmp/oteryn-action-bottom-one-end.png`.

## Custom action entry points

New Action opens a menu with Assign Spell, Assign Object and Assign Text. The spell
editor is a separate searchable spell list with artwork, spell words, level,
Only show learnt spells and a parameter field. This is not a free-text command
editor. It was inspected without accepting an assignment; the other two editors
and all spell-dependent options remain to inspect. Private evidence:
`/tmp/oteryn-custom-action-editor.png` and `/tmp/oteryn-custom-spell-editor.png`
(the latter is a partial modal capture and does not establish its full layout).

## Work still required before a replacement is accepted

Complete the remaining dropdown memberships, scroll extents, custom action editor,
profile dialogs and chat-mode differences. Inspect all shortcut-discovered panels,
context menus and nested module tabs. Existing page screenshots and 213 extracted
setting names are discovery aids, not proof of exhaustive coverage. Record each
control's type, choices, grouping, dependency, action, consumer and evidence.
Then implement complete interaction groups using a consistent modern Oteryn
component system. Preserve working runtime code where useful, but do not let
existing incomplete forms define the target design. Missing game mechanics remain
explicit engine work; their absence is not a reason to omit the intended interface.
