# Client parity audit F

Oteryn `a4ad4164` and OTClient `4955a3fa22d5f5456ec6c658187fd2901545b750`. Counts describe client operations in source, conditional on session capabilities. Opening, confirmation and dismissal are included in the underlying workflow unless they have an independent player purpose. OTClient’s quick-loot search returns immediately (quickloot.lua:323).

| Module | Operations | PRESENT | PARTIAL | MISSING | NOT-APPLICABLE |
|---|---:|---:|---:|---:|---:|
| game_battle | 37 | 16 | 6 | 15 | 0 |
| game_healthcircle | 16 | 10 | 2 | 4 | 0 |
| game_healthinfo | 3 | 3 | 0 | 0 | 0 |
| game_creatureinformation | 10 | 6 | 2 | 2 | 0 |
| game_imbuing | 12 | 9 | 1 | 2 | 0 |
| game_imbuementtracker | 6 | 2 | 4 | 0 | 0 |
| game_forge | 15 | 13 | 1 | 1 | 0 |
| game_prey | 14 | 10 | 4 | 0 | 0 |
| game_highscore | 13 | 12 | 0 | 1 | 0 |
| game_questlog | 20 | 8 | 3 | 9 | 0 |
| game_quickloot | 13 | 6 | 4 | 3 | 0 |
| Total | 159 | 95 | 27 | 37 | 0 |

