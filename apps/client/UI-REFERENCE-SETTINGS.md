# Reference interaction inventory: settings

Observed in the running reference client on 2026-10-09. This is a specification
input, not proof of Oteryn implementation. Original images remain private on NAS
under `/tmp/oteryn-*.png`. Current account values are not factory defaults.
No original settings were applied, profiles changed, or assignments accepted.

## Hotkey and action editors

- All nine action bars have a last slot numbered 50. Bottom bars are numbered
  1–3, left 4–6, right 7–9. Separate filters and end-of-list captures checked every
  bar, rather than extrapolating the first. Evidence: `action-bottom-one-end`
  and `action-row-2-end` through `action-row-9-end`.
- Profile Add and Copy open a name-entry dialog; Rename prefills the current
  name. All expose OK/Cancel. Removal was not invoked; its confirmation behavior
  is unverified. Evidence: `profile-add`, `profile-copy`, `profile-rename`.
- General, Action Bar and Custom hotkeys share profile selection and automatic
  profile switching, Chat Mode On/Off and delayed search with clear control.
- Chat On/Off changes the binding context. In the observed profile movement
  letter bindings were empty in Chat On and populated in Chat Off; do not copy
  the account's bindings as product defaults.
- Per-action editing displays action name, chat context and captured key, with
  OK/Clear/Cancel. Movement requires a single key. Capturing a key already bound
  to a different movement action displays a red conflict warning. The probe was
  cancelled, not saved. Evidence: `binding-editor`, `binding-conflict`.
- New Action offers Assign Spell, Assign Object, Assign Text.
- Assign Spell: searchable list, spell graphic/name/words/level, learnt-spell
  filter and parameter field. Full spell-dependent behavior remains data-limited.
- Assign Text: text input and Send automatically, OK/Apply/Cancel. No text was sent.
- Assign Object hides settings and asks for an object selection. Selecting a rope
  opened a preview with Select Object and six use modes: Use on yourself, Use on
  target, With crosshair, Use at cursor position, Equip/unequip, Use. Availability
  depends on the item. OK/Apply/Cancel; assignment was cancelled.
- Evidence: `custom-text-editor`, `custom-object-editor` (picker transition),
  `object-rope-editor`, `movement-chat-on`, `movement-chat-off`.

## Verified choices and full page controls

| Page / control | Observed choices or controls |
| --- | --- |
| Controls / Mouse preset | Classic Controls, Regular Controls, Left Smart-Click; the built-in guide shows an additional loot-button selector under Classic Controls, still to verify interactively |
| Interface / Colourise Loot Value | None, Frames, Corners |
| Game Window / Mark Target Visually | Frame & Highlight, Frame Only, Highlight Only, None |
| Graphics / Engine on Linux | auto-select, Vulkan, OpenGL; platform-specific, not a Windows renderer contract |
| Graphics / Antialiasing | None, Antialiasing, Smooth Retro |
| Graphics | Fullscreen, integral-multiple scaling, V-Sync, No Frame Rate Limit, limit slider and measured current FPS |
| Effects | Light master with dependent ambient light, level separator, clouds/indoor controls; separate own/other/creature/boss-area spell opacity |
| Sound device | auto-select and this machine's Dummy Output; enumerate runtime devices instead of hardcoding |
| Sound | Master, Music plus Anthem, Ambience, Item plus Food and Beverages / Move Item, Event volume |
| Battle Sounds | Own and Other Players groups each have volume, Spells with Attack/Healing/Support and Weapons; Creatures have volume, Creature Noises, Death, Attacks and Spells |
| UI Sounds | Volume; UI interactions; join/leave party; VIP login/logout; console master with Party, Guild, Private Messages in Local Chat, Private Messages, NPCs, Global, Team Finder, Raid Announcements, System Announcements |
| Misc | Confirm purchases, stowing contents, sorting nested containers, moving nested contents; stay logged in for session, connection stability, quick login |
| Gameplay | Allow All to Inspect Me, Auto Chase Off, Quick Loot Nearby Corpses |
| Screenshots | Only Capture Game Window; five-second backlog; automatic master; triggers below; Open Screenshot Folder |
| Help | Client Help, Compendium, Rule Violations, Manual, FAQ, Info, Export All Options, Export Minimap, Import Options/Minimap, Reset All Options |

Screenshot triggers: Level Up, Skill Up, Achievement, Bestiary Entry Unlocked,
Bestiary Entry Completed, Treasure Found, Valuable Loot, Boss Defeated, Death PvE,
Death PvP, Player Kill, Player Kill Assist, Player Attacking, Highest Damage Dealt,
Highest Healing Done, Low Health, Gift of Life Triggered. These are event consumers,
not merely stored booleans.

Game Window toggles: textual effects, messages, private messages, potion sound
effects, spells, spells of others, hotkey notifications, loot messages, loot
highlighting, boosted creature, offline training progress, store notifications in
combat, combat frames, PvP frames, melee attack animation, info banner.

Action Bars: three edge masters and three independent bars per edge; assigned
hotkey, amount, spell parameters, graphical cooldown, cooldown seconds, tooltip,
auto-insert new spells; separate Clear action for each of the nine bars. No Clear
was invoked. The meanings of edge master and retained child selection require
behavioral verification rather than inference from current checkbox values.

Evidence suffixes: `loot-colour-choices`, `target-marking-choices`,
`graphics-engine-choices`, `antialias-choices`, `graphics-effects-current`,
`audio-device-choices`, `sound-current`, `battle-sounds-current`,
`ui-sounds-current`, `misc-current`, `gameplay-current`, `screenshots-current`,
`help-current`, `game-window-options`, `action-bars-options`.

## Full displayed shortcut list

Three overlapping views reached the bottom; available list was empty in this
account. Order is account configuration, not asserted factory order.

1. Skills
2. Battle List
3. Spell List
4. VIP List
5. Lenshelp
6. Quest Log
7. Compendium
8. Cyclopedia
9. Highscores
10. Player Guide
11. Manage Shortcuts
12. Party List
13. Wheel of Destiny
14. Quest Tracker
15. Unjustified Points
16. Prey Dialog
17. Kill Tracker
18. Reward Wall
19. Analytics Selector
20. Bosstiary
21. Boss Slots
22. Bosstiary Tracker
23. Bestiary Tracker
24. Imbuement Tracker
25. Weapon Proficiency
26. Exaltation Forge
27. Social
28. Task Board

Evidence: `shortcuts-current`, `shortcuts-middle`, `shortcuts-end`.
The list is complete for this account; it does not imply every nested panel
interaction has been exercised. Refer to the module inventory for that scope.
