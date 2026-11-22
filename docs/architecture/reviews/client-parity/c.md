# Wheel, Skills and Weapon Proficiency

| Module | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE | Operations |
|---|---:|---:|---:|---:|---:|
| game_wheel | 16 | 7 | 11 | 0 | 34 |
| game_skills | 12 | 5 | 3 | 0 | 20 |
| game_proficiency | 7 | 0 | 9 | 0 | 16 |
| **Total** | 35 | 12 | 23 | 0 | 70 |

Source audit of Oteryn at `a4ad4164` against OTClient at `4955a3fa22d5f5456ec6c658187fd2901545b750`. Each row counts a player goal. Scrolling and pagination serve the same browsing operation. Alternative entry points count when they expose the same information or action. Searches covered `apps/client/src`, including the shared shell and native entry, and the source trees of the `client-*` crates.

## game_wheel

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Open and close the Wheel | PRESENT | `modules/game_wheel/wheel.lua:128` | `apps/client/src/bevy_shell/ui/toolbar/shortcuts.rs:68` opens the panel and requests its current allocation at `apps/client/src/features/0148-wheel-allocation/input.rs:29`. Close clears it via `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:306` and `apps/client/src/features/0148-wheel-allocation/shared.rs:132`. |
| Inspect available points and their sources | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:3470` | `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:95` totals the budget, spent and remaining points. The source breakdown by level, gems, achievements and individual scrolls is absent. |
| Inspect vessel rank and its installed gem | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:3347` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:218` allows vessel selection and `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:204` includes equipped status in gem labels. The four domain vessel ranks and their effects are absent. |
| Jump from the Wheel vessel to its gem in the Atelier | MISSING | `modules/game_wheel/classes/wheelclass.lua:3445` | Searched vessel redirects and Atelier entry. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:29` opens the collection without selecting the vessel’s installed gem. |
| Inspect a slice and its perks | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:1217` | `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:230` shows the node name, points and capacity. The board lacks a selection-only details view for per-point, full-slot and revelation effects. |
| Review the whole build’s perk summary | MISSING | `modules/game_wheel/classes/wheelclass.lua:1366` | Searched Wheel widgets for current/draft effects and dedication, conviction and revelation totals. The diagram text formatter has no production caller rendering that summary. |
| Add or remove one point | PRESENT | `modules/game_wheel/classes/wheelclass.lua:950` | Primary and secondary clicks at `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:224` reach the budget and connectivity checks at `apps/client/src/features/0148-wheel-allocation/shared.rs:27`. |
| Fill or empty a slice in bulk | MISSING | `modules/game_wheel/classes/wheelclass.lua:889` | Searched node controls for Max and modifier-key quantities. `apps/client/src/features/0148-wheel-allocation/shared.rs:38` always adjusts by one. |
| Reset the allocation | PRESENT | `modules/game_wheel/classes/wheelclass.lua:1172` | `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:292` checks removal eligibility and `apps/client/src/features/0148-wheel-allocation/shared.rs:166` resets the draft for submission. |
| Apply the allocation | PRESENT | `modules/game_wheel/classes/wheelclass.lua:1996` | `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:287` enables Save for a valid changed draft. `apps/client/src/features/0148-wheel-allocation/wheel_presentation.rs:50` submits the reviewed allocation. |
| Save a named build from an empty or copied allocation | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:2672` | Reset or load points, then enter a new name at `apps/client/src/bevy_shell/game_ui/wheel/presets.rs:22` and save with `apps/client/src/bevy_shell/game_ui/wheel/presets.rs:48`. `apps/client/src/features/0148-wheel-allocation/presets.rs:48` stores points only, omitting equipped gems from a copied build. |
| Load a saved preset | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:3037` | `apps/client/src/bevy_shell/game_ui/wheel/widgets.rs:268` binds each named row. `apps/client/src/features/0148-wheel-allocation/shared.rs:142` restores its points into the draft. Equipped-gem choices are absent from the stored build. |
| Rename a preset | PRESENT | `modules/game_wheel/classes/wheelclass.lua:2830` | `apps/client/src/bevy_shell/game_ui/wheel/presets.rs:49` dispatches the new field value and `apps/client/src/features/0148-wheel-allocation/presets.rs:93` checks collisions. |
| Delete a preset | PRESENT | `modules/game_wheel/classes/wheelclass.lua:2851` | `apps/client/src/bevy_shell/game_ui/wheel/presets.rs:50` targets the selected name and `apps/client/src/features/0148-wheel-allocation/presets.rs:74` removes its entry. |
| Import a shared build | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:2316` | `apps/client/src/bevy_shell/game_ui/wheel/presets.rs:53` and `apps/client/src/features/0148-wheel-allocation/presets.rs:89` import Oteryn points. The OW1 decoder lacks the reference base64 build format. |
| Export a shared build | PARTIAL | `modules/game_wheel/classes/wheelclass.lua:2511` | `apps/client/src/bevy_shell/game_ui/wheel/presets.rs:51` and `apps/client/src/features/0148-wheel-allocation/presets.rs:84` generate an Oteryn points code or URL. The reference client cannot consume its OW1 encoding. |
| Browse the Gem Atelier and Fragment Workshop | PRESENT | `modules/game_wheel/wheel.lua:200` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:29` opens the combined scrolling collection and modifier list. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:97` sets its open state and `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:135` requests current data. |
| Inspect available gold, fragments and unrevealed gems | PRESENT | `modules/game_wheel/wheel.lua:312` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:173` renders balances from the confirmed resource snapshot at `crates/client-ui/src/features/0148-wheel-allocation/state/gems/view.rs:3`. |
| Inspect a gem and its modifiers | PRESENT | `modules/game_wheel/classes/gematelier.lua:578` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:197` lists confirmed gem IDs. `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:74` checks the revision and `crates/client-ui/src/features/0148-wheel-allocation/state/gems/view.rs:91` lists their modifier positions. |
| Reveal a gem of the chosen quality | PRESENT | `modules/game_wheel/classes/gematelier.lua:948` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:208` selects quality and `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:237` stages Reveal. `apps/client/src/features/0148-wheel-allocation/input.rs:65` prepares the review and `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:22` sends the confirmed operation. |
| Lock or unlock a gem | PRESENT | `modules/game_wheel/classes/gematelier.lua:1011` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:244` dispatches the lock action and `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:102` enqueues ToggleLock. |
| Place or replace a gem in a vessel | PRESENT | `modules/game_wheel/classes/gematelier.lua:819` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:218` selects the vessel and `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:245` equips the gem via `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:93`. |
| Remove a gem from its vessel | PRESENT | `modules/game_wheel/styles/gemMenu.otui:1030` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:246` clears the chosen vessel via `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:94`. |
| Switch a gem’s domain affinity | MISSING | `modules/game_wheel/classes/gematelier.lua:955` | Searched SwitchDomain and affinity mutation. The current encoder explicitly rejects it in `crates/client-ui/src/features/0483-gem-atelier/protocol.rs:39`. |
| Dismantle a gem | PRESENT | `modules/game_wheel/classes/gematelier.lua:969` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:238` stages dismantling and `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:22` sends only after review. |
| Search gems by modifier text | MISSING | `modules/game_wheel/classes/gematelier.lua:1100` | Searched gem search, modifier text and list predicates. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:198` enumerates the full collection. |
| Filter gems by affinity | MISSING | `modules/game_wheel/classes/gematelier.lua:1070` | Searched affinity, domain and gem filter controls. Vessel selection changes the target socket. |
| Filter gems by quality | MISSING | `modules/game_wheel/classes/gematelier.lua:1040` | Searched quality filters. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:198` lists all gems and `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:90` sets the quality for reveal. |
| Show only locked gems | MISSING | `modules/game_wheel/classes/gematelier.lua:1021` | Searched locked-only and list predicates. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:244` changes the lock on one gem. |
| Compare a workshop modifier across grades | PRESENT | `modules/game_wheel/classes/workshop.lua:243` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:225` selects a server modifier, `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:83` binds its identity and `crates/client-ui/src/features/0148-wheel-allocation/state/gems/view.rs:99` renders the comparison. |
| Enhance a modifier’s grade | PRESENT | `modules/game_wheel/classes/workshop.lua:360` | `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:241` stages GradeUp, with a reviewed command sent at `apps/client/src/features/0148-wheel-allocation/gem_presentation.rs:22`. |
| Search workshop modifiers by name | MISSING | `modules/game_wheel/classes/workshop.lua:636` | Searched workshop text search and name predicate. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:225` iterates every current modifier. |
| Filter workshop modifiers by basic, supreme or in-vessel status | MISSING | `modules/game_wheel/classes/workshop.lua:592` | Searched workshop kind and equipped-only controls. `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:225` has an unfiltered list. |
| Jump from a gem modifier to its workshop entry | MISSING | `modules/game_wheel/classes/gematelier.lua:1253` | Searched mod redirect and gem detail actions. `crates/client-ui/src/features/0148-wheel-allocation/state/gems/view.rs:91` renders text and `apps/client/src/bevy_shell/game_ui/wheel/atelier.rs:225` requires manual selection. |