## game_battle

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Sort by Name (asc) | PRESENT | `modules/game_battle/battle.lua:35` | `apps/client/src/features/0017-attack/editor.rs:15` → `apps/client/src/features/0017-attack/preferences.rs:113` |
| Sort by HitPoints (asc) | PRESENT | `modules/game_battle/battle.lua:33` | `apps/client/src/features/0017-attack/editor.rs:16` → `apps/client/src/features/0017-attack/preferences.rs:114` |
| Sort by HitPoints (desc) | PRESENT | `modules/game_battle/battle.lua:34` | `apps/client/src/features/0017-attack/editor.rs:17` → `apps/client/src/features/0017-attack/preferences.rs:115` |
| Sort by DisplayTime (asc) | PRESENT | `modules/game_battle/battle.lua:29` | `apps/client/src/features/0017-attack/editor.rs:14` → `apps/client/src/features/0017-attack/preferences.rs:107` |
| Sort by DisplayTime (desc) | MISSING | `modules/game_battle/battle.lua:30` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:107` |
| Sort by Distance (asc) | MISSING | `modules/game_battle/battle.lua:31` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:112` |
| Sort by Distance (desc) | MISSING | `modules/game_battle/battle.lua:32` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:112` |
| Sort by Name (desc) | MISSING | `modules/game_battle/battle.lua:36` | Available comparison branches: `apps/client/src/features/0017-attack/preferences.rs:113` |
| Rename a list | PRESENT | `modules/game_battle/battle.lua:333` | `apps/client/src/features/0017-attack/editor.rs:81` → `apps/client/src/features/0017-attack/editor.rs:147`. |
| Create another list | PRESENT | `modules/game_battle/battle.lua:334` | `apps/client/src/features/0017-attack/editor.rs:117` → `apps/client/src/features/0017-attack/editor.rs:147`. |
| Remove a secondary list | PRESENT | `modules/game_battle/battle.lua:337` | `apps/client/src/features/0017-attack/editor.rs:131` → `apps/client/src/features/0017-attack/editor.rs:147`. |
| Hide players | PRESENT | `modules/game_battle/battle.otui:196` | `apps/client/src/features/0017-attack/editor.rs:84` → `apps/client/src/features/0017-attack/preferences.rs:100`. |
| Hide knights | PARTIAL | `modules/game_battle/battle.otui:201` | The profession selector matches one vocation. It cannot exclude that vocation independently (apps/client/src/features/0017-attack/editor.rs:90). |
| Hide paladins | PARTIAL | `modules/game_battle/battle.otui:206` | The profession selector matches one vocation. It cannot exclude that vocation independently (apps/client/src/features/0017-attack/editor.rs:90). |
| Hide druids | PARTIAL | `modules/game_battle/battle.otui:211` | The profession selector matches one vocation. It cannot exclude that vocation independently (apps/client/src/features/0017-attack/editor.rs:90). |
| Hide sorcerers | PARTIAL | `modules/game_battle/battle.otui:216` | The profession selector matches one vocation. It cannot exclude that vocation independently (apps/client/src/features/0017-attack/editor.rs:90). |
| Hide monks | PARTIAL | `modules/game_battle/battle.otui:221` | The profession selector matches one vocation. It cannot exclude that vocation independently (apps/client/src/features/0017-attack/editor.rs:90). |
| Hide summons | MISSING | `modules/game_battle/battle.otui:226` | Searched battle predicates and editor actions for hide summons. The accepted fields are in `apps/client/src/features/0017-attack/editor.rs:81`. |
| Hide Npcs | MISSING | `modules/game_battle/battle.otui:231` | Searched battle predicates and editor actions for hide npcs. The accepted fields are in `apps/client/src/features/0017-attack/editor.rs:81`. |
| Hide non-skull players | MISSING | `modules/game_battle/battle.otui:241` | Searched battle predicates and editor actions for hide non-skull players. The accepted fields are in `apps/client/src/features/0017-attack/editor.rs:81`. |
| Hide party members | MISSING | `modules/game_battle/battle.otui:246` | Searched battle predicates and editor actions for hide party members. The accepted fields are in `apps/client/src/features/0017-attack/editor.rs:81`. |
| Clear Configurations | MISSING | `modules/game_battle/battle.lua:336` | Searched battle predicates and editor actions for clear configurations. The accepted fields are in `apps/client/src/features/0017-attack/editor.rs:81`. |
| Hide creatures | PRESENT | `modules/game_battle/battle.otui:236` | `apps/client/src/features/0017-attack/editor.rs:85` → `apps/client/src/features/0017-attack/preferences.rs:101`. |
| Display two lists simultaneously | PARTIAL | `modules/game_battle/battle.otui:344` | The editor stores multiple lists but projects one active selection (apps/client/src/features/0017-attack/editor.rs:105). |
| Attack or stop attacking a listed target | PRESENT | `modules/game_battle/battle.lua:2427` | `apps/client/src/bevy_shell/game_ui/hud.rs:185` → `apps/client/src/features/0017-attack/input.rs:92`. |
| Open or close battle list | PRESENT | `modules/game_battle/battle.otui:177` | `apps/client/src/features/0017-attack/input.rs:100` → `apps/client/src/features/0017-attack/input.rs:109`. |
| Open creature context actions | PRESENT | `modules/game_battle/battle.lua:2420` | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:153` → `apps/client/src/bevy_shell/game_ui/hud/battle.rs:164`. |
| Cycle attack target forward or backward | MISSING | `modules/game_battle/battle.lua:1989` | Searched battle input, row actions and normalized target commands for previous/next or wraparound targeting. |
| Exclude own guild | PRESENT | `modules/game_battle/battle.otui:251` | `apps/client/src/features/0017-attack/editor.rs:24` → `apps/client/src/features/0017-attack/preferences.rs:184`. |
| Read a listed creature’s health | PRESENT | `modules/game_battle/battle.lua:1364` | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:431` → `apps/client/src/features/0017-attack/preferences.rs:80`. |
| Identify current attack target | PRESENT | `modules/game_battle/battle.lua:1532` | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:392` → `apps/client/src/bevy_shell/game_ui/hud/battle.rs:379`. |
| Identify followed creature in the list | MISSING | `modules/game_battle/battle.lua:1533` | The row frame checks combat.target only. Searched battle row projection for following identity. |
| Read PvP and guild marks in the list | MISSING | `modules/game_battle/battle.lua:1361` | Searched battle row children for skull or emblem badges. World marks exist, but the list builds a target frame, name and health track. |
| Highlight the corresponding creature on map hover from list | MISSING | `modules/game_battle/battle.lua:1408` | Searched battle pointer observers and scene marks for a hover-to-world highlight. |
| Identify a creature by battle-list portrait | MISSING | `modules/game_battle/battle.lua:1363` | Searched battle row construction for outfit sprite or portrait. The icon node draws a selection border (apps/client/src/bevy_shell/game_ui/hud/battle.rs:383). |
| Collapse the battle filter controls | MISSING | `modules/game_battle/battle.lua:1982` | Searched battle HUD for a filter-row visibility toggle. The sort/menu toggle leaves filters visible. |
| Inspect a listed target | PRESENT | `modules/game_battle/battle.lua:2418` | `apps/client/src/bevy_shell/game_ui/hud/battle.rs:161` → `apps/client/src/features/0011-world-interaction/input.rs:199`. |

