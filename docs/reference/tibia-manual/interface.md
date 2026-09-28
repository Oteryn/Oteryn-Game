# Tibia manual notes: interface

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=interface>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `add758397b3019c625e418023b0598132e6b2db5de71fa0c24e56c739bbe4663`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Client / UI (with the server rules behind it)
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §interface <heading>, capture 2026-09-28`.

## 3 Interface
- [client] Minimap, inventory, status bars, buttons, sidebars, containers are movable/resizable; unlimited sidebars space permitting.
- [client] Sidebar layout (position/size) persists per client install across logout and restart, but not reliably across different computers/resolutions.

## 3.1 Game Window
- [client] Main viewport, always character-centered.
- [client] Resizing it (drag console's top bar) inversely resizes the console; more sidebars forces a smaller game window.

## 3.2 Maps
### 3.2.1 Minimap
- [client] Bird's-eye view centered on the character, wider coverage than the game window; character shown as a black/white cross.
- [client] Left-click on the minimap moves the character there (same reachability constraint as game-window click-to-move).
- [client] Unexplored tiles render as black fog.
- [server] Marks: right-click a spot → "Set Mark" (or edit/delete an existing one); each mark has a chosen icon + short text description; marks can also be set by asking certain NPCs (e.g. for depot/bank/shop locations).
- [client] Compass rose: click a direction to scroll; Alt+cursor-keys scroll alternative; clicking the rose center re-centers on the character (same as the "Centre" button); also indicates day/night when light effects are disabled.
- [client] Magnifying glasses: top = zoom out, bottom = zoom in; Alt+Home / Alt+End are the equivalent shortcuts.
- [client] Level Selector slider: switches the minimap to floors above/below the character's current floor, when such floors exist; Alt+PageUp / Alt+PageDown shortcuts.

### 3.2.2 Cyclopedia Map
- [client] Opened via its button, or "M" if console chat is disabled.
- [client] Display filter: which marks show (single/all); discovered areas can also show passages/NPCs.
- [client] Views: Surface (in-game graphic of surface + floors above) vs Map (classic minimap style); floors below surface always use Map view. Level separator shows multiple floors at once, only from surface or ≥1 floor above ground.
- [client] Navigation: compass rose, magnifying glasses, level selector (as minimap), plus free mouse pan/scroll-zoom.
- [server] Map-click movement limited to the same floor and to a reachable tile.
- Discover Areas and Subareas:
  - [server] Click highlights the subarea, shows parent area + % discovered.
  - [server] Each subarea has several points of interest (POIs); first POI found starts a quest; POI progress is per-character, not shared, and resettable.
  - [server] Subarea "discovered" at 7 POIs found; subarea count per area varies.
  - [server] Unlocks by % of area's subareas discovered: ≥30% NPC locations shown; ≥70% stair-symbol passages/teleports shown; 100% creature list shown on click (unrevealed Bestiary entries appear as silhouettes) plus a 1-hour advance raid warning.
  - [server] Full area discovery grants a permanent speed bonus there; more fully-discovered areas stack a higher bonus.
  - [server] NPC Charos milestones: 10 areas = new outfit; 15 = 1st addon; 20 = 2nd addon.
- Improved Respawn Rate:
  - [server] One area/server save is randomly chosen for a faster respawn rate that day; a second area can be selected by player gold donation reaching a minimum by server save.
  - [server] Tie at highest donation = random winner; winning area's gold resets to 0, others carry their gold forward.

## 3.3 Status Bars
- [client] Two bars: health (red) and mana (blue); bar fill shows current/max ratio, adjacent numbers show exact current values.
- [client] Configurable placement: rightmost sidebar, one/both sides of the game window, both, or hidden entirely — via Options → Interface → HUD, or by clicking a status bar directly.
- [client] Around-game-window status bars are further customizable: can add level, magic level, and any subset of skills; position and visual style (multiple styles available) are also configurable.

## 3.4 Inventory
- [client] Shows the active character's equipped-item portrait, divided into fixed body slots.
- [server] Items may only go in their designated slot type (e.g. a sword cannot go in the head slot).
- [server] Some items have vocation and/or level requirements to equip (e.g. heavy armor needs knight/paladin; witch hat needs sorcerer/druid).
- [client] Inventory panel can be minimized via a top-left button.

### 3.4.1 Body Slots
- [server] Armor Slots (4): feet, legs, torso, head.
- [server] Weapon Slot: any weapon (sword/axe/bow/rod/wand/snowball); some weapons need min. level and/or vocation.
- [server] Shield Slot: shield, or spellbook (sorcerer/druid), or quiver (paladin); knights/druids/monks/sorcerers lose this slot while wielding a 2H weapon; paladins keep quiver+2H bow/crossbow.
- [server] Amulet Slot: necklace/amulet. Ring Slot: rings only.
- [server] Container Slot: bags/backpacks; also exposes "Manage Containers" via context menu.
- [server] Extra Slot: free-form (e.g. a torch).

## 3.4.2 Blessings Dialog
- [client] Opened by clicking the ankh icon; shows which blessings protect the character and links to the Store to buy missing ones; also shows how blessings/promotion/Premium status affect the next-death penalty.
- [client] "History" sub-view logs when/where each blessing was gained and which death consumed it.
- [server] Ankh color states: grey = no blessing protection; green = protected by all regular blessings plus the PvP blessing "Twist of Fate" (only where available on that world); yellow = protected by at least one blessing.
- [server] Adventurer's Blessing (special yellow state, inventory also renders yellow): Open-PvP-world-only; on a PvP death while protected, character loses no items, experience, or skill points; the blessing is permanently lost the moment the character attacks another character first, or upon first reaching level 21 — whichever comes first; inventory reverts to normal grey display once lost.

## 3.4.3 Store Inbox
- [server] All portable Store purchases (potions, runes, decoration items, etc.) are delivered to the Store inbox.
- [server] Store-purchased items are usable only by the purchasing character; Store-purchased furniture/decoration can only be unwrapped in a house owned by that same character.
- [client] Store inbox is accessible from anywhere in the world (no travel needed to restock).
- [server] Restriction: cannot purchase potions or runes while the character has a protection-zone block or a battle sign active.

## 3.4.4 Indicators
- [server] Soul Point Indicator: max 100 soul points normally, 200 for promoted characters; consumed by casting rune spells and certain item-generating spells (ammo/food); regenerated by earning experience ≥ the character's level from a single monster kill, or by sleeping in a bed.
- [server] Capacity Indicator: current carry capacity; attempting to pick up an over-capacity item produces an error; item weight is viewable via Look.
- [server] Condition Indicator: shows active special conditions (e.g. poison, burning, logout block) via corresponding symbols.

## 3.4.5 Store Button
- [client] Opens the Store, where Tibia Coins purchase in-game products.

## 3.5 Combat Controls
- [client] 3 primary buttons to the right of the inventory (movement mode + expert mode toggle); Expert Mode unlocked adds 4 more buttons below the Secure Mode button.

### 3.5.1 Combat Movement
- [server] "Stand While Fighting" (default): character holds position unless pushed or manually moved.
- [server] "Chase Opponent": character auto-closes distance to its current attack target via the shortest path; manually stoppable by switching back to Stand mode or moving via mouse/keys.
- [server] If "Auto Chase Off" is disabled in Options, releasing a movement key while chasing reverts the character back to "Chase Opponent" mode automatically.

### 3.5.2 Expert Mode
- [client] Toggle button enables/disables 4 additional PvP-targeting modes; only one active at a time.
- [server] Dove Mode (default): can only attack/block characters that have been aggressive toward the player.
- [server] White Hand Mode: can attack/block characters aggressive toward the player OR toward a party/guild member.
- [server] Yellow Hand Mode: can attack/block any skulled character, except skulled characters in the player's own party or guild.
- [server] Red Fist Mode: can attack/block any character regardless of skull status, except party/guild members; unavailable to characters carrying a black skull; on hardcore PvP worlds this mode is permanently forced on and cannot be changed.

### 3.5.3 Secure Mode
- [server] Secure Mode (default): cannot attack characters without a skull mark (prevents accidental attacks); self-defense is always allowed (aggressors get auto-marked); does not prevent incidental skull marks from indirect damage (e.g. fire fields, explosions).
- [server] Normal Mode: can freely attack any character; attacking an unmarked character grants the attacker a skull mark.
- [server] This switch has no effect on Optional PvP or Hardcore PvP worlds.

## 3.6 Shortcuts
- [client] A configurable row of buttons below inventory/combat controls; Options menu controls which shortcuts show and their order.
- [client] "Stop" button (or Escape key) halts the character's current action.
- [client] "Shortcuts" button toggles visibility of the whole shortcut row.

### 3.6.1 Skills
- [client] Opens a subwindow with experience, level, HP, mana, magic level, and skills; numeric value plus a progress bar per stat.
- [client] Hover a bar for a tooltip with percent progress; experience tooltip also shows XP needed for next level and, while hunting, XP/hour + estimated time to next level.
- [server] XP/hour estimate needs ≥15 minutes of continuous hunting for accuracy; stopping for 15 minutes resets the counter to 0.
- [client] Right-click the skill window to reset the experience counter, or to hide specific/all bars.

### 3.6.2 Battle List
- [client] Lists all characters/monsters within battle range: mini-portrait, health bar, skull marks; supports directing attacks/spells directly from the list.
- [client] Arrow-button filter row: toggle visibility per group (players, knights, paladins, monks, druids, sorcerers, summons, NPCs, monsters, non-skulled players, party members, guild members).
- [server] Premium players can open secondary battle lists (e.g. one per group) via a dedicated button.
- [client] List-button menu: rename the list, sort by display time, distance, hit points, or name.

### 3.6.3 Party List
- [client] Lists current party members and their summons, with mini-portraits.
- [server] Health bar, mana, and skull marks are only shown for members within 30 fields of the viewer.
- [client] Same group-filter (players, knights, monks, paladins, druids, sorcerers, summons) and rename/sort-by-time/distance/HP/name options as Battle List.

### 3.6.4 VIP List
- [server] Capacity: 20 characters for free accounts, 100 for Premium (matches §controls_communication 4.2.6).
- [client] Shows online status of tracked friends/enemies.

### 3.6.5 Quest Log
- [client] Opens the quest log listing started/completed quests.

### 3.6.6 Quest Tracker
- [client] Opens a tracking window for currently tracked quests; a quest is added to it via a checkbox inside its Quest Log entry.

### 3.6.7 Spell List
- [client] Detailed per-spell info list; filterable by criteria such as vocation, learned-only, or spell group.

### 3.6.8 Unjustified Points
- [server] Widget shows unjustified-kill count, remaining time on red/black skulls, and count of open PvP situations participated in.
- [server] Progress-bar fill level/color reflects proximity to earning a skull; involvement in unjustified kills fills the bar faster.

### 3.6.9 Prey Dialog
- [server] Select/activate a Prey; requires the character to have left Rookgaard or Newhaven first.

### 3.6.10 Kill Tracker
- [client] Shows creatures still needed for an active Prey / Bounty Task / Weekly Task, bonuses/rewards, kills so far, and remaining Prey time; clicking inside the window opens the Task Board.

### 3.6.11 Task Board
- [client] Start Bounty or Weekly Tasks, or visit the Hunting Task Shop.

### 3.6.12 Analytics Selector
- [client] Hunting-analysis tools: Hunting Session Analyser (duration, XP gain, XP/h, loot, kills), Loot/Supply Analysers (last-hour items + gold value by NPC/market/custom price), Impact/Damage Input Analysers (damage or healing dealt/received), XP Analyser (XP/h, with/without bonuses), Drop Tracker (per-item drop-rate, set up via Cyclopedia), Party Hunt Analyser (session stats for up to 50 members individually + aggregate), Boss Cooldowns (per-boss wait time, sortable by name/cooldown).

### 3.6.13 Reward Wall
- [server] Daily reward pickup requires standing next to a reward shrine, or using an Instant Reward Access (Store item); reward is delivered to the Store inbox.
- [server] Pickup is allowed once per 2 server saves.
- [server] "Reward streak" increments each consecutive daily pickup window; a streak ≥2 grants resting-area bonuses.
- [server] Missed days can be covered with a "daily reward joker" to preserve the streak/bonuses; 1 joker granted per account on the 1st of each month; max 3 jokers held at once.

### 3.6.14 Compendium
- [client] Short summaries of various game topics/features; button highlights when new entries are added.
- [client] Hosts the Player Guide, whose content unlocks progressively as the character levels up.

### 3.6.15 Player Guide Widget
- [client] Tracks Player Guide progress; surfaces notifications for level-ups and newly unlocked spells.

### 3.6.16 Cyclopedia
- [client] Nine sections: Items, Bestiary, Charms, Map, Houses, Character, Bosstiary, Boss Slots, Magical Archive.
- Items: [client] per-item Market info (buy/sell NPCs+cities, NPC price, world average price); starts Drop Tracker tracking. [server] Opt-in loot-value coloring (NPC price / market average / Premium custom value) feeds Analytics Selector profit/waste calc. [server] Per-item checkbox to loot-always or loot-skip.
- Bestiary: [server] Excludes boss/raid monsters. [server] Silhouette until first kill unlocks name/sprite/difficulty. [server] 3 unlock stages: 1 = HP/speed/armor/common loot; 2 = uncommon loot, resistances, hunting spots; 3 = semi-rare/rare loot; required kill counts scale with difficulty. [server] Completed entry unlocks Charm assignment.
- Charms: [server] Unlocked with Charm Points; target creature needs a completed Bestiary entry. [server] 1 creature at a time per Charm; max 2 Charms/creature (1 major+1 minor). [server] Active cap: 2 (free) / 6 (Premium, more via Charm Expansion). [server] Removal costs a level-scaled fee. [server] Charm levels 1→3 via Charm Points; "reset all" costs a level-scaled fee except the first reset (free).
- Map: [client] same as Cyclopedia Map §3.2.2 (area/subarea browsing, respawn-rate donation).
- Houses: [client] browse rentable houses, auction-buy, peer-to-peer sell/buy (see houses section).
- Character: [client] General Stats (XP, skills, badges, offence/defence/misc incl. blessings); Battle Results (30-day deaths + PvP kills); Achievements; Item Summary (inventory/depot/stash/inbox/Store inbox); Appearances (outfits/mounts/familiars); Store Summary; Character Titles ([server] unlocked titles selectable as display title, visible to others per Friends-window visibility settings).
- Bosstiary: [server] Silhouette until first kill. [server] Frequency tiers: Bane (>1 kill/20h), Archfoe (1 per 20–48h), Nemesis (>48h or rarer). [server] Kill thresholds — Few: 1–24(Bane)/1–4(Archfoe); Prowess: 25/5/1; Expertise: 100/20/3; Mastery: 300/60/5 (Bane/Archfoe/Nemesis).
- Boss Slots: [server] Requires Prowess level. [server] Free swap once/server save, further same-day swaps cost rising gold. [server] +25% extra equipment-loot-set chance (capped-drop items excluded); +25% more at Mastery. [server] Boss Points (earned per progress unlock) raise the bonus up to 182% total; 2nd slot at 1500 points.
- Boosted Boss: [server] Random Archfoe daily; kills count 3x for Bosstiary; bonus loot-drop chance; cooldown force-reset for every character at server save.
- Magical Archive: [client] Per spell: vocations, level requirement, animation/AoE; Combat Stats (mana cost, group, base power, cooldown, magic type, range, magic-level/weapon scaling); price/source/cities/lore; rune spells get dedicated group/level/cooldown details.

### 3.6.17 Bestiary Tracker
- [client] Shows kill progress toward unlocking further Bestiary stages for a chosen creature.

### 3.6.18 Bosstiary Tracker
- [server] Tracks kills needed for a selected boss's next progress level; a boss must have been killed at least once before it can be tracked.

### 3.6.19 Bosstiary Dialog
- [client] Shortcut button to open the Bosstiary dialog inside the Cyclopedia.

### 3.6.20 Boss Slots Dialog
- [client] Shortcut button to open the Boss Slots dialog inside the Cyclopedia.

### 3.6.21 Exaltation Forge
- [client] Shows collected forge resources and what they can be spent on.
- [server] Actual forge operations (tier transfer, fusion, resource conversion) happen at the physical Exaltation Forge, located north of the Adventurers' Guild.

### 3.6.22 Wheel of Destiny
- [server] Spend promotion points here to unlock power-increasing perks; intended for experienced characters.

### 3.6.23 Social Dialog
- [client] 7 tabs: Join Team, Assemble Team, Friend List, Invitations, Search, Badges, Friends Config.
- Join Team: [client] filterable search of open teams; pencil icon sends a join request. [server] team leader accepts/invites/rejects; accepted or invited members auto-join the team channel (open until team is full).
- Assemble Team: [client] set level range (shield icon auto-fills the shared-XP-optimal range), vocations, size, free slots, start time, activity (only field required by default; blank = open to all). [client] can add existing party members directly; "Start Assembling Team" publishes it; pencil icon accepts/rejects/invites applicants; "Unlist" removes it. [server] invited-not-yet-accepted characters can join the team channel early to discuss.
- Friend List: [client] friends' main characters + basic info; pencil icon edits an entry (e.g. friend group).
- Invitations: [client] sent/received open friend requests, accept or reject.
- Search: [client] cross-world character search, non-hidden results only; invite as friend or blacklist from inviting; click picture for more info.
- Badges: [client] badge list + requirements; choose which earned badges others can see.
- Friends Config: [client] per-group visibility settings for what others can see.

### 3.6.24 Imbuement Tracker
- [client] Lists imbuements on currently equipped items and their remaining duration.

### 3.6.25 Weapon Proficiency
- [client] Shows weapon-specific progress and lets the player select perks for that weapon.

### 3.6.26 Highscores
- [client] Top-character leaderboard; filterable by vocation, game world, world type, or specific skill.

### 3.6.27 Client Help
- [client] Toggle mode; while enabled, hovering any interface element shows an explanatory infobox.

### 3.6.28 Options
- [client] Opens the Options menu (also reachable from the title screen).
- [client] Only "most important" settings show by default; "Show Advanced Options" (bottom of menu) reveals the full option set.
- [client] Per-option help text appears on mouse-over of the control or its info icon.

### 3.6.29 Manage Buttons
- [client] Opens the Control Buttons options: sort, hide, and show available control buttons (customizes §3.6's shortcut row contents/order).

### 3.6.30 Leave the Game
- [client] Returns to title screen/character list for quick re-login.
- [server] Blocked by an active logout block; character list auto-closes after 30s idle there.
- [server] Closing the client without proper logout can leave the character in-game if logout-blocked or disconnected.
- [server] Idle auto-logout after 16 minutes of no input; red warning shown at 15 minutes idle.

## 3.7 Console
- [client] Handles player-to-player and player-to-NPC communication; log area + entry line; usually defaults to local chat.
- [client] Switch channels via Tab key or clicking a tab.
- [client] Drag a channel tab to the far right of the console to open a read-only mirrored copy that displays the same messages; the copy cannot be typed in (must use the original tab to send); closing the original also closes its copy.
- [server] Optional-PvP-world Exiva permission control: players can whitelist who may locate them via the Exiva spell — by group (e.g. all guild members, all party members) or by individual character/guild name.
- [server] Guild-war override: during an active guild war, all members of both participating guilds can always Exiva-locate each other regardless of individual Exiva permission settings.
- [client] "No Entry" icon (top-right of console) opens the ignore/whitelist dialog (see §controls_communication 4.2.4–4.2.5).

## 3.8 Cooldown Bar
- [client] Displayed directly above the console; shows remaining cooldown per spell/spell group.
- [server] Casting a spell puts a cooldown on both that spell and every spell group it belongs to; spell and spell-group cooldown durations are independent and not necessarily equal.
- [client] Cooldown remaining is shown as a shrinking white overlay line on the icon; icon greys out (spell group) or disappears (individual spell) once the cooldown ends.
- [client] Spell-group icons are always visible; individual-spell icons show only while their cooldown is active.
- [client] The whole cooldown bar can be hidden via a checkbox in Options → General.

## 3.9 Action Bars
- [server] Multiple bars, 50 buttons/bar; configuration can be stored per hotkey preset.
- [client] Right-click a button to assign: Spell (from spell list, optional known-only filter, parameter field for directional/targeted spells, drag&drop supported); Object (crosshair-select item; equip/unequip or use; runes/potions choose aim target: self/current target/cursor/crosshair) — Smart Mode needed to toggle equip/unequip of time-limited items (e.g. soft boots) from one button, always using whichever matching item is closest to the first container; Multi-Action (up to 3 fallback slots — spell/item/text; fires slot I, falls to II then III if the prior slot is on cooldown); Text (auto-send on press, or placed in entry line needing Enter); Passive Ability (bind purely to show/track its cooldown, even if not yet unlocked); Hotkey (key-capture dialog; red warning if already bound elsewhere).
- [client] Lock toggle: unlocked allows drag&drop reassignment (hotkeys stay put); locked blocks drag&drop (context-menu assignment still works) to prevent accidental changes mid-hunt.

## Open questions for Oteryn
- No numeric detail on exact XP thresholds, Bestiary kill-count-per-difficulty formula, or Charm point costs.
- Boss-slot swap gold cost formula (rises "with each change") is not quantified.
- Exact area-discovery speed-bonus values are not given, only that it scales with fully-discovered area count.
- Reward-streak resting-area bonus magnitude beyond "streak ≥2" is unspecified.
- Action Bar Multi-Action: unclear if slot III itself checks cooldown or always fires when I/II are on cooldown.
- No stated numeric cap on simultaneous sidebars (only "space permitting").