## game_skills

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Open or close the Skills widget | PRESENT | `modules/game_skills/skills.lua:974` | `apps/client/src/bevy_shell/ui/toolbar/grid.rs:302` toggles the Skills pane mapped at `apps/client/src/bevy_shell/game_ui/regions.rs:238`. |
| Check level, experience and level progress | PRESENT | `modules/game_skills/skills.lua:1104` | `apps/client/src/bevy_shell/game_ui/hud/skills.rs:194` reads complete progress, renders XP and level, and sizes the level bar at `apps/client/src/bevy_shell/game_ui/hud/skills.rs:243`. `apps/client/src/features/0279-skill-progress/protocol.rs:84` consumes the reply. |
| Check XP per hour and estimated time to level | PRESENT | `modules/game_skills/skills.lua:992` | `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:326` shows the estimate calculated from timestamped XP observations at `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:33`. |
| Reset the experience counter | PRESENT | `modules/game_skills/skills.lua:490` | `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:150` binds Reset and `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:52` resets the measurement baseline. |
| Inspect the XP gain multiplier and its components | PARTIAL | `modules/game_skills/skills.lua:1338` | Boost percentages are formatted in `crates/client-ui/src/features/0151-daily-boost/view.rs:121`. The Skills tooltip at `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:324` lacks the combined base, stamina and bonus rate breakdown. |
| Open the XP boost offer | PARTIAL | `modules/game_skills/skills.otui:84` | `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:179` opens the general store. It supplies no boost offer or product selection. |
| Check health and mana | PRESENT | `modules/game_skills/skills.lua:1118` | `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:88` displays the live current and maximum values obtained from actor vitals at `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:193`. |
| Check soul points | PRESENT | `modules/game_skills/skills.lua:1128` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:61` displays soul from the actor-vitals snapshot at `apps/client/src/bevy_shell/game_ui/inventory/view.rs:103`. |
| Inspect remaining, total and base carrying capacity | PARTIAL | `modules/game_skills/skills.lua:1132` | `apps/client/src/bevy_shell/game_ui/inventory/view/slots.rs:15` displays free ounces. The inventory readout lacks the total/base comparison from the reference tooltip. |
| Compare current and base movement speed | MISSING | `modules/game_skills/skills.lua:1242` | Searched current_speed, base_speed and speed readouts. Map ground speed describes tiles, and the Skills widget exposes no character speed comparison. |
| Check stamina and offline training time remaining | PRESENT | `modules/game_skills/skills.lua:1181` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:5` renders the General Stats cards. `crates/client-ui/src/features/0199-cyclopedia-character/view.rs:82` converts both server balances to hours and minutes. |
| Check remaining food regeneration time | PRESENT | `modules/game_skills/skills.lua:1211` | The HUD status line at `apps/client/src/bevy_shell/game_ui/hud.rs:315` includes resource status via `apps/client/src/features/local-spell-action-bar/view.rs:50`. `apps/client/src/play/resource_hud.rs:243` formats the confirmed food timer. |
| Check trained and modified skill levels and progress to advance | PRESENT | `modules/game_skills/skills.lua:1263` | `apps/client/src/bevy_shell/game_ui/hud/skills.rs:218` renders trained, loyalty and current values. `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:305` shows progress and remaining training points from the reply consumed at `apps/client/src/features/0279-skill-progress/protocol.rs:84`. |
| Inspect offensive combat stats | PRESENT | `modules/game_skills/skills.lua:1427` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:10` renders the Offence rows, including crit, leech, attack and conversion, projected from confirmed combat state at `crates/client-ui/src/features/0199-cyclopedia-character/view.rs:145`. |
| Inspect defensive combat stats and resistances | PARTIAL | `modules/game_skills/skills.lua:1520` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:10` and `crates/client-ui/src/features/0199-cyclopedia-character/view.rs:145` show defence, mitigation and seven elemental absorbs. The combat catalogue lacks drown, healing, life drain and mana drain resistance rows. |
| Inspect miscellaneous forge bonuses | PRESENT | `modules/game_skills/skills.lua:1571` | `apps/client/src/bevy_shell/game_ui/cyclopedia/rows.rs:10` projects the Misc tab from combat state at `crates/client-ui/src/features/0199-cyclopedia-character/view.rs:145`, including momentum, transcendence and amplification. |
| Toggle all skill progress bars | PRESENT | `modules/game_skills/skills.lua:534` | `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:149` binds the control and `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:202` updates the saved progress preference. |
| Toggle one progress bar independently | MISSING | `modules/game_skills/skills.lua:424` | Searched family visibility and per-bar preferences. `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:195` removes the entire family row and `apps/client/src/bevy_shell/game_ui/hud/skills_interaction.rs:202` toggles bars globally. |
| Show or hide combat-stat groups in Skills | MISSING | `modules/game_skills/skills.lua:340` | Searched skills_layout and offence, defence and misc visibility. The Cyclopedia tabs do not change the Skills widget. |
| Compare skills and choose one for offline training | PARTIAL | `modules/game_skills/offlinetraining1524.lua:153` | The character picker supports training and `apps/client/src/features/0054-training/protocol.rs:8` encodes an offline target handle. The picker lacks the live per-skill comparison and direct skill choice in the reference modal. |

