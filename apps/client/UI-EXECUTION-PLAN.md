# Native client fidelity execution plan

Owner direction (2026-10-09): retain inspected Tibia controls, panel organization
and observed behaviors, with a modern Oteryn visual identity. Legacy textures,
pixel-art chrome and rough procedural shortcut sketches are not the visual target. Draft #1942 remains
AUTHORING. This plan is the execution order, not a completion declaration.

## Acceptance rule

For every screen record its private reference capture, visible controls, dropdown
members, conditional/enabled states, geometry, actual client consumer and engine
requirements. Unknown details remain explicitly unverified. A screen passes only
when its compiled native capture and interactions satisfy both visual and behavior
criteria. Serialization tests and panel/control counts do not establish parity.

Visual criteria: coherent Oteryn slate/gold surfaces, legible typography, licensed
consistent icons, hierarchy, group/row order, proportions, frames,
icons, table columns, spacing, scrolling and default dialog reachability.
Behavior criteria: selection, dependencies, add/remove/reorder, conflict checks,
Apply/Cancel/reset, failed save, restart, modal focus and actual effect in game.
A prepared control without its consumer is not a completed function.

## 1. Preferences shell and HUD

- Compare with private options-basic.png, hud.png and the advanced page captures.
- Remove extra reference-page search/footer content that steals the required body
  area; retain Oteryn-specific access through clearly separate navigation.
- Match own/other HUD groups, Harmony radio controls, arc group, condition table,
  independent HUD/bar flags, condition selection/order and status-bar controls.
- Verify all known controls at the native default dialog size in PL and EN.
- Resolve undiscovered arc sizes, conditions and ordering from the reference;
  do not invent membership, selected defaults or server state.
- Integrate available actor/resource projection without fabricating missing data.

## 2. Remaining preference functions and hotkeys

- Complete Controls, Interface, Console, Game Window, Action Bars, Shortcuts,
  Graphics/Effects, Sound/Battle/UI Sounds, Misc, Gameplay, Screenshots and Help.
- Implement the actual General/Action/Custom Hotkeys tables, profile lifecycle,
  primary/secondary bindings, chat-on/off routing, filtering and conflict handling.
- Use existing typed action-row bindings and accepted supported commands. A profile
  name or arbitrary text box cannot stand in for a working action registry.
- Implement safe local consumers first (layout, input, display, capture, clipboard,
  audio); map server mechanics separately to accepted owning contracts.
- Preserve settings compatibility, bounded files and authentication safeguards.

## 3. Gameplay panels and dialogs

- Docked HUD: minimap/tools, equipment, containers, battle, spells, skills, VIP,
  chat and action bars; verify opening, closing, resizing and shortcut placement.
- Progress/content: quest log/tracker, Compendium, all inspected Cyclopedia pages,
  Bestiary/Charms/Bosstiary/Boss Slots, Highscores and player information.
- Activities: Prey, rewards, analytics, party/team finder, Forge, Wheel,
  Gem/Fragment workshops, Weapon Proficiency, Task Board and social panels.
- Match each inspected context menu and dialog independently. Record any account,
  level or server restrictions. No fabricated creatures, inventory, progression,
  allocation, rewards or commerce may make a panel appear functional.
- Replace rough placeholder artwork with independently authored consistent assets;
  original proprietary textures/icons are private reference, not redistributable.

## 4. Login and whole-client qualification

- Welcome/sign-in, account/character selection, creation, world/channel choice,
  loading, errors/retry, account switching and return from the game.
- Continue the approved secure native-session work; browser-independent recurring
  sign-in requires the accepted vault/device-session integration. No password
  storage or credential webview workaround.
- Verify the real test account/character path with the current local Platform and
  Game server; preserve generation fencing and WorldId/ChannelId separation.
- Run applicable native tests/Clippy/release, capture actual Windows screens and
  exercise resize/restart/input. Linux GUI parity needs its own runtime evidence.

## Delivery and current state

Save coherent qualified batches to the existing draft using the sole guarded
publisher, fresh expected remote head and recovery bundle. No merge, production
change, database reset or unrelated diagnostic overwrite is authorized here.

Previous authoring checkpoints prepared dedicated option compositions, but most
pending settings and several panels still lack consumers. UI-FIDELITY.md is the
screen matrix. The current first acceptance target is the preferences shell/HUD;
remaining phases stay open until their own evidence exists.