## game_healthcircle

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Monitor health arc | PRESENT | `modules/game_healthcircle/option_healthcircle.otui:12` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:49` → `apps/client/src/bevy_shell/health_circle.rs:78`. |
| Monitor mana arc | PRESENT | `modules/game_healthcircle/option_healthcircle.otui:30` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:55` → `apps/client/src/bevy_shell/health_circle.rs:85`. |
| Adjust arc distance | PRESENT | `modules/game_healthcircle/option_healthcircle.otui:115` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:62` → `apps/client/src/bevy_shell/health_circle.rs:106`. |
| Adjust arc opacity | PRESENT | `modules/game_healthcircle/option_healthcircle.otui:130` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:71` → `apps/client/src/bevy_shell/health_circle.rs:110`. |
| Show experience progress arc | MISSING | `modules/game_healthcircle/option_healthcircle.otui:48` | Searched arc options and projection for show experience progress arc. The renderer iterates only health and mana in `apps/client/src/bevy_shell/health_circle.rs:75`. |
| Show skill progress arc | MISSING | `modules/game_healthcircle/option_healthcircle.otui:66` | Searched arc options and projection for show skill progress arc. The renderer iterates only health and mana in `apps/client/src/bevy_shell/health_circle.rs:75`. |
| Choose the tracked skill | MISSING | `modules/game_healthcircle/option_healthcircle.otui:83` | Searched arc options and projection for choose the tracked skill. The renderer iterates only health and mana in `apps/client/src/bevy_shell/health_circle.rs:75`. |
| Choose status bar style | PARTIAL | `modules/game_healthcircle/option_healthcircle.otui:144` | Compact status changes the HUD projection (apps/client/src/bevy_shell/game_ui/hud.rs:311). Large and parallel styles lack equivalent selectors. |
| Position or hide the status bar | PARTIAL | `modules/game_healthcircle/game_healthcircle.lua:744` | Visibility is configurable (apps/client/src/bevy_shell/game_ui/projection.rs:197). The top bar has a fixed location, with no top/bottom selector. |
| Monitor remaining mana shield on the arc | MISSING | `modules/game_healthcircle/game_healthcircle.lua:278` | Searched arc resource selection and mana-shield status rendering. The status icon reports the condition but the arc consumes ordinary mana. |
| Choose conditions shown on HUD | PRESENT | `modules/game_healthcircle/statusiconbar.lua:192` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:110` → `apps/client/src/features/local-reference-preferences/status_order.rs:100`. |
| Choose conditions shown on status bar | PRESENT | `modules/game_healthcircle/statusiconbar.lua:198` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:110` → `apps/client/src/features/local-reference-preferences/status_order.rs:100`. |
| Reorder condition icons | PRESENT | `modules/game_healthcircle/statusiconbar.lua:276` | `apps/client/src/bevy_shell/game_ui/options/conditions.rs:65` → `apps/client/src/features/local-reference-preferences/status_order.rs:97`. |
| Read Harmony charges | PRESENT | `modules/game_healthcircle/game_healthcircle_monk.lua:187` | `apps/client/src/bevy_shell/game_ui/hud.rs:281` → `apps/client/src/bevy_shell/game_ui/hud.rs:244`. |
| Read Serene state | PRESENT | `modules/game_healthcircle/game_healthcircle_monk.lua:182` | `apps/client/src/bevy_shell/game_ui/hud.rs:300` → `apps/client/src/bevy_shell/game_ui/hud.rs:244`. |
| Monitor active conditions | PRESENT | `modules/game_healthcircle/statusiconbar.lua:673` | `apps/client/src/bevy_shell/game_ui/hud/status_icons.rs:126` → `apps/client/src/play/resource_hud.rs:273`. |