## game_proficiency

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Open or close Weapon Proficiency | PRESENT | `modules/game_proficiency/proficiency.lua:854` | `apps/client/src/bevy_shell/ui/toolbar/shortcuts.rs:67` invokes `apps/client/src/features/0195-weapon-proficiency/input.rs:67` to open. `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:268` and `apps/client/src/features/0195-weapon-proficiency/input.rs:99` close the panel. |
| Monitor the equipped weapon’s progress on the HUD | MISSING | `modules/game_proficiency/proficiency.lua:321` | Searched proficiencyTopBar, progress_entry and HUD consumers. The catalogue progress bar at `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:139` requires opening the panel. |
| Notice an unused weapon perk on the proficiency shortcut | MISSING | `modules/game_proficiency/proficiency.lua:627` | Searched advancement icon, unused perk and open_track. `apps/client/src/features/0195-weapon-proficiency/input.rs:17` has no production HUD activation caller. |
| Open proficiency directly on an equipped or inspected weapon | MISSING | `modules/game_proficiency/proficiency.lua:863` | Searched inventory actions and equipped-weapon redirects. `apps/client/src/features/0195-weapon-proficiency/input.rs:75` opens the catalogue without an item argument. |
| Browse weapons and select one to inspect | PRESENT | `modules/game_proficiency/proficiency.lua:2176` | `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:98` renders selectable catalogue entries and `apps/client/src/features/0195-weapon-proficiency/input.rs:175` handles the selected row. |
| Search weapons by name | MISSING | `modules/game_proficiency/proficiency.lua:975` | Searched text input and assignments to panel.search. `crates/client-ui/src/features/0195-weapon-proficiency/view/presentation.rs:39` consumes the string but the live list at `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:98` provides no editor. |
| Filter weapons by weapon class | MISSING | `modules/game_proficiency/proficiency.lua:960` | Searched catalogue controls for sword, axe, club, distance, wand and fist predicates. `crates/client-ui/src/features/0195-weapon-proficiency/view/presentation.rs:38` only applies name and progress predicates. |
| Filter weapons by character level | MISSING | `modules/game_proficiency/proficiency.lua:2216` | Searched required-level and character-level catalogue filters. `crates/client-ui/src/features/0195-weapon-proficiency/view/presentation.rs:40` filters mastery progress instead. |
| Filter weapons by vocation | MISSING | `modules/game_proficiency/proficiency.lua:2243` | Searched vocation eligibility and proficiency catalogue controls. `crates/client-ui/src/features/0195-weapon-proficiency/view/presentation.rs:38` lacks a vocation predicate. |
| Filter one- and two-handed weapons | MISSING | `modules/game_proficiency/proficiency.lua:2268` | Searched handedness, two-handed and slot filters. `crates/client-ui/src/features/0195-weapon-proficiency/view/presentation.rs:38` has no handedness predicate. |
| Inspect weapon level, XP and perk unlock thresholds | PRESENT | `modules/game_proficiency/proficiency.lua:1204` | `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:100` displays catalogue XP and `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:172` compares it with each threshold. `crates/client-ui/src/features/0195-weapon-proficiency/view/presentation.rs:10` derives the level from server thresholds. |
| Inspect perk descriptions and current selections | PRESENT | `modules/game_proficiency/proficiency.lua:2109` | `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:197` renders each server description and selected index from the weapon entry obtained at `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:159`. |
| Preview a perk choice | PRESENT | `modules/game_proficiency/proficiency.lua:1951` | `apps/client/src/bevy_shell/game_ui/proficiency/widgets.rs:212` binds each choice. `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:68` validates the unlock threshold and records an uncommitted selection. |
| Apply perk choices, optionally closing afterward | PRESENT | `modules/game_proficiency/proficiency.lua:2363` | `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:257` exposes Save and Close alongside Apply at `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:262`. `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:154` dispatches each selected perk via `apps/client/src/features/0195-weapon-proficiency/input.rs:184`. |
| Discard pending edits on close | PRESENT | `modules/game_proficiency/proficiency.lua:812` | `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:258` offers Discard. `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:117` checks the draft and handles the close decision. |
| Reset committed perks for the selected weapon | MISSING | `modules/game_proficiency/proficiency.lua:2345` | Searched reset, empty perk selection and Clear. `apps/client/src/bevy_shell/game_ui/proficiency/draft.rs:110` resets only the local draft. Clear at `crates/client-ui/src/features/0195-weapon-proficiency/protocol.rs:70` removes a slot modification. |

