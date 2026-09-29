# Tibia manual notes: controls

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=controls>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `b9ea4cbc31589b58124dcf6ac25ae13eb96ed82ee8596e739cb6fbfc2a972a15`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Client / UI (with the server rules behind it)
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §controls <heading>, capture 2026-09-28`.

## 4.1 Basic Controls
- [client] Most interactions go through right-click context menus on a character/item/map field; menu contents vary by target type (item vs NPC vs player vs monster vs own character vs empty map field).
- [server] Action availability rules: "Look" works on everything; "Attack" only on monsters/other characters; "Trade" only on most takeable items; "open in new window" only on containers; clicking a stacked map field lets the player browse the stack.
- [client] Bracketed letters/keys next to a menu action denote a shortcut for that action.
- [both] Two global shortcuts noted here: Shift+left-click = Look at item; Ctrl+click = Use item.

## 4.1.1 Set Character Outfit, Mount and Familiar
- [server] Base outfits are free; further outfits/addons/mounts/familiars require Premium, Store purchase, or quest rewards.
- [client] Opened via context menu on own character → "Customise Character"; Preview pane + Configure pane.
- [client] Configure options: Movement (preview walk animation); Show Outfit (unticking hides the sprite, showing only the mount; locked if no mount selected); Show Familiar (locked if none owned); Show Floor (tiled-floor preview); Addon 1/2 (locked unless earned for that outfit); Mount (locked if none owned); Random Mount (random pick among owned mounts each time "mount" is active).
- [client] Model rotatable via arrows; right-hand column lists owned/purchasable items with an owned-only filter toggle.
- [client] Outfit parts (and matching mounts) are individually colorable; mount coloring can mirror the outfit or be set independently.
- [server] Premium-gated cosmetics are suspended (not lost) while Premium Time lapses, reactivating once restored.

## 4.1.2 Moving Your Character
- Basic Movement: [client] mouse left-click auto-pathfinds shortest route, stops if unreachable. Diagonal walking: hold left mouse on the character and drag diagonally ([server] short extra delay applies); default auto-pathing avoids diagonals. Keyboard: arrows or numpad (7/9/1/3 = NW/NE/SW/SE, Num Lock must be off); keys rebindable (e.g. WASD). Hold Ctrl + direction key turns the character to face that way without moving (useful for directional spells).
- Moving Up/Down Floors: [client] stairs/ramps/floor holes — just walk onto them. [server] a ceiling hole ("rope spot") needs a rope item or the "Magic Rope" spell used on it; impossible otherwise. [client] sewer grates and ladders: right-click → "Use" (ladders: click the floor tile in front). [server] some transitions also work by climbing items or via "Levitate".
- Movement Modes: [server] "Stand While Fighting"/"Chase Opponent" govern auto-combat movement (see §interface 3.5.1). [client] "Follow": right-click a friendly character → "Follow"; stays near target without attacking, auto-cancels if target leaves screen; manually cancel via "Stop Follow" or Escape.

## 4.1.3 Looking Around & Inspecting
- [client] "Look": right-click → "Look", or Shift+left-click, on an item/creature.
- [server] Some items show extended descriptions only when adjacent to or carried by the looking character. Looking at a character can reveal vocation/level. Books/scrolls/letters show only a generic description on Look — reading/writing requires "Use".
- Inspect Character: [server] requires sending/accepting a context-menu inspect request (not unilateral). [client] reveals equipped body slots + name/level/guild/vocation/active preys; copy-to-clipboard icon; close via Escape/Close.
- Inspect Object: [client] on takeable objects, shows graphic/name/weight and (if applicable) capacity, attack/defense, required body slot, imbuements, remaining charges; same copy/close controls.

## 4.1.4 Moving Items
- Drag&Drop: [client] left-click-hold and drag; cursor shows crosshairs + a mini preview. Dropping on a backpack picks the item up (character auto-walks over if not adjacent); dropping on the game window throws it onto that tile. [server] cannot throw through obstacles (walls); heavy items (furniture) move only 1 tile per drag.
- Moving Stacks: [client] opens a quantity dialog (slider or numeric entry); hold Shift/Ctrl/Shift+Ctrl while dragging the slider for bigger steps. Drag shortcuts: Shift = move exactly 1 item; Ctrl = move the whole stack.
- Moving Wall Decorations: [client] drag onto a wall while standing in front of it. [server] only south/east-facing walls accept decorations (not windows, doors, SE corner walls); 1 decoration per wall; some decorations are flagged non-movable.

## 4.1.5 Pushing Creatures
- [client] Left-click-hold a creature/character and drag in the push direction.
- [server] Push happens after a short delay, and only if the path is unobstructed and the target hasn't moved away in the interim.
- [server] Pushing a creature/character while in combat makes the pusher skip their next attack.
- [server] Pushing (self or others) moves slower than normal walking to the same destination.
- [server] Many strong monsters cannot be pushed at all, and some can themselves push items or weaker monsters out of their way.

## 4.1.6 Using Items
- [client] "Use" via right-click; effect depends on item type (torches light, food eaten, scrolls open for editing).
- [client] "Use with ...": cursor becomes crosshairs, left-click the target item/character/map spot to apply. [server] typical targets: ropes, shovels, potions, runes; crafting combos exist (e.g. water on flour → dough for bread).
- [server] Weapons can "use with" smash large destructible items (chairs, chests); each strike has a limited success chance so multiple hits may be needed; most items can't be destroyed this way.
- [client] Frequently-used items can be bound to hotkeys or Action Bar buttons.
- [server] Editable items (blackboards, letters, scrolls) show the most recent editor's name above the input field.
- [both] Ctrl+click on an item is a shortcut for "Use".

## 4.1.7 Rotating Items
- [server] Only applies to large furniture-type items (chairs, chests, etc.).
- [client] Right-click → "Rotate"; each activation turns the item 90° clockwise (e.g. facing south → now facing west).

## 4.1.8 Combat
- [client] Right-click opponent → "Attack" starts the fight; a red frame highlights the attacked target.
- [client] A flashing black frame appears around the player's own character on each incoming hit from a retaliating opponent.
- [client] Stop attacking via context menu → "Stop attack", or Escape.
- [both] Alt+click on an opponent toggles attack on/off as a shortcut.
- [server] Actual damage application is range-gated: the attacker must be in range of the target for the attack to land (character "actively attacks ... as soon as he is in range").
- Cross-reference: consult combat-control modes (Stand/Chase, Expert Mode) — see interface notes §interface 3.5 — before engaging.

## 4.1.9 Looting
- [client] Unlooted corpses can be highlighted (if enabled) until looted, opened, or decomposed; a stack/field stays highlighted while any corpse in it is unlooted.
- [server] Damage-priority: for 10s after a kill, only the top-damage character (or their party) may loot or move the corpse; does NOT apply to dead-player corpses.
- [client] Loot Click: right-click → "Open in new window" → right-click item → "Loot".
- [client] Quick Loot Nearby Corpses: right-click → "Loot corpse"; if enabled, loots up to 30 monster corpses across own+adjacent tiles into bags/backpacks (subject to free space/capacity).
- [server] Loot filtering (skip/accept items) is configured in Cyclopedia → Items (see §interface 3.6.16 / §controls 4.1.11).

## 4.1.10 Sorting Containers
- [client] Open any container → click the 3-horizontal-lines button at top → choose sort criterion: name, weight, expiry (time or remaining charges), or stack size.
- [client] "Sort Containers First": lists all containers ahead of plain items.
- [server] "Sort Nested Containers": recursively sorts contents of nested containers too, but there is a limit to nesting depth this applies to (limit unspecified).
- [client] "Use Manual Sort Mode": disables the default "new/moved item goes to first free slot" behavior so items can be placed in a chosen slot; while active, container names render orange as a visual flag.
- [client] With target containers configured via Manage Containers (Obtain column), sorting can move items directly into those designated containers; a checkbox also extends this to nested-container contents.

## 4.1.11 Manage Containers
- [server] Auto-loot-into-designated-containers is Premium-only; also applies (if enabled) to NPC purchases, quest rewards, and stash/depot retrieval.
- [client] Opened via a container's context menu, the Cyclopedia, or a configurable hotkey.
- [server] Containers assigned to categories (e.g. gold, boots, weapons); one container can hold multiple categories. Unmatched loot routes to the "Unassigned Loot" container, or the main container if none is set. Purchased/stash items with no match always go to the main container.
- [client] Assigned containers show a small icon; hover reveals assigned categories.
- Quick-loot options: "Use main container as fallback" (used if neither category nor Unassigned Loot container exists); "Skipped Loot" (per-item opt-out for "Loot All", set in Cyclopedia Items, cleared via "Clear Skipped Loot List"); "Accepted Loot" (inverse allow-list for "Loot All", cleared via "Clear Accepted Loot List").

## 4.1.12 Keyboard Shortcuts
- [client] All shortcuts below are client defaults and rebindable via Options → General Hotkeys dialog.
- Chat Channels: Ctrl+E close current channel; Tab / Shift+Tab switch to next/previous channel; Ctrl+O open channel list; Ctrl+T open help channel; Ctrl+D switch to local chat.
- Chat Text: Ctrl+C copy selected text (flagged with a link-safety confirmation prompt); Ctrl+A select all text in the current entry field.
- Chat: Return sends the current line; Ctrl+M toggles server messages visibility in the open channel.
- Dialogs: Ctrl+Z bug report; Ctrl+I ignore list; Ctrl+K Options-Hotkeys; Ctrl+Y Prey dialog; Ctrl+U quest log.
- Minimap: Alt+Home/End zoom out/in; Alt+CursorKeys scroll; Alt+PageUp/PageDown switch floor up/down.
- Miscellaneous: Ctrl+H Lenshelp; Ctrl+G change character; Alt+W clear oldest game-window message; Ctrl+L logout; Ctrl+J next hotkey preset.
- Movement: Ctrl+R mount/dismount; Esc stop all actions.
- UI: Ctrl+N show/hide creature names & bars; Alt+F8 show/hide FPS/lag indicator; Ctrl+F toggle fullscreen.
- Windows: Ctrl+P VIP list; Ctrl+B battle list; Alt+S skills window.
- Action Bar: F1–F12 trigger action buttons 1.01–1.12 (i.e. bar 1, slots 1–12).

## Open questions for Oteryn
- Exact pathfinding/tie-breaking algorithm for auto-walk is unspecified.
- Push delay duration and distance-per-push are not numerically specified.
- "Weapon smash" success chance for destructible items has no stated probability.
- Nested-container sort depth limit is not quantified.
- Whether Quick Loot's 30-corpse cap has any cooldown between activations is not stated.
- Whether Ctrl+click also works for "Use with ..." targeting, not just plain "Use", is unclear.