## game_healthinfo

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Show or hide the status panel | PRESENT | `modules/game_healthinfo/healthinfo.lua:62` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:45` → `apps/client/src/bevy_shell/game_ui/projection.rs:197`. |
| Read current and maximum health | PRESENT | `modules/game_healthinfo/healthinfo.lua:32` | `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:211` → `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:193`. |
| Read current and maximum mana | PRESENT | `modules/game_healthinfo/healthinfo.lua:33` | `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:211` → `apps/client/src/bevy_shell/game_ui/hud/top_bar.rs:193`. |

## game_creatureinformation

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Identify a creature by name | PRESENT | `modules/game_creatureinformation/creatureinformation.lua:160` | `apps/client/src/features/0006-world-spatial-entities/scene.rs:267` → `apps/client/src/features/0006-world-spatial-entities/scene.rs:259`. |
| Identify party membership | PRESENT | `modules/game_creatureinformation/creatureinformation.lua:164` | `apps/client/src/features/0023-presentation/actor_hud.rs:117` → `apps/client/src/features/0023-presentation/actor_hud.rs:66`. |
| Identify a PvP skull | PRESENT | `modules/game_creatureinformation/creatureinformation.lua:163` | `apps/client/src/features/0023-presentation/actor_hud.rs:124` → `apps/client/src/features/0023-presentation/actor_hud.rs:79`. |
| Identify guild relationship | PRESENT | `modules/game_creatureinformation/creatureinformation.lua:165` | `apps/client/src/features/0023-presentation/actor_hud.rs:139` → `apps/client/src/features/0023-presentation/actor_hud.rs:137`. |
| Read creature status icons | PARTIAL | `modules/game_creatureinformation/creatureinformation.lua:162` | The name overlay appends counts, but only five category/ID pairs have names. Other icons appear as Unknown (crates/client-ui/src/features/0372-native-creature-icons/protocol.rs:14). |
| Read own mana beneath overhead health | MISSING | `modules/game_creatureinformation/creatureinformation.lua:185` | Searched actor HUD and creature label rendering for an overhead mana gauge. Sidebar and circle gauges are available. |
| Recognize creature type by overhead badge | PARTIAL | `modules/game_creatureinformation/creatureinformation.lua:161` | NPC badges exist in apps/client/src/features/0023-presentation/actor_hud.rs:57. Searched actor HUD and native icon mapping for type badges across creature kinds. |
| Distinguish a covered creature by its grey label | MISSING | `modules/game_creatureinformation/creatureinformation.lua:79` | Searched world labels and actor HUD for covered/occluded name and health colours. |
| Show or hide creature health bars and monitor health | PRESENT | `modules/game_creatureinformation/creatureinformation.lua:50` | `apps/client/src/bevy_shell/creature_bars.rs:37` → `apps/client/src/features/0023-presentation/view/creature_bars.rs:13`. |
| Show or hide overhead names | PRESENT | `modules/game_creatureinformation/creatureinformation.lua:176` | `apps/client/src/bevy_shell/game_ui/options/metadata/interface.rs:29` → `apps/client/src/features/0006-world-spatial-entities/scene.rs:251`. |

## game_imbuing

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Apply imbuement | PRESENT | `modules/game_imbuing/new_design/classes/imbuementitem.lua:417` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:63` → `apps/client/src/features/0068-imbue/input.rs:127` |
| Clear imbuement | PRESENT | `modules/game_imbuing/new_design/classes/imbuementitem.lua:192` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:67` → `apps/client/src/features/0068-imbue/input.rs:127` |
| Select item | PRESENT | `modules/game_imbuing/new_design/classes/imbuementselection.lua:90` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:282` → `apps/client/src/features/0068-imbue/input.rs:127` |
| Select an equipment slot | PRESENT | `modules/game_imbuing/old_design/imbuing.otui:116` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:283` → `apps/client/src/features/0068-imbue/input.rs:135`. |
| Choose quality and effect | PRESENT | `modules/game_imbuing/new_design/t_imbui.otui:197` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:293` → `apps/client/src/features/0068-imbue/input.rs:136`. |
| Leave the shrine | PRESENT | `modules/game_imbuing/new_design/t_imbui.otui:872` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:276` → `apps/client/src/features/0068-imbue/input.rs:127`. |
| Imbue a blank scroll | MISSING | `modules/game_imbuing/new_design/t_imbui.otui:643` | Searched shrine targets, scroll catalogue and apply intents for a writable blank scroll. Equipment selection is the only target in the shrine. |
| Review success chance | PRESENT | `modules/game_imbuing/old_design/imbuing.otui:175` | `apps/client/src/bevy_shell/game_ui/imbuing/widgets.rs:83` → `apps/client/src/bevy_shell/game_ui/imbuing/costs.rs:35`. |
| Buy protection against imbuement failure | MISSING | `modules/game_imbuing/old_design/imbuing.lua:177` | Searched shrine actions and quotes for a protection purchase. The cost panel explicitly reports it unavailable. |
| Check affordability before applying | PARTIAL | `modules/game_imbuing/old_design/imbuing.lua:138` | Requirements are shown but the shrine lacks available stock and spendable balance (apps/client/src/bevy_shell/game_ui/imbuing/costs.rs:6). |
| Inspect installed effect and remaining time | PRESENT | `modules/game_imbuing/old_design/imbuing.otui:296` | `apps/client/src/bevy_shell/game_ui/imbuing.rs:133` → `crates/client-ui/src/features/0068-imbue/view.rs:20`. |
| Inspect required materials and gold | PRESENT | `modules/game_imbuing/old_design/imbuing.otui:160` | `apps/client/src/bevy_shell/game_ui/imbuing/widgets.rs:207` → `apps/client/src/bevy_shell/game_ui/imbuing/costs.rs:25`. |

## game_imbuementtracker

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Refresh duration tracker | PRESENT | `modules/game_imbuementtracker/imbuementtracker.lua:45` | `apps/client/src/bevy_shell/game_ui/imbuement_tracker.rs:208` → `apps/client/src/features/0068-imbue/input/tracker.rs:94` |
| Monitor equipment slot durations | PRESENT | `modules/game_imbuementtracker/imbuementtracker.lua:222` | `apps/client/src/bevy_shell/game_ui/imbuement_tracker.rs:57` → `apps/client/src/features/0068-imbue/input/tracker.rs:24`. |
| Show imbuements that last less than 1h | PARTIAL | `modules/game_imbuementtracker/imbuementtracker.otui:21` | OTClient filters whole items by the longest active slot, or by absence of any active slot (imbuementtracker.lua:239). Oteryn filters individual active or unused slots (crates/client-ui/src/features/0068-imbue/state/equipment_tracker.rs:175). |
| Show imbuements that last between 1h and 3h | PARTIAL | `modules/game_imbuementtracker/imbuementtracker.otui:25` | OTClient filters whole items by the longest active slot, or by absence of any active slot (imbuementtracker.lua:239). Oteryn filters individual active or unused slots (crates/client-ui/src/features/0068-imbue/state/equipment_tracker.rs:175). |
| Show imbuements that last more than 3h | PARTIAL | `modules/game_imbuementtracker/imbuementtracker.otui:29` | OTClient filters whole items by the longest active slot, or by absence of any active slot (imbuementtracker.lua:239). Oteryn filters individual active or unused slots (crates/client-ui/src/features/0068-imbue/state/equipment_tracker.rs:175). |
| Show items with no active imbuement | PARTIAL | `modules/game_imbuementtracker/imbuementtracker.otui:33` | OTClient filters whole items by the longest active slot, or by absence of any active slot (imbuementtracker.lua:239). Oteryn filters individual active or unused slots (crates/client-ui/src/features/0068-imbue/state/equipment_tracker.rs:175). |

## game_forge

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Open portable forge | PRESENT | `modules/game_forge/game_forge.lua:125` | `apps/client/src/features/0152-item-forge/input.rs:21` → `apps/client/src/features/0152-item-forge/input.rs:30`. |
| Choose source and target items | PRESENT | `modules/game_forge/game_forge.lua:759` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:260` → `apps/client/src/features/0152-item-forge/bevy.rs:105`. |
| Fuse items | PRESENT | `modules/game_forge/game_forge.lua:322` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:392` → `apps/client/src/features/0152-item-forge/bevy.rs:116`. |
| Transfer a tier | PRESENT | `modules/game_forge/game_forge.lua:655` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:86` → `apps/client/src/features/0152-item-forge/bevy.rs:80`. |
| Enable convergence fusion or transfer | PRESENT | `modules/game_forge/game_forge.lua:806` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:220` → `apps/client/src/features/0152-item-forge/bevy.rs:89`. |
| Spend cores for higher fusion chance | PRESENT | `modules/game_forge/game_forge.lua:597` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:305` → `apps/client/src/features/0152-item-forge/bevy.rs:107`. |
| Spend cores to reduce tier loss | PRESENT | `modules/game_forge/game_forge.lua:606` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:318` → `apps/client/src/features/0152-item-forge/bevy.rs:111`. |
| Review cost and eligibility | PRESENT | `modules/game_forge/game_forge.lua:240` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:383` → `apps/client/src/features/0152-item-forge/bevy.rs:28`. |
| Convert dust to slivers | PRESENT | `modules/game_forge/game_forge.lua:1150` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:119` → `apps/client/src/features/0152-item-forge/bevy.rs:125`. |
| Convert slivers to cores | PRESENT | `modules/game_forge/game_forge.lua:1159` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:120` → `apps/client/src/features/0152-item-forge/bevy.rs:125`. |
| Raise dust capacity | PRESENT | `modules/game_forge/game_forge.lua:1168` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:121` → `apps/client/src/features/0152-item-forge/bevy.rs:125`. |
| Inspect recorded forge outcomes | PRESENT | `modules/game_forge/game_forge.lua:1223` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:152` → `apps/client/src/features/0152-item-forge/bevy.rs:125`. |
| Page through forge history | MISSING | `modules/game_forge/game_forge.lua:1252` | Searched history tab and history request for previous/next and page index. Next in the item selector advances candidates only. |
| Inspect available forge resources | PRESENT | `modules/game_forge/game_forge.lua:48` | `apps/client/src/bevy_shell/game_ui/forge/widgets.rs:353` → `apps/client/src/features/0152-item-forge/bevy.rs:39`. |
| Interpret the bonus on a forge result | PARTIAL | `modules/game_forge/game_forge.lua:522` | Receipts show bonus ID and resource deltas, without explaining which bonus occurred (crates/client-ui/src/features/0152-item-forge/protocol/reply.rs:170). |