## Gaps ranked by player impact

### 1. Build decisions, permanent selections and training

- Inspect a slice and its perks (game_wheel, PARTIAL).
- Review the whole build’s perk summary (game_wheel, MISSING).
- Switch a gem’s domain affinity (game_wheel, MISSING).
- Compare skills and choose one for offline training (game_skills, PARTIAL).
- Monitor the equipped weapon’s progress on the HUD (game_proficiency, MISSING).
- Reset committed perks for the selected weapon (game_proficiency, MISSING).

### 2. Finding or comparing builds, equipment and character statistics

- Inspect vessel rank and its installed gem (game_wheel, PARTIAL).
- Fill or empty a slice in bulk (game_wheel, MISSING).
- Save a named build from an empty or copied allocation (game_wheel, PARTIAL).
- Load a saved preset (game_wheel, PARTIAL).
- Import a shared build (game_wheel, PARTIAL).
- Export a shared build (game_wheel, PARTIAL).
- Search gems by modifier text (game_wheel, MISSING).
- Filter gems by affinity (game_wheel, MISSING).
- Filter gems by quality (game_wheel, MISSING).
- Show only locked gems (game_wheel, MISSING).
- Search workshop modifiers by name (game_wheel, MISSING).
- Filter workshop modifiers by basic, supreme or in-vessel status (game_wheel, MISSING).
- Inspect the XP gain multiplier and its components (game_skills, PARTIAL).
- Open the XP boost offer (game_skills, PARTIAL).
- Inspect remaining, total and base carrying capacity (game_skills, PARTIAL).
- Compare current and base movement speed (game_skills, MISSING).
- Inspect defensive combat stats and resistances (game_skills, PARTIAL).
- Notice an unused weapon perk on the proficiency shortcut (game_proficiency, MISSING).
- Open proficiency directly on an equipped or inspected weapon (game_proficiency, MISSING).
- Search weapons by name (game_proficiency, MISSING).
- Filter weapons by weapon class (game_proficiency, MISSING).
- Filter weapons by character level (game_proficiency, MISSING).
- Filter weapons by vocation (game_proficiency, MISSING).
- Filter one- and two-handed weapons (game_proficiency, MISSING).

### 3. Navigation and display preferences

- Inspect available points and their sources (game_wheel, PARTIAL).
- Jump from the Wheel vessel to its gem in the Atelier (game_wheel, MISSING).
- Jump from a gem modifier to its workshop entry (game_wheel, MISSING).
- Toggle one progress bar independently (game_skills, MISSING).
- Show or hide combat-stat groups in Skills (game_skills, MISSING).