## game_prey

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Display the gold price before paid list reroll | PARTIAL | `modules/game_prey/prey.lua:78` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:224` warns of a possible charge but cannot quote it. `apps/client/src/features/0140-prey/input.rs:42` sends the request. |
| Choose an offered creature | PRESENT | `modules/game_prey/prey.lua:82` | `apps/client/src/bevy_shell/game_ui/prey.rs:278` → `apps/client/src/features/0140-prey/input.rs:42`. |
| Choose any creature with wildcards | PRESENT | `modules/game_prey/prey.lua:85` | `apps/client/src/bevy_shell/game_ui/prey.rs:227` → `apps/client/src/features/0140-prey/input.rs:42`. |
| Reroll the creature list | PRESENT | `modules/game_prey/prey.lua:78` | `apps/client/src/bevy_shell/game_ui/prey.rs:113` → `apps/client/src/features/0140-prey/input.rs:42`. |
| Search the creature catalogue | PARTIAL | `modules/game_prey/prey.otui:428` | Wildcard selection accepts typed names but has no filterable full race list (apps/client/src/bevy_shell/game_ui/prey.rs:194). |
| Reroll active bonus | PRESENT | `modules/game_prey/prey.lua:818` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:367` → `apps/client/src/features/0140-prey/input.rs:42`. |
| Configure automatic bonus reroll | PRESENT | `modules/game_prey/prey.lua:1571` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:371` → `apps/client/src/features/0140-prey/input.rs:42`. |
| Lock current bonus on renewal | PRESENT | `modules/game_prey/prey.lua:1571` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:378` → `apps/client/src/features/0140-prey/input.rs:42`. |
| Monitor prey in a separate tracker | PARTIAL | `modules/game_prey/prey.lua:311` | Searched Prey UI installation and slot projections for a separate miniwindow. Bonus and remaining time appear in the main dialog. |
| Monitor active prey, bonus and time left | PRESENT | `modules/game_prey/prey.lua:1645` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:304` → `apps/client/src/features/0140-prey/protocol.rs:9`. |
| Open Store for wildcards or a slot | PRESENT | `modules/game_prey/prey.otui:17` | `apps/client/src/bevy_shell/game_ui/prey.rs:276` → `apps/client/src/features/0154-store/input.rs:6`. |
| Identify offered prey by appearance | PARTIAL | `modules/game_prey/prey.lua:1713` | Searched prey reply and candidate rows for outfit rendering. The UI receives creature keys and displays names without the reference portraits. |
| Check free reroll countdown | PRESENT | `modules/game_prey/prey.lua:516` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:261` → `apps/client/src/features/0140-prey/protocol.rs:9`. |
| Inspect wildcard balance | PRESENT | `modules/game_prey/prey.lua:1910` | `apps/client/src/bevy_shell/game_ui/prey/view.rs:84` → `apps/client/src/features/0140-prey/protocol.rs:9`. |

## game_highscore

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Select ranking category | PRESENT | `modules/game_highscore/game_highscore.lua:116` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:7` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Select vocation | PRESENT | `modules/game_highscore/game_highscore.lua:115` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:18` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Next ranking page | PRESENT | `modules/game_highscore/game_highscore.lua:241` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:32` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Previous ranking page | PRESENT | `modules/game_highscore/game_highscore.lua:243` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:26` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Show own rank | PRESENT | `modules/game_highscore/game_highscore.lua:257` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:43` → `apps/client/src/features/0009-highscores/input.rs:15` |
| Filter by world | PRESENT | `modules/game_highscore/game_highscore.otui:160` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:22` → `apps/client/src/features/0009-highscores/input.rs:15`. |
| Filter by world type | PRESENT | `modules/game_highscore/game_highscore.otui:238` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:23` → `apps/client/src/features/0009-highscores/input.rs:15`. |
| Refresh rankings | PRESENT | `modules/game_highscore/game_highscore.otui:267` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:24` → `apps/client/src/features/0009-highscores/input.rs:15`. |
| Jump to first page | PRESENT | `modules/game_highscore/game_highscore.otui:434` | `apps/client/src/bevy_shell/game_ui/highscores.rs:272` → `apps/client/src/bevy_shell/game_ui/highscores.rs:113`. |
| Jump to last page | PRESENT | `modules/game_highscore/game_highscore.otui:404` | `apps/client/src/bevy_shell/game_ui/highscores.rs:272` → `apps/client/src/bevy_shell/game_ui/highscores.rs:113`. |
| Read the world’s BattlEye status | MISSING | `modules/game_highscore/game_highscore.lua:132` | Searched ranking reply, catalogue and UI for protection status. OTClient populates one server-supplied value. |
| Read ranking rows | PRESENT | `modules/game_highscore/game_highscore.otui:293` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:74` → `apps/client/src/features/0009-highscores/protocol.rs:26`. |
| Check ranking age | PRESENT | `modules/game_highscore/game_highscore.otui:414` | `apps/client/src/bevy_shell/game_ui/highscores/model.rs:104` → `apps/client/src/features/0009-highscores/protocol.rs:26`. |

## game_questlog

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Request quest list | PRESENT | `modules/game_questlog/game_questlog.lua:548` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:178` → `apps/client/src/features/0016-quest-log/input.rs:18` |
| Open quests and request mission details | PRESENT | `modules/game_questlog/game_questlog.lua:438` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:66` → `apps/client/src/features/0016-quest-log/input.rs:18` |
| Clear entire tracker | MISSING | `modules/game_questlog/game_questlog.lua:693` | Searched journal actions and quest tracker state for bulk untracking. The journal toggles a selected quest. |
| Hide completed quests | PRESENT | `modules/game_questlog/game_questlog.lua:1120` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:175` → `apps/client/src/bevy_shell/game_ui/journal/model.rs:98`. |
| Read selected mission description | PRESENT | `modules/game_questlog/game_questlog.lua:845` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:72` → `apps/client/src/features/0016-quest-log/view.rs:20`. |
| Pin a quest above the sorted list | MISSING | `modules/game_questlog/game_questlog.lua:401` | Searched quest row actions, tracker state and journal projection for pin a quest above the sorted list. |
| Hide or restore an individual quest | MISSING | `modules/game_questlog/game_questlog.lua:480` | Searched quest row actions, tracker state and journal projection for hide or restore an individual quest. |
| Show hidden quests | MISSING | `modules/game_questlog/game_questlog.lua:1124` | Searched quest row actions, tracker state and journal projection for show hidden quests. |
| Open the matching mission from the tracker | MISSING | `modules/game_questlog/game_questlog.lua:1170` | Searched quest row actions, tracker state and journal projection for open the matching mission from the tracker. |
| Create an additional tracker window | MISSING | `modules/game_questlog/game_questlog.lua:799` | Searched quest row actions, tracker state and journal projection for create an additional tracker window. |
| Remove completed entries from tracker | PARTIAL | `modules/game_questlog/game_questlog.lua:732` | Tracker reconciliation supports automatic removal, but journal actions lack a player-triggered bulk cleanup. |
| Sort completed quests first | PRESENT | `modules/game_questlog/game_questlog.html:11` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:185` → `crates/client-ui/src/features/0016-quest-log/view/list.rs:74`. |
| Sort completed quests last | PRESENT | `modules/game_questlog/game_questlog.html:12` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:185` → `crates/client-ui/src/features/0016-quest-log/view/list.rs:73`. |
| Sort quests alphabetically A–Z | MISSING | `modules/game_questlog/game_questlog.html:9` | Searched quest comparators. Ascending and descending use quest_index (crates/client-ui/src/features/0016-quest-log/view/list.rs:71). |
| Sort quests alphabetically Z–A | MISSING | `modules/game_questlog/game_questlog.html:10` | Searched quest comparators. Ascending and descending use quest_index (crates/client-ui/src/features/0016-quest-log/view/list.rs:71). |
| Track or untrack an individual mission | PARTIAL | `modules/game_questlog/game_questlog.lua:998` | The journal sends quest indexes. Toggling removes or adds the entire quest, without a mission index (crates/client-ui/src/features/0016-quest-log/input.rs:19). |
| Open a separate quest tracker | PARTIAL | `modules/game_questlog/game_questlog.lua:567` | Tracker state opens the shared journal and renders inline. It lacks an independently visible tracking panel (apps/client/src/bevy_shell/game_ui/journal.rs:173). |
| Read completed quest count | PRESENT | `modules/game_questlog/game_questlog.lua:381` | `apps/client/src/bevy_shell/game_ui/journal/model.rs:169` → `apps/client/src/bevy_shell/game_ui/journal/model.rs:170`. |
| Read hidden quest count | MISSING | `modules/game_questlog/game_questlog.lua:383` | Searched journal summary and quest state for a hidden-quest count. The summary reports completed and tracked counts. |
| Search quest list | PRESENT | `modules/game_questlog/game_questlog.lua:966` | `apps/client/src/bevy_shell/game_ui/journal/actions.rs:189` → `crates/client-ui/src/features/0016-quest-log/view/list.rs:65`. |

## game_quickloot

| Operation | Status | OTClient file:line | Oteryn evidence or what is missing |
|---|---|---|---|
| Loot nearby corpses | PRESENT | `modules/game_quickloot/quickloot.lua:34` | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:334` → `apps/client/src/features/0098-quick-loot/input.rs:3`. |
| Toggle fallback to main backpack | PRESENT | `modules/game_quickloot/quickloot.otui:131` | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:178` → `apps/client/src/features/0098-quick-loot/input.rs:3`. |
| Select accepted or skipped mode | PRESENT | `modules/game_quickloot/quickloot.otui:189` | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:246` → `apps/client/src/features/0098-quick-loot/input.rs:3`. |
| Clear the active filter list | PRESENT | `modules/game_quickloot/quickloot.otui:217` | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:256` → `apps/client/src/features/0098-quick-loot/input.rs:3`. |
| Add an item to the active list | PRESENT | `modules/game_quickloot/quickloot.otui:227` | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:273` → `apps/client/src/features/0098-quick-loot/input.rs:91`. |
| Remove an item from the active list | PRESENT | `modules/game_quickloot/quickloot.otui:30` | `apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:283` → `apps/client/src/features/0098-quick-loot/input.rs:93`. |
| Review saved loot destinations | PARTIAL | `modules/game_quickloot/quickloot.otui:113` | The panel only shows assignments confirmed during this session (apps/client/src/bevy_shell/game_ui/quick_loot/widgets.rs:81). |
| Assign a category destination | PARTIAL | `modules/game_quickloot/quickloot.otui:58` | The control supports main backpack and the form accepts container IDs, but only five categories are addressable (apps/client/src/features/0098-quick-loot/input.rs:99). |
| Unassign a category destination | PARTIAL | `modules/game_quickloot/quickloot.otui:67` | The reset sends a main-container destination with fallback enabled (apps/client/src/features/0098-quick-loot/input.rs:52). |
| Open the assigned destination | PARTIAL | `modules/game_quickloot/quickloot.lua:333` | The managed action opens the inventory container without looking up the selected category (apps/client/src/features/0098-quick-loot/input.rs:37). |
| Open the item catalogue from the loot filter | MISSING | `modules/game_quickloot/quickloot.otui:232` | Searched quick-loot widget actions for a Cyclopedia/items link. The add picker lists carried items only. |
| Open Store from quick-loot premium prompt | MISSING | `modules/game_quickloot/quickloot.otui:265` | Searched quick-loot controls for Store/premium navigation. |
| Assign a separate destination for obtained items | MISSING | `modules/game_quickloot/quickloot.otui:89` | Searched destination request and controls for obtain/secondary category mappings. The request addresses one recipient per loot category. |

## Gaps ranked by player impact

Within each group, operations share the indicated priority.

1. **Target selection, survival, spending and loot routing**
   - game_battle — Sort by Distance (asc) (missing), Sort by Distance (desc) (missing), Hide non-skull players (missing), Hide party members (missing), Cycle attack target forward or backward (missing).
   - game_healthcircle — Monitor remaining mana shield on the arc (missing).
   - game_imbuing — Imbue a blank scroll (missing), Buy protection against imbuement failure (missing), Check affordability before applying (partial).
   - game_prey — Display the gold price before paid list reroll (partial).
   - game_quickloot — Review saved loot destinations (partial), Assign a category destination (partial), Unassign a category destination (partial), Open the assigned destination (partial), Assign a separate destination for obtained items (missing).

2. **Combat awareness, equipment monitoring and progression**
   - game_battle — Hide knights (partial), Hide paladins (partial), Hide druids (partial), Hide sorcerers (partial), Hide monks (partial), Hide summons (missing), Hide Npcs (missing), Display two lists simultaneously (partial), Identify followed creature in the list (missing), Read PvP and guild marks in the list (missing), Identify a creature by battle-list portrait (missing).
   - game_healthcircle — Show experience progress arc (missing), Show skill progress arc (missing), Choose the tracked skill (missing).
   - game_creatureinformation — Read creature status icons (partial), Read own mana beneath overhead health (missing), Recognize creature type by overhead badge (partial).
   - game_imbuementtracker — Show imbuements that last less than 1h (partial), Show imbuements that last between 1h and 3h (partial), Show imbuements that last more than 3h (partial), Show items with no active imbuement (partial).
   - game_forge — Interpret the bonus on a forge result (partial).
   - game_prey — Search the creature catalogue (partial), Monitor prey in a separate tracker (partial), Identify offered prey by appearance (partial).
   - game_questlog — Clear entire tracker (missing), Pin a quest above the sorted list (missing), Show hidden quests (missing), Open the matching mission from the tracker (missing), Remove completed entries from tracker (partial), Track or untrack an individual mission (partial), Open a separate quest tracker (partial).
   - game_quickloot — Open the item catalogue from the loot filter (missing).

3. **Navigation, presentation and list management**
   - game_battle — Sort by DisplayTime (desc) (missing), Sort by Name (desc) (missing), Clear Configurations (missing), Highlight the corresponding creature on map hover from list (missing), Collapse the battle filter controls (missing).
   - game_healthcircle — Choose status bar style (partial), Position or hide the status bar (partial).
   - game_creatureinformation — Distinguish a covered creature by its grey label (missing).
   - game_forge — Page through forge history (missing).
   - game_highscore — Read the world’s BattlEye status (missing).
   - game_questlog — Hide or restore an individual quest (missing), Create an additional tracker window (missing), Sort quests alphabetically A–Z (missing), Sort quests alphabetically Z–A (missing), Read hidden quest count (missing).
   - game_quickloot — Open Store from quick-loot premium prompt (missing).
